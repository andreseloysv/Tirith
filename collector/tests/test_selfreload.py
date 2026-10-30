import sys

import pytest

from tirith import selfreload
from tirith.collector import Collector
from tirith.selfreload import CodeWatch, ReloadRequested


class Clock:
    t = 100.0

    def __call__(self):
        return self.t


def _pkg(tmp_path):
    pkg = tmp_path / "tirith"
    pkg.mkdir()
    (pkg / "a.py").write_text("x = 1\n")
    (tmp_path / "pyproject.toml").write_text("[project]\n")
    return pkg


def test_unchanged_code_never_reloads(tmp_path):
    clock = Clock()
    watch = CodeWatch(_pkg(tmp_path), clock=clock)
    clock.t += 60
    assert not watch.changed()


def test_change_waits_until_stable(tmp_path):
    pkg, clock = _pkg(tmp_path), Clock()
    watch = CodeWatch(pkg, clock=clock)
    (pkg / "a.py").write_text("x = 22\n")
    assert not watch.changed()  # just seen
    clock.t += 3
    (pkg / "b.py").write_text("y = 1\n")  # the merge is still writing: timer restarts
    assert not watch.changed()
    clock.t += 3
    assert not watch.changed()
    clock.t += 3
    assert watch.changed()


def test_pyproject_counts(tmp_path):
    pkg, clock = _pkg(tmp_path), Clock()
    watch = CodeWatch(pkg, clock=clock)
    (tmp_path / "pyproject.toml").write_text("[project]\nname='x'\n")
    watch.changed()
    clock.t += 6
    assert watch.changed()


def test_reexec_uses_fake_exec(monkeypatch, capsys):
    monkeypatch.setattr(sys, "orig_argv", [sys.executable, "-m", "tirith", "collect"], raising=False)
    calls = []
    selfreload.reexec(lambda path, argv: calls.append((path, argv)))
    assert calls == [(sys.executable, [sys.executable, "-m", "tirith", "collect"])]
    assert "code changed; reloading" in capsys.readouterr().err


def test_run_stops_between_cycles_when_reload_requested(tmp_path, monkeypatch):
    collector = Collector(tmp_path)
    cycles = []
    monkeypatch.setattr(collector, "cycle", lambda: cycles.append(1) or {})
    monkeypatch.setattr("tirith.collector.time.sleep", lambda s: None)
    answers = iter([False, True])
    with pytest.raises(ReloadRequested):
        collector.run(interval=0.2, reload_check=lambda: next(answers))
    assert len(cycles) == 2  # first run, then one more before the reload


def test_reload_with_busy_background_pool_does_not_hang(tmp_path, monkeypatch):
    import threading
    import time

    from tirith.activity import ActivityTracker

    collector = Collector(tmp_path)
    release = threading.Event()
    monkeypatch.setattr("tirith.activity.git_files", lambda ws, start: release.wait(10) and ([], (0, 0, 0)))
    lane = {"id": "l1", "workspace": str(tmp_path), "start_commit": None}
    collector.activity._live_files["l1"] = (time.monotonic() - 3600, ([], (0, 0, 0)))
    collector.activity.files(lane, ended=False)  # starts the busy background refresh
    collector.worker.start()
    collector.rooms.poller.start()
    calls = []
    started = time.monotonic()
    selfreload.reexec(lambda path, argv: calls.append(argv), quiesce=lambda: collector.quiesce(0.5))
    release.set()
    assert calls and time.monotonic() - started < 3  # exec reached although the pool was busy
    assert collector.worker._thread is not None and not collector.worker._thread.is_alive()
    assert not collector.rooms.poller._thread.is_alive()


def test_quiesce_reports_clean_when_idle(tmp_path):
    collector = Collector(tmp_path)
    assert collector.quiesce(0.5)
