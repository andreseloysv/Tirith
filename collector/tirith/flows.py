"""Information flows: who passed what to whom (``flows.jsonl`` -> ``flows`` in state.json).

Endpoints: ``steward``, ``owner``, ``guard``, ``agent:<lane-id>``, ``room:<room-id>``,
``pipeline:<run>``. See ``docs/CONTRACT-flows.md``.
"""

from __future__ import annotations

import re
from pathlib import Path

from tirith.home import now_iso
from tirith.store import append_jsonl, read_json, read_jsonl, write_json_atomic

KINDS = ("brief", "message", "report", "request", "answer", "decision", "merge", "alert")
STATE_FLOWS = 100
MAX_TEXT = 300
MAX_WHY = 600
_ENDPOINT_RE = re.compile(r"^(steward|owner|guard|(agent|room|pipeline):.+)$")


def valid_endpoint(value: str) -> bool:
    return bool(_ENDPOINT_RE.match(value or ""))


def one_line(text: str, limit: int = MAX_TEXT) -> str:
    return " ".join(str(text or "").split())[:limit]


def _next_id(rows: list[dict]) -> str:
    top = 0
    for row in rows:
        match = re.match(r"^f-(\d+)$", str(row.get("id") or ""))
        if match:
            top = max(top, int(match.group(1)))
    return f"f-{top + 1}"


_CURRENT = object()


def append_flow(home: Path, src: str, dst: str, kind: str, text: str,
                ref: str | None = None, session=_CURRENT, project: str | None = None,
                why: str | None = None, at: str | None = None) -> dict:
    """Record one flow; returns the stored row.

    ``session`` defaults to the current session (env or file); pass an explicit id or None
    when the writer is not the owner's shell (the collector)."""
    from tirith.projects import current_session
    path = Path(home) / "flows.jsonl"
    row = {"id": _next_id(read_jsonl(path)), "at": at or now_iso(), "from": src, "to": dst,
           "kind": kind, "text": one_line(text), "ref": str(ref) if ref else None,
           "session": current_session(home) if session is _CURRENT else session}
    if project:
        row["project"] = project
    if why:
        row["why"] = one_line(why, MAX_WHY)
    append_jsonl(path, row)
    return row


def first_line(text: str | None) -> str:
    for line in str(text or "").splitlines():
        if line.strip():
            return one_line(line, 200)
    return ""


def endpoint_name(endpoint: str, agents: dict[str, str]) -> str:
    """Display name: hero name, The Steward, You, a room id or a run id."""
    if endpoint == "steward":
        return "The Steward"
    if endpoint == "owner":
        return "You"
    if endpoint == "guard":
        return "The Guard"
    kind, _, ident = endpoint.partition(":")
    if kind == "agent":
        return agents.get(ident, ident)
    return ident


def flows_state(home: Path, agents: dict[str, str], steward_name: str | None = None,
                projects: dict[str, str] | None = None, limit: int | None = STATE_FLOWS) -> list[dict]:
    """The last flows with resolved names (malformed rows are skipped).

    ``projects`` maps endpoints (``agent:<lane>``, ``pipeline:<run>``, ``room:<id>``) to project
    ids; a flow's project is its first mapped endpoint, else the project stored on the row."""
    projects = projects or {}
    out: list[dict] = []
    for row in read_jsonl(Path(home) / "flows.jsonl"):
        src, dst = str(row.get("from") or ""), str(row.get("to") or "")
        if not (valid_endpoint(src) and valid_endpoint(dst)) or not row.get("id"):
            continue
        kind = str(row.get("kind") or "message")
        out.append({
            "id": str(row["id"]), "at": row.get("at"), "from": src, "to": dst,
            "from_name": endpoint_name(src, agents), "to_name": endpoint_name(dst, agents),
            "kind": kind if kind in KINDS else "message",
            "text": one_line(row.get("text")), "ref": row.get("ref"),
            "session": row.get("session") or None,
            "project": projects.get(src) or projects.get(dst) or row.get("project") or None,
            **({"why": one_line(row["why"], MAX_WHY)} if row.get("why") else {}),
        })
    return out if limit is None else out[-limit:]


def seen_set(home: Path, key: str) -> set[str]:
    data = read_json(Path(home) / "flows-state.json", {})
    return set(data.get(key) or []) if isinstance(data, dict) else set()


def remember(home: Path, key: str, values: set[str]) -> None:
    path = Path(home) / "flows-state.json"
    data = read_json(path, {})
    if not isinstance(data, dict):
        data = {}
    data[key] = sorted(values)
    write_json_atomic(path, data)


# -- decisions written in lane reports ---------------------------------------------------------

_BULLET = r"(?:[-*+]\s+|\d+[.)]\s+)?"
_DECISION_RE = re.compile(
    rf"^\s*{_BULLET}(?:[*_]{{2}})?(?:decision|decided)(?:[*_]{{2}})?\s*:\s*(?:[*_]{{2}})?\s*(\S.*)$", re.I)
_WHY_LINE_RE = re.compile(rf"^\s*{_BULLET}(?:[*_]{{2}})?why(?:[*_]{{2}})?\s*:\s*(?:[*_]{{2}})?\s*(\S.*)$", re.I)
_WHY_INLINE_RE = re.compile(
    r"^(?P<what>.*?)(?:\s+[—–-]{1,2}\s*(?:because|why:?)\s+|\s+because\s+|\s*[;(]\s*why:\s*)(?P<why>.+?)\)?\s*$", re.I)
_HEADING_RE = re.compile(r"^\s*(?:#{1,6}\s*(.+?)|(?:[*_]{2})([^*_]+?)(?:[*_]{2})\s*:?|([A-Za-z][A-Za-z ]{2,30}):)\s*#*\s*$")
_NOT_DECISIONS = re.compile(r"open|pending|todo|to do|next|question|unresolved|risk|blocked|follow", re.I)


def extract_decisions(report: str | None) -> list[tuple[str, str]]:
    """``(decision, why)`` for every explicit ``Decision:`` / ``Decided:`` line of a lane report.

    ``why`` is the inline ``because …`` / ``why: …`` part, or a ``Why:`` line right after, or "".
    Lines under a heading such as "Left open" or "Next steps" are never decisions."""
    lines = str(report or "").splitlines()
    out: list[tuple[str, str]] = []
    skipping = False
    for index, line in enumerate(lines):
        head = _HEADING_RE.match(line)
        if head and not _DECISION_RE.match(line):
            skipping = bool(_NOT_DECISIONS.search(next(g for g in head.groups() if g)))
            continue
        match = _DECISION_RE.match(line)
        if not match or skipping:
            continue
        text, why = match.group(1).strip(), ""
        inline = _WHY_INLINE_RE.match(text)
        if inline:
            text, why = inline.group("what").strip(), inline.group("why").strip()
        else:
            following = next((x for x in lines[index + 1:] if x.strip()), "")
            why_line = _WHY_LINE_RE.match(following)
            if why_line:
                why = why_line.group(1).strip()
        text = text.strip(" *_")
        if text:
            out.append((one_line(text), one_line(why, MAX_WHY)))
    return out
