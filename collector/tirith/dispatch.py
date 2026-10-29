"""The dispatcher: the owner's inbox requests are handled by code, not by anyone's memory.

Every collector cycle ``run`` walks the open requests once (idempotent, state in
``<home>/dispatch.json``): a ``finish`` is answered by Tirith (the lane that already has the step, or a new
one started in-process like ``tirith launch``); an ``ask`` is answered by the brain (see
``Collector._inbox``) and stays pending for the manager; anything else is left to the manager. A request the
manager leaves unanswered for ``ESCALATE_AFTER_S`` is escalated to the owner. The manager session itself is
held to the inbox by hooks (``manager_context`` / ``manager_stop`` here, called from ``tirith.hooks``).
See ``docs/CONTRACT-inbox.md``.
"""

from __future__ import annotations

import io
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable

from tirith import autopilot as autopilot_mod
from tirith import autoreview as autoreview_mod
from tirith import flows as flows_mod
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith.home import Config, now_iso, parse_iso
from tirith.store import append_jsonl, read_json, read_jsonl, write_json_atomic

STATE_FILE = "dispatch.json"
STOP_AFTER_S = 60.0
STOP_BLOCKS_MAX = 3
ESCALATE_AFTER_S = 600.0
TEXT_SHOWN = 160
ROLE_ENV = "TIRITH_ROLE"
MANAGER_ROLE = "manager"
OLD_MANAGER_ROLE = "steward"  # the old name, still accepted in TIRITH_ROLE


# -- state ------------------------------------------------------------------------------------


def load_state(home: Path) -> dict:
    data = read_json(Path(home) / STATE_FILE, {})
    data = data if isinstance(data, dict) else {}
    for key in ("handled", "escalated", "stop", "reviews", "overdue", "autopilot"):
        if not isinstance(data.get(key), dict):
            data[key] = {}
    return data


def save_state(home: Path, state: dict) -> None:
    write_json_atomic(Path(home) / STATE_FILE, state)


def _age_s(req: dict, now: datetime) -> float:
    asked = parse_iso(req.get("at"))
    return (now - asked).total_seconds() if asked else 0.0


def _short(text: str, limit: int = TEXT_SHOWN) -> str:
    return flows_mod.one_line(text, limit)


def _step_name(req: dict) -> str:
    return f"{req['project']} step {req['step']}"


# -- owner items and flows --------------------------------------------------------------------


def add_owner_item(home: Path, item_id: str, project: str | None, text: str, options: list[dict],
                   at: str, actions: list[dict] | None = None, link: str | None = None) -> None:
    """One item on the owner's waiting list with ``options`` (``[{label, command}]``), Run ``actions`` and
    an optional ``link`` to open; once per id."""
    if not options and not actions:
        raise ValueError(f"owner item {item_id} has no option: every item for the owner needs a call to action")
    if any(r.get("id") == item_id for r in read_jsonl(Path(home) / "waits.jsonl")):
        return
    append_jsonl(Path(home) / "waits.jsonl", {
        "id": item_id, "at": at, "project": project, "text": text, "link": link, "done_at": None,
        "session": None, "options": options, **({"actions": actions} if actions else {})})
    flows_mod.append_flow(home, "tirith", "owner", "request", text, project=project, session=None, at=at)


def close_owner_item(home: Path, item_id: str, at: str) -> None:
    rows = [r for r in read_jsonl(Path(home) / "waits.jsonl") if r.get("id") == item_id]
    if rows and not rows[-1].get("done_at"):
        append_jsonl(Path(home) / "waits.jsonl", {**rows[-1], "done_at": at})


def _flow(home: Path, src: str, dst: str, kind: str, text: str, req: dict, at: str) -> None:
    flows_mod.append_flow(home, src, dst, kind, text, ref=req["id"], session=None,
                          project=req["project"] or None, at=at)


# -- the cycle --------------------------------------------------------------------------------


def _default_launcher(home: Path, project: str, step: str, request_id: str | None = None) -> str:
    from tirith.cli import launch_step  # late: cli imports the collector, which imports this module
    try:
        return launch_step(home, project, step, out=io.StringIO(), request_id=request_id)
    except SystemExit as err:  # a helper of the CLI gave up with a message
        raise launch_mod.LaunchError(str(err.code or "launch failed"))


def refusal_fix(project: str, step: str, reason: str) -> str:
    """The exact command that fixes (or starts over after) a refused launch."""
    if "tirith check fails" in reason:
        return f"tirith check --project {project}"
    if "no agent command" in reason:
        return "add [launch] command = \"<agent> ... {brief}\" to ~/.tirith/config.toml"
    if "already done" in reason or "no items" in reason:
        return f"tirith inbox list --open  (pick another step of {project})"
    return f"tirith launch --project {project} --step {step}"


def _finish(home: Path, req: dict, config: Config, agents: list[dict], lanes: list[dict],
            status_of: Callable, launcher: Callable, now: datetime) -> str | None:
    """Handle one ``finish``; returns what was done (``manager``: left to the manager)."""
    at = now_iso(now)
    _flow(home, "owner", "tirith", "request",
          f"Finish {_step_name(req)}" + (f": {_short(req['text'])}" if req["text"] else ""), req, at)
    busy = launch_mod.working_lane(lanes, req["project"], req["step"], status_of)
    if busy:
        row = next((a for a in agents if a.get("id") == busy["id"]), {})
        name = row.get("name") or busy["id"]
        doing = _short(row.get("summary") or "") or "no activity line yet"
        inbox_mod.add_answer(home, req["id"], f"{name} (lane `{busy['id']}`) is already working on "
                             f"{_step_name(req)}. Now: {doing}", by="tirith", auto=True, at=at,
                             lane=busy["id"])
        _flow(home, "tirith", f"agent:{busy['id']}", "message",
              f"{_step_name(req)} is already yours; the owner asked for it again", req, at)
        return f"running:{busy['id']}"
    if not config.dispatch_auto_launch:
        return "manager"
    try:
        lane_id = launcher(home, req["project"], req["step"], req["id"])
    except launch_mod.LaunchError as err:
        reason = str(err)
    except Exception as err:  # never let one request stop the cycle
        reason = f"{type(err).__name__}: {err}"
    else:
        inbox_mod.add_answer(home, req["id"], f"Started lane `{lane_id}` on {_step_name(req)}.",
                             by="tirith", auto=True, at=at, lane=lane_id)
        _flow(home, "tirith", f"agent:{lane_id}", "brief", f"Finish {_step_name(req)}", req, at)
        return f"launched:{lane_id}"
    fix = refusal_fix(req["project"], req["step"], reason)
    inbox_mod.add_answer(home, req["id"], f"Could not start an agent on {_step_name(req)}: {reason}\n\n"
                         f"Fix: `{fix}`", by="tirith", auto=True, at=at)
    _flow(home, "tirith", "owner", "alert", f"Could not start {_step_name(req)}: {_short(reason)}", req, at)
    add_owner_item(home, f"dispatch-{req['id']}", req["project"] or None,
                   f"Tirith could not start an agent on {_step_name(req)}: {_short(reason)}",
                   [{"label": "Fix it", "command": fix},
                    {"label": "Try again", "command": f"tirith launch --project {req['project']} --step {req['step']}"}],
                   at)
    return "refused"


def macos_notify(title: str, message: str) -> None:
    """No operating-system notifications, ever (the owner's rule): escalations appear in Tirith only."""
    return None


def escalate(home: Path, req: dict, notify: Callable[[str, str], None], at: str) -> None:
    text = f"The manager has not answered: {_short(req['text']) or req['kind'] + ' ' + _step_name(req)}"
    notify("Tirith", text)
    _flow(home, "tirith", "owner", "alert", text, req, at)
    project, step = req["project"], req["step"]
    add_owner_item(home, f"manager-silent-{req['id']}", project or None, text,
                   [{"label": "Wait", "command": None},
                    {"label": "Start an agent", "command": f"tirith launch --project {project} --step {step}"},
                    {"label": "Answer it with the brain", "command": f"tirith inbox brain {req['id']}"}], at)


def run(home: Path, config: Config, now: datetime, *, agents: list[dict], lanes: list[dict],
        status_of: Callable, launcher: Callable | None = None,
        notify: Callable[[str, str], None] = macos_notify, spawn: Callable | None = None,
        projects: list[dict] | None = None) -> list[dict]:
    """One dispatcher pass; returns the requests re-read afterwards. Safe to call every cycle."""
    home = Path(home)
    if not config.dispatch_enabled:
        return inbox_mod.requests(home)
    state = load_state(home)
    launcher = launcher or _default_launcher
    changed = False
    if autopilot_mod.run(home, config, now, state, agents=agents, projects=projects or [], launcher=launcher):
        changed = True
    for req in inbox_mod.requests(home):
        rid = req["id"]
        if req["status"] == "done" or req["kind"] == "autopilot":
            continue
        if rid not in state["handled"]:
            if req["kind"] == "finish" and not req["answers"]:
                done = _finish(home, req, config, agents, lanes, status_of, launcher, now)
            else:
                done = "manager" if req["kind"] != "finish" else "answered"
                if req["kind"] != "finish":
                    _flow(home, "owner", "tirith", "request",
                          f"{req['kind'].capitalize()} on {_step_name(req)}: {_short(req['text'])}", req, now_iso(now))
            if done:
                state["handled"][rid] = done
                changed = True
        if req["manager_pending"] and _age_s(req, now) >= ESCALATE_AFTER_S and rid not in state["escalated"]:
            escalate(home, req, notify, now_iso(now))
            state["escalated"][rid] = now_iso(now)
            changed = True
    if autoreview_mod.run(home, config, now, state, lanes, spawn=spawn, notify=notify):
        changed = True
    for rid in list(state["escalated"]):  # the manager caught up: take the item off the owner's list
        req = next((r for r in inbox_mod.requests(home) if r["id"] == rid), None)
        if req is not None and not req["manager_pending"]:
            close_owner_item(home, f"manager-silent-{rid}", now_iso(now))
    if changed:
        save_state(home, state)
    return inbox_mod.requests(home)


def note_brain_answer(home: Path, req: dict, at: str) -> None:
    """The flow for an automatic brain answer (owner -> tirith -> brain was recorded as it happened)."""
    _flow(home, "tirith", "brain", "request", f"Question on {_step_name(req)}: {_short(req['text'])}", req, at)
    _flow(home, "brain", "owner", "answer", f"Answered the question on {_step_name(req)}", req, at)


# -- the manager session ----------------------------------------------------------------------


def is_manager(config: Config, environ, cwd) -> bool:
    """The manager session: ``TIRITH_ROLE=manager``, or a cwd inside ``[manager] session_cwd``.

    A lane (``TIRITH_LANE``) is never the manager, whatever its folder."""
    if environ.get("TIRITH_LANE"):
        return False
    if environ.get(ROLE_ENV) in (MANAGER_ROLE, OLD_MANAGER_ROLE):
        return True
    root = config.manager_session_cwd
    if not root or not cwd:
        return False
    root, here = str(root).rstrip("/"), str(cwd).rstrip("/")
    return here == root or here.startswith(root + "/")


def pending(home: Path, now: datetime, min_age_s: float = 0.0) -> list[dict]:
    return [r for r in inbox_mod.requests(home)
            if r["manager_pending"] and _age_s(r, now) >= min_age_s]


def _age_text(seconds: float) -> str:
    seconds = int(seconds)
    if seconds < 90:
        return f"{seconds}s"
    return f"{seconds // 60}m" if seconds < 5400 else f"{seconds // 3600}h"


def pending_text(rows: list[dict], now: datetime) -> str:
    lines = []
    for r in rows:
        said = [a for a in r["answers"] if a["by"] != "manager"]
        lines.append(
            f"- {r['id']} ({r['kind']}, {r['project']} step {r['step']}, {_age_text(_age_s(r, now))} ago): "
            f"{_short(r['text'], 300) or '(no text)'}"
            + (f" [already answered by {said[-1]['by']}; follow up]" if said else "")
            + f"\n    tirith inbox answer {r['id']} \"<markdown>\"   then   tirith inbox done {r['id']}")
    return "\n".join(lines)


def manager_context(home: Path, now: datetime) -> str:
    """Text for ``additionalContext`` at ``SessionStart`` / ``UserPromptSubmit`` ('' = nothing to add)."""
    rows = pending(home, now)
    if not rows:
        return ""
    return (f"Tirith inbox: the owner is waiting on {len(rows)} request(s) addressed to you. Answer each "
            "before anything else, with the commands shown.\n" + pending_text(rows, now))


def _old_reviews(home: Path, now: datetime) -> list[dict]:
    """Open ``review`` items of the manager queue older than ``STOP_AFTER_S``."""
    from tirith import asks as asks_mod
    out = []
    for item in asks_mod.queue_items(home, only_open=True):
        at = parse_iso(item.get("at") or "")
        if item["kind"] == "review" and at and (now - at).total_seconds() >= STOP_AFTER_S:
            out.append(item)
    return out


def _reviews_text(items: list[dict], now: datetime) -> str:
    return "\n".join(
        f"- {i['id']} (review, {i['project']}, {_age_text((now - parse_iso(i['at'])).total_seconds())} ago): "
        f"{_short(i['text'], 300)}\n    tirith lane review {i['lane']}   then   tirith queue done {i['id']}"
        for i in items)


def manager_stop(home: Path, now: datetime) -> str | None:
    """The reason to block a ``Stop`` (None = let it stop).

    Blocks while a request pending for more than ``STOP_AFTER_S`` is unanswered by the manager, at most
    ``STOP_BLOCKS_MAX`` times in a row for the same set; then it records an ``inbox-ignored`` violation
    once and lets the session stop."""
    rows = pending(home, now, STOP_AFTER_S)
    reviews = _old_reviews(home, now)
    state = load_state(home)
    if not rows and not reviews:
        if state["stop"]:
            state["stop"] = {}
            save_state(home, state)
        return None
    key = ",".join(sorted(r["id"] for r in rows + reviews))
    stop = state["stop"] if state["stop"].get("key") == key else {"key": key, "count": 0}
    if stop["count"] >= STOP_BLOCKS_MAX:
        if not stop.get("violation"):
            from tirith import gates
            gates.record_violation(
                home, gates.Violation("inbox-ignored",
                                      f"the manager stopped with {len(rows) + len(reviews)} unanswered inbox request(s) / review item(s): {key}",
                                      "tirith inbox answer <id> \"<markdown>\" for each, then tirith inbox done <id>"),
                ts=now_iso(now), agent="anthropic", lane=None, session=None, project=(rows or reviews)[0]["project"] or None,
                tool=None, cwd=None)
            stop["violation"] = True
            state["stop"] = stop
            save_state(home, state)
        return None
    stop["count"] += 1
    state["stop"] = stop
    save_state(home, state)
    return ("Tirith inbox: you cannot stop yet, the owner is waiting (block "
            f"{stop['count']} of {STOP_BLOCKS_MAX}).\n"
            + "\n".join(t for t in (pending_text(rows, now), _reviews_text(reviews, now)) if t))


def record_event(home: Path, event: str, text: str, ts: str) -> None:
    """``kind: "inbox-injection"`` / ``"inbox-block"`` row in ``events.jsonl``."""
    append_jsonl(Path(home) / "events.jsonl", {
        "kind": "inbox-block" if event == "Stop" else "inbox-injection", "ts": ts, "event": event,
        "agent": "anthropic", "summary": _short(text.splitlines()[0] if text else "", 200)})
