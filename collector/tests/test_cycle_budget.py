"""A collector cycle stays fast on a realistic home: many lanes, logs, waits and worktrees."""

from __future__ import annotations

import json
import subprocess
import time
from pathlib import Path

import pytest

from conftest import make_repo, write_config
from tirith import activity as activity_mod
from tirith import collector as collector_mod
from tirith import diskdiet as diskdiet_mod
from tirith.collector import Collector
from tirith.store import append_jsonl, write_lane

LANES = 150
LIVE = 6
WORKTREES = 8
BUDGET_S = 4.0  # the real aim is under 3 s on the owner's machine; this bound only has to catch a regression


def _log(path: Path, turns: int = 120) -> None:
    rows = []
    for n in range(turns):
        rows.append({"type": "assistant", "message": {"content": [
            {"type": "text", "text": f"step {n}: looking at the code"},
            {"type": "tool_use", "id": f"t{n}", "name": "Bash", "input": {"command": f"pytest -q tests/test_{n}.py"}}]}})
        rows.append({"type": "user", "message": {"content": [
            {"type": "tool_result", "tool_use_id": f"t{n}", "content": "ok\n" * 6}]}})
    rows.append({"type": "result", "total_cost_usd": 1.5, "duration_ms": 90000})
    path.write_text("\n".join(json.dumps(r) for r in rows) + "\n", encoding="utf-8")


@pytest.fixture
def busy_home(home: Path, tmp_path: Path) -> Path:
    repo = make_repo(tmp_path / "demo")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}])
    start = subprocess.run(["git", "-C", str(repo), "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
    (home / "logs").mkdir(exist_ok=True)
    for n in range(LANES):
        workspace = repo
        if n < WORKTREES:
            workspace = tmp_path / "demo-lanes" / f"lane-{n}"
            subprocess.run(["git", "-C", str(repo), "worktree", "add", "-q", "-b", f"lane/lane-{n}", str(workspace)],
                           check=True)
        log = home / "logs" / f"lane-{n}.log"
        _log(log)
        lane = {"id": f"lane-{n}", "parent": "steward", "project": "demo", "class": "stonewright",
                "feature": f"Feature {n}", "workspace": str(workspace), "log": str(log), "pid": None,
                "brief": None, "started_at": "2999-01-01T00:00:00Z", "start_commit": start}
        if n >= LIVE:
            lane.update(ended_at="2999-01-01T00:10:00Z", result="done", report="All done.\n\nDetails follow.")
        else:
            lane["pid"] = 1  # launchd: alive
        write_lane(home, lane)
    for n in range(60):
        append_jsonl(home / "waits.jsonl", {"id": f"w{n}", "project": "demo", "text": f"question {n}",
                                           "at": "2999-01-01T00:00:00Z"})
    return home


@pytest.mark.slow
def test_a_cycle_on_a_busy_home_stays_within_the_budget(busy_home: Path) -> None:
    collector = Collector(busy_home, client=None)
    collector.cycle()  # cold: every log parsed once
    while collector.cycle()["collector"]["warming"]:  # every lane's files read once, over a few cycles
        pass
    spent = []
    for _ in range(3):
        started = time.monotonic()
        collector.cycle()
        spent.append(time.monotonic() - started)
    assert max(spent) < BUDGET_S, f"cycles took {spent}"


@pytest.mark.slow
def test_a_warm_cycle_runs_no_git_for_lanes_it_already_read(busy_home: Path, monkeypatch) -> None:
    monkeypatch.setattr(activity_mod, "FILES_TTL_S", 3600)
    collector = Collector(busy_home, client=None)
    while collector.cycle()["collector"]["warming"]:
        pass
    calls: list[list[str]] = []
    real = subprocess.run

    def spy(cmd, *args, **kwargs):
        calls.append(list(cmd))
        return real(cmd, *args, **kwargs)

    monkeypatch.setattr(activity_mod.subprocess, "run", spy)
    monkeypatch.setattr(diskdiet_mod.subprocess, "run", spy)
    collector.cycle()
    assert [c for c in calls if "diff" in c or "ls-files" in c] == []
    assert [c for c in calls if "worktree" in c or "merge-base" in c] == []


@pytest.mark.slow
def test_the_log_caches_hold_every_lane(busy_home: Path) -> None:
    from tirith import livesteps, logview
    Collector(busy_home, client=None).cycle()
    assert len(logview._cache) >= LANES and len(livesteps._cache) >= LANES


@pytest.mark.slow
def test_the_collector_logs_a_phase_timing_line_every_period(busy_home: Path, monkeypatch, capsys) -> None:
    monkeypatch.setattr(collector_mod, "TIMING_EVERY", 3)
    collector = Collector(busy_home, client=None)
    for _ in range(3):
        collector.cycle()
    err = capsys.readouterr().err
    line = next(l for l in err.splitlines() if "mean per cycle" in l)
    for phase in ("lanes=", "pipelines=", "inbox=", "slim=", "write=", "cycle="):
        assert phase in line
