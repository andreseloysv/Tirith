"""The dispatcher: the owner's inbox requests are handled by code, not by anyone's memory.

Every collector cycle ``run`` walks the open requests once (idempotent, state in
``<home>/dispatch.json``): a ``finish`` is answered by Tirith (the lane that already has the step, or a new
one started in-process like ``tirith launch``); a ``plan-refresh`` / ``plan-new`` starts the auditor / planner
lane of ``tirith.planlane`` (one refresh at a time per project); a ``plan-edit`` changes the plan file by code, commits it and queues one
``plan-refresh`` per project; an ``ask`` is answered by the brain (see
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

from tirith import asks as asks_mod
from tirith import autopilot as autopilot_mod
from tirith import autoreview as autoreview_mod
from tirith import diskdiet as diskdiet_mod
from tirith import fallback as fallback_mod
from tirith import flows as flows_mod
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith import managerd as managerd_mod
from tirith import manifest as manifest_mod
from tirith import howto as howto_mod
from tirith import ownergate as ownergate_mod
from tirith import planlane as planlane_mod
from tirith import verifylane as verifylane_mod
from tirith import projects as projects_mod
from tirith import roadmap as roadmap_mod
from tirith import selfcheck as selfcheck_mod
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
    for key in ("handled", "escalated", "stop", "reviews", "overdue", "autopilot", "manager"):
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
    if req["kind"] == "chat":
        return "the chat"
    return f"{req['project']} step {req['step']}"


# -- owner items and flows --------------------------------------------------------------------


def add_owner_item(home: Path, item_id: str, project: str | None, text: str, options: list[dict],
                   at: str, actions: list[dict] | None = None, link: str | None = None,
                   why_owner: str | None = None, why_detail: str | None = None, howto=None) -> str:
    """One item on the owner's waiting list with ``options`` (``[{label, command}]``), Run ``actions`` and
    an optional ``link`` to open; once per id. It passes ``ownergate.admit``: ``why_owner`` is a key of
    ``asks.WHY_OWNER`` (why only the owner can do it), else the text goes to the manager queue as "agent can
    do" (or is dropped when it is about something out of plan). Returns ``owner``, ``manager`` or ``dropped``.
    ``howto`` (``tirith.howto``: steps, done_check, already_done) must be concrete or the item goes back to the
    manager. A text that promises a button the item does not have is rewritten without it."""
    if not options and not actions:
        raise ValueError(f"owner item {item_id} has no option: every item for the owner needs a call to action")
    verdict = ownergate_mod.admit(home, item_id, project, text, why_owner, via="owner-item", at=at, howto=howto)
    if verdict != ownergate_mod.OWNER:
        return verdict
    why = asks_mod.why_key(why_owner)
    if asks_mod.missing_buttons(text, options, actions, link):
        text = asks_mod.drop_button_claims(text)
    if any(r.get("id") == item_id for r in read_jsonl(Path(home) / "waits.jsonl")):
        return ownergate_mod.OWNER
    append_jsonl(Path(home) / "waits.jsonl", {
        "id": item_id, "at": at, "project": project, "text": text, "link": link, "done_at": None,
        "session": None, "options": options, **({"actions": actions} if actions else {}),
        "why_owner": why, **({"why_detail": why_detail} if why_detail else {}),
        **({"howto": parsed} if (parsed := howto_mod.parse(howto)) else {}),
        "reason_tag": asks_mod.reason_tag(why, why_detail)})
    flows_mod.append_flow(home, "tirith", "owner", "request", text, project=project, session=None, at=at)
    return ownergate_mod.OWNER


def refused(home: Path, item_id: str, project: str | None, text: str) -> None:
    """A launch Tirith could not do is the manager's to fix, never the owner's: one manager queue item."""
    asks_mod.queue_add(home, str(project or ""), "", text, kind="launch-refused", item_id="sq-" + item_id)


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
        return launch_step(home, project, step, out=io.StringIO(), request_id=request_id, by_tirith=True)
    except SystemExit as err:  # a helper of the CLI gave up with a message
        raise launch_mod.LaunchError(str(err.code or "launch failed"))


def _default_plan_launcher(home: Path, project: str, kind: str, request_id: str | None = None) -> str:
    from tirith.cli import launch_plan
    try:
        return launch_plan(home, project, kind, request_id, out=io.StringIO(), by_tirith=True)
    except SystemExit as err:
        raise launch_mod.LaunchError(str(err.code or "launch failed"))


def _default_verify_launcher(home: Path, project: str, text: str) -> str:
    from tirith.cli import launch_verify
    try:
        return launch_verify(home, project, text, out=io.StringIO(), by_tirith=True)
    except SystemExit as err:
        raise launch_mod.LaunchError(str(err.code or "launch failed"))


PLAN_WHAT = {"plan-refresh": "Update the plan", "plan-new": "Start a new plan"}


def _plan(home: Path, req: dict, config: Config, lanes: list[dict], status_of: Callable,
          plan_launcher: Callable, now: datetime) -> str | None:
    """Handle one ``plan-refresh`` / ``plan-new``; returns what was done (``manager``: left to the manager)."""
    at, kind, project = now_iso(now), req["kind"], req["project"]
    what = PLAN_WHAT[kind]
    _flow(home, "owner", "tirith", "request", f"{what} on {project}"
          + (f": {_short(req['plan']['title'])}" if kind == "plan-new" else ""), req, at)
    busy = launch_mod.working_lane(lanes, project, planlane_mod.LANE_STEPS[kind], status_of) \
        if kind == "plan-refresh" else None
    if busy:
        inbox_mod.add_answer(home, req["id"], f"The plan of {project} is already being updated by lane "
                             f"`{busy['id']}`. Its summary will show here when it lands.", by="tirith",
                             auto=True, at=at, lane=busy["id"])
        return f"running:{busy['id']}"
    if not config.dispatch_auto_launch:
        return "manager"
    try:
        lane_id = plan_launcher(home, project, kind, req["id"])
    except launch_mod.LaunchError as err:
        reason = str(err)
    except Exception as err:  # never let one request stop the cycle
        reason = f"{type(err).__name__}: {err}"
    else:
        inbox_mod.add_answer(home, req["id"], f"Started lane `{lane_id}`: " + (
            "it compares every item of the plan with the code, the merged lanes and your requests, and will "
            "show its summary here when it lands." if kind == "plan-refresh" else
            "it writes the new plan and registers it; it will show here when it lands."),
            by="tirith", auto=True, at=at, lane=lane_id)
        _flow(home, "tirith", f"agent:{lane_id}", "brief", f"{what} on {project}", req, at)
        return f"launched:{lane_id}"
    retry = f"tirith inbox add --kind {kind} --project {project}"
    inbox_mod.add_answer(home, req["id"], f"Could not start an agent for \"{what}\" on {project}: {reason}",
                         by="tirith", auto=True, at=at)
    _flow(home, "tirith", "owner", "alert", f"Could not start \"{what}\" on {project}: {_short(reason)}", req, at)
    refused(home, f"dispatch-{req['id']}", project,
            f"Tirith could not start an agent for \"{what}\" on {project}: {_short(reason)}\nTry again: {retry}")
    return "refused"


def _git(repo: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True)


def _plan_file(repo: Path, plan: str) -> Path | None:
    """The plan file with id ``plan`` (file name without ``.md``); empty ``plan``: the current plan."""
    paths = roadmap_mod.plan_paths(repo, manifest_mod.load(repo))
    return next((p for p in paths if p.stem == plan), None) if plan else (paths[0] if paths else None)


def _apply_edit(repo: Path, req: dict) -> str:
    """Change the plan file as ``req['edit']`` says and commit only that file; returns the commit message.

    Raises ``roadmap_mod.EditError`` with the reason when it must not (or cannot) be done."""
    edit, step = req["edit"], req["step"]
    path = _plan_file(repo, edit["plan"])
    if path is None:
        raise roadmap_mod.EditError(f"the plan {edit['plan'] or 'file'} of {req['project']} was not found")
    if not path.is_relative_to(repo):
        raise roadmap_mod.EditError("the plan file is outside the project folder")
    rel = str(path.relative_to(repo))
    if _git(repo, "status", "--porcelain", "--", rel).stdout.strip():
        raise roadmap_mod.EditError(f"{rel} has uncommitted changes, so it was left as it is")
    text = path.read_text(encoding="utf-8")
    new = roadmap_mod.edit(text, step, edit.get("item_text", ""), edit.get("new_text"),
                           bool(edit.get("delete")), edit.get("add"))
    message = f"Roadmap: {inbox_mod.edit_verb(edit)} in step {step} (owner)"
    path.write_text(new, encoding="utf-8")
    _git(repo, "add", "--", rel)
    made = _git(repo, "commit", "-q", "-m", message, "--", rel)
    if made.returncode != 0:  # the guard (or git) refused: leave the plan as it was
        _git(repo, "reset", "-q", "--", rel)
        _git(repo, "checkout", "--", rel)
        raise roadmap_mod.EditError(f"committing the plan failed: {_short(made.stderr or made.stdout, 200)}")
    return message


def _refresh_queued(home: Path, project: str) -> bool:
    return any(r["kind"] == "plan-refresh" and r["project"] == project and r["status"] == "open"
               and not r["answers"] for r in inbox_mod.requests(home))


def _plan_edit(home: Path, req: dict, config: Config, now: datetime) -> str:
    """Handle one ``plan-edit`` by code: change the project's plan file, commit that file, and queue one
    ``plan-refresh`` so the plan is checked again. Returns what was done."""
    at, project = now_iso(now), req["project"]
    verb = inbox_mod.edit_verb(req["edit"])
    _flow(home, "owner", "tirith", "request", f"Plan {verb} on {_step_name(req)}", req, at)
    proj = config.project(project)
    repo = projects_mod.real(proj.repo) if proj and proj.repo else None
    if repo is None:
        auto = next((p for p in projects_mod.load_auto(home)["projects"] if str(p["id"]) == project), None)
        repo = projects_mod.real(auto["folder"]) if auto else None
    try:
        if repo is None:
            raise roadmap_mod.EditError(f"project '{project}' has no repo folder")
        message = _apply_edit(repo, req)
    except roadmap_mod.EditError as err:
        inbox_mod.add_answer(home, req["id"], f"Could not {verb} the plan: {err}", by="tirith", auto=True, at=at)
        _flow(home, "tirith", "owner", "alert", f"Plan {verb} refused on {_step_name(req)}: {_short(str(err))}",
              req, at)
        return "refused"
    if _refresh_queued(home, project):
        follow = "A check of the plan is already queued."
    else:
        inbox_mod.add_request(home, "plan-refresh", project, "", at=at)
        follow = "The plan will be checked again."
    inbox_mod.add_answer(home, req["id"], f"Done: {message}. {follow}", by="tirith", auto=True, at=at)
    return "applied"


def _default_relauncher(home: Path, lane: dict, alias: str) -> str:
    from tirith.cli import relaunch_lane  # late, like _default_launcher
    try:
        return relaunch_lane(home, lane, alias, by_tirith=True)
    except SystemExit as err:
        raise launch_mod.LaunchError(str(err.code or "relaunch failed"))


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
    refused(home, f"dispatch-{req['id']}", req["project"] or None,
            f"Tirith could not start an agent on {_step_name(req)}: {_short(reason)}\nFix it: {fix}\n"
            f"Try again: tirith launch --project {req['project']} --step {req['step']}")
    return "refused"


def macos_notify(title: str, message: str) -> None:
    """No operating-system notifications, ever (the owner's rule): escalations appear in Tirith only."""
    return None


def escalate(home: Path, req: dict, notify: Callable[[str, str], None], at: str) -> None:
    text = f"The manager has not answered: {_short(req['text']) or req['kind'] + ' ' + _step_name(req)}"
    notify("Tirith", text)
    _flow(home, "tirith", "owner", "alert", text, req, at)
    project, step = req["project"], req["step"]
    options = [{"label": "Wait", "command": None}]
    if req["kind"] != "chat":  # a chat message is not about a step
        options.append({"label": "Start an agent", "command": f"tirith launch --project {project} --step {step}"})
    options.append({"label": "Answer it with the brain", "command": f"tirith inbox brain {req['id']}"})
    add_owner_item(home, f"manager-silent-{req['id']}", project or None, text, options, at,
                   why_owner="owner-decision", why_detail="choice: wait, start an agent or use the brain")


def run(home: Path, config: Config, now: datetime, *, agents: list[dict], lanes: list[dict],
        status_of: Callable, launcher: Callable | None = None,
        notify: Callable[[str, str], None] = macos_notify, spawn: Callable | None = None,
        plan_launcher: Callable | None = None, projects: list[dict] | None = None,
        verify_launcher: Callable | None = None,
        relauncher: Callable | None = None, manager_spawn: Callable | None = None, manager_kill: Callable | None = None,
        free: Callable | None = None) -> list[dict]:
    """One dispatcher pass; returns the requests re-read afterwards. Safe to call every cycle."""
    home = Path(home)
    if not config.dispatch_enabled:
        return inbox_mod.requests(home)
    state = load_state(home)
    launcher = launcher or _default_launcher
    plan_launcher = plan_launcher or _default_plan_launcher
    changed = False
    # before the autopilot, so a relaunched lane is known as the step's lane
    if fallback_mod.run(home, config, now, state, lanes, relauncher or _default_relauncher):
        changed = True
    if diskdiet_mod.check(home, config, state, now, free):
        changed = True
    room = not diskdiet_mod.low(state)  # disk almost full: no new lane (the manager queue says why)
    if autopilot_mod.run(home, config, now, state, agents=agents, projects=projects or [], launcher=launcher,
                         launch=room):
        changed = True
    # what the owner gate found an agent can do becomes a verification lane, never an owner item
    if verifylane_mod.run(home, config, verify_launcher or _default_verify_launcher, room=room):
        changed = True
    for req in inbox_mod.requests(home):
        rid = req["id"]
        if req["status"] == "done" or req["kind"] == "autopilot":
            continue
        if rid not in state["handled"]:
            if not room and not req["answers"] and (req["kind"] == "finish" or req["kind"] in inbox_mod.PLAN_KINDS):
                done = None  # stays open, tried again when there is room
            elif req["kind"] == "finish" and not req["answers"]:
                done = _finish(home, req, config, agents, lanes, status_of, launcher, now)
            elif req["kind"] in inbox_mod.PLAN_KINDS and not req["answers"]:
                done = _plan(home, req, config, lanes, status_of, plan_launcher, now)
            elif req["kind"] == "plan-edit" and not req["answers"]:
                done = _plan_edit(home, req, config, now)
            else:
                own = req["kind"] in ("finish", "plan-edit") or req["kind"] in inbox_mod.PLAN_KINDS
                done = "answered" if own else "manager"
                if not own and req["kind"] != "chat":  # a chat message lives in the chat thread, not the flows
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
    diskdiet_mod.cleanup(home, config, now)
    # before the on-call manager, so a restart it asks for happens in this pass
    if selfcheck_mod.run(home, config, now, state, lanes=lanes, agents=agents, projects=projects or [],
                         spawn=spawn, kill=manager_kill):
        changed = True
    if config.dispatch_auto_launch and managerd_mod.run(home, config, now, state, lanes, spawn=manager_spawn,
                                                          kill=manager_kill):
        changed = True  # nobody attached and something pending: the on-call manager
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
            + ("\n" + inbox_mod.attachment_lines(r) if r.get("attachments") else "")
            + f"\n    tirith inbox answer {r['id']} \"<markdown>\"   then   tirith inbox done {r['id']}")
    return "\n".join(lines)


def manager_context(home: Path, now: datetime) -> str:
    """Text for ``additionalContext`` at ``SessionStart`` / ``UserPromptSubmit`` ('' = nothing to add)."""
    rows = pending(home, now)
    results = inbox_mod.pending_action_results(home)
    text = ""
    if rows:
        text = (f"Tirith inbox: the owner is waiting on {len(rows)} request(s) addressed to you. Answer each "
                "before anything else, with the commands shown.\n" + pending_text(rows, now))
    if results:
        text += ("\n" if text else "") + (f"Tirith inbox: {len(results)} action(s) the owner ran failed.\n"
                                          + inbox_mod.pending_results_text(results))
    waiting = _agent_waiting(home)
    if waiting:
        text += ("\n" if text else "") + (
            f"Tirith queue: {len(waiting)} working agent(s) look to be waiting on something (a guess from "
            "their logs; the owner is not told). Check each, unblock the agent or ask the owner with "
            "`tirith wait add ... --option ... --why-owner KEY` only if only the owner can do it "
            "(credentials, payment, personal-account, physical-device, owner-decision; without a key "
            "nothing reaches the owner).\n"
            + "\n".join(f"- {i['id']} ({i['project']}, lane {i['lane']}): {_short(i['text'], 300)}\n"
                        f"    tirith queue done {i['id']}" for i in waiting))
    return text


def _agent_waiting(home: Path) -> list[dict]:
    from tirith import asks as asks_mod
    return [i for i in asks_mod.queue_items(home, only_open=True) if i["kind"] == asks_mod.AGENT_WAITING_KIND]


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
