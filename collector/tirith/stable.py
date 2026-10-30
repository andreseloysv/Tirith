"""Tirith runs from a stable copy that only advances when the full test passes on the new version.

``<home>/stable/releases/<id>/`` holds copies of the collector (``tirith/``, ``tests/``, ``pyproject.toml``,
``uv.lock``); ``<home>/stable/current`` points at the one Tirith runs from and ``previous`` at the one before.
:func:`promote` copies the source into a new release, runs the test on that copy and moves ``current`` only when
it passes; a red test throws the copy away and ``current`` stays. :func:`guard` is fed the health level of every
dispatcher pass: red for ``RED_S`` right after a promotion (within ``WATCH_S``) puts ``previous`` back by itself.
The record is ``<home>/stable/stable.json``. See ``docs/CONTRACT-selfcheck.md`` ("Stable copy").
"""

from __future__ import annotations

import os
import shutil
from datetime import datetime
from pathlib import Path
from typing import Callable

from tirith.home import now_iso, parse_iso
from tirith.store import read_json, write_json_atomic

WATCH_S = 1800.0   # a new release is on trial this long after it became current
RED_S = 120.0      # health must stay red this long (no flapping) before the release is rolled back
KEEP = 3           # releases kept on disk (current and previous always)
COPY = ("tirith", "tests", "pyproject.toml", "uv.lock")
_IGNORE = shutil.ignore_patterns("__pycache__", "*.pyc", ".pytest_cache", ".venv")


def stable_root(home: Path) -> Path:
    return home / "stable"


def current_dir(home: Path) -> Path | None:
    """The folder Tirith runs from (None before the first promotion)."""
    link = stable_root(home) / "current"
    return link.resolve() if link.is_symlink() and link.resolve().is_dir() else None


def read_record(home: Path) -> dict:
    data = read_json(stable_root(home) / "stable.json", {})
    return data if isinstance(data, dict) else {}


def _save(home: Path, record: dict) -> None:
    write_json_atomic(stable_root(home) / "stable.json", record)


def _point(home: Path, name: str, target: Path | None) -> None:
    """Atomically make ``stable/<name>`` a symlink to ``target`` (or remove it)."""
    link = stable_root(home) / name
    if target is None:
        link.unlink(missing_ok=True)
        return
    tmp = link.with_name(f".{name}.tmp")
    tmp.unlink(missing_ok=True)
    tmp.symlink_to(target)
    os.replace(tmp, link)


def _name(path: Path | None) -> str | None:
    return path.name if path else None


def copy_release(source: Path, dest: Path) -> None:
    dest.mkdir(parents=True)
    for entry in COPY:
        src = source / entry
        if src.is_dir():
            shutil.copytree(src, dest / entry, ignore=_IGNORE)
        elif src.is_file():
            shutil.copy2(src, dest / entry)


def _release_id(home: Path, now: datetime | None) -> str:
    stamp = now_iso(now).replace("-", "").replace(":", "")
    base, n = stamp, 1
    while (stable_root(home) / "releases" / base).exists():
        n += 1
        base = f"{stamp}-{n}"
    return base


def _prune(home: Path) -> None:
    releases = stable_root(home) / "releases"
    keep = {p.resolve() for p in (stable_root(home) / "current", stable_root(home) / "previous") if p.is_symlink()}
    folders = sorted((p for p in releases.iterdir() if p.is_dir()), key=lambda p: p.name, reverse=True)
    for folder in folders[KEEP:]:
        if folder.resolve() not in keep:
            shutil.rmtree(folder, ignore_errors=True)


def promote(home: Path, source: Path, test: Callable[[Path], bool], now: datetime | None = None) -> dict:
    """Copy ``source`` to a new release, run ``test(release_dir)`` on it, advance ``current`` only if it passes.

    Returns ``{"promoted": bool, "release": id, "reason": str}``."""
    root = stable_root(home)
    (root / "releases").mkdir(parents=True, exist_ok=True)
    release = root / "releases" / _release_id(home, now)
    copy_release(source, release)
    try:
        passed = bool(test(release))
    except Exception as exc:  # a test run that cannot even start is a red one
        passed = False
        reason = f"the test could not run ({type(exc).__name__}: {exc})"
    else:
        reason = "the full test passed" if passed else "the full test failed on the new version"
    record = read_record(home)
    if not passed:
        shutil.rmtree(release, ignore_errors=True)
        record["last_rejected"] = {"at": now_iso(now), "reason": reason}
        _save(home, record)
        return {"promoted": False, "release": release.name, "reason": reason}
    old = current_dir(home)
    if old is not None:
        _point(home, "previous", old)
    _point(home, "current", release)
    record.update(current=release.name, previous=_name(old), promoted_at=now_iso(now), red_since=None,
                  last_promoted={"at": now_iso(now), "release": release.name})
    _save(home, record)
    _prune(home)
    return {"promoted": True, "release": release.name, "reason": reason}


def rollback(home: Path, reason: str, now: datetime | None = None) -> dict:
    """Put ``previous`` back as ``current``. Returns ``{"rolled_back": bool, "release": id|None, "reason": str}``."""
    prev = stable_root(home) / "previous"
    if not prev.is_symlink() or not prev.resolve().is_dir():
        return {"rolled_back": False, "release": None, "reason": "there is no earlier copy to go back to"}
    target, bad = prev.resolve(), current_dir(home)
    _point(home, "current", target)
    _point(home, "previous", None)  # the bad release is not a place to return to
    record = read_record(home)
    record.update(current=target.name, previous=None, promoted_at=None, red_since=None,
                  last_rollback={"at": now_iso(now), "from": _name(bad), "to": target.name, "reason": reason})
    _save(home, record)
    if bad is not None:
        shutil.rmtree(bad, ignore_errors=True)
    return {"rolled_back": True, "release": target.name, "reason": reason}


def _clear_red(home: Path, record: dict) -> None:
    if record.get("red_since"):
        record["red_since"] = None
        _save(home, record)


def guard(home: Path, level: str, now: datetime | None = None) -> dict | None:
    """Feed one health level (``green|amber|red``). Rolls back a release on trial whose health stays red.

    Returns the rollback result when one happened, else None."""
    record = read_record(home)
    promoted = parse_iso(record.get("promoted_at"))
    current = parse_iso(now_iso(now))
    if promoted is None or current is None or (current - promoted).total_seconds() > WATCH_S or level != "red":
        _clear_red(home, record)
        return None
    since = parse_iso(record.get("red_since"))
    if since is None:
        record["red_since"] = now_iso(now)
        _save(home, record)
        return None
    if (current - since).total_seconds() < RED_S:
        return None
    return rollback(home, "health went red after the new version started", now)


def status(home: Path) -> dict:
    record = read_record(home)
    return {"current": record.get("current"), "previous": record.get("previous"),
            "on_trial": bool(record.get("promoted_at")), "last_rejected": record.get("last_rejected"),
            "last_rollback": record.get("last_rollback")}
