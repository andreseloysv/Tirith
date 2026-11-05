"""On-demand roles: when a worker becomes a manager.

With ``[roles] manager = "on-demand"`` no manager runs until the work needs one. A worker is promoted to
manager when any of these holds (``promote_to_manager`` returns the first reason, else ``None``):

* the task does not fit in one lane (``plangate.size_problem`` on the item text, or a lane diff beyond
  ``plangate.budget_note``);
* 2 or more items run in parallel (``PARALLEL_ITEMS``);
* the task asks for sub-agents (``tirith lane run`` / ``tirith launch`` / "sub-agent" in the item).

``"always"`` never needs promoting (the manager exists) and ``"off"`` never promotes.
"""

from __future__ import annotations

import re
from datetime import datetime
from pathlib import Path

from tirith import plangate as plangate_mod
from tirith import roadmap as roadmap_mod
from tirith import store
from tirith.home import now_iso, parse_iso

COMPANY_LOG = "company.jsonl"

PARALLEL_ITEMS = 2
CEO_WORKERS = 2            # a CEO is created with MORE than this many workers at once
CEO_UNATTENDED_HOURS = 1   # ... or with more than this many hours of work nobody watches
RETIRE_IDLE_SECONDS = 3600  # an on-demand CEO or manager retires after this long with nothing to do
_SUBAGENT_RE = re.compile(r"sub-?agent|subagente|tirith (?:lane run|launch)", re.IGNORECASE)


def wants_subagents(text: str) -> bool:
    return bool(_SUBAGENT_RE.search(text or ""))


def company_path(home: Path) -> Path:
    return Path(home) / COMPANY_LOG


def manager_decision(mode: str, task: str = "", parallel_items: int = 1, subagents: bool = False,
                     diff: tuple[int, int, int] | None = None) -> dict | None:
    """The first rule that promotes the worker to manager: ``{rule, measured, threshold, reason}``, or ``None``."""
    if mode != "on-demand":
        return None
    problem = plangate_mod.size_problem(task) if task else None
    if problem is not None:
        return {"rule": "task_size", "measured": len(roadmap_mod.without_check(task)),
                "threshold": plangate_mod.MAX_ITEM_CHARS, "reason": f"the task does not fit in one lane: {problem}"}
    if diff is not None:
        problem = plangate_mod.budget_note(*diff)
        if problem:
            files, plus, minus = diff
            over_files = files > plangate_mod.MAX_LANE_FILES
            return {"rule": "lane_diff", "measured": files if over_files else plus + minus,
                    "threshold": plangate_mod.MAX_LANE_FILES if over_files else plangate_mod.MAX_LANE_LINES,
                    "reason": f"the task does not fit in one lane: {problem}"}
    if parallel_items >= PARALLEL_ITEMS:
        return {"rule": "parallel_items", "measured": parallel_items, "threshold": PARALLEL_ITEMS,
                "reason": f"{parallel_items} items run in parallel"}
    if subagents or wants_subagents(task):
        return {"rule": "subagents", "measured": 1, "threshold": 1, "reason": "the task asks for sub-agents"}
    return None


def promote_to_manager(mode: str, task: str = "", parallel_items: int = 1, subagents: bool = False,
                       diff: tuple[int, int, int] | None = None) -> str | None:
    """Why the worker should become a manager, or ``None`` to stay a worker.

    ``mode`` is ``[roles] manager``; ``diff`` is ``(files, plus, minus)`` of the lane so far."""
    decision = manager_decision(mode, task, parallel_items, subagents, diff)
    return decision["reason"] if decision else None


def ceo_decision(mode: str, workers: int = 0, unattended_hours: float = 0.0,
                 selfcheck_failed: bool = False) -> dict | None:
    """The first rule that creates a CEO: ``{rule, measured, threshold, reason}``, or ``None``.

    Only with ``[roles] ceo = "on-demand"``. Rules: more than ``CEO_WORKERS`` workers at once, more than
    ``CEO_UNATTENDED_HOURS`` of work with nobody watching, or a failure the self-check does not fix."""
    if mode != "on-demand":
        return None
    if workers > CEO_WORKERS:
        return {"rule": "workers", "measured": workers, "threshold": CEO_WORKERS,
                "reason": f"{workers} workers run at once"}
    if unattended_hours > CEO_UNATTENDED_HOURS:
        return {"rule": "unattended", "measured": unattended_hours, "threshold": CEO_UNATTENDED_HOURS,
                "reason": f"{unattended_hours:g} h of work with nobody watching"}
    if selfcheck_failed:
        return {"rule": "selfcheck", "measured": 1, "threshold": 0,
                "reason": "a failure the self-check does not fix"}
    return None


def promote_to_ceo(mode: str, workers: int = 0, unattended_hours: float = 0.0,
                   selfcheck_failed: bool = False) -> str | None:
    """Why a CEO should be created, or ``None`` to go on without one."""
    decision = ceo_decision(mode, workers, unattended_hours, selfcheck_failed)
    return decision["reason"] if decision else None


def retire_decision(mode: str, role: str, workers: int = 0, idle_seconds: float = 0.0,
                    parallel_items: int = 1) -> dict | None:
    """The rule that retires an on-demand CEO or manager: ``{rule, measured, threshold, reason}``, or ``None``.

    Only with ``[roles] <role> = "on-demand"``, and only when the work is small again (the worker count is at
    or below ``CEO_WORKERS``, and for a manager fewer than ``PARALLEL_ITEMS`` items run in parallel) and the
    role has had nothing to do for more than ``RETIRE_IDLE_SECONDS``."""
    if mode != "on-demand" or role not in ("ceo", "manager"):
        return None
    if workers > CEO_WORKERS or (role == "manager" and parallel_items >= PARALLEL_ITEMS):
        return None
    if idle_seconds > RETIRE_IDLE_SECONDS:
        return {"rule": "idle_seconds", "measured": idle_seconds, "threshold": RETIRE_IDLE_SECONDS,
                "reason": f"small work again and nothing to do for {idle_seconds:g} s"}
    return None


def retire_if_idle(home: Path, mode: str, role: str, workers: int = 0, idle_seconds: float = 0.0,
                   parallel_items: int = 1, now: datetime | None = None) -> dict | None:
    """Retire the role when ``retire_decision`` says so and record it in ``company.jsonl``; returns the row."""
    decision = retire_decision(mode, role, workers, idle_seconds, parallel_items)
    if decision is None:
        return None
    return log_change(home, "retire", role, decision["rule"], decision["measured"], decision["threshold"],
                      decision["reason"], now)


_ROLE_LANE_PREFIX = {"ceo": "ceo-", "manager": "manager-"}


def _idle_seconds(role_lanes: list[dict], now: datetime) -> tuple[float, str | None]:
    """Seconds since the role last did anything (0 while one of its lanes runs), and that last moment."""
    if any(not l.get("ended_at") for l in role_lanes):
        return 0.0, None
    stamps = [t for l in role_lanes if (t := parse_iso(l.get("ended_at")) or parse_iso(l.get("started_at")))]
    if not stamps:
        return 0.0, None
    last = max(stamps)
    return max(0.0, (now - last).total_seconds()), last.isoformat()


def retire_on_tick(home: Path, projects: list[dict], lanes: list[dict], now: datetime, state: dict) -> bool:
    """Called on the dispatcher's regular pass: measure how long each on-demand CEO and manager has been idle
    (since its last lane ended) and call ``retire_if_idle``. A retirement is recorded once per period of
    activity (``state["retired"]``). Returns True when ``state`` changed."""
    retired = state.setdefault("retired", {})
    changed = False
    for project in projects:
        pid = str(project.get("id") or "")
        roles = (project.get("roadmap") or {}).get("roles") or {}
        mine = [l for l in lanes if isinstance(l, dict) and l.get("project") in (pid, None, "")]
        workers = [l for l in mine if not l.get("ended_at")
                   and not any(str(l.get("id", "")).startswith(p) for p in _ROLE_LANE_PREFIX.values())]
        for role, prefix in _ROLE_LANE_PREFIX.items():
            if roles.get(role) != "on-demand":
                continue
            idle, last = _idle_seconds([l for l in mine if str(l.get("id", "")).startswith(prefix)], now)
            key = f"{pid}:{role}"
            if last is None:
                if retired.pop(key, None) is not None:  # active again: may retire again later
                    changed = True
                continue
            if retired.get(key) == last:
                continue
            if retire_if_idle(home, "on-demand", role, workers=len(workers), idle_seconds=idle,
                              parallel_items=len(workers), now=now):
                retired[key] = last
                changed = True
    return changed


def log_change(home: Path, event: str, role: str, rule: str, measured, threshold, reason: str = "",
               now: datetime | None = None) -> dict:
    """Append a promotion or retirement to ``<home>/company.jsonl``: when, which role, the rule, the value
    measured and the threshold it was compared with."""
    if event not in ("promote", "retire"):
        raise ValueError(f"event must be promote or retire, not {event!r}")
    row = {"ts": now_iso(now), "event": event, "role": role, "rule": rule, "measured": measured,
           "threshold": threshold, "reason": reason}
    store.append_jsonl(company_path(home), row)
    return row


def log_decision(home: Path, decision: dict, role: str = "manager", now: datetime | None = None) -> dict:
    """Record a ``manager_decision`` as a promotion."""
    return log_change(home, "promote", role, decision["rule"], decision["measured"], decision["threshold"],
                      decision.get("reason", ""), now)


def read_log(home: Path) -> list[dict]:
    return store.read_jsonl(company_path(home))
