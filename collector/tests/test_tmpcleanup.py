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
