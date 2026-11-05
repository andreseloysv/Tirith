"""The "now" bar at the top of the app: what merged into main today, what is in progress, what is stuck, and who has it.

Built from the agent rows of ``state.json`` (never from what agents report on their own): a lane whose review
status is ``merged`` and whose merge time is today is *merged*; a lane that is still running is *doing*; a lane
that stalled, was sent back, or whose merge or suite failed is *stuck*. Each item names the lane's agent alias
(``anthropic``, ``muse``, …), never a product. Pure function, only reads its arguments.
"""

from __future__ import annotations

from datetime import datetime, timezone

from tirith.home import parse_iso

STUCK_REVIEW = ("conflict", "red-suite", "sent-back", "tests-failed", "claims-failed", "merge-failed", "push-failed")
KEPT = 12  # items per list in state.json


def _item(row: dict, why: str | None = None) -> dict:
    review = row.get("review") if isinstance(row.get("review"), dict) else {}
    return {"lane": row.get("id"), "project": row.get("project"), "agent": row.get("agent") or "unnamed",
            "name": row.get("name"), "item": row.get("item") or row.get("feature") or row.get("summary") or "",
            "step": row.get("step"), "status": row.get("status"), "review": review.get("status"),
            "at": review.get("at") or row.get("ended_at") or row.get("started_at"), "why": why}


def _why(row: dict) -> str | None:
    review = row.get("review") if isinstance(row.get("review"), dict) else {}
    if review.get("status") in STUCK_REVIEW:
        return str(review.get("message") or review["status"])
    if row.get("status") == "stalled":
        return "no activity for a while"
    if row.get("status") == "failed":
        return "the lane failed"
    return None


def build(agents: list[dict], now: datetime | None = None) -> dict:
    """``{merged: [...], doing: [...], stuck: [...], counts: {...}, day: "YYYY-MM-DD"}``, newest first."""
    now = (now or datetime.now(timezone.utc)).astimezone()
    day = now.date()
    merged, doing, stuck = [], [], []
    for row in agents:
        if not isinstance(row, dict) or row.get("session") == "manager" or str(row.get("id", "")).startswith("manager"):
            continue
        review = row.get("review") if isinstance(row.get("review"), dict) else {}
        if review.get("status") == "merged":
            at = parse_iso(review.get("at"))
            if at is not None and at.astimezone().date() == day:
                merged.append(_item(row))
            continue
        why = _why(row)
        if why is not None and not (row.get("status") == "failed" and not row.get("item")):
            stuck.append(_item(row, why))
        elif row.get("status") == "working" and not row.get("ended_at"):
            doing.append(_item(row))
    merged.sort(key=lambda i: str(i["at"] or ""), reverse=True)
    doing.sort(key=lambda i: str(i["at"] or ""), reverse=True)
    stuck.sort(key=lambda i: str(i["at"] or ""), reverse=True)
    return {"day": day.isoformat(), "merged": merged[:KEPT], "doing": doing[:KEPT], "stuck": stuck[:KEPT],
            "counts": {"merged": len(merged), "doing": len(doing), "stuck": len(stuck)}}
