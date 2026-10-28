"""Reading and writing the small input files under TIRITH_HOME.

The CLI writes lane registrations, pipeline events, notes, waits and the
steward activity; the collector reads them back every cycle. Keeping all
file formats here means the CLI and collector cannot drift apart.
"""

from __future__ import annotations

import json
import os
import tempfile
from pathlib import Path

from tirith.home import now_iso


def read_json(path: Path, default):
    """Read a JSON file, returning ``default`` when missing or broken."""
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return default


def write_json_atomic(path: Path, payload) -> None:
    """Write JSON via a temp file in the same directory, then rename."""
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=str(path.parent), prefix=path.name + ".", suffix=".tmp")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            json.dump(payload, handle, indent=2, sort_keys=False)
            handle.write("\n")
        os.replace(tmp, path)
    except BaseException:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


def write_text_atomic(path: Path, text: str) -> None:
    """Write text via a temp file in the same directory, then rename."""
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=str(path.parent), prefix=path.name + ".", suffix=".tmp")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            handle.write(text)
        os.replace(tmp, path)
    except BaseException:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


def state_js(payload) -> str:
    """The ``state.js`` text: the same JSON as ``window.TIRITH_STATE`` for file:// pages."""
    body = json.dumps(payload, indent=2).replace("</", "<\\/")  # never closes a <script> tag
    return "window.TIRITH_STATE = " + body + ";\n"


def append_jsonl(path: Path, record: dict) -> None:
    """Append one JSON object as a line (creating the file if needed)."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(record) + "\n")


def read_jsonl(path: Path) -> list[dict]:
    """Read a JSON-lines file; broken lines are skipped, missing means []."""
    if not path.exists():
        return []
    rows: list[dict] = []
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return []
    for line in lines:
        line = line.strip()
        if not line:
            continue
        try:
            row = json.loads(line)
        except ValueError:
            continue
        if isinstance(row, dict):
            rows.append(row)
    return rows


def lane_path(home: Path, lane_id: str) -> Path:
    return home / "lanes" / f"{lane_id}.json"


def read_lanes(home: Path) -> list[dict]:
    """All lane registrations, sorted by start time then id."""
    lanes: list[dict] = []
    folder = home / "lanes"
    if folder.is_dir():
        for path in sorted(folder.glob("*.json")):
            lane = read_json(path, None)
            if isinstance(lane, dict) and lane.get("id"):
                lanes.append(lane)
    lanes.sort(key=lambda row: (str(row.get("started_at") or ""), str(row.get("id"))))
    return lanes


def write_lane(home: Path, lane: dict) -> Path:
    path = lane_path(home, str(lane["id"]))
    write_json_atomic(path, lane)
    return path


def tail_lines(path: str | Path | None, count: int = 20) -> list[str]:
    """Last ``count`` readable lines of a log file ([] when missing/unreadable).

    Only the end of the file is read (logs can be large transcripts). Lines that are JSON objects
    (JSONL agent transcripts) are reduced to their human-readable text, and dropped when they carry
    none, so summaries and log tails never show raw JSON.
    """
    if not path:
        return []
    try:
        with open(path, "rb") as fh:
            fh.seek(0, 2)
            size = fh.tell()
            fh.seek(max(0, size - _TAIL_BYTES))
            text = fh.read().decode("utf-8", errors="replace")
    except OSError:
        return []
    lines = text.splitlines()
    if size > _TAIL_BYTES and lines:
        lines = lines[1:]  # the first line is probably cut
    out: list[str] = []
    for line in reversed(lines):
        readable = readable_line(line)
        if readable:
            out.append(readable)
            if len(out) >= count:
                break
    return list(reversed(out))


_TAIL_BYTES = 256 * 1024


def readable_line(line: str) -> str:
    """A log line as text: plain lines unchanged; JSON lines reduced to their text, or ''."""
    stripped = line.strip()
    if not stripped.startswith("{"):
        return line.rstrip()
    try:
        obj = json.loads(stripped)
    except ValueError:
        return line.rstrip()
    texts: list[str] = []
    _collect_text(obj, texts)
    joined = " ".join(" ".join(t.split()) for t in texts if t.strip())
    if joined.startswith(("{", "[")):
        try:
            json.loads(joined)
            return ""  # the text is itself JSON (e.g. a tool result): not readable
        except ValueError:
            pass
    return joined[:300]


def _collect_text(node, out: list[str]) -> None:
    """Gather human text from a transcript record: text blocks, string contents, commands."""
    if isinstance(node, dict):
        if node.get("type") == "text" and isinstance(node.get("text"), str):
            out.append(node["text"])
            return
        if node.get("type") == "tool_use" and isinstance(node.get("input"), dict):
            cmd = node["input"].get("command") or node["input"].get("description")
            if isinstance(cmd, str):
                out.append(f"$ {cmd}")
            return
        for key in ("message", "content", "text"):
            value = node.get(key)
            if isinstance(value, str) and key != "message":
                out.append(value)
            elif isinstance(value, (dict, list)):
                _collect_text(value, out)
    elif isinstance(node, list):
        for item in node:
            _collect_text(item, out)


def stamp(record: dict) -> dict:
    """Return a copy of ``record`` with an ``at`` timestamp if absent."""
    if not record.get("at"):
        record = {"at": now_iso(), **record}
    return record
