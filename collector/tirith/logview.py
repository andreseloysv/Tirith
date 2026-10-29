"""An agent's log as structured, human-readable entries, newest first.

``log_entries(path, workspace)`` turns the tail of a lane log (JSONL transcripts, ``--json`` event
streams or plain text) into ``[{ts, kind, text, detail, fields, path}]``. The web only renders them.
JSON never reaches the owner raw: known shapes become purpose-built lines, unknown objects become a
``fields`` dict (key -> value) with the original pretty-printed in ``detail``, and pure bookkeeping
records become ``system`` entries that the page hides and counts.
Contract: ``docs/CONTRACT-activity.md`` ("Log entries").
"""

from __future__ import annotations

import json
import os
import re
from pathlib import Path

from tirith.activity import is_test_command, muse_tool
from tirith.home import now_iso, parse_iso

KINDS = ("say", "command", "output", "edit", "read", "error", "test", "spawn", "done", "system")
TAIL_BYTES = 256 * 1024
VISIBLE_MAX = 60
SYSTEM_MAX = 30
HEAD_LINES = 6
LINE_MAX = 300
DETAIL_MAX = 4096
FIELDS_MAX = 24
VALUE_MAX = 300

_EDIT_TOOLS = {"Edit", "MultiEdit", "Write", "NotebookEdit", "file_change"}
_READ_TOOLS = {"Read", "Grep", "Glob", "WebFetch", "WebSearch", "LS", "NotebookRead"}
_SPAWN_TOOLS = {"Task", "Agent"}
_PASSED = re.compile(r"\b(\d+) passed\b")
_FAILED = re.compile(r"\b(\d+) (?:failed|errors?)\b")
_TRACE_HEAD = "Traceback (most recent call last)"
_ERROR_LINE = re.compile(r"^\s*(?:error|fatal|fail(?:ed)?)\b[:!]|^\s*\w*(?:Error|Exception)\b:", re.I)
_NOISE_TYPES = {"rate_limit_event", "ping", "keep_alive", "keepalive", "heartbeat", "stream_event",
                "content_block_start", "content_block_stop", "message_start", "message_stop",
                "message_delta", "thread.started", "turn.started"}

CACHE_MAX = 2048  # more than the lanes a home keeps, so a cycle never re-parses every log
_cache: dict[str, tuple[tuple, list[dict]]] = {}


# -- helpers ----------------------------------------------------------------------------------


def _clip(text, limit: int = LINE_MAX) -> str:
    text = str(text or "").replace("\t", "  ")
    return text if len(text) <= limit else text[:limit - 1] + "…"


def short_path(path, workspace) -> str:
    """A path relative to the workspace (or ``~``), long foreign paths cut to their last parts."""
    path = str(path or "")
    ws = str(workspace or "").rstrip("/")
    if ws and path == ws:
        return "."
    if ws and path.startswith(ws + "/"):
        return path[len(ws) + 1:]
    home = str(Path.home())
    if path.startswith(home + "/"):
        path = "~" + path[len(home):]
    parts = path.split("/")
    if len(parts) > 5:
        path = "…/" + "/".join(parts[-3:])
    return path


def _shorten_text(text: str, workspace) -> str:
    ws = str(workspace or "").rstrip("/")
    if ws and len(ws) > 1:
        text = text.replace(ws + "/", "").replace(ws, ".")
    return text.replace(str(Path.home()) + "/", "~/")


def _split(text: str) -> tuple[str, str]:
    """First lines (``text``) and the rest (``detail``, at most 4 KB) of a long block."""
    lines = [_clip(line) for line in str(text or "").strip("\n").splitlines()]
    while lines and not lines[0].strip():
        lines.pop(0)
    head, rest = "\n".join(lines[:HEAD_LINES]), "\n".join(lines[HEAD_LINES:])
    return head, rest[:DETAIL_MAX]


def _ts(record: dict, fallback):
    for key in ("timestamp", "ts", "at", "created_at"):
        value = record.get(key)
        if isinstance(value, str) and (moment := parse_iso(value)):
            return now_iso(moment)
    return fallback


def _entry(ts, kind: str, text: str = "", detail: str = "", fields=None, path=None) -> dict:
    return {"ts": ts, "kind": kind, "text": text, "detail": detail,
            "fields": fields or None, "path": path}


def _humanize_count(n) -> str:
    n = float(n)
    return f"{n / 1000:.1f}k" if n >= 1000 else str(int(n))


def _first_sentence(text: str, limit: int = 160) -> str:
    text = " ".join(str(text or "").split())
    cut = re.split(r"(?<=[.!?])\s", text, maxsplit=1)[0]
    return _clip(cut, limit)


def _pretty(obj) -> str:
    try:
        return json.dumps(obj, indent=2, ensure_ascii=False, default=str)[:DETAIL_MAX]
    except (TypeError, ValueError):
        return ""


def _bound(value, depth: int = 0):
    """A JSON value cut down so a card stays small: long strings clipped, big containers trimmed."""
    if isinstance(value, str):
        return _clip(value, VALUE_MAX)
    if isinstance(value, dict):
        if depth >= 3:
            return {"…": len(value)}
        return {str(k): _bound(v, depth + 1) for k, v in list(value.items())[:FIELDS_MAX]}
    if isinstance(value, list):
        return [_bound(v, depth + 1) for v in value[:FIELDS_MAX]]
    return value


# -- JSON values (a whole line, a tool input, a command's output) -------------------------------


def parse_json_text(text: str):
    """The JSON object/array a text consists of, else None."""
    stripped = str(text or "").strip()
    if not stripped or stripped[0] not in "{[" or stripped[-1] not in "}]":
        return None
    try:
        value = json.loads(stripped)
    except ValueError:
        return None
    return value if isinstance(value, (dict, list)) else None


def entry_from_json(ts, value, workspace=None) -> dict:
    """One entry for a JSON value found in output or a plain line: known shape, else a card."""
    if isinstance(value, dict):
        passed = value.get("passed", value.get("tests_passed"))
        failed = value.get("failed", value.get("tests_failed"))
        if isinstance(passed, int) and isinstance(failed, int) and not isinstance(passed, bool):
            return _entry(ts, "test", "tests", "failed" if failed else "passed",
                          {"passed": passed, "failed": failed})
        err = value.get("error")
        if err and not value.get("result"):
            msg = err.get("message") if isinstance(err, dict) else err
            return _entry(ts, "error", _clip(f"Error: {msg}"), "", _bound(value) if isinstance(err, dict) else None)
        usage = _usage_line(value)
        if usage:
            return _entry(ts, "system", usage, _pretty(value), _bound(value))
        res = value.get("result")
        if isinstance(res, str) and res.strip():
            return _entry(ts, "done", "Finished: " + _first_sentence(res), "", None)
        if str(value.get("type") or "") in _NOISE_TYPES:
            return _entry(ts, "system", _noise_label(value), _pretty(value), _bound(value))
    fields = _bound(value) if isinstance(value, dict) else {"items": _bound(value)}
    return _entry(ts, "say", "", _pretty(value), fields)


def _usage_line(value: dict) -> str:
    usage = value.get("usage") if isinstance(value.get("usage"), dict) else value
    tokens = 0
    for key in ("input_tokens", "output_tokens", "total_tokens", "tokens"):
        n = usage.get(key)
        if isinstance(n, (int, float)) and not isinstance(n, bool):
            tokens = max(tokens, n) if key in ("total_tokens", "tokens") else tokens + n
    if not tokens:
        return ""
    parts = [f"{_humanize_count(tokens)} tokens"]
    ms = value.get("duration_ms") or usage.get("duration_ms")
    if isinstance(ms, (int, float)) and ms:
        parts.append(f"{round(ms / 1000)} s")
    cost = value.get("total_cost_usd") or value.get("cost_usd")
    if isinstance(cost, (int, float)) and cost:
        parts.append(f"${cost:.2f}")
    return ", ".join(parts)


def _noise_label(record: dict) -> str:
    kind = str(record.get("type") or record.get("payload_type") or "event")
    sub = record.get("subtype")
    return "System: " + kind + (f" ({sub})" if sub else "")


# -- one transcript --------------------------------------------------------------------------


class _Builder:
    def __init__(self, workspace) -> None:
        self.workspace = workspace
        self.out: list[dict] = []  # oldest first
        self.calls: dict[str, dict] = {}  # tool call id -> its entry

    def add(self, entry: dict) -> dict:
        self.out.append(entry)
        return entry

    def rel(self, path) -> str:
        return short_path(path, self.workspace)

    # -- tool calls ------------------------------------------------------------------------

    def tool(self, ts, name: str, data: dict, call_id=None) -> None:
        data = data if isinstance(data, dict) else {}
        path = data.get("file_path") or data.get("path") or data.get("notebook_path")
        if name == "Bash" or (name != "tool" and "command" in data and name not in _EDIT_TOOLS):
            command = str(data.get("command") or data.get("cmd") or "")
            test = is_test_command(command)
            entry = _entry(ts, "test" if test else "command",
                           _clip(_shorten_text(command, self.workspace)),
                           "running" if test else "")
        elif name in _SPAWN_TOOLS:
            who = data.get("subagent_type")
            what = data.get("description") or data.get("prompt") or ""
            entry = _entry(ts, "spawn", _clip(what or who or "sub-agent", 160),
                           str(who or ""))
        elif name in _EDIT_TOOLS:
            entry = self._edit(ts, name, data, path)
        elif name in _READ_TOOLS:
            what = path or data.get("pattern") or data.get("query") or data.get("url") or ""
            where = data.get("path") if data.get("pattern") and path is None else None
            text = self.rel(what) if path else _clip(what)
            if where:
                text += f" in {self.rel(where)}"
            entry = _entry(ts, "read", f"{name} {text}".strip(), "", None, str(path) if path else None)
        else:
            shown = {k: v for k, v in data.items() if v not in (None, "")}
            entry = _entry(ts, "say", f"Used {name}", _pretty(data), _bound(shown) or None)
        self.add(entry)
        if call_id:
            self.calls[str(call_id)] = entry

    def _edit(self, ts, name: str, data: dict, path) -> dict:
        if name == "file_change":
            return _entry(ts, "edit", self.rel(path), "edited", None, str(path) if path else None)
        old, new = data.get("old_string"), data.get("new_string")
        if name == "Write" or (name == "NotebookEdit" and data.get("edit_mode") == "insert"):
            detail = "created"
        elif name == "MultiEdit" and isinstance(data.get("edits"), list):
            ins = sum(len(str(e.get("new_string") or "").splitlines()) for e in data["edits"] if isinstance(e, dict))
            dele = sum(len(str(e.get("old_string") or "").splitlines()) for e in data["edits"] if isinstance(e, dict))
            detail = f"+{ins} −{dele}"
        elif isinstance(old, str) and isinstance(new, str):
            detail = f"+{len(new.splitlines())} −{len(old.splitlines())}"
        else:
            detail = "edited"
        return _entry(ts, "edit", self.rel(path), detail, None, str(path) if path else None)

    # -- tool results ----------------------------------------------------------------------

    def result(self, ts, call_id, content, is_error: bool, exit_code=None) -> None:
        call = self.calls.get(str(call_id)) if call_id else None
        text = content if isinstance(content, str) else "\n".join(
            str(b.get("text") or "") for b in content if isinstance(b, dict)) if isinstance(content, list) else ""
        text = _shorten_text(text, self.workspace)
        failed = bool(is_error) or (exit_code not in (None, 0))
        if call is not None and call["kind"] == "test":
            passed, bad = _PASSED.search(text), _FAILED.search(text)
            bad_n = int(bad.group(1)) if bad else 0
            failed = failed or bad_n > 0
            call["detail"] = "failed" if failed else "passed"
            if passed or bad:
                call["fields"] = {"passed": int(passed.group(1)) if passed else 0, "failed": bad_n}
            if failed and text.strip():
                self._output(ts, text, True)
            return
        if call is not None and call["kind"] in ("edit", "read", "spawn") and not failed:
            return
        if not text.strip():
            return
        self._output(ts, text, failed)

    def _output(self, ts, text: str, failed: bool) -> None:
        value = parse_json_text(text)
        if value is not None:
            entry = entry_from_json(ts, value, self.workspace)
            if failed and entry["kind"] == "say":
                entry["kind"] = "error"
                entry["text"] = "Error"
            self.add(entry)
            return
        head, rest = _split(text)
        if failed or _TRACE_HEAD in text:
            self.add(_entry(ts, "error", head, rest))
        else:
            self.add(_entry(ts, "output", head, rest))


def _feed(b: _Builder, record: dict, fallback_ts) -> None:
    ts = _ts(record, fallback_ts)
    kind = record.get("type")
    if kind == "assistant":
        for block in _blocks(record):
            if block.get("type") == "text" and str(block.get("text") or "").strip():
                head, rest = _split(block["text"])
                b.add(_entry(ts, "say", head, rest))
            elif block.get("type") == "tool_use":
                b.tool(ts, str(block.get("name") or "tool"), block.get("input"), block.get("id"))
    elif kind == "user":
        for block in _blocks(record):
            if block.get("type") == "tool_result":
                b.result(ts, block.get("tool_use_id"), block.get("content"), bool(block.get("is_error")))
            elif block.get("type") == "text" and str(block.get("text") or "").strip():
                b.add(_entry(ts, "say", *_split(block["text"])))
    elif kind in ("item.started", "item.completed"):
        _item(b, record, ts)
    elif kind == "result":
        answer = str(record.get("result") or "").strip()
        failed = bool(record.get("is_error")) or str(record.get("subtype") or "").startswith("error")
        head, rest = _split(answer or str(record.get("subtype") or "done"))
        usage = _usage_line(record)
        b.add(_entry(ts, "error" if failed else "done", head, rest,
                     {"usage": usage} if usage else None))
    elif kind in ("system", "summary") or kind in _NOISE_TYPES:
        b.add(_entry(ts, "system", _noise_label(record), _pretty(record), _bound(record)))
    elif "payload_type" in record:
        _muse(b, record, ts)
    elif kind == "error" or isinstance(record.get("error"), (str, dict)):
        err = record.get("error") if record.get("error") else record.get("message")
        msg = err.get("message") if isinstance(err, dict) else err
        b.add(_entry(ts, "error", _clip(f"Error: {msg}"), _pretty(record)))
    else:
        b.add(entry_from_json(ts, record, b.workspace))


def _blocks(record: dict) -> list[dict]:
    message = record.get("message")
    content = message.get("content") if isinstance(message, dict) else None
    if isinstance(content, str):
        return [{"type": "text", "text": content}]
    return [x for x in content if isinstance(x, dict)] if isinstance(content, list) else []


def _item(b: _Builder, record: dict, ts) -> None:
    item = record.get("item")
    if not isinstance(item, dict):
        return
    started = record["type"] == "item.started"
    item_id = str(item.get("id") or "")
    kind = item.get("type")
    if kind == "command_execution":
        if item_id not in b.calls:
            b.tool(ts, "Bash", {"command": item.get("command")}, item_id)
        if not started:
            b.result(ts, item_id, item.get("aggregated_output") or "", False, item.get("exit_code"))
    elif kind == "file_change" and not started:
        for change in item.get("changes") or []:
            if isinstance(change, dict):
                how = {"add": "created", "delete": "deleted"}.get(str(change.get("kind")), "edited")
                b.add(_entry(ts, "edit", b.rel(change.get("path")), how, None, str(change.get("path") or "")))
    elif kind == "agent_message" and str(item.get("text") or "").strip() and not started:
        b.add(_entry(ts, "say", *_split(item["text"])))
    elif kind == "error":
        b.add(_entry(ts, "error", _clip("Error: " + str(item.get("message") or "")), ""))


_MUSE_QUIET = ("task.", "session.", "turn.", "run.model.", "run.lifecycle.", "run.output.")
_MUSE_EDIT = re.compile(r"write|edit|patch|create|replace", re.I)
_MUSE_READ = re.compile(r"read|grep|glob|search|list|find|fetch", re.I)


def _muse(b: _Builder, record: dict, ts) -> None:
    ptype = str(record.get("payload_type") or "")
    payload = record.get("payload") if isinstance(record.get("payload"), dict) else {}
    if ptype.startswith("run.terminal"):
        failed = not ptype.endswith("completed")
        text = str(payload.get("text") or payload.get("reason") or ptype.rsplit(".", 1)[-1])
        head, rest = _split(text)
        b.add(_entry(ts, "error" if failed else "done", head, rest))
    elif ptype == "run.usage":
        usage = _usage_line(_muse_usage(payload))
        b.add(_entry(ts, "system", usage or "System: run.usage", _pretty(payload), _bound(payload) or None))
    elif ptype == "tool.result":
        name, path, failed = muse_tool(payload)
        call_id = payload.get("call_id")
        if path and _MUSE_EDIT.search(name):
            b.tool(ts, "Write" if "write" in name or "create" in name else "Edit", {"path": path}, call_id)
        elif path and _MUSE_READ.search(name):
            b.tool(ts, "Read", {"path": path}, call_id)
        else:
            b.tool(ts, name, {}, call_id)
        b.result(ts, call_id, str(payload.get("text") or ""), failed)
    elif ptype.startswith("task.lifecycle.") and ptype.rsplit(".", 1)[-1] in ("failed", "cancelled"):
        event = payload.get("event") if isinstance(payload.get("event"), dict) else {}
        b.add(_entry(ts, "error", _clip("Task " + ptype.rsplit(".", 1)[-1] + ": "
                                          + str(event.get("message") or event.get("reason") or "")), ""))
    elif ptype.startswith("runtime."):
        b.add(_entry(ts, "system", "System: " + ptype, _pretty(record), _bound(payload) or None))
    elif ptype.startswith(_MUSE_QUIET):
        return  # bookkeeping of the event stream: nothing a person wants to read
    elif ptype.startswith("tool.") or re.search(r"(^|\.)(command|file)(\.|_|$)", ptype):
        name = str(payload.get("tool") or payload.get("tool_name") or payload.get("name") or "tool")
        call_id = payload.get("tool_call_id") or payload.get("call_id") or payload.get("id")
        if call_id and str(call_id) in b.calls:
            if payload.get("exit_code") is not None or payload.get("output") is not None:
                b.result(ts, call_id, str(payload.get("output") or ""), False, payload.get("exit_code"))
            return
        data = dict(payload)
        if payload.get("command"):
            name = "Bash"
        elif payload.get("path") and name == "tool":
            name = "Edit" if "file" in ptype else "Read"
        b.tool(ts, name, data, call_id)
        if payload.get("exit_code") is not None:
            b.result(ts, call_id, str(payload.get("output") or ""), False, payload.get("exit_code"))
    else:
        b.add(entry_from_json(ts, payload or record, b.workspace))


def _muse_usage(payload: dict) -> dict:
    """Token totals of a ``run.usage`` payload in the shape ``_usage_line`` reads."""
    rows = [m for m in payload.get("models") or [] if isinstance(m, dict)]
    return {"input_tokens": sum(m.get("input_tokens") or 0 for m in rows),
            "output_tokens": sum(m.get("output_tokens") or 0 for m in rows)}


# -- plain text lines -----------------------------------------------------------------------


def _plain(b: _Builder, lines: list[str], index: int, fallback_ts) -> int:
    """Consume one plain-text entry starting at ``lines[index]``; returns the next index."""
    line = lines[index]
    if line.startswith(_TRACE_HEAD):  # the whole traceback up to and including its error line
        end = index + 1
        while end < len(lines) and (lines[end].startswith((" ", "\t")) or lines[end].startswith(("File ", "During"))):
            end += 1
        if end < len(lines):
            end += 1
        block = lines[index:end]
        last = block[-1].strip() if len(block) > 1 else _TRACE_HEAD
        b.add(_entry(None, "error", _clip(_shorten_text(last, b.workspace)),
                     _shorten_text("\n".join(block), b.workspace)[:DETAIL_MAX]))
        return end
    stripped = line.strip()
    value = parse_json_text(stripped)
    if value is not None:
        b.add(entry_from_json(None, value, b.workspace))
    elif stripped.startswith("$ "):
        cmd = stripped[2:]
        b.add(_entry(None, "test" if is_test_command(cmd) else "command",
                     _clip(_shorten_text(cmd, b.workspace)), "running" if is_test_command(cmd) else ""))
    elif _ERROR_LINE.match(line):
        b.add(_entry(None, "error", _clip(_shorten_text(stripped, b.workspace))))
    elif stripped:
        b.add(_entry(None, "say", _clip(_shorten_text(stripped, b.workspace))))
    return index + 1


# -- the public entry point -----------------------------------------------------------------


def parse_lines(lines: list[str], workspace=None, fallback_ts=None) -> list[dict]:
    """Entries (oldest first) for the given log lines."""
    b = _Builder(workspace)
    i = 0
    while i < len(lines):
        line = lines[i]
        record = None
        if line.lstrip().startswith("{"):
            try:
                record = json.loads(line)
            except ValueError:
                record = None
        if isinstance(record, dict):
            _feed(b, record, fallback_ts)
            i += 1
        elif line.strip():
            i = _plain(b, lines, i, fallback_ts)
        else:
            i += 1
    return b.out


def finish(entries: list[dict]) -> list[dict]:
    """Newest first, system records capped, a still-``running`` test left as it is."""
    entries = entries[::-1]
    visible = [e for e in entries if e["kind"] != "system"][:VISIBLE_MAX]
    system = [e for e in entries if e["kind"] == "system"][:SYSTEM_MAX]
    keep = {id(e) for e in visible + system}
    return [e for e in entries if id(e) in keep]


def log_entries(path, workspace=None) -> list[dict]:
    """Structured entries (newest first) from the end of one log file; [] when missing."""
    if not path:
        return []
    try:
        info = os.stat(path)
        stamp = (info.st_size, info.st_mtime, str(workspace))
        cached = _cache.get(str(path))
        if cached and cached[0] == stamp:
            return cached[1]
        with open(path, "rb") as fh:
            fh.seek(max(0, info.st_size - TAIL_BYTES))
            text = fh.read().decode("utf-8", errors="replace")
    except OSError:
        return []
    lines = text.splitlines()
    if info.st_size > TAIL_BYTES and lines:
        lines = lines[1:]  # the first line is probably cut
    entries = finish(parse_lines(lines, workspace))
    if len(_cache) > CACHE_MAX:
        _cache.clear()
    _cache[str(path)] = (stamp, entries)
    return entries


def entries_from_events(events: list[dict], workspace=None) -> list[dict]:
    """Entries (newest first) built from hook events, for an agent that has no log file."""
    b = _Builder(workspace)
    for event in events:
        tool, summary, ts = event.get("tool"), str(event.get("summary") or ""), event.get("ts")
        if event.get("event") not in ("PreToolUse", "tool", "done", "Stop", "SubagentStop"):
            continue
        if event.get("event") in ("done", "Stop", "SubagentStop"):
            b.add(_entry(ts, "done", *_split(summary)))
        elif tool == "Bash":
            cmd = summary[len("Bash: "):] if summary.startswith("Bash: ") else summary
            b.add(_entry(ts, "test" if is_test_command(cmd) else "command", _clip(cmd), ""))
        elif tool in _EDIT_TOOLS:
            path = summary.split(" ", 1)[1] if " " in summary else summary
            b.add(_entry(ts, "edit", short_path(path, workspace), "edited", None, path))
        elif tool in _READ_TOOLS:
            b.add(_entry(ts, "read", summary))
        elif tool in _SPAWN_TOOLS or summary.startswith("Sub-agent"):
            b.add(_entry(ts, "spawn", summary.replace("Sub-agent", "", 1).strip(": ") or "sub-agent"))
        else:
            b.add(_entry(ts, "say", summary))
    return finish(b.out)
