"""A project's plan, read from its markdown checklist (``docs/CONTRACT-roadmap.md``).

Each ``## <n>. <title>`` heading is a step; ``- [ ]`` / ``- [x]`` lines are its items.
Parsing never raises: a missing or unreadable file gives ``None``.
"""

from __future__ import annotations

import re
from pathlib import Path

from tirith import manifest as manifest_mod

MAX_BYTES = 256 * 1024
DEFAULT_FILES = ("docs/TODO.md", "TODO.md")
_STEP_RE = re.compile(r"^##\s+(\d+)\.\s*(.+?)\s*#*\s*$")
_HEADING_RE = re.compile(r"^#{1,6}\s")
_ITEM_RE = re.compile(r"^\s*[-*+]\s+\[([ xX])\]\s*(.*)$")
_BULLET_RE = re.compile(r"^\s*[-*+]\s")

_cache: dict[str, tuple[int, int, list[dict]]] = {}


def parse(text: str) -> list[dict]:
    """Steps in file order: ``{id, title, items: [{text, done}]}``."""
    steps: list[dict] = []
    step: dict | None = None
    item: dict | None = None
    for line in text.splitlines():
        found = _STEP_RE.match(line)
        if found:
            step = {"id": found.group(1), "title": found.group(2), "items": []}
            steps.append(step)
            item = None
            continue
        if _HEADING_RE.match(line):
            step = item = None
            continue
        if step is None:
            continue
        found = _ITEM_RE.match(line)
        if found:
            item = {"text": found.group(2).strip(), "done": found.group(1) != " "}
            step["items"].append(item)
        elif not line.strip() or _BULLET_RE.match(line):
            item = None
        elif item is not None:
            item["text"] = f"{item['text']} {line.strip()}"
    return steps


def intro(text: str) -> str:
    """The first paragraph after the title, before the first ``##`` heading (may be empty)."""
    lines: list[str] = []
    for line in text.splitlines():
        if line.startswith("##"):
            break
        if line.startswith("#") or (not line.strip() and lines):
            if lines:
                break
            continue
        if line.strip():
            lines.append(line.strip())
    return " ".join(lines)


def read_intro(path: Path) -> str:
    try:
        return intro(path.read_text(encoding="utf-8")) if path.stat().st_size <= MAX_BYTES else ""
    except (OSError, UnicodeDecodeError):
        return ""


def step_status(done: int, total: int, has_agent: bool) -> str:
    if not total:
        return "empty"
    if done == total:
        return "done"
    return "active" if done or has_agent else "todo"


def find_file(folder: Path, configured: str | None) -> Path | None:
    """The roadmap file: the configured path, else ``docs/TODO.md``, else ``TODO.md``."""
    for name in ([configured] if configured else list(DEFAULT_FILES)):
        path = Path(name).expanduser()
        path = path if path.is_absolute() else folder / path
        if path.is_file():
            return path
    return None


def _parsed(path: Path) -> list[dict] | None:
    try:
        stat = path.stat()
        if stat.st_size > MAX_BYTES:
            return None
        key = (stat.st_mtime_ns, stat.st_size)
        hit = _cache.get(str(path))
        if hit and hit[:2] == key:
            return hit[2]
        steps = parse(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError):
        return None
    _cache[str(path)] = (*key, steps)
    return steps


def build(folder: str | Path | None, project: str, agents: list[dict]) -> dict | None:
    """``state.projects[].roadmap`` for a project folder; None when it has no roadmap file."""
    if not folder:
        return None
    folder = Path(folder)
    manifest = manifest_mod.load(folder)
    path = find_file(folder, manifest.roadmap)
    steps = _parsed(path) if path else None
    if steps is None:
        return None
    working: dict[str, list[str]] = {}
    for agent in agents:
        if agent.get("project") == project and agent.get("status") == "working" and agent.get("step"):
            working.setdefault(str(agent["step"]), []).append(str(agent["id"]))
    rows = []
    for step in steps:
        items = step["items"]
        done = sum(1 for i in items if i["done"])
        mine = working.get(step["id"], [])
        rows.append({"id": step["id"], "title": step["title"],
                     "status": step_status(done, len(items), bool(mine)),
                     "done": done, "total": len(items),
                     "items": [dict(i) for i in items], "agents": mine})
    return {"file": str(path), "deadline": manifest.deadline,
            "done": sum(r["done"] for r in rows), "total": sum(r["total"] for r in rows),
            "steps": rows}


def tick(text: str, step_id: str, wanted: set[str]) -> tuple[str, list[str]]:
    """``text`` with the open items of step ``step_id`` whose text is in ``wanted`` ticked; and which ones.

    Only the ``[ ]`` of the item's first line changes, so a wrapped item stays as it was written."""
    lines = text.splitlines(keepends=True)
    ticked: list[str] = []
    in_step = False
    item_at: int | None = None
    parts: list[str] = []

    def flush() -> None:
        joined = " ".join(parts).strip()
        if item_at is not None and joined in wanted:
            lines[item_at] = lines[item_at].replace("[ ]", "[x]", 1)
            ticked.append(joined)

    for index, raw in enumerate(lines):
        line = raw.rstrip("\r\n")
        found = _STEP_RE.match(line)
        if found or _HEADING_RE.match(line):
            flush()
            item_at, parts = None, []
            in_step = bool(found) and found.group(1) == str(step_id)
            continue
        if not in_step:
            continue
        item = _ITEM_RE.match(line)
        if item:
            flush()
            item_at, parts = (index, [item.group(2)]) if item.group(1) == " " else (None, [])
        elif not line.strip() or _BULLET_RE.match(line):
            flush()
            item_at, parts = None, []
        elif item_at is not None:
            parts.append(line.strip())
    flush()
    return "".join(lines), ticked
