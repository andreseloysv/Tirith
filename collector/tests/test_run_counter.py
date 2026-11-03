"""`tirith run --new-run` started at the same moment gets distinct run numbers."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

from tirith import runstage
from tirith.store import read_json

CODE = "import sys; from pathlib import Path; from tirith import runstage; " \
       "print(runstage.resolve_run(Path(sys.argv[1]), 'p', 'ci', None, True))"


def test_parallel_new_runs_get_distinct_numbers(tmp_path: Path) -> None:
    home = tmp_path / "home"
    home.mkdir()
    root = Path(__file__).resolve().parents[1]
    procs = [subprocess.Popen([sys.executable, "-c", CODE, str(home)], cwd=root, stdout=subprocess.PIPE, text=True)
             for _ in range(12)]
    runs = [p.communicate()[0].strip() for p in procs]
    assert sorted(runs, key=lambda r: int(r.split("#")[1])) == [f"ci#{n}" for n in range(1, 13)]
    assert read_json(home / "pipelines.json", {})["p/ci"]["counter"] == 12
    assert runstage.resolve_run(home, "p", "ci", None, False) == "ci#12"


CODE_STAGE = ("import sys; from pathlib import Path; from tirith import runstage; "
              "code, _ = runstage.run_stage(Path(sys.argv[1]), 'p', 'ci', 'test', "
              "['/bin/sh', '-c', 'sleep 0.4; echo ' + sys.argv[2]], sys.argv[3], worktree=sys.argv[3], echo=False); "
              "import json; from tirith.store import read_jsonl; "
              "rows = [r for r in read_jsonl(Path(sys.argv[1]) / 'events.jsonl') if r.get('worktree') == sys.argv[3]]; "
              "print(rows[0]['run'], rows[0]['log'])")


def test_concurrent_runs_separate_logs(tmp_path: Path) -> None:
    """Two stage runs at once from different worktrees never share a run number or a log file."""
    home = tmp_path / "home"
    home.mkdir()
    root = Path(__file__).resolve().parents[1]
    trees = []
    for name in ("a", "b", "c", "d"):
        (tmp_path / name).mkdir()
        trees.append(str(tmp_path / name))
    procs = [subprocess.Popen([sys.executable, "-c", CODE_STAGE, str(home), Path(t).name, t], cwd=root,
                              stdout=subprocess.PIPE, text=True) for t in trees]
    seen = [p.communicate()[0].split() for p in procs]
    assert len({run for run, _ in seen}) == 4, seen
    assert len({log for _, log in seen}) == 4, seen
    for (run, log), tree in zip(seen, trees):
        assert Path(log).read_text().strip() == Path(tree).name  # only its own output in its own log


def test_concurrent_runs_separate_logs_same_worktree_keeps_its_run(tmp_path: Path) -> None:
    """Stages of one worktree after each other keep sharing the run (build then test are one run)."""
    home = tmp_path / "home"
    home.mkdir()
    first = runstage.resolve_run(home, "p", "ci", None, False, "/w/a")
    assert runstage.resolve_run(home, "p", "ci", None, False, "/w/a") == first
    other = runstage.resolve_run(home, "p", "ci", None, False, "/w/b")
    assert other != first
    assert runstage.resolve_run(home, "p", "ci", None, False, "/w/a") not in (first, other)
