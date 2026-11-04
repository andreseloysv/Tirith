"""Access audit: one line per agent tool call in ``<home>/access/<project>.ndjson``.

Docs: ``docs/CONTRACT-hooks.md`` ("Access audit"). Called by the tool hook (``PreToolUse``), so it only
appends, never touches the network and never raises. Secret values are never stored: the action text is
cut to its first line and every token, password, key or ``NAME=value`` secret is replaced by ``***``.
"""

from __future__ import annotations

import re
from pathlib import Path

from tirith.store import append_jsonl, lane_path, read_json

ACCESS_DIR = "access"
ACTION_MAX = 200
LOOSE = "_loose"  # calls outside any project

_SECRET_NAME = r"[A-Za-z0-9_.-]*(?:SECRET|TOKEN|PASSWORD|PASSWD|PWD|API[_-]?KEY|PRIVATE[_-]?KEY|CREDENTIAL|AUTH)[A-Za-z0-9_.-]*"
_PATTERNS = (
    (re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----.*", re.S), "***"),
    (re.compile(r"(?i)\b(bearer|basic|token)(\s+)[^\s'\"]+"), r"\1\2***"),
    (re.compile(r"(?i)\b(authorization\s*[:=]\s*)(?!(?:bearer|basic|token)\b)[^\s'\"]+"), r"\1***"),
    (re.compile(r"(?i)(--?(?:password|passwd|token|secret|api-?key|auth)[=\s]+)[^\s'\"]+"), r"\1***"),
    (re.compile(r"(?i)\b(" + _SECRET_NAME + r")(\s*[=:]\s*)(\"[^\"]*\"|'[^']*'|[^\s'\"]+)"), r"\1\2***"),
    (re.compile(r"(?i)(://[^/\s:@]+:)[^@\s/]+@"), r"\1***@"),
    (re.compile(r"\b(?:sk|pk|ghp|gho|ghs|github_pat|xox[a-z]|AKIA|AIza)[-_A-Za-z0-9]{12,}"), "***"),
    (re.compile(r"\b[A-Fa-f0-9]{32,}\b"), "***"),
    (re.compile(r"\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_.-]+"), "***"),
)
_URL = re.compile(r"https?://[^\s'\"]+")
_SYSTEMS = (
    ("git", re.compile(r"(?:^|[;&|(]\s*|\s)git\s")),
    ("github", re.compile(r"(?:^|[;&|(]\s*)gh\s")),
    ("tirith", re.compile(r"(?:^|[;&|(]\s*)tirith\s")),
    ("network", re.compile(r"(?:^|[;&|(]\s*)(?:curl|wget|ssh|scp|rsync|nc)\s|https?://")),
    ("package", re.compile(r"(?:^|[;&|(]\s*)(?:npm|pnpm|yarn|pip3?|uv|brew|cargo)\s")),
)
_TOOL_SYSTEMS = {"Read": "files", "Glob": "files", "Grep": "files", "Edit": "files", "Write": "files",
                 "MultiEdit": "files", "NotebookEdit": "files", "WebFetch": "network",
                 "WebSearch": "network", "Task": "agent", "Agent": "agent", "Bash": "shell"}


def scrub(text, limit: int = ACTION_MAX) -> str:
    """First line of ``text`` with secret values masked, cut to ``limit`` characters."""
    out = ""
    for line in str(text or "").splitlines():
        if line.strip():
            out = line.strip()
            break
    for pattern, repl in _PATTERNS:
        out = pattern.sub(repl, out)
    return out if len(out) <= limit else out[:limit - 1] + "…"


def system_of(tool, tool_input) -> str:
    """Which system a call touches: ``git``, ``github``, ``network``, ``files``, ``shell``…"""
    tool = str(tool or "")
    data = tool_input if isinstance(tool_input, dict) else {}
    if tool == "Bash":
        command = str(data.get("command") or data.get("cmd") or "")
        for name, pattern in _SYSTEMS:
            if pattern.search(command):
                return name
        return "shell"
    if tool.startswith("mcp__"):
        return "mcp:" + (tool.split("__")[1] if tool.count("__") >= 2 else tool)
    return _TOOL_SYSTEMS.get(tool, "other")


def action_of(tool, tool_input, cwd=None) -> str:
    """Short, secret-free description of the call (never the file content or a prompt body)."""
    data = tool_input if isinstance(tool_input, dict) else {}
    tool = str(tool or "")
    if tool == "Bash":
        return "Bash: " + scrub(data.get("command") or data.get("cmd"))
    path = data.get("file_path") or data.get("path") or data.get("notebook_path")
    if path:
        base = str(cwd or "").rstrip("/")
        text = str(path)
        return f"{tool} " + scrub(text[len(base) + 1:] if base and text.startswith(base + "/") else text)
    url = data.get("url")
    if url:
        return f"{tool} " + scrub(_URL.search(str(url)).group(0) if _URL.search(str(url)) else url)
    if tool in ("Task", "Agent"):
        return f"{tool}: " + scrub(data.get("description") or data.get("subagent_type"))
    for key in ("pattern", "query"):
        if data.get(key):
            return f"{tool}: " + scrub(data[key])
    return tool


def plan_point(home: Path, lane_id) -> str | None:
    """The plan point a lane works on: ``<step>: <item>`` from its lane file, or None."""
    if not lane_id:
        return None
    lane = read_json(lane_path(home, lane_id), {}) or {}
    item = str(lane.get("item") or "").strip()
    step = str(lane.get("item_step") or lane.get("step") or "").strip()
    if not item and not step:
        return None
    return (f"{step}: " if step else "") + scrub(item, 160)


def record(home: Path, event: dict, tool_input=None) -> dict | None:
    """Append the access line for one ``PreToolUse`` hook row; other events and tool-less rows are skipped."""
    if event.get("event") != "PreToolUse" or not event.get("tool"):
        return None
    home = Path(home)
    project = re.sub(r"[^A-Za-z0-9._-]", "_", str(event.get("project") or LOOSE)).lstrip(".") or LOOSE
    row = {"ts": event.get("ts"), "agent": event.get("agent"), "lane": event.get("lane"),
           "plan": plan_point(home, event.get("lane")), "system": system_of(event["tool"], tool_input),
           "tool": event["tool"], "action": action_of(event["tool"], tool_input, event.get("cwd"))}
    append_jsonl(home / ACCESS_DIR / f"{project}.ndjson", row)
    return row
