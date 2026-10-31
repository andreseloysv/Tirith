"""A collector cycle stays fast on a realistic home: many lanes, logs, waits and worktrees."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

import pytest

from conftest import cpu_seconds, make_repo, write_config
from tirith import activity as activity_mod
from tirith import collector as collector_mod
from tirith import diskdiet as diskdiet_mod
from tirith.collector import Collector
from tirith.store import append_jsonl, write_lane

LANES = 150
LIVE = 6
WORKTREES = 8
BUDGET_S = 4.0  # CPU seconds, so load does not fail it; the aim is under 3 s on the owner's machine; this only has to catch a regression


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
        started = cpu_seconds()
        collector.cycle()
        spent.append(cpu_seconds() - started)
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


ENDED = 400
IN_REVIEW = 20


@pytest.fixture
def ended_home(home: Path, tmp_path: Path, monkeypatch) -> tuple[Path, list[dict]]:
    """400 lanes that ended days ago; 20 of them still wait in the plan as "(en revisión)"."""
    from conftest import write_plan
    from tirith.cli import register_lane
    from tirith.store import read_lanes
    repo = make_repo(tmp_path / "demo")
    items = [f"Item {n}. (en revisión)" if n < IN_REVIEW else f"Item {n}." for n in range(ENDED)]
    write_plan(repo, "# TODO\n\n## 1. Work\n\n" + "".join(
        f"- [{' ' if n < IN_REVIEW else 'x'}] {text}\n" for n, text in enumerate(items)))
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}])
    for n in range(ENDED):
        lane = register_lane(home, lane_id=f"old-{n}", project="demo", hero_class="warden", feature="Work",
                             workspace=str(repo), log=None, pid=None, brief=None, step="1", item=f"Item {n}.")
        lane.update(ended_at="2026-01-01T00:00:00Z", result="done", outcome={"status": "done", "applied": True})
        write_lane(home, lane)
    return home, read_lanes(home)


def _count_git(monkeypatch) -> list[list[str]]:
    calls: list[list[str]] = []
    real = subprocess.Popen

    class Spy(real):
        def __init__(self, argv, *args, **kwargs):
            if isinstance(argv, (list, tuple)) and argv and Path(str(argv[0])).name == "git":
                calls.append(list(argv))
            super().__init__(argv, *args, **kwargs)

    monkeypatch.setattr(subprocess, "Popen", Spy)
    return calls


def test_cycle_budget_ended_lanes_cost_a_bounded_number_of_git_calls(ended_home, monkeypatch) -> None:
    from datetime import datetime, timezone
    from tirith import autoreview, mergeq, plangate
    home, lanes = ended_home
    config = collector_mod.load_config(home)
    monkeypatch.setattr(plangate, "TICK_EVERY_S", 60.0)
    plangate._ticked_at.clear()
    nothing = lambda *a, **k: None  # noqa: E731
    calls = _count_git(monkeypatch)

    def one_cycle() -> int:
        before = len(calls)
        plangate.sync(home, config, lanes, nothing)
        mergeq.run(home, config, lanes, spawn=nothing, dry_run=True)
        autoreview.run(home, config, datetime.now(timezone.utc), {}, lanes, spawn=nothing, notify=nothing)
        return len(calls) - before

    first = one_cycle()
    assert first <= 4 * IN_REVIEW  # only the lanes still in review ask git, never the 380 settled ones
    assert one_cycle() <= 2  # the same lanes a minute later are not asked again


def test_cycle_budget_merge_runner_stops_after_an_entry_when_the_code_changed(home: Path, monkeypatch) -> None:
    from tirith import mergeq
    rows = [{"lane": f"l{n}", "result": mergeq.MERGED} for n in range(3)]
    monkeypatch.setattr(mergeq, "process", lambda *a, **k: [rows.pop(0)] if rows else [])

    class Code:
        def __init__(self, changed: bool) -> None:
            self.answer = changed

        def changed(self) -> bool:
            return self.answer

    assert [r["lane"] for r in mergeq.drain(home, None, watch=Code(False))] == ["l0", "l1", "l2"]
    rows[:] = [{"lane": f"l{n}", "result": mergeq.MERGED} for n in range(3)]
    assert [r["lane"] for r in mergeq.drain(home, None, watch=Code(True))] == ["l0"]  # one pass, then a fresh runner
    assert len(rows) == 2
