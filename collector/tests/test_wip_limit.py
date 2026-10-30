"""WIP limit: with 3 or more lanes waiting in the merge queue no new lane starts; started and sent-back ones run."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from test_launch import PLAN, project  # noqa: F401  (fixture)
from tirith import launch as launch_mod
from tirith import mergeq as mergeq_mod
from tirith.cli import launch_plan, launch_step, launch_verify


def queue(home: Path, n: int, status: str = "queued") -> None:
    rows = [{"lane": f"lane-q{i}", "project": "demo", "status": status} for i in range(n)]
    (home / "mergeq.json").write_text(json.dumps({"queue": rows}), encoding="utf-8")


def test_wip_limit_blocks_every_launch_kind(home: Path, project) -> None:  # noqa: F811
    _, started = project
    queue(home, mergeq_mod.WIP_LIMIT)
    with pytest.raises(launch_mod.LaunchError, match="3 lanes are waiting in the merge queue"):
        launch_step(home, "demo", "2", item="Add the button.")
    with pytest.raises(launch_mod.LaunchError, match="stays open"):
        launch_plan(home, "demo", "plan-refresh")
    with pytest.raises(launch_mod.LaunchError, match="waiting"):
        launch_verify(home, "demo", "something")
    assert started == []


def test_wip_limit_counts_running_and_allows_below(home: Path, project) -> None:  # noqa: F811
    _, started = project
    queue(home, mergeq_mod.WIP_LIMIT, "running")
    assert mergeq_mod.wip_block(home)
    queue(home, mergeq_mod.WIP_LIMIT - 1)
    assert mergeq_mod.wip_block(home) == ""
    launch_step(home, "demo", "2", item="Add the button.")
    assert len(started) == 1


def test_wip_limit_does_not_stop_a_sent_back_lane(home: Path, project) -> None:  # noqa: F811
    from tirith import cli
    queue(home, 5)
    src = inspect_source(cli.relaunch_lane)
    assert "wip_block" not in src


def inspect_source(fn) -> str:
    import inspect
    return inspect.getsource(fn)
