"""The guard's protection flag on a real repository's hooks survives the test suite (30 Oct 18:58:19Z: the
test cleanup cleared it through a symlink / worktree common dir; cleanup now stays inside temp folders)."""

from __future__ import annotations

import os
import stat
import sys
from pathlib import Path

import pytest

from conftest import _unlock_tree, inside_temp, make_hooks_mutable, make_repo

pytestmark = pytest.mark.skipif(not hasattr(os, "chflags") or sys.platform == "win32", reason="BSD flags only")


def _lock(p: Path) -> None:
    os.chflags(p, p.stat().st_flags | stat.UF_IMMUTABLE)


def _locked(p: Path) -> bool:
    return bool(p.stat().st_flags & stat.UF_IMMUTABLE)


def test_guard_flags_stay_outside_temp_is_never_unlocked(tmp_path: Path, monkeypatch) -> None:
    real = Path(__file__).resolve().parent.parent  # a real checkout, not a temp folder
    assert not inside_temp(real)
    assert inside_temp(tmp_path)


def test_guard_flags_stay_through_a_symlink_from_temp(tmp_path: Path, monkeypatch) -> None:
    import tempfile
    # A "real" repo outside the pytest temp dirs: a folder beside the checkout is not in a temp root.
    outside = Path(__file__).resolve().parent / "data" / "_flags_probe"
    outside.mkdir(parents=True, exist_ok=True)
    hook = outside / "pre-commit"
    hook.write_text("#!/bin/sh\n")
    try:
        _lock(hook)
        (tmp_path / "link").symlink_to(outside)
        _unlock_tree(tmp_path)
        assert _locked(hook)
        make_hooks_mutable(tmp_path / "link")
        assert _locked(hook)
    finally:
        os.chflags(hook, hook.stat().st_flags & ~stat.UF_IMMUTABLE)
        hook.unlink()
        outside.rmdir()


def test_guard_flags_stay_in_temp_repos_still_unlock(tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    hook = repo / ".git" / "hooks" / "pre-commit"
    hook.write_text("#!/bin/sh\n")
    _lock(hook)
    make_hooks_mutable(repo)
    assert not _locked(hook)
