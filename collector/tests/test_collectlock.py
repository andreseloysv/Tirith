"""``tirith collect`` holds <TIRITH_HOME>/collector.lock; a second one says so and exits 0."""

from __future__ import annotations

import os
import subprocess
import sys
import time
from pathlib import Path

import pytest

from tirith import collectlock
from tirith.collectlock import CollectorRunning, hold_collector_lock


def _collect(home: Path, *extra: str) -> subprocess.Popen:
    env = {**os.environ, "TIRITH_HOME": str(home)}
    return subprocess.Popen(
        [sys.executable, "-m", "tirith", "collect", "--interval", "0.2", *extra],
        env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)


def test_second_collector_refuses_while_first_holds_lock(home: Path) -> None:
    first = _collect(home)
    try:
        lock = home / "collector.lock"
        deadline = time.time() + 90  # a loaded host can take long to start the subprocess
        while time.time() < deadline and not (lock.exists() and lock.read_text().strip()):
            time.sleep(0.1)
        assert lock.read_text().strip() == str(first.pid)
        second = _collect(home)
        out, _ = second.communicate(timeout=30)
        assert second.returncode == 0
        assert out.strip() == f"a collector is already running, pid {first.pid}"
    finally:
        first.kill()
        first.communicate()


def test_lock_is_free_after_holder_ends(home: Path) -> None:
    with hold_collector_lock(home):
        with pytest.raises(CollectorRunning) as info:
            with hold_collector_lock(home):
                pass
        assert info.value.pid == os.getpid()
    with hold_collector_lock(home):  # released: a new collector may start
        pass


def test_once_does_not_take_the_lock(home: Path) -> None:
    with hold_collector_lock(home):
        done = _collect(home, "--once")
        done.communicate(timeout=60)
        assert done.returncode == 0


def test_sigterm_ends_collector_and_frees_lock(home: Path) -> None:
    import signal

    proc = _collect(home)
    try:
        lock = home / "collector.lock"
        deadline = time.time() + 90  # a loaded host can take long to start the subprocess
        while time.time() < deadline and not (lock.exists() and lock.read_text().strip()):
            time.sleep(0.1)
        assert lock.read_text().strip() == str(proc.pid)
        proc.send_signal(signal.SIGTERM)
        proc.communicate(timeout=30)
        assert proc.returncode == 0
        with hold_collector_lock(home):  # free again
            pass
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.communicate()


def test_second_sigterm_is_silent(home: Path) -> None:
    import signal

    proc = _collect(home)
    try:
        lock = home / "collector.lock"
        deadline = time.time() + 90  # a loaded host can take long to start the subprocess
        while time.time() < deadline and not (lock.exists() and lock.read_text().strip()):
            time.sleep(0.1)
        assert lock.read_text().strip() == str(proc.pid)
        proc.send_signal(signal.SIGTERM)
        proc.send_signal(signal.SIGTERM)  # uv forwards a second one during shutdown
        _, err = proc.communicate(timeout=30)
        assert proc.returncode == 0
        assert "Exception ignored" not in err and "Traceback" not in err
        with hold_collector_lock(home):
            pass
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.communicate()


def test_running_pid_only_tries_the_lock(home: Path) -> None:
    from tirith.collectlock import running_pid
    assert running_pid(home) is None  # no lock file yet
    with hold_collector_lock(home):
        assert running_pid(home) == os.getpid()
        assert running_pid(home) == os.getpid()  # checking does not take or release it
    assert running_pid(home) is None


def test_a_second_collector_refuses_before_its_expensive_start_up(home: Path, monkeypatch, capsys) -> None:
    from tirith import cli
    def boom(*a, **k):
        raise AssertionError("the login shell must not be asked while another collector runs")
    monkeypatch.setattr(cli.ownerenv_mod, "load", boom)
    monkeypatch.setenv("TIRITH_HOME", str(home))
    with hold_collector_lock(home):
        assert cli.main(["collect", "--interval", "2"]) == 0
    assert capsys.readouterr().out.strip() == f"a collector is already running, pid {os.getpid()}"


class _Launchctl:
    def __init__(self, loaded: bool) -> None:
        self.loaded, self.calls = loaded, []

    def __call__(self, argv, **kw):
        self.calls.append(argv[1])
        code = 0 if (argv[1] == "bootstrap" or self.loaded) else 113
        return subprocess.CompletedProcess(argv, code, b"", b"")


def test_the_launchd_fallback_is_loaded_again_when_missing(home: Path, monkeypatch) -> None:
    from tirith import collectlock
    monkeypatch.setattr(collectlock.sys, "platform", "darwin")
    assert collectlock.ensure_launchd(home, run=_Launchctl(False)) == "none"  # the owner did not set it up
    (home / "launchd").mkdir()
    (home / collectlock.LAUNCHD_PLIST).write_text("<plist/>", encoding="utf-8")
    missing = _Launchctl(False)
    assert collectlock.ensure_launchd(home, run=missing) == "loaded" and missing.calls == ["print", "bootstrap"]
    there = _Launchctl(True)
    assert collectlock.ensure_launchd(home, run=there) == "present" and there.calls == ["print"]
    monkeypatch.setattr(collectlock.sys, "platform", "linux")
    assert collectlock.ensure_launchd(home, run=_Launchctl(False)) == "none"


def test_revive_brings_a_dead_collector_back_once(tmp_path: Path, monkeypatch) -> None:
    calls: list[list[str]] = []

    def run(argv, **kw):
        calls.append(argv)
        return subprocess.CompletedProcess(argv, 1 if argv[1] == "print" else 0)

    monkeypatch.setattr(collectlock.sys, "platform", "darwin")
    home = tmp_path / "home"
    (home / "launchd").mkdir(parents=True)
    (home / collectlock.LAUNCHD_PLIST).write_text("<plist/>")
    state = home / "state.json"
    state.write_text("{}")
    now = state.stat().st_mtime
    assert collectlock.revive(home, now + 10, run=run) == "alive" and calls == []
    assert collectlock.revive(home, now + collectlock.REVIVE_STALE_S + 1, run=run) == "loaded"
    assert [c[1] for c in calls] == ["print", "bootstrap"]
    assert collectlock.revive(home, now + collectlock.REVIVE_STALE_S + 60, run=run) == "recent"  # once per while
    with collectlock.hold_collector_lock(home):
        assert collectlock.revive(home, now + 2 * collectlock.REVIVE_STALE_S + 120, run=run) == "running"


def test_revive_starts_a_loaded_job_that_is_not_running(tmp_path: Path, monkeypatch) -> None:
    calls: list[list[str]] = []
    monkeypatch.setattr(collectlock.sys, "platform", "darwin")

    def run(argv, **kw):
        calls.append(argv)
        return subprocess.CompletedProcess(argv, 0)
    home = tmp_path / "home"
    (home / "launchd").mkdir(parents=True)
    (home / collectlock.LAUNCHD_PLIST).write_text("<plist/>")
    (home / "state.json").write_text("{}")
    assert collectlock.revive(home, (home / "state.json").stat().st_mtime + 900, run=run) == "kicked"
    assert calls[-1][1] == "kickstart"


def test_revive_without_a_state_file_does_nothing(tmp_path: Path) -> None:
    assert collectlock.revive(tmp_path) == "no-state"
