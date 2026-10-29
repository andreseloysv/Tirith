"""``tirith hook``: agent hook calls become standard Tirith events.

Agent CLIs run a command on lifecycle events and pass a JSON payload on stdin.
``tirith hook --agent <name>`` turns each call into one line of
``events.jsonl`` (``kind: "hook"``) tagged with the lane, session and project,
so the owner sees tools, files, commands, sub-agents and prompts live.

The command is fast and never breaks the agent: no network, append only,
every exception swallowed, exit 0.
"""

from __future__ import annotations

import json
import os
import re
import shlex
import shutil
import sys
from datetime import datetime, timezone
from pathlib import Path

from tirith import dispatch, gates, managerd
from tirith import projects as projects_mod
from tirith.home import load_config, now_iso
from tirith.store import append_jsonl, read_json, write_json_atomic

EVENTS_FILE = "events.jsonl"
REQUESTS_DIR = "requests"
SUMMARY_MAX = 160
PROMPT_EVENTS = ("UserPromptSubmit",)
SUBAGENT_TOOLS = ("Task", "Agent")
INSTALL_EVENTS = ("SessionStart", "UserPromptSubmit", "PreToolUse", "PostToolUse",
                  "SubagentStop", "Stop")

# -- adapters: payload -> event fields --------------------------------------------------------


def _clip(text, limit: int = SUMMARY_MAX) -> str:
    """First non-empty line of ``text``, cut to ``limit`` characters."""
    for line in str(text or "").splitlines():
        line = line.strip()
        if line:
            return line if len(line) <= limit else line[:limit - 1] + "…"
    return ""


def _rel(path, cwd) -> str:
    """``path`` relative to ``cwd`` when it lives below it."""
    text = str(path or "")
    base = str(cwd or "").rstrip("/")
    return text[len(base) + 1:] if base and text.startswith(base + "/") else text


def _tool_summary(tool: str, tool_input, cwd) -> str:
    """Short human text for one tool call: ``Edit web/js/ui.js``, ``Bash: uv run pytest``."""
    data = tool_input if isinstance(tool_input, dict) else {}
    if tool in SUBAGENT_TOOLS:
        what = data.get("description") or data.get("prompt")
        kind = data.get("subagent_type")
        return "Sub-agent" + (f" {kind}" if kind else "") + (f": {_clip(what)}" if what else "")
    if tool == "Bash":
        return "Bash: " + _clip(data.get("command") or data.get("cmd"))
    path = data.get("file_path") or data.get("path") or data.get("notebook_path")
    if path:
        return f"{tool} {_rel(path, cwd)}"
    for key in ("pattern", "query", "url", "command"):
        if data.get(key):
            return f"{tool}: {_clip(data[key])}"
    return tool


def _pick(payload: dict, *keys):
    for key in keys:
        if payload.get(key) not in (None, ""):
            return payload[key]
    return None


def adapt_anthropic(payload: dict, event: str | None = None) -> dict:
    """Anthropic's coding CLI payload (``hook_event_name``, ``tool_name``, ``tool_input``, ``prompt``)."""
    name = str(payload.get("hook_event_name") or event or "")
    cwd = payload.get("cwd")
    tool = str(payload.get("tool_name") or "") or None
    prompt = payload.get("prompt")
    if name == "UserPromptSubmit":
        summary = _clip(prompt) or "Prompt"
    elif tool:
        summary = _tool_summary(tool, payload.get("tool_input"), cwd)
    elif name == "SessionStart":
        summary = "Session " + str(payload.get("source") or "start")
    elif name in ("Stop", "SubagentStop"):
        summary = _clip(payload.get("last_assistant_message")) or name
    else:
        summary = name
    return {"event": name, "tool": tool, "summary": summary, "cwd": cwd,
            "agent_session_id": payload.get("session_id"),
            "prompt": prompt if name in PROMPT_EVENTS else None}


# Other agent CLIs name their tools differently (lowercase shell tool, patch tools); the rules know one set.
TOOL_ALIASES = {"bash": "Bash", "shell": "Bash", "shell_command": "Bash", "exec_command": "Bash",
                "local_shell": "Bash", "run_shell_command": "Bash",
                "edit": "Edit", "write": "Write", "create": "Write", "str_replace": "Edit",
                "str_replace_editor": "Edit", "apply_patch": "Edit", "multiedit": "MultiEdit"}


def canonical_tool(tool) -> str | None:
    """The tool name the rules use (``bash`` -> ``Bash``); unknown names stay as they are."""
    if not tool:
        return None
    return TOOL_ALIASES.get(str(tool).lower(), str(tool))


def adapt_generic(payload: dict, event: str | None = None) -> dict:
    """Any agent: reads the common field names (also used for Muse Code, see ``ADAPTERS``)."""
    name = str(_pick(payload, "hook_event_name", "event", "event_name", "hook") or event or "")
    cwd = _pick(payload, "cwd", "workdir", "working_directory")
    tool = canonical_tool(_pick(payload, "tool_name", "toolName", "tool"))
    tool_input = _pick(payload, "tool_input", "input", "arguments", "args", "toolArgs")
    prompt = _pick(payload, "prompt", "user_prompt", "message", "text")
    if name == "UserPromptSubmit":
        summary = _clip(prompt) or "Prompt"
    elif tool:
        summary = _tool_summary(tool, tool_input, cwd)
    else:
        summary = _clip(_pick(payload, "last_assistant_message", "summary")) or name
    return {"event": name, "tool": tool, "summary": summary, "cwd": cwd,
            "agent_session_id": _pick(payload, "session_id", "sessionId", "session"),
            "prompt": prompt if name in PROMPT_EVENTS else None}


# Agents are named by alias in code ("anthropic" for Anthropic's coding CLI, "muse" for Muse Code);
# the real executable and settings path of each alias live in the owner's config.toml
# ([agents.<alias>] command = "...", settings = "..."), written at setup, never in this repo.
# Muse Code lists the same event names as the anthropic CLI, but its docs (dev.meta.ai
# /docs/muse-code/extending) give no stdin payload fields: the generic reader covers it.
# Codex and Copilot CLI send the same snake_case payload as the anthropic CLI to PascalCase hooks.
ADAPTERS = {"anthropic": adapt_anthropic, "muse": adapt_generic, "codex": adapt_generic,
            "copilot": adapt_generic}


def adapt(agent: str, payload, event: str | None = None) -> dict:
    """Adapter for ``agent`` (generic for unknown names); non-dict payloads become empty."""
    data = payload if isinstance(payload, dict) else {}
    return ADAPTERS.get(agent, adapt_generic)(data, event)


# -- recording --------------------------------------------------------------------------------


def _project_for(home: Path, cwd) -> str | None:
    """Project id whose folder is the longest prefix of ``cwd`` (as the collector does)."""
    if not cwd:
        return None
    try:
        items = projects_mod.entries(load_config(home), projects_mod.load_auto(home))
        found = projects_mod.match(cwd, items)
        return found.id if found else None
    except Exception:
        return None


def _save_request(home: Path, session_id, prompt, ts: str) -> None:
    """Remember the owner's last request of this agent session for lanes to link to."""
    if not session_id or not prompt:
        return
    safe = re.sub(r"[^A-Za-z0-9._-]", "_", str(session_id))
    write_json_atomic(home / REQUESTS_DIR / f"{safe}.json",
                      {"agent_session_id": str(session_id), "prompt": str(prompt), "at": ts})


def build_event(home: Path, agent: str, payload, event: str | None = None,
                environ=None) -> tuple[dict, str | None]:
    """The events.jsonl row for one hook call plus the prompt to store (if any)."""
    env = os.environ if environ is None else environ
    fields = adapt(agent, payload, event)
    cwd = fields["cwd"] or os.getcwd()
    row = {"kind": "hook", "ts": now_iso(), "agent": agent, "event": fields["event"],
           "lane": env.get("TIRITH_LANE") or None,
           "session": projects_mod.current_session(home),
           "project": _project_for(home, cwd),
           "tool": fields["tool"], "summary": fields["summary"],
           "agent_session_id": fields["agent_session_id"], "cwd": str(cwd)}
    return row, fields["prompt"]


def record(home: Path, agent: str, payload, event: str | None = None) -> dict:
    """Append the event for one hook call (and the prompt, for ``UserPromptSubmit``)."""
    row, prompt = build_event(home, agent, payload, event)
    append_jsonl(home / EVENTS_FILE, row)
    _save_request(home, row["agent_session_id"], prompt, row["ts"])
    return row


def _manager_inbox(agent: str, event: dict, home: Path, environ=None) -> int:
    """Hold the manager session to the inbox (``tirith.dispatch``); other sessions are never touched.

    ``SessionStart`` / ``UserPromptSubmit``: the pending requests as ``additionalContext``. ``Stop``: exit 2
    with the list on stderr while a request older than 60 s is unanswered (3 blocks at most per set)."""
    name = event.get("event")
    if agent != "anthropic" or name not in ("SessionStart", "UserPromptSubmit", "PostToolUse", "Stop"):
        return 0
    env = os.environ if environ is None else environ
    config = load_config(home)
    if not config.dispatch_enabled or not dispatch.is_manager(config, env, event.get("cwd")):
        return 0
    now = datetime.now(timezone.utc)
    managerd.heartbeat(home, event.get("agent_session_id"), now)  # the live session is attached while this keeps coming
    if name == "PostToolUse":
        return 0
    if name == "Stop":
        reason = dispatch.manager_stop(home, now)
        if reason is None:
            return 0
        dispatch.record_event(home, name, reason, event.get("ts") or now_iso())
        print(reason, file=sys.stderr)
        return 2
    text = dispatch.manager_context(home, now)
    if text:
        dispatch.record_event(home, name, text, event.get("ts") or now_iso())
        print(json.dumps({"hookSpecificOutput": {"hookEventName": name, "additionalContext": text}}))
    return 0


def decide(agent: str, event: dict, payload, home: Path | None = None) -> int:
    """Block a ``PreToolUse`` call that breaks a rule (``tirith.gates``): exit 2, message on stderr.

    Each violation is also recorded as a ``kind: "violation"`` event. Only ``[gates] enabled = false``
    in config.toml switches this off; no environment variable does.
    """
    if home is not None and event.get("event") != "PreToolUse":
        return _manager_inbox(agent, event, home)
    if event.get("event") != "PreToolUse" or home is None:
        return 0
    config = load_config(home)
    if not config.gates_enabled:
        return 0
    data = payload if isinstance(payload, dict) else {}
    fields = adapt(agent, data)
    tool_input = _pick(data, "tool_input", "input", "arguments", "args", "toolArgs")
    found = gates.check_call(config, event.get("tool"), tool_input, event.get("cwd"),
                             event.get("project"), home)
    for violation in found:
        gates.record_violation(
            home, violation, ts=event.get("ts") or now_iso(), agent=agent, lane=event.get("lane"),
            session=event.get("session"), project=event.get("project"), tool=event.get("tool"),
            cwd=fields.get("cwd") or event.get("cwd"))
        print(violation.message(), file=sys.stderr)
    return 2 if any(v.action == "block" for v in found) else 0


def run_hook(home: Path, agent: str, event: str | None, stdin) -> int:
    """Entry for ``tirith hook``: record, never raise; exit 2 only when a rule blocks the call."""
    try:
        try:
            payload = json.loads(stdin.read() or "{}")
        except Exception:
            payload = {}
        row = record(home, agent, payload, event)
        return decide(agent, row, payload, home)
    except Exception:
        return 0


# -- install / uninstall ----------------------------------------------------------------------


FLAT_AGENTS = ("copilot",)  # their hook entries are flat ({"type", "bash"}), not matcher groups


def builtin_settings(agent: str) -> Path | None:
    """Where an agent CLI reads user-level hooks when config.toml names no ``settings``.

    muse: ``~/.config/muse/settings.json``; codex: ``~/.codex/hooks.json``; copilot: its own file
    ``~/.copilot/hooks/tirith.json`` (``$COPILOT_HOME`` moves it), so no user file of theirs is edited."""
    home = Path.home()
    if agent == "muse":
        return home / ".config" / "muse" / "settings.json"
    if agent == "codex":
        return home / ".codex" / "hooks.json"
    if agent == "copilot":
        base = Path(os.environ["COPILOT_HOME"]).expanduser() if os.environ.get("COPILOT_HOME") else home / ".copilot"
        return base / "hooks" / "tirith.json"
    return None


def default_settings(agent: str, agents: dict | None = None) -> Path:
    """User-level settings file of ``agent``: ``[agents.<alias>] settings`` in config.toml."""
    configured = (agents or {}).get(agent) or {}
    if configured.get("settings"):
        return Path(str(configured["settings"])).expanduser()
    builtin = builtin_settings(agent)
    if builtin:
        return builtin
    raise ValueError(f"no settings path for agent {agent!r}: add [agents.{agent}] settings = "
                     f"\"<path to its user settings.json>\" to ~/.tirith/config.toml")


def hook_command(agent: str, event: str, tirith: str = "tirith", role: str | None = None) -> str:
    role = "manager" if role == "steward" else role
    prefix = f"{dispatch.ROLE_ENV}={shlex.quote(role)} " if role else ""
    return f"{prefix}{shlex.quote(tirith)} hook --agent {shlex.quote(agent)} --event {shlex.quote(event)}"


def _ours(command, agent: str) -> bool:
    return isinstance(command, str) and f"hook --agent {shlex.quote(agent)} " in command + " "


def _handlers(group, agent: str) -> list[dict]:
    """The command handlers of one entry: a matcher group holds them in ``hooks``; a flat agent entry is one."""
    if not isinstance(group, dict):
        return []
    items = [group] if agent in FLAT_AGENTS else group.get("hooks") or []
    return [h for h in items if isinstance(h, dict)]


def _command_of(handler: dict):
    return handler.get("bash") or handler.get("command")


def _new_entry(agent: str, command: str) -> dict:
    if agent in FLAT_AGENTS:
        return {"type": "command", "bash": command, "timeoutSec": 10}
    return {"hooks": [{"type": "command", "command": command}]}


def _load_settings(path: Path, agent: str) -> dict:
    if not path.exists():
        return {"schema_version": 1} if agent == "muse" else {"version": 1} if agent in FLAT_AGENTS else {}
    data = json.loads(path.read_text(encoding="utf-8") or "{}")
    if not isinstance(data, dict):
        raise ValueError(f"{path} is not a JSON object")
    return data


def backup(path: Path) -> Path | None:
    """Copy an existing file to ``<name>.tirith-bak`` before Tirith changes it (the first copy is kept)."""
    path = Path(path)
    saved = path.with_name(path.name + ".tirith-bak")
    if path.is_file() and not saved.exists():
        shutil.copy2(path, saved)
        return saved
    return None


def install(agent: str, settings: Path | None = None, tirith: str = "tirith",
            agents: dict | None = None, role: str | None = None) -> Path:
    """Add our hook entries to the user settings, keeping everything else; idempotent.

    ``role="manager"`` writes ``TIRITH_ROLE=manager`` into each command (the manager's own settings).
    An existing file is copied to ``<name>.tirith-bak`` first."""
    path = Path(settings) if settings else default_settings(agent, agents)
    data = _load_settings(path, agent)
    before = json.dumps(data, sort_keys=True)
    hooks = data.setdefault("hooks", {})
    for event in INSTALL_EVENTS:
        groups = hooks.setdefault(event, [])
        command = hook_command(agent, event, tirith, role)
        present = False
        for group in groups:
            for handler in _handlers(group, agent):
                if _ours(_command_of(handler), agent):
                    handler["bash" if "bash" in handler else "command"] = command  # refresh a moved executable
                    present = True
        if not present:
            groups.append(_new_entry(agent, command))
    if json.dumps(data, sort_keys=True) != before or not path.exists():
        backup(path)
        write_json_atomic(path, data)
    return path


def uninstall(agent: str, settings: Path | None = None, agents: dict | None = None) -> Path:
    """Remove only our hook entries (and groups/events left empty by that)."""
    path = Path(settings) if settings else default_settings(agent, agents)
    if not path.exists():
        return path
    data = _load_settings(path, agent)
    hooks = data.get("hooks")
    if isinstance(hooks, dict):
        for event in list(hooks):
            kept = []
            for group in hooks[event]:
                if agent in FLAT_AGENTS:
                    if not _ours(_command_of(group) if isinstance(group, dict) else None, agent):
                        kept.append(group)
                    continue
                group["hooks"] = [h for h in group.get("hooks", [])
                                  if not _ours(h.get("command"), agent)]
                if group["hooks"]:
                    kept.append(group)
            if kept:
                hooks[event] = kept
            else:
                del hooks[event]
        if not hooks:
            del data["hooks"]
    backup(path)
    write_json_atomic(path, data)
    return path


def _tilde(path: Path) -> str:
    text = str(path)
    home = str(Path.home())
    return "~" + text[len(home):] if text == home or text.startswith(home + os.sep) else text


def record_settings(home: Path, agents: dict, only: str | None = None) -> list[tuple[str, str]]:
    """Write ``settings = "<path>"`` into ``[agents.<alias>]`` of config.toml for each agent that lacks one.

    Text edit: comments and every other line stay. The file is copied to ``config.toml.tirith-bak`` first.
    Agents Tirith has no built-in location for are skipped. Returns ``[(alias, path written)]``."""
    config = Path(home) / "config.toml"
    lines = config.read_text(encoding="utf-8").splitlines()
    done: list[tuple[str, str]] = []
    for alias, table in agents.items():
        if (only and alias != only) or table.get("settings") or not builtin_settings(alias):
            continue
        value = _tilde(builtin_settings(alias))
        head = f"[agents.{alias}]"
        index = next((i for i, line in enumerate(lines) if line.strip() == head), None)
        if index is None:
            continue
        at = index + 1
        while at < len(lines) and lines[at].strip() and not lines[at].lstrip().startswith("["):
            at += 1
        lines.insert(at, f"settings = {json.dumps(value)}")
        done.append((alias, value))
    if done:
        backup(config)
        config.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return done


# -- protection: the hooks stay in the settings file ----------------------------------------------

PROTECT_FILE = "protect.json"
HOOKS_REMOVED = "hooks-removed"


def has_hooks(path: Path, agent: str) -> bool:
    """Every ``INSTALL_EVENTS`` entry of ``agent`` is in the settings file (read only)."""
    try:
        data = json.loads(Path(path).read_text(encoding="utf-8") or "{}")
    except (OSError, ValueError):
        return False
    hooks = data.get("hooks") if isinstance(data, dict) else None
    if not isinstance(hooks, dict):
        return False
    for event in INSTALL_EVENTS:
        groups = hooks.get(event)
        if not isinstance(groups, list) or not any(
                _ours(_command_of(h), agent) for g in groups for h in _handlers(g, agent)):
            return False
    return True


def _installed_form(path: Path, agent: str) -> tuple[str, str | None]:
    """The executable and role of the installed ``PreToolUse`` entry (so a heal writes the same)."""
    try:
        groups = json.loads(Path(path).read_text(encoding="utf-8"))["hooks"]["PreToolUse"]
        for group in groups:
            for handler in _handlers(group, agent):
                command = _command_of(handler)
                if _ours(command, agent):
                    tokens = shlex.split(command)
                    role = next((t.split("=", 1)[1] for t in tokens
                                 if t.startswith(dispatch.ROLE_ENV + "=")), None)
                    return tokens[tokens.index("hook") - 1], role
    except Exception:
        pass
    return "tirith", None


def protect_state(home: Path) -> dict:
    data = read_json(home / PROTECT_FILE, {})
    return data if isinstance(data, dict) else {}


def protect(home: Path, config, now: datetime | None = None) -> list[dict]:
    """Check the hooks of every configured agent; reinstall them when they were removed.

    Reinstalling merges (``install`` keeps every other key) and records a ``hooks-removed`` violation.
    Only an agent whose hooks were seen intact before is healed; one never installed is reported, not
    touched. Returns ``[{agent, ok, checked_at, healed_at, settings}]`` and keeps it in ``protect.json``."""
    stamp = now_iso(now)
    saved = protect_state(home)
    out: dict[str, dict] = {}
    for alias in config.agents:
        try:
            path = default_settings(alias, config.agents)
        except ValueError:
            continue
        prev = saved.get(alias) if isinstance(saved.get(alias), dict) else {}
        row = {"agent": alias, "settings": str(path), "checked_at": stamp,
               "healed_at": prev.get("healed_at"), "ok": has_hooks(path, alias),
               "tirith": prev.get("tirith") or "tirith", "role": prev.get("role"),
               "seen_ok": bool(prev.get("seen_ok"))}
        if row["ok"]:
            row["seen_ok"] = True
            row["tirith"], row["role"] = _installed_form(path, alias)
        elif row["seen_ok"] and config.gates_enabled:
            try:
                install(alias, path, row["tirith"], config.agents, row["role"])
                row["ok"] = has_hooks(path, alias)
            except (OSError, ValueError):
                pass
            if row["ok"]:
                row["healed_at"] = stamp
                gates.record_violation(
                    home, gates.Violation(
                        HOOKS_REMOVED, f"the tirith hooks of {alias} were removed from {path}; "
                        "Tirith put them back", gates.SETTINGS_HINT),
                    ts=stamp, agent=alias, lane=None, session=None, project=None, tool=None, cwd=None)
        out[alias] = row
    write_json_atomic(home / PROTECT_FILE, out)
    return list(out.values())


# -- collector side ---------------------------------------------------------------------------

EVENTS_PER_AGENT = 50
LOOSE_EVENTS = 50


def public(row: dict) -> dict:
    """The fields of a hook row that go into ``state.json``."""
    return {key: row.get(key) for key in
            ("ts", "agent", "event", "lane", "session", "project", "tool", "summary",
             "agent_session_id")}


def attach(rows: list[dict], lane_ids) -> tuple[dict[str, list[dict]], list[dict]]:
    """Group hook rows by lane (last 50, newest last); the rest is the loose list."""
    known = set(lane_ids)
    by_lane: dict[str, list[dict]] = {}
    loose: list[dict] = []
    for row in rows:
        lane = row.get("lane")
        if lane in known:
            by_lane.setdefault(lane, []).append(public(row))
        else:
            loose.append(public(row))
    return ({k: v[-EVENTS_PER_AGENT:] for k, v in by_lane.items()}, loose[-LOOSE_EVENTS:])


def read_hook_rows(path: Path, max_bytes: int = 1_000_000, kind: str = "hook") -> list[dict]:
    """Rows of ``kind`` (hook rows by default) from the last ``max_bytes`` of ``events.jsonl`` (bounded read)."""
    try:
        with path.open("rb") as handle:
            handle.seek(0, os.SEEK_END)
            size = handle.tell()
            handle.seek(max(0, size - max_bytes))
            raw = handle.read()
    except OSError:
        return []
    lines = raw.decode("utf-8", errors="replace").splitlines()
    if size > max_bytes:
        lines = lines[1:]  # the first line is cut in half
    hooks = []
    for line in lines:
        try:
            row = json.loads(line)
        except ValueError:
            continue
        if isinstance(row, dict) and row.get("kind") == kind:
            hooks.append(row)
    return hooks


VIOLATIONS_KEPT = 50


def read_violation_rows(path: Path, max_bytes: int = 1_000_000) -> list[dict]:
    """``kind: "violation"`` rows from the last ``max_bytes`` of ``events.jsonl`` (newest last)."""
    return read_hook_rows(path, max_bytes, kind="violation")
