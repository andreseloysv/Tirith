"""After a start the collector writes a useful state at once; the heavy per-lane work follows in later cycles."""

from __future__ import annotations

import json
import subprocess
import time
from pathlib import Path

import pytest

from conftest import make_repo, write_config
from tirith import activity as activity_mod
from tirith.collector import Collector
from tirith.store import read_json, write_lane

LANES = 300
LIVE = 4
FIRST_CYCLE_S = 30.0  # generous for CI; a start used to take minutes on a real home


@pytest.fixture
def big_home(home: Path, tmp_path: Path) -> Path:
    repo = make_repo(tmp_path / "demo")
    write_config(home, [dict(id="demo", name="Demo", repo=str(repo))])
    start = subprocess.run(["git", "-C", str(repo), "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
    (home / "logs").mkdir(exist_ok=True)
    for n in range(LANES):
        log = home / "logs" / f"lane-{n}.log"
        log.write_text(json.dumps(dict(type="result", total_cost_usd=1.0, duration_ms=1000)) + "\n", encoding="utf-8")
        lane = dict(id=f"lane-{n}", parent="steward", project="demo", feature=f"Feature {n}",
                    workspace=str(repo), log=str(log), pid=None, brief=None,
                    started_at="2999-01-01T00:00:00Z", start_commit=start)
        lane["class"] = "stonewright"
        if n >= LIVE:
            lane.update(ended_at="2999-01-01T00:10:00Z", result="done", report="All done.")
        else:
            lane["pid"] = 1
        write_lane(home, lane)
    return home


def _count_git(monkeypatch) -> list[list[str]]:
    calls: list[list[str]] = []
    real = subprocess.run

    def spy(cmd, *args, **kwargs):
        calls.append(list(cmd))
        return real(cmd, *args, **kwargs)

    monkeypatch.setattr(activity_mod.subprocess, "run", spy)
    return calls


def test_first_cycle_is_fast_then_the_full_pass_completes(big_home: Path, monkeypatch) -> None:
    monkeypatch.setattr(activity_mod, "WARM_BUDGET_S", 0.5)
    collector = Collector(big_home, client=None)
    started = time.monotonic()
    state = collector.cycle()
    assert time.monotonic() - started < FIRST_CYCLE_S
    assert (big_home / "state.json").exists()
    assert state["agents"]  # slimmed to what the app shows
    assert state["collector"]["warming"] is True and state["collector"]["pending"] > 0
    for _ in range(300):
        state = collector.cycle()
        if not state["collector"]["warming"]:
            break
    assert state["collector"] == dict(warming=False, pending=0)


def test_a_restart_reuses_the_kept_file_lists(big_home: Path, monkeypatch) -> None:
    first = Collector(big_home, client=None)
    for _ in range(300):
        if not first.cycle()["collector"]["warming"]:
            break
    assert len(read_json(big_home / activity_mod.FILES_CACHE, {})) >= LANES - LIVE
    calls = _count_git(monkeypatch)
    second = Collector(big_home, client=None)
    started = time.monotonic()
    state = second.cycle()
    assert time.monotonic() - started < FIRST_CYCLE_S
    assert state["collector"]["warming"] is False
    asked = [c for c in calls if "diff" in c or "ls-files" in c]
    assert len(asked) <= 3 * LIVE  # only the working lanes ask git
