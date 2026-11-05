"""The evidence of a finished plan item: what to show in the app.

A done item keeps its proof in ``docs/evidence/<item id>/`` of the project (``i-`` and 8 hex digits, see
``roadmap.item_id``): the test output (``tests.txt``, or any ``*.log`` / ``*.txt``) and, when the interface
changed, screenshots (``*.png``, ``*.jpg``, ``*.webp``). :func:`for_item` reads it for
``state.projects[].roadmap.steps[].items[].evidence``. See ``docs/CONTRACT-roadmap.md``, "Evidence".
"""

from __future__ import annotations

import re
from pathlib import Path

EVIDENCE_DIR = "docs/evidence"
IMAGE_SUFFIXES = (".png", ".jpg", ".jpeg", ".webp", ".gif")
TEXT_SUFFIXES = (".txt", ".log")
TESTS_MAX = 4000  # characters of test output kept (the end: the summary line is last)
SHOTS_MAX = 12
_DONE_RE = re.compile(r"\(hecho:\s*([^()]*?)\s*\)", re.IGNORECASE)
_ID_RE = re.compile(r"^i-[0-9a-f]{8}$")


def commit_of(text: str) -> str | None:
    """The commit in an item's ``(hecho: <sha>)`` note; None for no note or ``sin cambios``."""
    found = _DONE_RE.search(text or "")
    sha = found.group(1) if found else ""
    return sha if re.fullmatch(r"[0-9a-f]{7,40}", sha) else None


def folder_of(repo: Path, ident: str) -> Path | None:
    if not _ID_RE.match(ident):
        return None
    path = Path(repo) / EVIDENCE_DIR / ident
    return path if path.is_dir() else None


def _tail(text: str) -> str:
    text = text.strip()
    return text if len(text) <= TESTS_MAX else "…" + text[-TESTS_MAX:]


def for_item(repo: Path | str | None, ident: str, text: str) -> dict | None:
    """``{"commit", "tests", "shots"}`` for a done item, or None when there is nothing to show.

    ``tests`` is the end of the test output; ``shots`` are absolute file paths."""
    commit = commit_of(text)
    folder = folder_of(Path(repo), ident) if repo else None
    tests, shots = "", []
    if folder is not None:
        files = sorted(p for p in folder.iterdir() if p.is_file())
        for path in files:
            if path.suffix.lower() in TEXT_SUFFIXES:
                try:
                    tests += path.read_text(encoding="utf-8", errors="replace").strip() + "\n"
                except OSError:
                    pass
        shots = [str(p) for p in files if p.suffix.lower() in IMAGE_SUFFIXES][:SHOTS_MAX]
    if not (commit or tests or shots):
        return None
    out: dict = {"commit": commit, "tests": _tail(tests), "shots": shots}
    return out
