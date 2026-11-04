"""Repeat detection: a lane's agent that makes the same tool call again and again is told, then stopped.

``tirith hook`` calls :func:`note` on every ``PreToolUse`` of a lane: the same tool with the same input as the
call before adds one to the chain, anything else starts a new chain (an edit between two test runs breaks it).
At ``NUDGE_AT`` calls in a row the next ``PostToolUse`` gives the agent a reminder (:func:`nudge`); from
``BLOCK_AT`` on the call is blocked with a message (exit 2). The chain of each lane lives in
``<home>/repeats/<lane>.json``; when the lane ends (:func:`settle`) its totals move to the lane file as
``repeats: {nudged, blocked, longest}`` (the scoreboard reads them) and the file is removed.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

from tirith.store import read_json, write_json_atomic

DIR = "repeats"
NUDGE_AT = (3, 5)  # calls in a row that earn a reminder
BLOCK_AT = 8       # calls in a row from which the call is blocked
SHOWN_INPUT = 300  # characters of the repeated input quoted back to the agent


def _path(home: Path, lane: str) -> Path:
    return Path(home) / DIR / f"{lane}.json"


def _key(tool, tool_input) -> str:
    try:
        text = json.dumps(tool_input, sort_keys=True, default=str)
    except (TypeError, ValueError):
        text = str(tool_input)
    return hashlib.sha256(f"{tool}\n{text}".encode("utf-8", "replace")).hexdigest()[:16]


def _shown(tool_input) -> str:
    text = json.dumps(tool_input, sort_keys=True, default=str) if not isinstance(tool_input, str) else tool_input
    return text if len(text) <= SHOWN_INPUT else text[:SHOWN_INPUT] + "…"


def note(home: Path, lane: str, tool, tool_input) -> int:
    """Count one call of ``lane``; returns how many identical calls in a row it makes (1 = a new chain)."""
    path = _path(home, lane)
    book = read_json(path, {})
    book = book if isinstance(book, dict) else {}
    key = _key(tool, tool_input)
    count = int(book.get("count") or 0) + 1 if book.get("key") == key else 1
    book.update(key=key, count=count, tool=str(tool or ""), input=_shown(tool_input),
                longest=max(int(book.get("longest") or 0), count))
    if count >= BLOCK_AT:
        book["blocked"] = int(book.get("blocked") or 0) + 1
    path.parent.mkdir(parents=True, exist_ok=True)
    write_json_atomic(path, book)
    return count


def blocked_message(count: int, tool, tool_input) -> str:
    return (f"Tirith blocked this call: it is the same {tool} call with the same input {count} times in a row "
            f"({_shown(tool_input)}). Repeating it will not change the result. Stop and change your approach: "
            "read the error, try something different, or finish with a report that says what blocks you.")


def nudge(home: Path, lane: str) -> str | None:
    """The reminder for the ``PostToolUse`` of a call that reached a ``NUDGE_AT`` count (once per count)."""
    path = _path(home, lane)
    book = read_json(path, None)
    if not isinstance(book, dict):
        return None
    count = int(book.get("count") or 0)
    if count not in NUDGE_AT or book.get("nudged_at") == [book.get("key"), count]:
        return None
    book["nudged_at"] = [book.get("key"), count]
    book["nudged"] = int(book.get("nudged") or 0) + 1
    write_json_atomic(path, book)
    if count == NUDGE_AT[0]:
        return (f"Tirith: you made the same {book.get('tool')} call {count} times in a row with the same input. "
                "If it is not giving you what you need, try a different approach.")
    return (f"Tirith: this is the same {book.get('tool')} call {count} times in a row: {book.get('input')}. "
            f"It will be blocked from {BLOCK_AT} in a row. Change your approach, or report what blocks you.")


def settle(home: Path, lane: dict) -> None:
    """Move the totals of an ending lane into ``lane["repeats"]`` and remove its chain file."""
    path = _path(home, str(lane.get("id") or ""))
    book = read_json(path, None)
    if isinstance(book, dict) and (book.get("nudged") or book.get("blocked")):
        lane["repeats"] = {"nudged": int(book.get("nudged") or 0), "blocked": int(book.get("blocked") or 0),
                           "longest": int(book.get("longest") or 0)}
    path.unlink(missing_ok=True)
