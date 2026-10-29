"""Commands for the owner are structured actions, never text to copy.

An agent that wants the owner to run a command declares it with ``--run`` on ``tirith inbox answer`` or
``tirith wait add``; the page shows a "Run" button. Text that carries a shell command is refused
(``check_text``). Execution happens on the owner's click only: the Tauri app (``run_action``) or
``tirith action run`` (the ``tirith://run`` link). See ``docs/CONTRACT-actions.md``.
"""

from __future__ import annotations

import re
import secrets
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import parse_qs, urlencode, urlsplit

from tirith import tirithcmd
from tirith.home import now_iso, parse_iso
from tirith.store import append_jsonl, read_json, read_jsonl, tail_lines, write_json_atomic

ACTIONS_DIR = "actions"
RUN_KIND = "action-run"
COMMAND_MAX = 2000
LABEL_MAX = 120
LOG_TAIL = 20
STALE_S = 3600
PASSED_REASON = "action-passed"  # ``done_reason`` of an owner item closed because its actions passed
SHELL_LANGS = {"sh", "bash", "zsh", "shell", "console", "shell-session", "shellsession", "terminal", "fish"}
COMMAND_WORDS = {
    "cd", "uv", "uvx", "npm", "npx", "pnpm", "yarn", "git", "cargo", "pip", "pip3", "python", "python3",
    "brew", "sudo", "make", "tirith", "curl", "wget", "chmod", "mkdir", "rm", "cp", "mv", "open", "bash",
    "sh", "zsh", "export", "source", "kill", "launchctl", "docker", "gh", "node", "ln", "touch", "cat",
}
PROMPTS = ("$ ", "% ", "./", "~/")
FENCE = re.compile(r"^[ \t]*(```+|~~~+)[ \t]*([^\n`]*)\n(.*?)^[ \t]*\1[ \t]*$", re.S | re.M)
SPAN = re.compile(r"(?<!`)(`+)(?!`)(.+?)(?<!`)\1(?!`)")


class ActionError(Exception):
    """An action that cannot be recorded or run; the message says why."""


class OwnerCommandError(ActionError):
    """Text that carries a command for the owner instead of an action."""


# -- the hard rule ----------------------------------------------------------------------------


def looks_like_command(line: str) -> bool:
    line = line.strip()
    if not line or line.startswith("#"):
        return False
    return line.startswith(PROMPTS) or line.split()[0] in COMMAND_WORDS


def _is_command_span(code: str) -> bool:
    """A code span that is a whole command line: a command word (or prompt) and at least one argument."""
    code = code.strip()
    return " " in code and looks_like_command(code)


def find_owner_command(text: str) -> str | None:
    """The first shell command found in markdown ``text`` (fenced block or full command-line code span)."""
    text = str(text or "")
    for match in FENCE.finditer(text):
        info = match.group(2).strip().lower()
        lang = info.split()[0] if info else ""
        lines = [ln for ln in match.group(3).splitlines() if ln.strip()]
        if lang in SHELL_LANGS:
            return (lines[0] if lines else lang).strip()
        if not lang:
            found = next((ln for ln in lines if looks_like_command(ln)), None)
            if found:
                return found.strip()
    for match in SPAN.finditer(FENCE.sub("", text)):
        if _is_command_span(match.group(2)):
            return match.group(2).strip()
    return None


def refusal(kind: str = "answer") -> str:
    if kind == "wait":
        fix = 'tirith wait add "<text without the command>" --run "<command>" --label "<what it does>"'
    else:
        fix = ('tirith inbox answer <id> "<text without the command>" --run "<command>" '
               '--label "<what it does>"')
    return "tirith: owner-command: commands for the owner must be actions. Do this instead: " + fix


def check_text(text: str, kind: str = "answer") -> None:
    """Raise ``OwnerCommandError`` (agent-style message) when ``text`` carries a command for the owner."""
    if find_owner_command(text) is not None:
        raise OwnerCommandError(refusal(kind))


# -- tirith://run links -----------------------------------------------------------------------


def parse_run_url(url: str) -> str:
    """``tirith://run?action=<id>&token=..`` -> the action id; the link never carries command text."""
    parts = urlsplit(str(url).strip())
    if parts.scheme.lower() != "tirith" or (parts.netloc or parts.path.strip("/")).lower() != "run":
        raise ActionError("not a tirith://run link: " + repr(url))
    return parse_qs(parts.query).get("action", [""])[-1]


def build_run_url(action_id: str, token: str) -> str:
    return "tirith://run?" + urlencode({"action": action_id, "token": token})


# -- declaring --------------------------------------------------------------------------------


def new_id() -> str:
    return "ac-" + secrets.token_hex(4)


def make_actions(runs: list[str] | None, labels: list[str] | None = None, cwds: list[str] | None = None,
                 created_by: str = "manager", at: str | None = None) -> list[dict]:
    """``--run/--label/--cwd`` (repeatable, matched by position) -> action dicts.

    A single ``--cwd`` applies to every action; a missing label falls back to the command."""
    runs, labels, cwds = list(runs or []), list(labels or []), list(cwds or [])
    if len(labels) > len(runs) or len(cwds) > len(runs):
        raise ActionError("--label and --cwd need a --run each")
    out = []
    for index, command in enumerate(runs):
        command = str(command).strip()
        if not command:
            raise ActionError("--run needs a command")
        if len(command) > COMMAND_MAX:
            raise ActionError("--run is longer than " + str(COMMAND_MAX) + " characters")
        label = (labels[index] if index < len(labels) else "").strip() or command.splitlines()[0]
        cwd = cwds[index] if index < len(cwds) else (cwds[0] if len(cwds) == 1 else "")
        out.append({"id": new_id(), "label": label[:LABEL_MAX], "command": command,
                    "cwd": str(cwd or "").strip() or None, "created_by": created_by, "at": at or now_iso()})
    return out


# -- looking up -------------------------------------------------------------------------------


GENERATED_FILE = "generated-actions.json"


def generated(home: Path) -> dict[str, dict]:
    """Actions of items the collector itself raises (guard checks), by id; see ``register_generated``."""
    data = read_json(Path(home) / GENERATED_FILE, {})
    return {str(k): v for k, v in data.items() if isinstance(v, dict) and v.get("command")} \
        if isinstance(data, dict) else {}


def register_generated(home: Path, project: str, action_id: str, label: str, command: str,
                       cwd: str | None) -> dict:
    """Store (idempotently) a collector-made action under a stable id; returns it for the item.

    The command text only ever comes from this file, never from the page."""
    action = {"id": action_id, "label": label[:LABEL_MAX], "command": tirithcmd.rewrite(command, home), "cwd": cwd,
              "created_by": "tirith", "at": now_iso(), "project": project}
    known = generated(home)
    old = known.get(action_id)
    if old is None or any(old.get(k) != action[k] for k in ("label", "command", "cwd", "project")):
        known[action_id] = action
        write_json_atomic(Path(home) / GENERATED_FILE, known)
    else:
        action = old
    return {k: v for k, v in action.items() if k != "project"}


def _rows(home: Path):
    """``(action, project, source)`` for every action in inbox.jsonl answers and waits.jsonl."""
    inbox = read_jsonl(Path(home) / "inbox.jsonl")
    projects = {}
    for row in inbox:
        if row.get("kind") in ("finish", "ask", "chat") and row.get("id"):
            projects.setdefault(str(row["id"]), str(row.get("project") or ""))
    for row in inbox:
        if row.get("kind") == "answer":
            for action in row.get("actions") or []:
                if isinstance(action, dict) and action.get("id"):
                    yield action, projects.get(str(row.get("id")), ""), "inbox:" + str(row.get("id"))
    for action_id, action in generated(home).items():
        yield {**action, "id": action_id}, str(action.get("project") or ""), "generated:" + action_id
    latest: dict[str, dict] = {}
    for row in read_jsonl(Path(home) / "waits.jsonl"):
        if row.get("id"):
            latest[str(row["id"])] = row
    for wait_id, row in latest.items():
        for action in row.get("actions") or []:
            if isinstance(action, dict) and action.get("id"):
                yield action, str(row.get("project") or ""), "wait:" + wait_id


def find_action(home: Path, action_id: str) -> dict | None:
    """The action with ``project`` and ``source`` added, or None. The command text only ever comes from here."""
    for action, project, source in _rows(home):
        if action["id"] == action_id:
            return {**action, "project": project, "source": source}
    return None


def log_path(home: Path, action_id: str) -> Path:
    return Path(home) / ACTIONS_DIR / (action_id + ".log")


def run_rows(home: Path) -> list[dict]:
    return [r for r in read_jsonl(Path(home) / "inbox.jsonl") if r.get("kind") == RUN_KIND]


def last_run(home: Path, action_id: str, rows: list[dict] | None = None) -> dict | None:
    """``{status, exit_code, at, log_tail}`` from the latest ``action-run`` row, or None if never run."""
    rows = rows if rows is not None else run_rows(home)
    latest = next((r for r in reversed(rows) if r.get("action_id") == action_id), None)
    if latest is None:
        return None
    return {"status": latest.get("status"), "exit_code": latest.get("exit_code"), "at": latest.get("at"),
            "log_tail": tail_lines(log_path(home, action_id), LOG_TAIL)}


def list_actions(home: Path) -> list[dict]:
    rows, seen = run_rows(home), {}
    for action, project, source in _rows(home):
        seen.setdefault(action["id"], {**action, "project": project, "source": source,
                                       "last_run": last_run(home, action["id"], rows)})
    return list(seen.values())


def dedupe_actions(actions: list[dict]) -> list[dict]:
    """Each action once: unique by id, then by command + label (the first one wins)."""
    out, ids, shapes = [], set(), set()
    for action in actions:
        shape = (str(action.get("command") or ""), str(action.get("label") or ""))
        if str(action.get("id") or "") in ids or shape in shapes:
            continue
        ids.add(str(action.get("id") or ""))
        shapes.add(shape)
        out.append(action)
    return out


def attach_last_runs(home: Path, containers: list[dict]) -> None:
    """Set ``last_run`` on every action of ``containers`` (answers or waits) in place."""
    rows = run_rows(home)
    for container in containers:
        for action in container.get("actions") or []:
            action["last_run"] = last_run(home, action["id"], rows)


def all_passed(actions: list[dict]) -> bool:
    """True when there is at least one action and each one's last run (``attach_last_runs``) passed."""
    return bool(actions) and all((a.get("last_run") or {}).get("status") == "passed" for a in actions)


# -- running ----------------------------------------------------------------------------------


def is_running(home: Path, action_id: str) -> bool:
    """A ``running`` row counts for ``STALE_S`` seconds (a run whose process died must not block forever)."""
    run = last_run(home, action_id)
    if not run or run["status"] != "running":
        return False
    started = parse_iso(run["at"])
    return started is None or (datetime.now(timezone.utc) - started).total_seconds() < STALE_S


def run_action(home: Path, action_id: str, project_folder: Path | None = None,
               timeout: float | None = None) -> dict:
    """Run the stored command with ``/bin/sh -c``; log to ``<home>/actions/<id>.log``; record the rows."""
    action = find_action(home, action_id)
    if action is None:
        raise ActionError("no action " + action_id)
    if is_running(home, action_id):
        raise ActionError("action " + action_id + " is already running")
    cwd = Path(str(action["cwd"])).expanduser() if action.get("cwd") else (project_folder or Path.home())
    if not cwd.is_dir():
        raise ActionError("the folder " + str(cwd) + " does not exist")
    log = log_path(home, action_id)
    log.parent.mkdir(parents=True, exist_ok=True)
    inbox = Path(home) / "inbox.jsonl"
    append_jsonl(inbox, {"kind": RUN_KIND, "action_id": action_id, "status": "running",
                         "exit_code": None, "at": now_iso()})
    try:
        with log.open("wb") as out:
            code = subprocess.run(["/bin/sh", "-c", tirithcmd.rewrite(action["command"], home)], cwd=cwd, stdin=subprocess.DEVNULL,
                                  stdout=out, stderr=subprocess.STDOUT, timeout=timeout).returncode
    except (OSError, subprocess.TimeoutExpired) as error:
        with log.open("ab") as out:
            out.write(("tirith: " + str(error) + "\n").encode())
        code = 127
    status = "passed" if code == 0 else "failed"
    append_jsonl(inbox, {"kind": RUN_KIND, "action_id": action_id, "status": status,
                         "exit_code": code, "at": now_iso()})
    return {"action_id": action_id, "status": status, "exit_code": code}
