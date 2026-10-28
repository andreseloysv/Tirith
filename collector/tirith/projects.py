"""Projects are folders, plus sessions (see ``docs/CONTRACT-sessions.md``).

Registered projects live in ``config.toml`` as ``[[projects]]`` rows (the folder is the
``repo`` key; ``folder`` is accepted as an alias).  Folders the owner never registered but
that lanes, pipeline stages or rooms point at are derived into *auto* projects and persisted
in ``projects-auto.json`` so their ids stay stable.  Sessions live in ``sessions.json`` and
the current one in the ``session`` file (or env ``TIRITH_SESSION``).
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

from tirith.home import Config, load_config, now_iso
from tirith.store import read_json, write_json_atomic, write_text_atomic

PALETTE = ("#2f7d5b", "#3b6fb6", "#b5654a", "#8a5fb0", "#b08d2f", "#2f8a94", "#a84e72", "#6b7f3a")
_COLOR_RE = re.compile(r"^#[0-9a-fA-F]{6}$")
AUTO_FILE = "projects-auto.json"
SESSIONS_FILE = "sessions.json"
SESSION_FILE = "session"


def slug(text: str, fallback: str = "project", limit: int = 40) -> str:
    out = re.sub(r"[^a-z0-9]+", "-", (text or "").lower()).strip("-")[:limit].strip("-")
    return out or fallback


def color_for(key: str) -> str:
    return PALETTE[int(hashlib.sha1(key.encode("utf-8")).hexdigest(), 16) % len(PALETTE)]


def valid_color(value: str) -> bool:
    return bool(_COLOR_RE.match(value or ""))


def real(path: str | Path) -> Path:
    """Absolute, symlink-free path (works for folders that do not exist yet)."""
    return Path(path).expanduser().resolve()


def git_toplevel(folder: Path) -> Path | None:
    if not folder.is_dir():
        return None
    try:
        out = subprocess.run(["git", "-C", str(folder), "rev-parse", "--show-toplevel"],
                             capture_output=True, text=True, timeout=10)
    except (OSError, subprocess.SubprocessError):
        return None
    top = out.stdout.strip()
    return real(top) if out.returncode == 0 and top else None


def project_root(folder: str | Path) -> Path:
    """The folder a project derived from ``folder`` would use: git top-level, else itself."""
    folder = real(folder)
    return git_toplevel(folder) or folder


# -- project entries ------------------------------------------------------------------------


@dataclass
class Entry:
    id: str
    name: str
    folder: Path | None
    color: str
    auto: bool


def load_auto(home: Path) -> dict:
    data = read_json(Path(home) / AUTO_FILE, {})
    if not isinstance(data, dict):
        data = {}
    projects = [p for p in data.get("projects") or []
                if isinstance(p, dict) and p.get("id") and p.get("folder")]
    ignored = [str(f) for f in data.get("ignored") or []]
    return {"projects": projects, "ignored": ignored}


def save_auto(home: Path, data: dict) -> None:
    write_json_atomic(Path(home) / AUTO_FILE, data)


def entries(config: Config, auto: dict) -> list[Entry]:
    out: list[Entry] = []
    for proj in config.projects:
        out.append(Entry(proj.id, proj.name, real(proj.repo) if proj.repo else None,
                         proj.color, False))
    seen = {e.id for e in out}
    for row in auto["projects"]:
        if str(row["id"]) in seen:
            continue
        out.append(Entry(str(row["id"]), str(row.get("name") or row["id"]),
                         real(row["folder"]), str(row.get("color") or color_for(str(row["id"]))),
                         True))
    return out


def match(folder: str | Path | None, items: list[Entry]) -> Entry | None:
    """The project whose folder is the longest prefix of ``folder``."""
    if not folder:
        return None
    target = real(folder)
    best: Entry | None = None
    for item in items:
        if item.folder is None:
            continue
        if target == item.folder or item.folder in target.parents:
            if best is None or len(item.folder.parts) > len(best.folder.parts):  # type: ignore[union-attr]
                best = item
    return best


def unique_id(wanted: str, taken: set[str]) -> str:
    candidate, number = wanted, 2
    while candidate in taken:
        candidate = f"{wanted}-{number}"
        number += 1
    return candidate


def discover(home: Path, config: Config, folders: list[str | Path]) -> list[Entry]:
    """Register auto projects for folders outside every known project; returns all entries."""
    auto = load_auto(home)
    items = entries(config, auto)
    changed = False
    for folder in folders:
        if not folder or match(folder, items) is not None:
            continue
        root = project_root(folder)
        if str(root) in auto["ignored"] or match(root, items) is not None:
            continue
        pid = unique_id(slug(root.name), {e.id for e in items})
        row = {"id": pid, "name": root.name or pid, "folder": str(root), "color": color_for(pid)}
        auto["projects"].append(row)
        items.append(Entry(pid, row["name"], root, row["color"], True))
        changed = True
    if changed:
        save_auto(home, auto)
    return items


# -- config.toml editing --------------------------------------------------------------------


def _toml_str(value: str) -> str:
    return json.dumps(value)


def _project_block(pid: str, name: str, folder: Path, color: str | None,
                   words: list[str] | None) -> str:
    lines = ["[[projects]]", f"id = {_toml_str(pid)}", f"name = {_toml_str(name)}",
             f"repo = {_toml_str(str(folder))}"]
    if color:
        lines.append(f"color = {_toml_str(color)}")
    if words:
        lines.append("guard_words = [" + ", ".join(_toml_str(w) for w in words) + "]")
    return "\n".join(lines) + "\n"


def add_project(home: Path, folder: str | Path, name: str | None = None,
                color: str | None = None, words: list[str] | None = None) -> tuple[str, str, bool]:
    """Register a folder. Returns ``(id, folder, created)``; created is False when the folder
    is already registered (nothing is written)."""
    root = real(folder)
    if not root.is_dir():
        raise ValueError(f"not a folder: {folder}")
    if color is not None and not valid_color(color):
        raise ValueError(f"--color must look like #2f7d5b, got {color!r}")
    config = load_config(home)
    for proj in config.projects:
        if proj.repo is not None and real(proj.repo) == root:
            return proj.id, str(root), False
    auto = load_auto(home)
    # Registering an auto-discovered folder renames/promotes it: drop the auto row.
    promoted = [p for p in auto["projects"] if real(p["folder"]) == root]
    wanted_name = name or (str(promoted[0].get("name")) if promoted else "") \
        or root.name or "project"
    taken = {p.id for p in config.projects} | {str(p["id"]) for p in auto["projects"]
                                                if p not in promoted}
    pid = unique_id(slug(name or root.name), taken)
    if color is None and promoted and promoted[0].get("color"):
        color = str(promoted[0]["color"])
    path = Path(home) / "config.toml"
    existing = path.read_text(encoding="utf-8") if path.exists() else ""
    if existing and not existing.endswith("\n"):
        existing += "\n"
    separator = "\n" if existing.strip() else ""
    write_text_atomic(path, existing + separator
                      + _project_block(pid, wanted_name, root, color or color_for(pid), words))
    ignored = [f for f in auto["ignored"] if f != str(root)]
    if promoted or len(ignored) != len(auto["ignored"]):
        auto["projects"] = [p for p in auto["projects"] if p not in promoted]
        auto["ignored"] = ignored
        save_auto(home, auto)
    return pid, str(root), True


_ID_LINE_RE = re.compile(r"""^\s*id\s*=\s*["']([^"']+)["']""")


def _remove_from_config(home: Path, pid: str) -> bool:
    path = Path(home) / "config.toml"
    if not path.exists():
        return False
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    out: list[str] = []
    removed = False
    index = 0
    while index < len(lines):
        if lines[index].strip() == "[[projects]]":
            end = index + 1
            while end < len(lines) and not (
                    lines[end].strip().startswith("[")
                    and not lines[end].strip().startswith("[projects.")):
                end += 1
            block = lines[index:end]
            found = next((m.group(1) for m in map(_ID_LINE_RE.match, block) if m), None)
            if found == pid:
                removed = True
                # Keep trailing blank/comment lines: they belong to whatever follows.
                keep: list[str] = []
                while block and (not block[-1].strip() or block[-1].lstrip().startswith("#")):
                    keep.insert(0, block.pop())
                if out and not out[-1].strip():
                    out.pop()
                out.extend(keep)
                index = end
                continue
            out.extend(block)
            index = end
            continue
        out.append(lines[index])
        index += 1
    if removed:
        write_text_atomic(path, "".join(out))
    return removed


def remove_project(home: Path, pid: str) -> str | None:
    """Remove a registered or auto project. Returns ``"config"``/``"auto"`` or None."""
    config = load_config(home)
    if config.project(pid) is not None:
        if _remove_from_config(home, pid):
            return "config"
        raise ValueError(f"'{pid}' is defined in a form tirith cannot edit; remove it from "
                         f"{Path(home) / 'config.toml'} by hand")
    auto = load_auto(home)
    row = next((p for p in auto["projects"] if str(p["id"]) == pid), None)
    if row is None:
        return None
    auto["projects"].remove(row)
    folder = str(real(row["folder"]))
    if folder not in auto["ignored"]:
        auto["ignored"].append(folder)
    save_auto(home, auto)
    return "auto"


# -- sessions -------------------------------------------------------------------------------


def current_session(home: Path) -> str | None:
    """Env ``TIRITH_SESSION``, else the current-session file, else None."""
    env = (os.environ.get("TIRITH_SESSION") or "").strip()
    if env:
        return env
    try:
        text = (Path(home) / SESSION_FILE).read_text(encoding="utf-8").strip()
    except OSError:
        return None
    return text or None


def read_sessions(home: Path) -> list[dict]:
    data = read_json(Path(home) / SESSIONS_FILE, [])
    rows = data.get("sessions") if isinstance(data, dict) else data
    return [r for r in rows or [] if isinstance(r, dict) and r.get("id")]


def _write_sessions(home: Path, rows: list[dict]) -> None:
    write_json_atomic(Path(home) / SESSIONS_FILE, rows)


def start_session(home: Path, title: str, session_id: str | None = None,
                  now: datetime | None = None) -> dict:
    rows = read_sessions(home)
    taken = {str(r["id"]) for r in rows}
    if session_id:
        if session_id in taken:
            raise ValueError(f"session already exists: {session_id}")
        if not re.match(r"^[A-Za-z0-9._-]+$", session_id):
            raise ValueError("session id may only use letters, digits . _ -")
    else:
        day = (now or datetime.now()).strftime("%Y-%m-%d")
        session_id = unique_id(f"{day}-{slug(title, 'session', 32)}", taken)
    row = {"id": session_id, "title": title, "started_at": now_iso(), "ended_at": None}
    rows.append(row)
    _write_sessions(home, rows)
    write_text_atomic(Path(home) / SESSION_FILE, session_id + "\n")
    return row


def end_session(home: Path, session_id: str | None = None) -> dict | None:
    """Close a session (default: the current one). None when it is unknown."""
    session_id = session_id or current_session(home)
    if not session_id:
        return None
    rows = read_sessions(home)
    row = next((r for r in rows if str(r["id"]) == session_id), None)
    if row is None:
        return None
    if not row.get("ended_at"):
        row["ended_at"] = now_iso()
        _write_sessions(home, rows)
    try:
        if (Path(home) / SESSION_FILE).read_text(encoding="utf-8").strip() == session_id:
            (Path(home) / SESSION_FILE).unlink()
    except OSError:
        pass
    return row
