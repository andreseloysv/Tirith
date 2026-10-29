"""Thin integration over ``tirith_guard`` (the commit-word guard module).

The guard module itself stays untouched: same hooks, same encoded default
word list, same never-edit-an-existing-hook behaviour. This wrapper only
loads it (it lives next to the package, or installed as a top module) and
turns its exceptions into problem strings the collector can report.
"""

from __future__ import annotations

import importlib
import importlib.util
from pathlib import Path


def _load_guard_module():
    try:
        return importlib.import_module("tirith_guard")
    except ImportError:
        pass
    candidate = Path(__file__).resolve().parent.parent / "tirith_guard.py"
    if candidate.exists():
        spec = importlib.util.spec_from_file_location("tirith_guard", candidate)
        if spec and spec.loader:
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            return module
    raise ImportError("tirith_guard module not found")


_guard = None


def guard_module():
    """The loaded ``tirith_guard`` module (loaded once)."""
    global _guard
    if _guard is None:
        _guard = _load_guard_module()
    return _guard


def default_words() -> list[str]:
    """The guard module's encoded default word list, decoded."""
    return list(guard_module().DEFAULT_WORDS)


def verify_guard(repo: Path | None, words: list[str]) -> list[str]:
    """Guard problems for a repo ([] = intact). Errors become problems."""
    if repo is None:
        return ["no repo path configured"]
    if not Path(repo).exists():
        return [f"repo path does not exist: {repo}"]
    try:
        return list(guard_module().verify(Path(repo), list(words)))
    except Exception as exc:  # not a git repo, git missing, unreadable hooks
        return [f"guard check failed: {exc}"]


def install_guard(repo: Path, words: list[str]) -> list[str]:
    """Install missing hooks via the guard module; returns problems left."""
    try:
        return list(guard_module().install(Path(repo), list(words)))
    except Exception as exc:
        return [f"guard install failed: {exc}"]


def guard_upgrade_steps(repo: Path, words: list[str]) -> list[str]:
    """Owner steps when the guard is intact but older than the current rendering ([] otherwise)."""
    try:
        return list(guard_module().outdated(Path(repo), list(words)))
    except Exception:
        return []
