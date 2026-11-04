"""The lane scoreboard: first-try rate, rounds, merges, cost and common gaps per group (tirith.scoreboard)."""

from __future__ import annotations

import json
from datetime import datetime, timedelta, timezone

from tirith import scoreboard
from tirith.cli import main
from tirith.store import write_lane

NOW = datetime(2026, 11, 4, 12, 0, tzinfo=timezone.utc)


def _lane(lane_id: str, **extra) -> dict:
    return {"id": lane_id, "project": "demo", "class": "builder", "agent": "anthropic",
            "ran": {"model": "sonnet"}, "item": "Do it", "item_step": "1", "result": "done",
            "started_at": "2026-11-04T10:00:00Z", "ended_at": "2026-11-04T11:00:00Z", **extra}


LANES = [
    _lane("clean", outcome={"status": "done"}, review={"status": "merged"}),
    _lane("twice", outcome={"status": "done"}, review={"status": "conflict"},
          keep_going={"round": 2, "missing": ["claim check: said 3 passed", "uncommitted changes in your worktree: x"]}),
    _lane("stuck", result="failed", keep_going={"round": 1, "stalled": True,
                                                "missing": ["claim check: said merged",
                                                            "branch lane/stuck does not merge cleanly with main: m"]},
          repeats={"nudged": 2, "blocked": 1, "longest": 8}),
    _lane("other", ran={"model": "large"}, outcome={"status": "done"}, review={"status": "merged"}),
    _lane("planner", item=None, item_step=None, ran={}, agent=None),
]
COST = {"clean": 2.0, "twice": 5.0, "stuck": 3.0, "other": 9.0}


def test_groups_by_agent_and_model_with_rates_and_cost() -> None:
    board = scoreboard.score(LANES, "agent", cost=lambda lane: COST.get(lane["id"]))
    rows = {r["group"]: r for r in board["rows"]}
    sonnet = rows["anthropic/sonnet"]
    assert sonnet["lanes"] == 3 and sonnet["first_try"] == 1 and sonnet["first_try_pct"] == 33
    assert sonnet["avg_rounds"] == 1.0 and sonnet["stalled"] == 1 and sonnet["failed"] == 1
    assert sonnet["merged"] == 1 and sonnet["merge_trouble"] == 1 and sonnet["repeat_blocked"] == 1
    assert sonnet["cost_usd"] == 10.0 and sonnet["usd_per_merged"] == 10.0
    assert rows["anthropic/large"]["usd_per_merged"] == 9.0
    planner = rows["unnamed/default"]
    assert planner["first_try"] == 1 and planner["cost_usd"] is None and planner["usd_per_merged"] is None


def test_gaps_are_counted_by_kind_once_per_lane() -> None:
    board = scoreboard.score(LANES, "project", cost=lambda lane: None)
    assert board["gaps"][0] == ("claim check", 2)
    assert dict(board["gaps"]) == {"claim check": 2, "uncommitted changes in your worktree": 1,
                                   "branch does not merge cleanly": 1}
    assert [r["group"] for r in board["rows"]] == ["demo"]


def test_only_ended_agent_lanes_of_the_window_are_scored(home) -> None:
    old = (NOW - timedelta(days=30)).strftime("%Y-%m-%dT%H:%M:%SZ")
    for lane in [_lane("recent"), _lane("old", ended_at=old), _lane("running", ended_at=None),
                 _lane("manager-1"), _lane("mine", interactive=True)]:
        write_lane(home, lane)
    assert [l["id"] for l in scoreboard.ended_lanes(home, 14, NOW)] == ["recent"]


def test_lane_stats_command_prints_the_table_and_json(home, capsys) -> None:
    write_lane(home, _lane("clean", outcome={"status": "done"}, review={"status": "merged"},
                           ended_at=datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")))
    assert main(["lane", "stats"]) == 0
    out = capsys.readouterr().out
    assert "1 ended agent lanes" in out and "anthropic/sonnet" in out and "100%" in out
    assert main(["lane", "stats", "--by", "model", "--json"]) == 0
    board = json.loads(capsys.readouterr().out)
    assert board["rows"][0]["group"] == "sonnet" and board["rows"][0]["merged"] == 1


def test_no_lanes_says_so(home, capsys) -> None:
    assert main(["lane", "stats", "--days", "3"]) == 0
    assert "No ended agent lanes in the last 3 days." in capsys.readouterr().out
