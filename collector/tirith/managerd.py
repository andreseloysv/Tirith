"""A manager that lives inside Tirith: the owner can close the terminal and keep talking in the chat.

Since the persistent session (``msession.py``, ``run_session`` below) there is ONE manager process per home,
owned by the collector, that the chat feeds; the one-shot lanes described next remain for ``[manager] session =
false`` and as the fallback when the session cannot start.

Two halves. **Presence**: the interactive manager session writes a heartbeat (``<home>/manager.json``:
``pid``, ``session``, ``at``) from its hooks; it counts as *attached* while the heartbeat is younger than
``ATTACHED_S`` and the pid is alive. **On call**: when something is pending for the manager (chat message,
ask, setup request, failed action, manager-queue item) and no manager is attached, the dispatcher starts ONE
headless manager lane (``tirith lane run --class steward --feature Manager``) with a generated brief; it
answers everything pending and exits, and the next pending item starts another. Its rolling memory is
``<home>/manager-notes.md`` (``tirith manager-note``). See ``docs/CONTRACT-inbox.md``.
"""

from __future__ import annotations

import json
import os
import signal
import subprocess
import sys
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable

from tirith import agentrun as agentrun_mod
from tirith import howto as howto_mod
from tirith import asks as asks_mod
from tirith import flows as flows_mod
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith import models as models_mod
from tirith import msession as msession_mod
from tirith import plangate as plangate_mod
from tirith.home import Config, now_iso, parse_iso
from tirith.store import lane_path, read_json, write_json_atomic, write_lane

PRESENCE_FILE = "manager.json"
NOTES_FILE = "manager-notes.md"
ATTACHED_S = 120.0
SPAWN_GRACE_S = 180.0  # a spawned lane that has not registered yet still counts as running
RETRY_AFTER_S = 60.0
REVIEW_GRACE_S = 300.0
PAUSE_AFTER = 2  # manager lanes that failed in a row pause the on-call manager
PAUSE_ITEM = "sq-manager-paused"
MAX_TRIES = 2  # per set of pending ids: a manager that leaves the same items open twice is not started again
NOTES_SHOWN = 4000
NOTES_KEPT = 20000
LANE_PREFIX = "manager-"
HERO_CLASS = "steward"
FEATURE = "Manager"
DEFAULT_ALIAS = "anthropic"
LINGER_S = 30.0  # a manager process still alive this long after its ``result`` event is terminated
TAIL_BYTES = 65536
TURNS_KEPT = 30

RULES = (
    "You are the owner's manager, started by Tirith because the owner is not at a terminal. The owner talks to "
    "you only through the Tirith chat.",
    "Delegate everything to sub-agents (`tirith lane run`, `tirith launch --project P --step N`). Never edit "
    "code or files of a project yourself.",
    "The plan is the only way to start an agent, no exceptions: every start needs an open item of the plan, "
    "`tirith lane run ... --step N --item \"<exact item text>\"` or `tirith launch --project P --step N --item "
    "\"<exact item text>\"`. An item that is not in the plan yet is added first (you are the one who updates the "
    "TODO, e.g. a review): `tirith plan add --project P --step N --text \"<item>\"`. Nobody edits `docs/TODO.md` "
    "by hand, not even you; Tirith writes it. An agent that ends without `tirith plan report` has its work "
    "thrown away.",
    "`tirith lane run` from here starts the lane detached, in its own worktree, and prints the lane id at "
    "once. Never run it as a background task of your own shell and do not wait for it: your turn ends when "
    "you have answered.",
    "A request a lane works on is settled only when the lane reports, not when you start it: do not run "
    "`tirith inbox done` for it at launch. Answer that the lane started and leave the request open; the "
    "lane's report comes back as a queue item, then close the request.",
    "Answer every pending message below with `tirith inbox answer <id> \"<markdown>\"`, then "
    "`tirith inbox done <id>` when it is settled. Answer the question that was asked, briefly.",
    "Put something on the owner's list (`tirith wait add ... --option ...`) only for what you must refuse or "
    "cannot do yourself. No operating-system notifications.",
    *howto_mod.RULES,
    "Queue items: handle each (look at the lane, unblock it, review it) and close it with "
    "`tirith queue done <id>`.",
    "Do not commit. Name agents only by their alias.",
    "Before you finish, append what you decided and launched with `tirith manager-note \"<text>\"`; the next "
    "manager reads it. Then exit: you are started again when more arrives.",
)


# -- presence ---------------------------------------------------------------------------------


def presence_path(home: Path) -> Path:
    return Path(home) / PRESENCE_FILE


def pid_alive(pid) -> bool:
    if not isinstance(pid, int) or isinstance(pid, bool) or pid <= 0:
        return False
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    except OSError:
        return False
    return True


def session_pid() -> int:
    """The pid of the agent session that runs this hook: the parent, or its parent when that is a shell."""
    pid = os.getppid()
    try:
        done = subprocess.run(["ps", "-o", "ppid=,comm=", "-p", str(pid)], capture_output=True, text=True,
                              timeout=2)
        ppid, _, comm = done.stdout.strip().partition(" ")
        if ppid.isdigit() and Path(comm.strip()).name in ("sh", "bash", "zsh", "dash", "fish"):
            return int(ppid)
    except (OSError, subprocess.SubprocessError):
        pass
    return pid


def heartbeat(home: Path, session: str | None, now: datetime, pid: int | None = None) -> None:
    """Record that the interactive manager session is alive (called from its hooks)."""
    write_json_atomic(presence_path(home), {
        "pid": pid if pid is not None else session_pid(), "session": session or None, "at": now_iso(now)})


def attached(home: Path, now: datetime) -> bool:
    """True while an interactive manager session is live: a heartbeat younger than ``ATTACHED_S``, pid alive."""
    beat = read_json(presence_path(home), {})
    if not isinstance(beat, dict):
        return False
    at = parse_iso(beat.get("at"))
    return bool(at and 0 <= (now - at).total_seconds() < ATTACHED_S and pid_alive(beat.get("pid")))


# -- notes ------------------------------------------------------------------------------------


def add_note(home: Path, text: str, now: datetime) -> None:
    path = Path(home) / NOTES_FILE
    old = path.read_text(encoding="utf-8") if path.is_file() else ""
    lines = (old + f"- {now_iso(now)} {' '.join(text.split())}\n").splitlines(keepends=True)
    while len(lines) > 1 and sum(len(l) for l in lines) > NOTES_KEPT:  # keep whole lines, newest last
        lines.pop(0)
    path.write_text("".join(lines), encoding="utf-8")


def notes_tail(home: Path) -> str:
    path = Path(home) / NOTES_FILE
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return ""
    if len(text) > NOTES_SHOWN:
        text = text[-NOTES_SHOWN:]
        text = text[text.find("\n- ") + 1:] if "\n- " in text else text
    return text.strip()


# -- what is pending --------------------------------------------------------------------------


def _wakes(item: dict, now: datetime) -> bool:
    """A queue item that needs the manager: not a guess about a waiting agent (those come and go by themselves),
    and a lane review only once the automatic review has had ``REVIEW_GRACE_S``."""
    if item.get("kind") == asks_mod.AGENT_WAITING_KIND:
        return False
    at = parse_iso(item.get("at") or "")
    return not (item.get("kind") == "review" and at and (now - at).total_seconds() < REVIEW_GRACE_S)


def pending_items(home: Path, now: datetime) -> dict:
    """What the manager owes the owner now: ``{requests, results, queue}`` (the same sets its hooks list)."""
    return {
        "requests": [r for r in inbox_mod.requests(home) if r["manager_pending"]],
        "results": inbox_mod.pending_action_results(home),
        "queue": [q for q in asks_mod.queue_items(home, only_open=True) if _wakes(q, now)],
    }


def pending_key(items: dict) -> str:
    return ",".join(sorted([r["id"] for r in items["requests"]]
                           + [f"act:{r.get('wait')}:{r.get('at')}" for r in items["results"]]
                           + [str(q["id"]) for q in items["queue"]]))


def has_work(items: dict) -> bool:
    return bool(items["requests"] or items["results"] or items["queue"])


def _attachments(req: dict) -> list[str]:
    raw = req.get("attachments") or req.get("files") or []
    out = []
    for a in raw if isinstance(raw, list) else []:
        path = a.get("path") if isinstance(a, dict) else a
        if path:
            out.append(str(path))
    return out


def _plan_summary(config: Config, home: Path) -> str:
    from tirith import manifest as manifest_mod
    from tirith import projects as projects_mod
    from tirith import roadmap as roadmap_mod
    lines = []
    for proj in config.projects:
        repo = projects_mod.real(proj.repo) if proj.repo else None
        if repo is None:
            continue
        try:
            path = roadmap_mod.find_file(repo, manifest_mod.load(repo).roadmap)
            steps = roadmap_mod._parsed(path) if path else None
        except Exception:
            steps = None
        if not steps:
            lines.append(f"- {proj.id}: no plan")
            continue
        total = sum(len(s["items"]) for s in steps)
        done = sum(1 for s in steps for i in s["items"] if i["done"])
        todo = [f"{s['id']}. {s['title']}" for s in steps if any(not i["done"] for i in s["items"])][:6]
        lines.append(f"- {proj.id}: {done}/{total} items done; open steps: " + ("; ".join(todo) or "none"))
    return "\n".join(lines) or "(no projects)"


def pending_sections(items: dict, now: datetime) -> list[str]:
    """The pending messages, failed actions and queue items as brief lines."""
    def age(at) -> str:
        parsed = parse_iso(at)
        return f"{int((now - parsed).total_seconds())}s ago" if parsed else "?"

    out = ["## Pending messages"]
    for r in items["requests"]:
        said = [a for a in r["answers"] if a["by"] != "manager"]
        out.append(f"- {r['id']} ({r['kind']}, {r['project'] or 'no project'}"
                   + (f" step {r['step']}" if r["step"] else "") + f", {age(r['at'])}): "
                   f"{inbox_mod.clip(r['text'], 2000) or '(no text)'}"
                   + (f" [already answered by {said[-1]['by']}; follow up]" if said else ""))
        for path in _attachments(r):
            out.append(f"    attachment: {path}")
    if not items["requests"]:
        out.append("(none)")
    if items["results"]:
        out += ["", "## Actions the owner ran that failed", inbox_mod.pending_results_text(items["results"])]
    if items["queue"]:
        out += ["", "## Manager queue"]
        for q in items["queue"]:
            out.append(f"- {q['id']} ({q.get('kind')}, {q.get('project') or '-'}, lane {q.get('lane') or '-'}): "
                       f"{flows_mod.one_line(str(q.get('text') or ''), 300)}")
    if items.get("evidence"):
        out += ["", "## Self-check", str(items["evidence"])]
    return out


def build_brief(home: Path, config: Config, items: dict, now: datetime) -> str:
    out = ["# Lane: Manager (on call)", "", "## Why",
           "The owner closed the terminal; Tirith started you to answer what is waiting in the chat and act on it.", "",
           "## Rules", *(f"- {r}" for r in RULES), "", *pending_sections(items, now)]
    out += ["", "## Plan", _plan_summary(config, home)]
    recent = flows_mod.flows_state(home, {}, limit=12)
    if recent:
        out += ["", "## Recent flows"]
        out += [f"- {f.get('from')} -> {f.get('to')} ({f.get('kind')}): {flows_mod.one_line(str(f.get('text') or ''), 160)}"
                for f in recent]
    notes = notes_tail(home)
    if notes:
        out += ["", "## Your notes from earlier runs", notes]
    return "\n".join(out) + "\n"


# -- the lane ---------------------------------------------------------------------------------


def manager_alias(config: Config) -> tuple[str, models_mod.Choice]:
    choice = models_mod.resolve_manager(config)
    return (choice.agent or DEFAULT_ALIAS), choice


def _project_for(home: Path, config: Config, items: dict):
    """The project the lane runs in: the first pending request's, else the first configured one."""
    wanted = [r["project"] for r in items["requests"] if r["project"]]
    for pid in wanted:
        proj = config.project(pid)
        if proj is not None and proj.repo:
            return proj
    return next((p for p in config.projects if p.repo), None)


def launch(home: Path, config: Config, items: dict, now: datetime, spawn: Callable | None = None) -> str:
    """Write the brief and start the headless manager lane, detached; returns its lane id."""
    from tirith import projects as projects_mod
    proj = _project_for(home, config, items)
    repo = projects_mod.real(proj.repo) if proj is not None and proj.repo else None
    if repo is None:
        raise launch_mod.LaunchError("no project with a repo folder to run the manager in")
    alias, choice = manager_alias(config)
    lane_id = LANE_PREFIX + uuid.uuid4().hex[:8]
    brief = Path(home) / "briefs" / f"{lane_id}.md"
    brief.parent.mkdir(parents=True, exist_ok=True)
    brief.write_text(build_brief(home, config, items, now), encoding="utf-8")
    model = choice.model if models_mod.applies_to(choice, alias) else None
    effort = choice.effort if models_mod.applies_to(choice, alias) else None
    try:  # fails early (with the reason) when the alias has no command
        agentrun_mod.build_command(config, alias, proj.id, "x", model=model, effort=effort)
    except SystemExit as err:
        raise launch_mod.LaunchError(str(err.code))
    try:  # the manager's own standing item, in the plan before it starts (``tirith.plangate``)
        item = plangate_mod.ops_item(home, repo, proj.id, plangate_mod.MANAGER_ITEM, lane_id)[1]
    except plangate_mod.PlanGateError as err:
        raise launch_mod.LaunchError(f"the manager's plan item could not be added: {err}")
    argv = [*launch_mod.tirith_argv(), "lane", "run", "--id", lane_id, "--project", proj.id,
            "--class", HERO_CLASS, "--feature", FEATURE, "--brief", str(brief), "--workspace", str(repo),
            "--agent", alias, "--by-tirith", "--item", item, "--item-step", plangate_mod.OPS_STEP]
    if model:
        argv += ["--model", str(model)]
    if effort:
        argv += ["--effort", str(effort)]
    (spawn or launch_mod.spawn_detached)(argv, home, Path(home) / "logs" / f"{lane_id}.launch.log")
    return lane_id


def result_at(lane: dict) -> datetime | None:
    """When the manager's log got its ``result`` event (the end of its turn), else None.

    The time is the log's last write: the ``result`` event is the last thing the agent writes."""
    log = lane.get("log")
    if not log:
        return None
    try:
        with open(log, "rb") as fh:
            fh.seek(0, 2)
            fh.seek(max(0, fh.tell() - TAIL_BYTES))
            text = fh.read().decode("utf-8", errors="replace")
        mtime = os.path.getmtime(log)
    except OSError:
        return None
    for line in reversed(text.splitlines()):
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            event = json.loads(line)
        except ValueError:
            continue
        if isinstance(event, dict) and event.get("type") == "result":
            return datetime.fromtimestamp(mtime, timezone.utc)
    return None


def terminate_group(lane: dict) -> None:
    """SIGTERM the manager's process group. Lanes it started are detached (own session), so they survive."""
    pgid, pid = lane.get("pgid"), lane.get("pid")
    try:
        if isinstance(pgid, int) and pgid > 1 and pgid != os.getpgrp():
            os.killpg(pgid, signal.SIGTERM)
        elif isinstance(pid, int) and pid > 1 and pid != os.getpid():
            os.kill(pid, signal.SIGTERM)
    except (OSError, AttributeError):
        pass


def reap_finished(state: dict, lanes: list[dict], now: datetime, kill: Callable | None = None) -> bool:
    """Terminate a manager still alive ``LINGER_S`` after its ``result``; True when it did."""
    mine = {state.get("lane"), state.get("ended")}  # "ended": its turn is over, the process may linger
    lane = next((l for l in lanes if l.get("id") in mine and l.get("id")), None)
    if lane is None or lane.get("ended_at") or not pid_alive(lane.get("pid")):
        return False
    done = result_at(lane)
    if done is None or (now - done).total_seconds() < LINGER_S:
        return False
    (kill or terminate_group)(lane)
    return True


def running(state: dict, lanes: list[dict], now: datetime) -> str | None:
    """Id of the headless manager whose turn is not finished, else None.

    The turn is finished by the ``result`` event in its log or by the process exiting, whichever is first."""
    mine = state.get("lane")
    if not mine:
        return None
    lane = next((l for l in lanes if l.get("id") == mine), None)
    if lane is None:
        started = parse_iso(state.get("at"))
        return mine if started and (now - started).total_seconds() < SPAWN_GRACE_S else None
    if lane.get("ended_at") or not pid_alive_or_unset(lane) or result_at(lane) is not None:
        return None
    return mine


def pid_alive_or_unset(lane: dict) -> bool:
    return lane.get("pid") is None or pid_alive(lane.get("pid"))


def _failed_in_a_row(lanes: list[dict], since: str | None) -> list[dict]:
    """The newest ended manager lanes (after ``since``) while they all failed, newest first."""
    ended = [l for l in lanes if str(l.get("id") or "").startswith(LANE_PREFIX) and l.get("ended_at") and not l.get("stop")
             and not (since and str(l["ended_at"]) <= since)]
    ended.sort(key=lambda l: str(l.get("ended_at")), reverse=True)
    out = []
    for lane in ended:
        if lane.get("result") != "failed":
            break
        out.append(lane)
    return out


def _log_tail(lane: dict, lines: int = 12) -> str:
    try:
        text = Path(str(lane.get("log"))).read_text(encoding="utf-8", errors="replace")
    except OSError:
        return "(no log)"
    return "\n".join(text.strip().splitlines()[-lines:])[-1500:] or "(empty log)"


def _pause_check(home: Path, book: dict, lanes: list[dict], now: datetime) -> bool:
    """Pause the on-call manager after ``PAUSE_AFTER`` failed manager lanes in a row, with one clear
    manager-queue item (the log tail). The pause ends when that item is closed. True while paused."""
    if book.get("paused"):
        open_ids = {q["id"] for q in asks_mod.queue_items(home, only_open=True)}
        if book["paused"] in open_ids:
            return True
        book.pop("paused")
        book["since"] = now_iso(now)  # failures from before the pause no longer count
        book["tries"] = 0
        return False
    fails = _failed_in_a_row(lanes, book.get("since"))
    if len(fails) < PAUSE_AFTER:
        return False
    item_id = f"{PAUSE_ITEM}-{fails[0]['id']}"
    text = (f"The on-call manager is paused: its last {len(fails)} lanes failed "
            f"({', '.join(l['id'] for l in fails)}). Fix the cause, then close this item with "
            f"`tirith queue done {item_id}` to start it again.\nLog tail of {fails[0]['id']}:\n"
            f"```\n{_log_tail(fails[0])}\n```")
    asks_mod.queue_add(home, str(fails[0].get("project") or ""), str(fails[0]["id"]), text,
                       kind="manager-paused", item_id=item_id)
    book["paused"] = item_id
    return True


def run_oneshot(home: Path, config: Config, now: datetime, state: dict, lanes: list[dict],
                spawn: Callable | None = None, kill: Callable | None = None) -> bool:
    """One pass: start the headless manager when something is pending and nobody is attached.
    Returns True when ``state`` (``state["manager"]``) changed."""
    book = state.setdefault("manager", {})
    reap_finished(book, lanes, now, kill)
    live = running(book, lanes, now)
    if live:
        return False
    changed = False
    if book.get("lane"):  # it ended
        book["ended"] = book.pop("lane")
        changed = True
    if attached(home, now):
        return changed
    was = book.get("paused")
    if _pause_check(home, book, lanes, now):
        return True
    changed = changed or was != book.get("paused")
    items = pending_items(home, now)
    if not has_work(items):
        if book.get("key"):
            book.pop("key"), book.pop("tries", None)
            changed = True
        return changed
    items["evidence"] = book.get("evidence")  # from tirith.selfcheck: why this turn was started again
    key = pending_key(items)
    fresh = book.get("key") != key
    if fresh:
        book["key"], book["tries"] = key, 0
    last = parse_iso(book.get("at"))
    # new messages after a finished turn start the next turn at once; only a retry of the same set waits
    if book["tries"] >= MAX_TRIES or (last and not fresh and (now - last).total_seconds() < RETRY_AFTER_S):
        return True
    try:
        lane_id = launch(home, config, items, now, spawn)
    except launch_mod.LaunchError as err:
        book.update(at=now_iso(now), error=str(err))
        book["tries"] += 1
        return True
    except Exception as err:  # never let the manager stop the cycle
        book.update(at=now_iso(now), error=f"{type(err).__name__}: {err}")
        book["tries"] += 1
        return True
    book.update(lane=lane_id, at=now_iso(now), tries=book["tries"] + 1)
    turns = book.setdefault("turns", {})  # lane id -> the request ids this turn answers (the chat's live block)
    turns[lane_id] = [r["id"] for r in items["requests"]]
    for old in list(turns)[:-TURNS_KEPT]:
        turns.pop(old)
    book.pop("error", None)
    book.pop("evidence", None)
    flows_mod.append_flow(home, "tirith", f"agent:{lane_id}", "brief",
                          "On-call manager started for the owner's pending messages", session=None, at=now_iso(now))
    return True


def session_busy(home: Path) -> bool:
    """True while the persistent session is in a turn."""
    sess = msession_mod.current(home)
    return bool(sess is not None and sess.alive and sess.busy)


def public(home: Path, state: dict, lanes: list[dict], now: datetime) -> dict:
    """``state.manager.presence``: ``{mode: live|on_call, working, lane, session}`` for the chat header."""
    live = attached(home, now)
    sess = msession_mod.current(home)
    if sess is not None and sess.alive:
        return {"mode": "live" if live else "on_call", "working": sess.busy, "lane": sess.lane_id,
                "session": {"lane": sess.lane_id, "busy": sess.busy, "turns": sess.turns_done,
                            "context_tokens": sess.context_tokens, "asks": len(sess.open_asks())}}
    lane = running(state.get("manager", {}), lanes, now)
    return {"mode": "live" if live else "on_call", "working": bool(lane), "lane": lane}


def turn_views(home: Path, state: dict) -> list[dict]:
    """The session's turns, oldest first: the finished ones (their files) and the one running now."""
    sess = msession_mod.current(home)
    live = {v["id"]: v for v in (sess.views() if sess is not None else [])}
    out = []
    for rec in ((state.get("manager") or {}).get("session") or {}).get("turns") or []:
        view = live.get(rec["id"]) or msession_mod.read_turn(home, rec["id"])
        if view and view.get("requests"):  # a /compact turn answers no one
            out.append(view)
    return out


# -- the persistent session -------------------------------------------------------------------

RULES_SESSION = (*RULES[:-1],
                 "Before you finish a turn, append what you decided and launched with `tirith manager-note "
                 "\"<text>\"`; a later manager reads it. You stay running: the next messages arrive as new "
                 "user messages in this same conversation.")
RESTART_AFTER_S = 15.0  # times the number of fast deaths in a row
FALLBACK_AFTER = 2  # fast deaths in a row: the one-shot manager answers for FALLBACK_S, then the session is tried again
FALLBACK_S = 600.0
RESUME_FAIL_S = 20.0  # a resumed session that dies this fast is started fresh next time


def _book(state: dict) -> dict:
    return state.setdefault("manager", {}).setdefault("session", {})


def _parts(items: dict) -> dict[str, dict]:
    """Pending key -> the part of ``items`` it stands for (a request is offered again when it changes)."""
    out = {}
    for r in items["requests"]:
        out[f"{r['id']}:{len(r['answers'])}:{len(r.get('action_runs') or [])}"] = {"requests": [r]}
    for r in items["results"]:
        out[f"act:{r.get('wait')}:{r.get('at')}"] = {"results": [r]}
    for q in items["queue"]:
        out[f"q:{q['id']}"] = {"queue": [q]}
    return out


def _subset(items: dict, keys: list[str]) -> dict:
    parts = _parts(items)
    out: dict = {"requests": [], "results": [], "queue": [], "evidence": items.get("evidence")}
    for key in keys:
        for name, rows in parts.get(key, {}).items():
            out[name] += rows
    return out


def build_message(home: Path, config: Config, items: dict, now: datetime, primed: bool) -> str:
    """The user message for one turn: the whole brief for a conversation the session is new to (rules, plan,
    its notes), else only what is new."""
    if not primed:
        out = ["# You are the owner's manager (a persistent session run by Tirith)", "",
               "The owner talks to you through the Tirith chat, whether the app is open or not. This conversation "
               "stays open: more messages keep arriving here.", "", "## Rules", *(f"- {r}" for r in RULES_SESSION),
               "", *pending_sections(items, now), "", "## Plan", _plan_summary(config, home)]
        notes = notes_tail(home)
        if notes:
            out += ["", "## Your notes from earlier sessions", notes]
        return "\n".join(out) + "\n"
    return "\n".join(["New for you:", "", *pending_sections(items, now)]) + "\n"


def _end_session(home: Path, book: dict, now: datetime, intentional: str | None = None) -> None:
    """The session is gone: remember why, and whether the next start may resume."""
    msession_mod.register(home, None)
    book["ended_at"] = now_iso(now)
    book.pop("lane", None)
    if intentional:
        book["fast"] = 0
        return
    started = parse_iso(book.get("started_at"))
    fast = bool(started and (now - started).total_seconds() < RESUME_FAIL_S)
    if book.get("resumed") and fast:
        book["session_id"], book["primed"] = None, False  # resume did not work: fresh, with the handoff
    book["fast"] = (book.get("fast", 0) + 1) if fast else 0


def _stop_session(home: Path, book: dict, sess, now: datetime, by: str, reason: str) -> None:
    """End the session on purpose: its lane file says who and why, then its process group gets SIGTERM."""
    lane = read_json(lane_path(home, sess.lane_id), None)
    if isinstance(lane, dict):
        lane["stop"] = {"by": by, "reason": reason, "at": now_iso(now)}
        write_lane(home, lane)
    sess.terminate()
    _end_session(home, book, now, intentional=by)


def _start_session(home: Path, config: Config, items: dict, now: datetime, book: dict,
                   popen: Callable | None) -> None:
    from tirith import projects as projects_mod
    proj = _project_for(home, config, items)
    repo = projects_mod.real(proj.repo) if proj is not None and proj.repo else None
    if repo is None:
        raise launch_mod.LaunchError("no project with a repo folder to run the manager in")
    alias, choice = manager_alias(config)
    model = choice.model if models_mod.applies_to(choice, alias) else None
    effort = choice.effort if models_mod.applies_to(choice, alias) else None
    resume = book.get("session_id") if book.get("primed") else None
    try:
        command = msession_mod.chat_command(config, alias, model, effort, resume)
    except SystemExit as err:
        raise launch_mod.LaunchError(str(err.code))
    lane_id = LANE_PREFIX + uuid.uuid4().hex[:8]
    brief = Path(home) / "briefs" / f"{lane_id}.md"
    brief.parent.mkdir(parents=True, exist_ok=True)
    brief.write_text("The owner's manager: one persistent session run by Tirith; the chat feeds it.\n",
                     encoding="utf-8")
    try:  # the manager's own standing item, in the plan before it starts (``tirith.plangate``)
        item = plangate_mod.ops_item(home, repo, proj.id, plangate_mod.MANAGER_ITEM, lane_id)[1]
    except plangate_mod.PlanGateError as err:
        raise launch_mod.LaunchError(f"the manager's plan item could not be added: {err}")
    argv = [*launch_mod.tirith_argv(), "lane", "run", "--interactive", "--id", lane_id, "--project", proj.id,
            "--class", HERO_CLASS, "--feature", FEATURE, "--brief", str(brief), "--workspace", str(repo),
            "--as-agent", alias, "--by-tirith", "--item", item, "--item-step", plangate_mod.OPS_STEP, "--", *command]
    try:
        proc = msession_mod.spawn(argv, home, Path(home) / "logs" / f"{lane_id}.launch.log", popen)
    except OSError as err:
        raise launch_mod.LaunchError(f"could not start the manager session: {err}")
    project_id = proj.id

    def decide(tool, data):
        return msession_mod.decide(config, home, alias, tool, data, repo, project_id)

    msession_mod.register(home, msession_mod.Session(home, lane_id, proc, decide, repo, resumed=bool(resume)))
    book.update(lane=lane_id, project=project_id, started_at=now_iso(now), resumed=bool(resume))
    if not resume:
        book["primed"], book["session_id"] = False, None
    flows_mod.append_flow(home, "tirith", f"agent:{lane_id}", "brief",
                          "Manager session started" + (" (resumed)" if resume else ""), session=None, at=now_iso(now))


def _close_wait(home: Path, wait_id: str, now: datetime) -> None:
    from tirith.dispatch import close_owner_item
    close_owner_item(home, wait_id, now_iso(now))


def _settle_asks(home: Path, book: dict, sess, now: datetime) -> bool:
    """Permission questions the code did not settle: one Allow / Deny item each on the owner's list, and the
    owner's answer written back to the session."""
    from tirith.cli import add_wait
    changed = False
    waits = book.setdefault("asks", {})
    open_asks = sess.open_asks() if sess is not None else {}
    replies = {r["wait"]: r for r in inbox_mod.replies(home)}
    for rid, ask in open_asks.items():
        if rid not in waits:
            waits[rid] = f"perm-{rid}"
            add_wait(home, str(book.get("project") or ""), f"The manager asks to run **{ask['summary']}**. Allow it?",
                     wait_id=waits[rid], kind="approval",
                     why_owner="owner-decision", why_detail="permission",
                     options=[{"label": "Allow", "value": "allow"}, {"label": "Deny", "value": "deny"}])
            changed = True
        reply = replies.get(waits[rid])
        if reply is not None:
            sess.answer_ask(rid, reply.get("value") == "allow")
            _close_wait(home, waits.pop(rid), now)
            changed = True
    for rid in [r for r in waits if r not in open_asks]:  # the turn ended or the session died
        _close_wait(home, waits.pop(rid), now)
        changed = True
    return changed


def _finish_turns(home: Path, sess, book: dict) -> bool:
    """Turns that ended since the last pass: the answer lands in the inbox when the manager did not write it
    itself, and a request left open is offered again (``MAX_TRIES`` at most)."""
    changed = False
    done = book.setdefault("done_turns", [])
    pending = {r["id"] for r in inbox_mod.requests(home) if r["manager_pending"]}
    for view in sess.views():
        if not view["ended_at"] or view["id"] in done:
            continue
        done.append(view["id"])
        changed = True
        record = next((t for t in book.get("turns", []) if t["id"] == view["id"]), {})
        for key in record.get("keys", []):
            book.get("sent", {}).pop(key, None)
            if view.get("interrupted"):
                book.setdefault("tries", {})[key] = MAX_TRIES  # the owner stopped it: not offered again
        text = view.get("answer") or ""
        for rid in view["requests"]:
            if rid in pending and text.strip() and not view.get("error"):
                inbox_mod.add_answer(home, rid, text, by="manager", lane=sess.lane_id)
        book["turns_total"] = book.get("turns_total", 0) + 1
    del done[:-msession_mod.TURNS_KEPT]
    return changed


def _control(home: Path, book: dict, now: datetime) -> bool:
    """``tirith manager interrupt|stop|restart`` (``manager/control.jsonl``)."""
    changed = False
    for row in msession_mod.new_controls(home, book.get("control_seen")):
        book["control_seen"] = row["id"]
        changed = True
        sess = msession_mod.current(home)
        if row["op"] == "interrupt":
            if sess is not None:
                sess.interrupt()
            continue
        if row["op"] == "stop":
            book["stopped"] = now_iso(now)
        else:
            book.pop("stopped", None)
            if row.get("text") == "fresh":
                book["session_id"], book["primed"] = None, False
        if sess is not None:
            _stop_session(home, book, sess, now, "owner", "stopped" if row["op"] == "stop" else "restarted")
    return changed


def _compact_or_restart(home: Path, config: Config, book: dict, sess, now: datetime) -> bool:
    """An idle session whose context is large or whose conversation is long gets ``/compact`` once; if that did
    not help (or it gets long again) it is restarted fresh with a handoff. True when it did either."""
    turns = book.get("turns_total", 0)
    compacted = book.get("compact_turn")
    big = sess.context_tokens >= config.manager_context_tokens
    long = turns - (compacted or 0) >= config.manager_max_turns
    if not (big or long):
        return False
    if compacted is None:
        if not sess.send("/compact", [], now):
            return False
        book["compact_turn"] = turns + 1
        book.setdefault("turns", []).append({"id": sess.turn.id, "keys": []})
        return True
    _stop_session(home, book, sess, now, "manager", "fresh start with a handoff: the conversation got long")
    book.update(session_id=None, primed=False, compact_turn=None, turns_total=0)
    return True


def run_session(home: Path, config: Config, now: datetime, state: dict, lanes: list[dict],
                popen: Callable | None = None, spawn: Callable | None = None,
                kill: Callable | None = None) -> bool:
    """One pass of the persistent session: keep it alive, hand it what is pending, settle its questions."""
    book = _book(state)
    changed = _control(home, book, now)
    sess = msession_mod.current(home)
    if sess is not None and not sess.alive:
        changed = _finish_turns(home, sess, book) or changed  # its last turn ended with the process
        if not sess.got_event:  # it never said a word: say why in the state, the manager queue has the lane
            log = Path(home) / "logs" / f"{sess.lane_id}.launch.log"
            tail = log.read_text(encoding="utf-8", errors="replace").strip().splitlines()[-3:] if log.is_file() else []
            book["error"] = "the session exited at once: " + (" / ".join(tail)[-300:] or f"code {sess.exit_code}")
            print(f"tirith: manager session {sess.lane_id}: {book['error']}; the one-shot manager answers meanwhile",
                  file=sys.stderr, flush=True)
        died_at_once = not sess.got_event
        _end_session(home, book, now)
        sess, changed = None, True
        if died_at_once and not attached(home, now):  # the owner's message waits for no restart delay
            changed = run_oneshot(home, config, now, state, lanes, spawn, kill) or changed
    if sess is not None:
        if sess.session_id and book.get("session_id") != sess.session_id:
            book["session_id"] = sess.session_id
            changed = True
        changed = _finish_turns(home, sess, book) or changed
    changed = _settle_asks(home, book, sess, now) or changed
    if attached(home, now) or (sess is not None and sess.busy):
        return changed  # a terminal manager answers / the session is in a turn
    if sess is not None and _compact_or_restart(home, config, book, sess, now):
        return True
    ended = parse_iso(book.get("ended_at"))
    if sess is None and book.get("fast", 0) >= FALLBACK_AFTER and ended and (now - ended).total_seconds() < FALLBACK_S:
        return run_oneshot(home, config, now, state, lanes, spawn, kill) or changed  # a session that keeps dying
    items = pending_items(home, now)
    if not has_work(items):
        return changed
    stopped = parse_iso(book.get("stopped"))
    if stopped is not None:  # stopped from the chat: a message written after that wants it back
        newest = max((t for t in (parse_iso(r["at"]) for r in items["requests"]) if t), default=None)
        if newest is None or newest <= stopped:
            return changed
        book.pop("stopped")
        changed = True
    if _pause_check(home, book.setdefault("pause", {}), lanes, now):
        return True
    items["evidence"] = state.get("manager", {}).get("evidence")  # from tirith.selfcheck: why it is asked again
    sent, tries = book.setdefault("sent", {}), book.setdefault("tries", {})
    parts = _parts(items)
    for key in [k for k in tries if k not in parts]:  # settled: forgotten
        tries.pop(key), sent.pop(key, None)
    fresh = [k for k in parts if k not in sent and tries.get(k, 0) < MAX_TRIES]
    if not fresh:
        return changed
    if sess is None:
        ended = parse_iso(book.get("ended_at"))
        if ended and (now - ended).total_seconds() < RESTART_AFTER_S * max(1, book.get("fast", 0)):
            return changed
        try:
            _start_session(home, config, items, now, book, popen)
        except Exception as err:  # the session cannot start: the one-shot manager still answers
            book.update(error=f"{type(err).__name__}: {err}", ended_at=now_iso(now), fast=book.get("fast", 0) + 1)
            return run_oneshot(home, config, now, state, lanes, spawn, kill) or True
        book.pop("error", None)
        sess, changed = msession_mod.current(home), True
    sub = _subset(items, fresh)
    if not sess.send(build_message(home, config, sub, now, bool(book.get("primed"))),
                     [r["id"] for r in sub["requests"]], now):
        return changed
    book["primed"] = True
    state["manager"].pop("evidence", None)
    for key in fresh:
        sent[key] = now_iso(now)
        tries[key] = tries.get(key, 0) + 1
    book.setdefault("turns", []).append({"id": sess.turn.id, "keys": fresh})
    del book["turns"][:-msession_mod.TURNS_KEPT]
    return True


def run(home: Path, config: Config, now: datetime, state: dict, lanes: list[dict],
        spawn: Callable | None = None, kill: Callable | None = None, popen: Callable | None = None) -> bool:
    """One pass of the manager: the persistent session, or (``[manager] session = false``, or when the
    session cannot start) one print-mode turn per batch of pending messages."""
    if config.manager_session:
        return run_session(home, config, now, state, lanes, popen=popen, spawn=spawn, kill=kill)
    return run_oneshot(home, config, now, state, lanes, spawn, kill)
