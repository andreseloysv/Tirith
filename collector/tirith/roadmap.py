"""A project's plan, read from its markdown checklist (``docs/CONTRACT-roadmap.md``).

Each ``## <n>. <title>`` heading is a step; ``- [ ]`` / ``- [x]`` lines are its items.
Parsing never raises: a missing or unreadable file gives ``None``.
"""

from __future__ import annotations

import hashlib
import re
from datetime import date
from pathlib import Path

from tirith import manifest as manifest_mod

MAX_BYTES = 256 * 1024
DEFAULT_FILES = ("docs/TODO.md", "TODO.md")
_STEP_RE = re.compile(r"^##\s+(\d+)\.\s*(.+?)\s*#*\s*$")
_HEADING_RE = re.compile(r"^#{1,6}\s")
_ITEM_RE = re.compile(r"^\s*[-*+]\s+\[([ xX])\]\s*(.*)$")
_BULLET_RE = re.compile(r"^\s*[-*+]\s")
_DEADLINE_RE = re.compile(r"^\s*deadline\s*:\s*(\S+)\s*$", re.IGNORECASE)
_OUT_RE = re.compile(r"\((?:fuera de este plan|out of this plan)", re.IGNORECASE)
_OWNER_RE = re.compile(r"\((?:dueño|owner)", re.IGNORECASE)
_FROM_RE = re.compile(r"\((?:desde|from)\s+(\d{4}-\d{2}-\d{2})", re.IGNORECASE)

_cache: dict[str, tuple[int, int, list[dict]]] = {}


def tag(text: str) -> tuple[str, str | None]:
    """``(who, date)`` from the tags in an item's text: ``out`` (not in this plan), ``owner``, ``date``
    (blocked until ``YYYY-MM-DD``) or ``agent``."""
    if _OUT_RE.search(text):
        return "out", None
    if _OWNER_RE.search(text):
        return "owner", None
    found = _FROM_RE.search(text)
    if found:
        try:
            date.fromisoformat(found.group(1))
            return "date", found.group(1)
        except ValueError:
            pass
    return "agent", None


def who_on(item: dict, today: date) -> str:
    """``agent|owner|date|out`` for an item today: a ``date`` item whose day has come is an agent's."""
    who, until = tag(item["text"])
    return "agent" if who == "date" and until and date.fromisoformat(until) <= today else who


def agent_items(step: dict, today: date) -> list[str]:
    """The texts of the open items of a parsed step that an agent can do today."""
    return [i["text"] for i in step["items"] if not i["done"] and who_on(i, today) == "agent"]


def owner_item_id(project: str, step_id: str, text: str) -> str:
    """The stable id of the owner item raised for an owner-only roadmap item."""
    return f"owner-{project}-{step_id}-" + hashlib.sha1(" ".join(text.split()).encode()).hexdigest()[:8]


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


def step_status(done: int, total: int, working: bool, agent_open: int = 0, owner_open: int = 0,
                date_open: int = 0) -> str:
    """``empty|done|active|ready|blocked-owner|blocked-date|todo`` (see ``docs/CONTRACT-roadmap.md``)."""
    if not total:
        return "empty"
    if done == total:
        return "done"
    if working:
        return "active"
    if agent_open:
        return "ready"
    if owner_open:
        return "blocked-owner"
    return "blocked-date" if date_open else "todo"


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


def title_of(text: str) -> str:
    """The first ``# `` heading (a leading ``TODO:`` / ``Plan:`` label is dropped); may be empty."""
    for line in text.splitlines():
        if line.startswith("# "):
            return re.sub(r"^(?:todo|plan)\s*:\s*", "", line[2:].strip(), flags=re.IGNORECASE)
    return ""


def deadline_of(text: str) -> str | None:
    """The value of a ``Deadline: <iso>`` line before the first step (may be None)."""
    for line in text.splitlines():
        if line.startswith("##"):
            break
        found = _DEADLINE_RE.match(line)
        if found:
            return found.group(1)
    return None


def plan_paths(folder: Path, manifest: manifest_mod.Manifest) -> list[Path]:
    """The current plan first (``roadmap``), then the files of ``plans`` (each once, existing only)."""
    out: list[Path] = []
    for path in [find_file(folder, manifest.roadmap)] + [find_file(folder, name) for name in manifest.plans]:
        if path is not None and path.resolve() not in {p.resolve() for p in out}:
            out.append(path)
    return out


def _plan_row(path: Path, steps: list[dict], manifest: manifest_mod.Manifest, current: bool,
              working: dict[str, list[str]]) -> dict:
    rows = []
    for step in steps:
        items = step["items"]
        done = sum(1 for i in items if i["done"])
        mine = working.get(step["id"], []) if current else []
        rows.append({"id": step["id"], "title": step["title"],
                     "status": step_status(done, len(items), bool(mine)),
                     "done": done, "total": len(items),
                     "items": [dict(i) for i in items], "agents": mine})
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        text = ""
    done, total = sum(r["done"] for r in rows), sum(r["total"] for r in rows)
    deadline = deadline_of(text) or (manifest.deadline if current else None)
    return {"id": path.stem, "title": title_of(text) or path.stem, "current": current, "file": str(path),
            "deadline": deadline, "done": done, "total": total, "finished": bool(total and done == total),
            "steps": rows}


def build_plans(folder: str | Path | None, project: str, agents: list[dict]) -> list[dict]:
    """``state.projects[].plans``: every plan of the project, the current one (``roadmap``) first."""
    if not folder:
        return []
    folder = Path(folder)
    manifest = manifest_mod.load(folder)
    working = _working(agents, project)
    out = []
    for path in plan_paths(folder, manifest):
        steps = _parsed(path)
        if steps is not None:
            out.append(_plan_row(path, steps, manifest, not out, working))
    return out


def _working(agents: list[dict], project: str) -> dict[str, list[str]]:
    working: dict[str, list[str]] = {}
    for agent in agents:
        if agent.get("project") == project and agent.get("status") == "working" and agent.get("step"):
            working.setdefault(str(agent["step"]), []).append(str(agent["id"]))
    return working


def build(folder: str | Path | None, project: str, agents: list[dict], today: date | None = None) -> dict | None:
    """``state.projects[].roadmap`` for a project folder; None when it has no roadmap file."""
    today = today or date.today()
    if not folder:
        return None
    folder = Path(folder)
    manifest = manifest_mod.load(folder)
    path = next(iter(plan_paths(folder, manifest)), None)  # the current plan
    steps = _parsed(path) if path else None
    if steps is None:
        return None
    working = _working(agents, project)
    rows = []
    for step in steps:
        items = []
        for raw in step["items"]:
            who = who_on(raw, today)
            item = {"text": raw["text"], "done": raw["done"], "who": who}
            if who == "date":
                item["until"] = tag(raw["text"])[1]
            elif who == "owner" and not raw["done"]:
                item["owner_item"] = owner_item_id(project, step["id"], raw["text"])
            items.append(item)
        counted = [i for i in items if i["who"] != "out"]
        done = sum(1 for i in counted if i["done"])
        mine = working.get(step["id"], [])
        open_by = {w: [i for i in counted if not i["done"] and i["who"] == w] for w in ("agent", "owner", "date")}
        row = {"id": step["id"], "title": step["title"],
               "status": step_status(done, len(counted), bool(mine), len(open_by["agent"]),
                                     len(open_by["owner"]), len(open_by["date"])),
               "done": done, "total": len(counted), "items": items, "agents": mine}
        if row["status"] == "blocked-date":
            row["until"] = min(i["until"] for i in open_by["date"])
        rows.append(row)
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
