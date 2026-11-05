"""The Status card: Tirith's development at a glance, in four lines, built by code from the state (no model).

1. Is it going well? ``level`` (ok / warn / bad) and ``why`` in plain words: the health, CI on main, nothing
   running while items are ready.
2. What is being worked on? The plan steps with an agent working, each with its progress.
3. What just landed? The last merges, one line each.
4. Is it your turn? The things waiting on the owner (only when there are some).

``state.status``; the Overview draws it first. The sentinel (``tirith/sentinel.py``) reads it too.
"""

from __future__ import annotations

from datetime import datetime

from tirith.home import parse_iso
from tirith.roadmap import split_status, without_check

RECENT = 3


def _today(iso, now: datetime) -> bool:
    at = parse_iso(iso)
    return at is not None and at.astimezone().date() == now.astimezone().date()


def build(state: dict, now: datetime, ci: dict | None = None, merged_today: int | None = None,
          lasting: list[str] | None = None) -> dict:
    """``merged_today``: counted from every lane record (the state keeps only recent agents); ``lasting``: the checks
    not ok for 20 minutes or more (the CEO's ``unwell``), the only amber that counts: a short healing is normal."""
    agents = [a for a in state.get("agents") or [] if isinstance(a, dict)]
    workers = [a for a in agents if not str(a.get("id", "")).startswith(("manager", "ceo-", "canary-"))]
    working = [a for a in workers if a.get("status") == "working"]
    merged = sorted((a for a in workers if (a.get("review") or {}).get("status") == "merged"),
                    key=lambda a: str((a.get("review") or {}).get("at") or ""), reverse=True)
    steps = []
    for project in state.get("projects") or []:
        road = project.get("roadmap") or {}
        for step in road.get("steps") or []:
            mine = [a for a in working if a.get("project") == project.get("id") and str(a.get("step")) == str(step.get("id"))]
            if mine:
                steps.append({"project": project.get("id"), "step": str(step.get("id")), "title": step.get("title"),
                              "done": step.get("done"), "total": step.get("total"), "agents": len(mine)})
    health = state.get("health") or {}
    bad_checks = [c for c in health.get("checks") or [] if c.get("level") == "failed"]
    ci = ci or {}
    ci_red = sorted(k for k, v in ci.items() if v == "failed")
    open_items = int((state.get("autopilot") or {}).get("open_items") or 0)
    level, why = "ok", "Moving" if working else "Idle: nothing to do"
    if bad_checks or ci_red:
        level = "bad"
        why = "; ".join([*(c.get("text") or c.get("label") or "" for c in bad_checks), *(f"CI red: {k}" for k in ci_red)])
    elif not working and open_items:
        level, why = "warn", f"Nothing working while {open_items} plan items are open"
    elif lasting:
        level = "warn"
        why = next((c.get("text") for c in health.get("checks") or [] if c.get("id") in lasting), "") or "A check is not ok"
    waits = [w for w in state.get("waiting_on_you") or [] if isinstance(w, dict)]
    limit = (state.get("autopilot") or {}).get("limit")
    return {
        "level": level, "why": why,
        "merged_today": merged_today if merged_today is not None else
        sum(1 for a in merged if _today((a.get("review") or {}).get("at"), now)),
        "working": len(working), "limit": limit, "ci": "red" if ci_red else ("green" if ci else None),
        "steps": steps,
        "recent": [{"text": without_check(split_status(str(a.get("item") or a.get("feature") or a.get("id")))[0]).strip()[:120],
                    "at": (a.get("review") or {}).get("at"), "project": a.get("project")} for a in merged[:RECENT]],
        "owner": [{"id": w.get("id"), "text": str(w.get("text") or "")[:100]} for w in waits][:5],
        "owner_count": len(waits),
    }
