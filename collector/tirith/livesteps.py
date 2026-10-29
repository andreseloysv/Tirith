"""A lane's stream-json log as a short live transcript, like the CLI shows it.

``steps_for(log, workspace, fallback_at)`` reads the log of a headless agent (``--output-format
stream-json --verbose``) and returns ``(steps, summary)``. A step is ``{kind: text|tool|result, name,
summary, at, error?}``: the agent's words, one line per tool call (``Read path``, ``Edit path``,
``Bash: <first line>``, ``Grep "pat"``) and each tool result cut to ``RESULT_LINES`` lines. ``summary`` is
``{steps, done, error, cost_usd, duration_s}``. Secrets are masked and raw JSON never shows. The chat reads
the steps from ``agents/<lane>.steps.js`` (see ``Collector.slim_state``), never from ``state.json``.
"""

from __future__ import annotations

import json
import os
import re
from datetime import datetime, timezone

from tirith.activity import muse_tool
from tirith.home import now_iso
from tirith.logview import short_path

STEPS_KEPT = 150
RESULT_LINES = 3
LINE_MAX = 200
TEXT_LINES = 4
READ_BYTES = 4 * 1024 * 1024
MASK = "***"

_SECRET_PAIR = re.compile(r"(?i)\b([\w-]*(?:token|env|password|passwd|secret|api[_-]?key|access[_-]?key|authorization)"
                          r"[\w-]*\s*[=:]\s*)(?:Bearer\s+)?[^&\s\"',;]+")
_SECRET_SHAPE = re.compile(r"\b(?:sk-[A-Za-z0-9_-]{16,}|gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}"
                           r"|xox[abprs]-[A-Za-z0-9-]{10,}|AKIA[0-9A-Z]{16}|eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}"
                           r"\.[A-Za-z0-9_-]{5,})\b")
_BEARER = re.compile(r"(?i)\bBearer\s+[A-Za-z0-9._~+/=-]{12,}")

CACHE_MAX = 2048  # more than the lanes a home keeps, so a cycle never re-parses every log
_cache: dict[str, tuple[tuple, list[dict], dict]] = {}
_stamps: dict[str, list] = {}  # log path -> the time each step was first seen


def mask(text) -> str:
    """``text`` with tokens, passwords and key-shaped strings replaced by ``***``."""
    text = _SECRET_PAIR.sub(lambda m: m.group(1) + MASK, str(text or ""))
    return _BEARER.sub("Bearer " + MASK, _SECRET_SHAPE.sub(MASK, text))


def _clip(text, limit: int = LINE_MAX) -> str:
    text = str(text or "").replace("\t", "  ").rstrip()
    return text if len(text) <= limit else text[:limit - 1] + "…"


def _lines(text, count: int) -> str:
    rows = [_clip(r) for r in str(text or "").strip("\n").splitlines() if r.strip()]
    out = rows[:count]
    if len(rows) > count:
        out.append(f"… +{len(rows) - count} lines")
    return "\n".join(out)


def tool_summary(name: str, data, workspace=None) -> str:
    """The CLI's one-line form of a tool call."""
    data = data if isinstance(data, dict) else {}
    path = data.get("file_path") or data.get("path") or data.get("notebook_path")
    rel = short_path(path, workspace) if path else ""
    if name == "Bash":
        first = next((l for l in str(data.get("command") or "").splitlines() if l.strip()), "")
        return "Bash: " + first.strip()
    if name in ("Read", "Edit", "MultiEdit", "Write", "NotebookEdit", "NotebookRead"):
        return f"{name} {rel}".strip()
    if name == "Grep":
        where = f" in {rel}" if rel else ""
        return f'Grep "{data.get("pattern") or ""}"{where}'
    if name == "Glob":
        return f"Glob {data.get('pattern') or ''}".strip()
    if name in ("Task", "Agent"):
        return f"{name}: {data.get('description') or data.get('subagent_type') or ''}".rstrip(": ")
    if name in ("WebFetch", "WebSearch"):
        return f"{name} {data.get('url') or data.get('query') or ''}".strip()
    for key in ("command", "description", "query", "pattern", "url", "prompt"):
        if isinstance(data.get(key), str) and data[key].strip():
            return f"{name}: {data[key].strip().splitlines()[0]}"
    return rel and f"{name} {rel}" or name


def _text_of(content) -> str:
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "\n".join(str(b.get("text") or "") for b in content if isinstance(b, dict))
    return ""


def parse(lines: list[str], workspace=None) -> tuple[list[dict], dict]:
    """``(steps oldest first, summary)`` for stream-json log lines; lines that are not events are skipped."""
    steps: list[dict] = []
    names: dict[str, str] = {}
    summary = {"steps": 0, "done": False, "error": False, "cost_usd": None, "duration_s": None}
    for line in lines:
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            rec = json.loads(line)
        except ValueError:
            continue
        if not isinstance(rec, dict):
            continue
        if "payload_type" in rec:
            _muse_record(rec, steps, summary, workspace)
            continue
        kind = rec.get("type")
        content = rec.get("message", {}).get("content") if isinstance(rec.get("message"), dict) else None
        content = [{"type": "text", "text": content}] if isinstance(content, str) else content
        blocks = [b for b in content if isinstance(b, dict)] if isinstance(content, list) else []
        if kind == "assistant":
            for b in blocks:
                if b.get("type") == "text" and str(b.get("text") or "").strip():
                    steps.append({"kind": "text", "name": "", "summary": mask(_lines(b["text"], TEXT_LINES)),
                                  "at": None})
                elif b.get("type") == "tool_use":
                    name = str(b.get("name") or "tool")
                    names[str(b.get("id"))] = name
                    steps.append({"kind": "tool", "name": name, "at": None,
                                  "summary": mask(_clip(tool_summary(name, b.get("input"), workspace)))})
        elif kind == "user":
            for b in blocks:
                if b.get("type") != "tool_result":
                    continue
                text = _text_of(b.get("content"))
                bad = bool(b.get("is_error"))
                if not text.strip() and not bad:
                    continue
                steps.append({"kind": "result", "name": names.get(str(b.get("tool_use_id")), ""), "at": None,
                              "summary": mask(_lines(text, RESULT_LINES)) or "(no output)", "error": bad})
        elif kind == "result":
            summary["done"] = True
            summary["error"] = bool(rec.get("is_error")) or str(rec.get("subtype") or "").startswith("error")
            cost = rec.get("total_cost_usd", rec.get("cost_usd"))
            summary["cost_usd"] = round(float(cost), 4) if isinstance(cost, (int, float)) else None
            ms = rec.get("duration_ms")
            summary["duration_s"] = round(ms / 1000) if isinstance(ms, (int, float)) else None
    summary["steps"] = len(steps)
    return steps, summary


def _muse_record(rec: dict, steps: list[dict], summary: dict, workspace) -> None:
    """One Muse Code event (``payload_type`` envelope): finished tool calls and the final answer."""
    ptype = str(rec.get("payload_type") or "")
    payload = rec.get("payload") if isinstance(rec.get("payload"), dict) else {}
    if ptype == "tool.result":
        name, path, failed = muse_tool(payload)
        steps.append({"kind": "tool", "name": name, "at": None,
                      "summary": mask(_clip(tool_summary(name, {"path": path}, workspace)))})
        text = str(payload.get("text") or "")
        if text.strip() or failed:
            steps.append({"kind": "result", "name": name, "at": None, "error": failed,
                          "summary": mask(_lines(text, RESULT_LINES)) or "(no output)"})
    elif ptype.startswith("run.terminal"):
        summary["done"] = True
        summary["error"] = not ptype.endswith("completed")
        text = str(payload.get("text") or payload.get("reason") or "")
        if text.strip():
            steps.append({"kind": "text", "name": "", "at": None,
                          "summary": mask(_lines(text, TEXT_LINES))})


def steps_for(log, workspace=None, fallback_at: str | None = None, now: datetime | None = None):
    """Live transcript of one log file: ``(steps, summary)``, at most ``STEPS_KEPT`` newest steps.

    A step gets the time it was first seen; steps already in the log when it is first read get
    ``fallback_at`` (the lane's end or start)."""
    if not log:
        return [], {}
    try:
        info = os.stat(log)
        stamp = (info.st_size, info.st_mtime, str(workspace), fallback_at)
        cached = _cache.get(str(log))
        if cached and cached[0] == stamp:
            return cached[1], cached[2]
        with open(log, "rb") as fh:
            fh.seek(max(0, info.st_size - READ_BYTES))
            text = fh.read().decode("utf-8", errors="replace")
    except OSError:
        return [], {}
    lines = text.splitlines()
    if info.st_size > READ_BYTES and lines:
        lines = lines[1:]
    steps, summary = parse(lines, workspace)
    seen = _stamps.get(str(log))
    first = seen is None
    seen = seen or []
    when = now_iso(now or datetime.now(timezone.utc))
    for i in range(len(seen), len(steps)):
        seen.append(fallback_at if first else when)
    _stamps[str(log)] = seen
    for step, at in zip(steps, seen):
        step["at"] = at
    summary["steps"] = len(steps)
    kept = steps[-STEPS_KEPT:]
    if len(_cache) > CACHE_MAX:
        _cache.clear()
        _stamps.clear()
    _cache[str(log)] = (stamp, kept, summary)
    return kept, summary
