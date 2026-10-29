"""A concrete, pre-filled how-to for an owner item: steps, a ``done_check`` and what is already done.

``howto = {"steps": [{"text", "url"?, "terminal"?}], "done_check": "<command>", "already_done": ["fact"]}``.
The owner gate (``ownergate.admit``) refuses a how-to that still has placeholders (``<TEAMID>``,
``YOUR_...``, ``xxx``) or a command that cannot run as it stands, and sends the item back to the manager
queue ("make it concrete: look up ..."). A step with ``terminal`` gets an "Open in Terminal" button; Tirith
runs ``done_check`` every ``CHECK_EVERY_S`` seconds for each open item and closes it when it exits 0
(``check_open``). See ``docs/CONTRACT-inbox.md`` ("Concrete how-to").
"""

from __future__ import annotations

import json
import re
import shlex
import subprocess
from datetime import datetime, timezone
from pathlib import Path

from tirith import timeouts as timeouts_mod
from tirith import flows as flows_mod
from tirith.home import now_iso, parse_iso
from tirith.store import append_jsonl, read_json, read_jsonl, write_json_atomic

DONE_REASON = "howto-done"  # ``done_reason`` of an owner item closed because its ``done_check`` passed
CHECK_EVERY_S = 180.0
CHECK_TIMEOUT_S = 30.0
STATE_FILE = "howto-checks.json"
STEPS_MAX = 12
TEXT_MAX = 600
COMMAND_MAX = 2000
FACTS_MAX = 12
MAKE_CONCRETE = "make it concrete: look up "

PLACEHOLDER = re.compile(
    r"<[A-Za-z][A-Za-z0-9 _.-]*>|\bYOUR[_-][A-Za-z0-9_-]+|\b[Xx]{3,}\b|\{\{[^}]*\}\}|\[(?:your|the)[ _-][^\]]*\]"
    r"|\bTODO\b|\bFIXME\b|\.\.\.", re.IGNORECASE)


class HowtoError(ValueError):
    """A how-to the owner gate sends back; the message says what to look up."""


def has_placeholder(text: str) -> str | None:
    """The first placeholder in ``text`` (``<TEAMID>``, ``YOUR_ID``, ``xxx``...), else ``None``."""
    found = PLACEHOLDER.search(str(text or ""))
    return found.group(0) if found else None


def _runnable(command: str) -> str | None:
    """Why ``command`` cannot run as it is, else ``None`` (a placeholder or broken quoting)."""
    if not command.strip():
        return "it is empty"
    holder = has_placeholder(command)
    if holder:
        return f"it still has the placeholder {holder}"
    try:
        shlex.split(command)
    except ValueError as error:
        return f"it does not parse ({error})"
    return None


def clean(raw) -> dict | None:
    """The normalised how-to (``steps``, ``done_check``, ``already_done``), ``None`` for an empty one.
    Raises ``HowtoError`` for a malformed or non-concrete one."""
    if raw in (None, "", {}, []):
        return None
    if not isinstance(raw, dict):
        raise HowtoError(MAKE_CONCRETE + "the facts and give howto as {steps, done_check, already_done}")
    steps = []
    for index, step in enumerate(raw.get("steps") or [], 1):
        if isinstance(step, str):
            step = {"text": step}
        if not isinstance(step, dict) or not str(step.get("text") or "").strip():
            raise HowtoError(f"{MAKE_CONCRETE}step {index}: every step needs a text")
        text = " ".join(str(step["text"]).split())[:TEXT_MAX]
        out = {"text": text}
        url, terminal = str(step.get("url") or "").strip(), str(step.get("terminal") or "").strip()
        if url:
            out["url"] = url
        if terminal:
            out["terminal"] = terminal[:COMMAND_MAX]
        for key, value in (("text", text), ("url", url), ("terminal", terminal)):
            holder = has_placeholder(value)
            if holder:
                raise HowtoError(f"{MAKE_CONCRETE}the real value of {holder} (step {index} {key}); "
                                 "read it from the machine, never ask the owner for it")
        if terminal:
            problem = _runnable(terminal)
            if problem:
                raise HowtoError(f"{MAKE_CONCRETE}step {index}: the command cannot run as it is, {problem}")
        if url and not re.match(r"^(?:https?|tirith)://\S+$", url):
            raise HowtoError(f"{MAKE_CONCRETE}step {index}: the url must be a full http(s) address")
        steps.append(out)
    if len(steps) > STEPS_MAX:
        raise HowtoError(f"{MAKE_CONCRETE}which {STEPS_MAX} steps matter; skip what is already done")
    done_check = str(raw.get("done_check") or "").strip()
    if done_check:
        problem = _runnable(done_check)
        if problem:
            raise HowtoError(f"{MAKE_CONCRETE}done_check: the command cannot run as it is, {problem}")
    facts = [" ".join(str(f).split())[:TEXT_MAX] for f in raw.get("already_done") or [] if str(f).strip()]
    if not steps and not done_check and not facts:
        return None
    return {"steps": steps, "done_check": done_check or None, "already_done": facts[:FACTS_MAX]}


def parse(value) -> dict | None:
    """``clean`` for a dict, or for the JSON text of ``--howto`` (a path to a JSON file is read)."""
    if isinstance(value, str):
        text = value.strip()
        if not text:
            return None
        try:
            if not text.startswith("{") and Path(text).is_file():
                text = Path(text).read_text(encoding="utf-8")
            value = json.loads(text)
        except (OSError, ValueError) as error:
            raise HowtoError(f"{MAKE_CONCRETE}howto must be JSON {{steps, done_check, already_done}} ({error})")
    return clean(value)


def terminal_command(howto: dict | None, index: int) -> str | None:
    """The ``terminal`` command of step ``index`` (0-based) of a stored how-to, else ``None``."""
    steps = (howto or {}).get("steps") or []
    if not isinstance(index, int) or not 0 <= index < len(steps):
        return None
    return str(steps[index].get("terminal") or "").strip() or None


def applescript(command: str) -> str:
    """The AppleScript that opens macOS Terminal with ``command`` typed in (the Tauri app builds the same text)."""
    escaped = command.replace("\\", "\\\\").replace('"', '\\"')
    return f'tell application "Terminal"\nactivate\ndo script "{escaped}"\nend tell'


# -- the done check ---------------------------------------------------------------------------


def _run_check(command: str) -> bool:
    try:
        return timeouts_mod.run(["/bin/sh", "-c", command], "generic", stdin=subprocess.DEVNULL,
                                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                timeout=CHECK_TIMEOUT_S).returncode == 0
    except OSError:
        return False


def check_open(home: Path, now: datetime | None = None, runner=_run_check) -> list[str]:
    """Run ``done_check`` of each open owner item whose last check is older than ``CHECK_EVERY_S``; an item
    whose check passes closes (``done_reason: "howto-done"``) and the manager is told. Returns the closed ids."""
    now = (now or datetime.now(timezone.utc)).astimezone(timezone.utc)
    path, state = Path(home) / "waits.jsonl", read_json(Path(home) / STATE_FILE, {})
    state = state if isinstance(state, dict) else {}
    latest: dict[str, dict] = {}
    for row in read_jsonl(path):
        if row.get("id"):
            latest[str(row["id"])] = row
    closed, changed = [], False
    for wait_id, row in latest.items():
        howto = row.get("howto") if isinstance(row.get("howto"), dict) else {}
        check = str(howto.get("done_check") or "")
        if row.get("done_at") or not check:
            continue
        last = parse_iso(state.get(wait_id))
        if last is not None and (now - last).total_seconds() < CHECK_EVERY_S:
            continue
        state[wait_id], changed = now_iso(now), True
        if not runner(check):
            continue
        at = now_iso(now)
        append_jsonl(path, {**row, "done_at": at, "done_reason": DONE_REASON})
        flows_mod.append_flow(home, "tirith", "manager", "alert",
                              f"Owner item {wait_id} is done: its check passed ({check}). {row.get('text', '')}"[:600],
                              ref=wait_id, session=None, project=row.get("project"), at=at)
        closed.append(wait_id)
    if changed:
        write_json_atomic(Path(home) / STATE_FILE, state)
    return closed


# -- the rules for every agent that writes an owner item --------------------------------------

RULES = (
    "Before you write an owner item, look up every fact the machine can tell (installed certificates and "
    "identities with `security find-identity -v -p codesigning`, ids and settings in config files, what "
    "already exists). Skip every step that is already done and list it under `already_done`.",
    "Pre-fill every command with the real values you found: never `<TEAMID>`, `YOUR_...` or `xxx`. A "
    "placeholder sends the item back to the manager (\"make it concrete: look up ...\").",
    "Give the owner item a `--howto` JSON: `steps` (each `text`, optional `url` for the one page to open, "
    "optional `terminal` for the one command to run) and a `done_check` command that exits 0 once it is done "
    "(e.g. `xcrun notarytool history --keychain-profile tirith-notary`); Tirith runs it every few minutes and "
    "closes the item itself.",
    "Never ask for a secret in chat or put one in a command. A command that needs a password is a `terminal` "
    "step: the owner types the secret only into the Terminal prompt.",
)
