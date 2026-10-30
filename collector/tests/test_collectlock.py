"""``tirith collect`` holds <TIRITH_HOME>/collector.lock; a second one says so and exits 0."""

from __future__ import annotations

import os
import subprocess
import sys
import time
from pathlib import Path

import pytest

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
