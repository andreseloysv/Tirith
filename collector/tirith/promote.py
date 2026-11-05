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
from tirith.home import now_iso

COMPANY_LOG = "company.jsonl"

PARALLEL_ITEMS = 2
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
