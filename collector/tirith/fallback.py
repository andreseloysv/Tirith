"""Agent fallback: a lane that ended failed because its agent is out of credits is started again with the
next agent of the chain (``[models.workers] fallback``). Never more than once per agent for one brief.

``out_of_credits`` reads the end of a lane log (and the error events of a JSON log); ``next_alias`` picks
the agent to continue with; ``run`` is the dispatcher pass that relaunches; ``status`` and ``pick_alias``
tell which agents hit the limit lately. See ``docs/CONTRACT-models.md``.
"""

from __future__ import annotations

import json
import re
from datetime import datetime
from pathlib import Path
from typing import Callable

from tirith import flows as flows_mod
from tirith import inbox as inbox_mod
from tirith import lanestop as lanestop_mod
from tirith import models as models_mod
from tirith.home import Config, now_iso, parse_iso
from tirith.store import read_json

WINDOW_S = 3600.0        # an agent counts as out of credits for this long after a lane said so
TAIL_BYTES = 16 * 1024
TAIL_LINES = 40
STATE_KEY = "fallbacks"  # dispatch.json: lane id -> the lane that continued it (or "none")
# case-insensitive regexes; ``[models] out_of_credits_patterns`` replaces them
DEFAULT_PATTERNS = (
    r"\b(?:http|status|error|code)\b\D{0,12}\b(?:402|429)\b",
    r"\b(?:402|429)\b\s*(?:payment required|too many requests|error)",
    r"payment required",
    r"too many requests",
    r"\bquota\b",
    r"\bcredits?\b",
    r"\binsufficient\b",
    r"rate[ _-]?limit",
    r"usage[ _-]?limit",
)


def patterns(config: Config) -> list[re.Pattern]:
    raw = models_mod.parse(config.models)["patterns"] or list(DEFAULT_PATTERNS)
    out = []
    for text in raw:
        try:
            out.append(re.compile(text, re.IGNORECASE))
        except re.error:
            out.append(re.compile(re.escape(text), re.IGNORECASE))
    return out


def _tail(path: str | Path | None) -> list[str]:
    if not path:
        return []
    try:
        with open(path, "rb") as fh:
            fh.seek(0, 2)
            size = fh.tell()
            fh.seek(max(0, size - TAIL_BYTES))
            text = fh.read().decode("utf-8", errors="replace")
    except OSError:
        return []
    lines = text.splitlines()
    if size > TAIL_BYTES and lines:
        lines = lines[1:]
    return [ln for ln in lines if ln.strip()][-TAIL_LINES:]


def _error_text(line: str) -> str | None:
    """The text of one log line that can say "out of credits": a plain line as it is, a JSON event only
    when it is an error (``error`` key, ``is_error``, an ``*error*`` type or subtype)."""
    line = line.strip()
    if not line.startswith("{"):
        return line
    try:
        event = json.loads(line)
    except ValueError:
        return line
    if not isinstance(event, dict):
        return None
    kinds = f"{event.get('type', '')} {event.get('subtype', '')} {event.get('event', '')}".lower()
    if event.get("error") or event.get("is_error") or "error" in kinds or "fail" in kinds:
        return json.dumps(event, ensure_ascii=False)
    return None


def out_of_credits(config: Config, log: str | Path | None) -> str | None:
    """The log line that shows the agent is out of credits, quota or rate-limited; None when none does."""
    pats = patterns(config)
    for line in reversed(_tail(log)):
        text = _error_text(line)
        if text and any(p.search(text) for p in pats):
            return text[:200]
    return None


# -- the chain ---------------------------------------------------------------------------------


def continued_by(state: dict, lane_id: str) -> str | None:
    """The lane that took over ``lane_id`` after it ran out of credits, once started."""
    new = (state.get(STATE_KEY) or {}).get(lane_id)
    if not new or new in ("none", "no", "stopped") or str(new).startswith("error"):
        return None
    return str(new)


def lane_agent(lane: dict) -> str | None:
    return lane.get("agent") or None


def tried_of(lane: dict) -> list[str]:
    return [str(a) for a in (lane.get("tried") or [])] + ([lane["agent"]] if lane.get("agent") else [])


def next_alias(config: Config, lane: dict) -> str | None:
    """The first agent of the lane's chain that this brief has not used yet."""
    tried = set(tried_of(lane))
    chain = models_mod.resolve_chain(config, lane.get("project"), lane.get("step"))
    return next((a for a in chain if a not in tried and a in config.agents), None)


def status(config: Config, lanes: list[dict], now: datetime) -> dict:
    """Per agent alias ``{state: "ok" | "out", since}``: ``out`` when a lane of the last hour ended out of
    credits (``since`` is when the first of those did)."""
    aliases = sorted(set(config.agents) | {"anthropic", "muse"})
    out = {a: {"state": "ok", "since": None} for a in aliases}
    for lane in lanes:
        alias, at = lane_agent(lane), parse_iso(lane.get("out_of_credits_at"))
        if alias and at and 0 <= (now - at).total_seconds() < WINDOW_S:
            row = out.setdefault(alias, {"state": "ok", "since": None})
            row["state"] = "out"
            if row["since"] is None or lane["out_of_credits_at"] < row["since"]:
                row["since"] = lane["out_of_credits_at"]
    return out


def pick_alias(config: Config, lanes: list[dict], now: datetime, project: str | None = None,
               step: str | None = None) -> str | None:
    """The first agent of the chain whose recent lanes did not hit out-of-credits; the first one when all
    did; None when no chain is configured."""
    chain = [a for a in models_mod.resolve_chain(config, project, step) if a in config.agents]
    if not chain:
        return None
    state = status(config, lanes, now)
    return next((a for a in chain if state.get(a, {}).get("state") != "out"), chain[0])


# -- the dispatcher pass -----------------------------------------------------------------------


def run(home: Path, config: Config, now: datetime, state: dict, lanes: list[dict],
        relauncher: Callable | None = None) -> bool:
    """One pass: every lane that ended failed in the last hour with its log showing out of credits is
    marked (``out_of_credits_at``) and, when the chain has another agent, started again with it once.
    Returns whether ``state`` changed."""
    done = state.setdefault(STATE_KEY, {})
    changed = False
    for lane in lanes:
        lid = str(lane.get("id"))
        ended = parse_iso(lane.get("ended_at"))
        if (lid in done or lane.get("result") != "failed" or not ended
                or not 0 <= (now - ended).total_seconds() < WINDOW_S):
            continue
        if lanestop_mod.marker(lane):  # the owner stopped it: not a credits problem
            done[lid], changed = "stopped", True
            continue
        alias = lane_agent(lane)
        hit = out_of_credits(config, lane.get("log")) if alias else None
        if not hit:
            done[lid], changed = "no", True
            continue
        lane["out_of_credits_at"] = lane.get("ended_at")
        _mark(home, lid, lane["out_of_credits_at"])
        target = next_alias(config, lane)
        if target is None or relauncher is None:
            done[lid], changed = "none", True
            continue
        try:
            new_id = relauncher(home, lane, target)
        except Exception as error:  # never let one lane stop the cycle
            done[lid] = f"error: {type(error).__name__}"
            changed = True
            continue
        done[lid] = new_id
        changed = True
        note(home, lane, target, new_id, now_iso(now))
    return changed


def _mark(home: Path, lane_id: str, at: str) -> None:
    from tirith.store import lane_path, write_lane
    stored = read_json(lane_path(home, lane_id), None)
    if isinstance(stored, dict):
        stored["out_of_credits_at"] = at
        write_lane(home, stored)


def note(home: Path, lane: dict, target: str, new_id: str, at: str) -> str:
    """Tell the Plan thread (and the flows) what happened; returns the text."""
    text = f"{lane_agent(lane)} ran out of credits → continued with {target}"
    request_id = lane.get("request_id")
    if request_id and inbox_mod.find_request(home, request_id):
        inbox_mod.add_answer(home, request_id, text, by="tirith", auto=True, at=at, lane=new_id)
    flows_mod.append_flow(home, "tirith", f"agent:{new_id}", "message", text,
                          session=None, project=str(lane.get("project") or "") or None, at=at)
    return text
