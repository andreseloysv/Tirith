"""A manager that lives inside Tirith: the owner can close the terminal and keep talking in the chat.

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
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable

from tirith import agentrun as agentrun_mod
from tirith import asks as asks_mod
from tirith import flows as flows_mod
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith import models as models_mod
from tirith.home import Config, now_iso, parse_iso
from tirith.store import read_json, write_json_atomic

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

RULES = (
    "You are the owner's manager, started by Tirith because the owner is not at a terminal. The owner talks to "
    "you only through the Tirith chat.",
    "Delegate everything to sub-agents (`tirith lane run`, `tirith launch --project P --step N`). Never edit "
    "code or files of a project yourself.",
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


def build_brief(home: Path, config: Config, items: dict, now: datetime) -> str:
    def age(at) -> str:
        parsed = parse_iso(at)
        return f"{int((now - parsed).total_seconds())}s ago" if parsed else "?"

    out = ["# Lane: Manager (on call)", "", "## Why",
           "The owner closed the terminal; Tirith started you to answer what is waiting in the chat and act on it.", "",
           "## Rules", *(f"- {r}" for r in RULES), "", "## Pending messages"]
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
    argv = [*launch_mod.tirith_argv(), "lane", "run", "--id", lane_id, "--project", proj.id,
            "--class", HERO_CLASS, "--feature", FEATURE, "--brief", str(brief), "--workspace", str(repo),
            "--agent", alias, "--by-tirith"]
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
    ended = [l for l in lanes if str(l.get("id") or "").startswith(LANE_PREFIX) and l.get("ended_at")
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


def run(home: Path, config: Config, now: datetime, state: dict, lanes: list[dict],
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
    book.pop("error", None)
    book.pop("evidence", None)
    flows_mod.append_flow(home, "tirith", f"agent:{lane_id}", "brief",
                          "On-call manager started for the owner's pending messages", session=None, at=now_iso(now))
    return True


def public(home: Path, state: dict, lanes: list[dict], now: datetime) -> dict:
    """``state.manager.presence``: ``{mode: live|on_call, working, lane}`` for the chat header."""
    live = attached(home, now)
    lane = running(state.get("manager", {}), lanes, now)
    return {"mode": "live" if live else "on_call", "working": bool(lane), "lane": lane}
