"""Speed and autonomy numbers for the app: how long an item takes to reach main, how many arrive per hour, and
how many times the owner was needed. Built from the lane rows and ``friction.jsonl`` (never from agent reports).

``build`` gives ``{window_hours, merged, per_hour, median_seconds, avg_seconds, owner_needed, owner_per_merged}``
over the last ``WINDOW_HOURS``: a lane counts when its review status is ``merged`` with a merge time in the
window; its time to main is merge time minus ``started_at``. ``owner_needed`` counts the ``person`` rows of the
friction log in the same window. Pure function, only reads its arguments.
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone
from pathlib import Path

from tirith.home import parse_iso

WINDOW_HOURS = 24


def build(agents: list[dict], friction_rows: list[dict] | None = None, now: datetime | None = None,
          hours: float = WINDOW_HOURS) -> dict:
    now = now or datetime.now(timezone.utc)
    since = now - timedelta(hours=hours)
    times: list[float] = []
    merged = 0
    for row in agents:
        if not isinstance(row, dict) or row.get("session") == "manager" or str(row.get("id", "")).startswith("manager"):
            continue
        review = row.get("review") if isinstance(row.get("review"), dict) else {}
        if review.get("status") != "merged":
            continue
        at = parse_iso(review.get("at"))
        if at is None or at < since or at > now:
            continue
        merged += 1
        started = parse_iso(row.get("started_at"))
        if started is not None and at >= started:
            times.append((at - started).total_seconds())
    owner = 0
    for r in friction_rows or []:
        at = parse_iso(r.get("at")) if isinstance(r, dict) else None
        if isinstance(r, dict) and r.get("kind") == "person" and at is not None and since <= at <= now:
            owner += 1
    times.sort()
    mid = len(times) // 2
    median = None if not times else (times[mid] if len(times) % 2 else (times[mid - 1] + times[mid]) / 2)
    return {"window_hours": hours, "merged": merged, "per_hour": round(merged / hours, 2),
            "median_seconds": None if median is None else round(median),
            "avg_seconds": round(sum(times) / len(times)) if times else None,
            "owner_needed": owner, "owner_per_merged": round(owner / merged, 2) if merged else None}


def load(home: Path, agents: list[dict], now: datetime | None = None) -> dict:
    from tirith import friction
    from tirith.store import read_jsonl
    return build(agents, read_jsonl(Path(home) / friction.FILE), now)
