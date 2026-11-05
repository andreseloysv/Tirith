"""Metrics: time to main, merges per hour and times the owner was needed (``tirith.metrics``)."""

from __future__ import annotations

from datetime import datetime, timedelta, timezone
from pathlib import Path

from tirith import friction, metrics

NOW = datetime(2026, 1, 10, 12, 0, tzinfo=timezone.utc)


def _iso(delta_min: float) -> str:
    return (NOW + timedelta(minutes=delta_min)).isoformat()


def _lane(i: str, start: float, merged: float | None, status: str = "merged") -> dict:
    review = {"status": status, "at": _iso(merged)} if merged is not None else {}
    return {"id": i, "started_at": _iso(start), "review": review}


def test_metrics_time_to_main_and_rate():
    agents = [_lane("a", -120, -60), _lane("b", -300, -240), _lane("c", -30, -10),
              _lane("old", -3000, -2900), _lane("x", -50, -5, "conflict"), _lane("manager1", -90, -80)]
    m = metrics.build(agents, [], NOW)
    assert m["merged"] == 3 and m["per_hour"] == round(3 / 24, 2)
    assert m["median_seconds"] == 3600 and m["avg_seconds"] == round((3600 + 3600 + 1200) / 3)
    assert m["owner_needed"] == 0 and m["owner_per_merged"] == 0.0


def test_metrics_owner_needed_counts_person_rows_in_window():
    rows = [{"at": _iso(-10), "kind": "person"}, {"at": _iso(-20), "kind": "wait"},
            {"at": _iso(-60 * 30), "kind": "person"}, {"at": _iso(-5), "kind": "person"}]
    m = metrics.build([_lane("a", -60, -10)], rows, NOW)
    assert m["owner_needed"] == 2 and m["owner_per_merged"] == 2.0


def test_metrics_empty():
    m = metrics.build([], None, NOW)
    assert m["merged"] == 0 and m["median_seconds"] is None and m["owner_per_merged"] is None


def test_metrics_load_reads_friction_log(home: Path):
    friction.record(home, "demo", friction.PERSON, "needs the key")
    m = metrics.load(home, [], datetime.now(timezone.utc) + timedelta(minutes=1))
    assert m["owner_needed"] == 1
