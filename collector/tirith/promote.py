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

from tirith import plangate as plangate_mod

PARALLEL_ITEMS = 2
_SUBAGENT_RE = re.compile(r"sub-?agent|subagente|tirith (?:lane run|launch)", re.IGNORECASE)


def wants_subagents(text: str) -> bool:
    return bool(_SUBAGENT_RE.search(text or ""))


def promote_to_manager(mode: str, task: str = "", parallel_items: int = 1, subagents: bool = False,
                       diff: tuple[int, int, int] | None = None) -> str | None:
    """Why the worker should become a manager, or ``None`` to stay a worker.

    ``mode`` is ``[roles] manager``; ``diff`` is ``(files, plus, minus)`` of the lane so far."""
    if mode != "on-demand":
        return None
    problem = plangate_mod.size_problem(task) if task else None
    if problem is None and diff is not None:
        problem = plangate_mod.budget_note(*diff)
    if problem:
        return f"the task does not fit in one lane: {problem}"
    if parallel_items >= PARALLEL_ITEMS:
        return f"{parallel_items} items run in parallel"
    if subagents or wants_subagents(task):
        return "the task asks for sub-agents"
    return None
