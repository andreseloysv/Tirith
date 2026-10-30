"""Tests clean up the temp folders they create, protected guard hooks included."""

from __future__ import annotations

import os
import stat

from conftest import _unlock_tree


def test_cleanup_removes_a_protected_hook(tmp_path) -> None:
    import tirith_guard
    hooks = tmp_path / "repo" / ".git" / "hooks"
    hooks.mkdir(parents=True)
    hook = hooks / "pre-commit"
    hook.write_text("#!/bin/sh\n", encoding="utf-8")
    tirith_guard._lock(hook)
    _unlock_tree(tmp_path)
    if hasattr(os, "chflags"):
        assert not hook.stat().st_flags & stat.UF_IMMUTABLE
    hook.unlink()  # would raise "Operation not permitted" while protected


def test_cleanup_policy_keeps_only_failed_runs(request) -> None:
    assert request.config.getini("tmp_path_retention_policy") == "failed"


# -- Tirith removes old test temp folders by itself (diskdiet.clear_temp_leftovers) -------------


import getpass
import time

from tirith import diskdiet as diskdiet_mod


def _locked_file(path) -> None:
    import tirith_guard
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("#!/bin/sh\n", encoding="utf-8")
    tirith_guard._lock(path)


def _clean(root, **kw) -> int:
    return diskdiet_mod.clear_temp_leftovers(root, now_s=time.time() + 7200, in_use=lambda base: set(), **kw)


def test_temp_cleanup_removes_old_folders_with_protected_hooks(tmp_path) -> None:
    root = tmp_path.resolve() / "tmp"
    base = root / f"pytest-of-{getpass.getuser()}"
    for folder in (base / "pytest-1" / "t0" / ".git" / "hooks", root / "garbage-abc" / "hooks",
                   root / "pytest-old-archive" / "pytest-9" / "hooks"):
        _locked_file(folder / "pre-commit")
    keep = root / "other-tool"
    keep.mkdir()
    assert _clean(root) > 0
    assert not (base / "pytest-1").exists() and not (root / "garbage-abc").exists()
    assert not (root / "pytest-old-archive").exists() and keep.is_dir()


def test_temp_cleanup_keeps_young_and_in_use_runs(tmp_path) -> None:
    root = tmp_path.resolve() / "tmp"
    base = root / f"pytest-of-{getpass.getuser()}"
    for n in (1, 2):
        (base / f"pytest-{n}").mkdir(parents=True)
    used = {base / "pytest-2"}
    diskdiet_mod.clear_temp_leftovers(root, now_s=time.time() + 7200, in_use=lambda b: used)
    assert not (base / "pytest-1").exists() and (base / "pytest-2").is_dir()
    (base / "pytest-3").mkdir()
    diskdiet_mod.clear_temp_leftovers(root, now_s=time.time() + 10, in_use=lambda b: set())
    assert (base / "pytest-3").is_dir()  # under an hour old


def test_temp_cleanup_never_follows_a_link_to_the_real_hooks(tmp_path) -> None:
    import tirith_guard
    real = tmp_path.resolve() / "realrepo" / ".git" / "hooks"
    _locked_file(real / "pre-commit")
    root = tmp_path.resolve() / "tmp"
    (root / "garbage-x").mkdir(parents=True)
    (root / "garbage-x" / "hooks-link").symlink_to(real, target_is_directory=True)
    (root / "garbage-x" / "file-link").symlink_to(real / "pre-commit")
    _clean(root)
    assert not (root / "garbage-x").exists()
    assert (real / "pre-commit").is_file() and tirith_guard._is_locked(real / "pre-commit")


def test_temp_cleanup_refuses_a_folder_outside_the_temp_root(tmp_path) -> None:
    import tirith_guard
    outside = tmp_path.resolve() / "repo" / "hooks"
    _locked_file(outside / "pre-commit")
    root = tmp_path.resolve() / "tmp"
    root.mkdir()
    assert diskdiet_mod._remove_tree(outside, root, time.monotonic() + 5) == 0
    assert diskdiet_mod._remove_tree(root, root, time.monotonic() + 5) == 0  # the root itself
    (root / "garbage-link").symlink_to(outside, target_is_directory=True)  # a garbage-* link to the repo
    _clean(root)
    assert (outside / "pre-commit").is_file() and tirith_guard._is_locked(outside / "pre-commit")


def test_temp_cleanup_runs_in_background_every_15_minutes_or_when_disk_is_low(home, tmp_path, monkeypatch) -> None:
    monkeypatch.setattr(diskdiet_mod, "TEMP_CLEANUP_ENABLED", True)
    monkeypatch.setitem(diskdiet_mod._cleanup_state, "at", None)
    monkeypatch.setitem(diskdiet_mod._cleanup_state, "thread", None)
    root = tmp_path.resolve() / "tmp"
    start = lambda free: diskdiet_mod.start_temp_cleanup(
        home, free, tmp_root=root, in_use=lambda b: set(), wait=True)
    for n in range(3):
        (root / f"garbage-{n}").mkdir(parents=True)
        (root / f"garbage-{n}" / "f").write_bytes(b"x" * 100)
    assert start(100 * 1024 ** 3)
    assert not start(100 * 1024 ** 3)  # less than 15 minutes ago
    assert start(5 * 1024 ** 3)  # under 20 GB: right away
    assert "old test temp folders" in (home / "diskdiet.jsonl").read_text(encoding="utf-8")


def test_temp_cleanup_is_bounded_in_time(tmp_path) -> None:
    root = tmp_path.resolve() / "tmp"
    (root / "garbage-1").mkdir(parents=True)
    (root / "garbage-1" / "f").write_text("x")
    diskdiet_mod.clear_temp_leftovers(root, in_use=lambda b: set(), budget_s=-1)
    assert (root / "garbage-1" / "f").exists()  # out of time: left for the next pass
