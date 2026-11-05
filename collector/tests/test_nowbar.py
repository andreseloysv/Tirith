"""The now bar: merged today, in progress, stuck, and who has each (tirith.nowbar)."""

from __future__ import annotations

from datetime import datetime, timezone

from tirith import nowbar

NOW = datetime(2026, 11, 4, 12, 0, tzinfo=timezone.utc)


def _row(lane_id: str, **extra) -> dict:
    return {"id": lane_id, "project": "demo", "agent": "anthropic", "name": lane_id, "item": f"item {lane_id}",
            "status": "done", "started_at": "2026-11-04T10:00:00Z", "ended_at": "2026-11-04T11:00:00Z", **extra}


def test_now_bar_groups_lanes_and_names_the_agent_alias() -> None:
    rows = [
        _row("shipped", review={"status": "merged", "at": "2026-11-04T11:30:00Z"}),
        _row("old", review={"status": "merged", "at": "2026-10-01T11:30:00Z"}),
        _row("busy", status="working", ended_at=None, agent="muse"),
        _row("quiet", status="stalled", ended_at=None),
        _row("clash", review={"status": "conflict", "message": "conflict in a.py"}),
        _row("manager-1", status="working", ended_at=None),
    ]
    bar = nowbar.build(rows, NOW)
    assert [i["lane"] for i in bar["merged"]] == ["shipped"]
    assert [(i["lane"], i["agent"]) for i in bar["doing"]] == [("busy", "muse")]
    assert {i["lane"] for i in bar["stuck"]} == {"quiet", "clash"}
    assert next(i for i in bar["stuck"] if i["lane"] == "clash")["why"] == "conflict in a.py"
    assert bar["counts"] == {"merged": 1, "doing": 1, "stuck": 2}


def test_now_bar_empty() -> None:
    assert nowbar.build([], NOW)["counts"] == {"merged": 0, "doing": 0, "stuck": 0}
