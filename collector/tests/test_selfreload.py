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
