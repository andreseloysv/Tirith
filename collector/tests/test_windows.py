"""Windows support: the commit guard rendered for Git for Windows and the safe-git shim (simulated)."""

from __future__ import annotations

import os
import stat
import sys
from pathlib import Path

import pytest

from conftest import make_hooks_mutable, make_repo
from tirith import safegit
from tirith.cli import main
from tirith.guardwrap import guard_module

WORDS = ["bannedword"]


@pytest.fixture
def windows_guard(monkeypatch: pytest.MonkeyPatch):
    mod = guard_module()
    monkeypatch.setattr(mod, "_windows", lambda: True)
    return mod


def test_windows_detected_from_platform(monkeypatch: pytest.MonkeyPatch) -> None:
    mod = guard_module()
    monkeypatch.setattr(sys, "platform", "win32")
    assert mod._windows() and mod._git() == "git"
    monkeypatch.setattr(sys, "platform", "darwin")
    assert not mod._windows() and mod._git() == "/usr/bin/git"


def test_windows_render_is_lf_and_uses_path_sh() -> None:
    mod = guard_module()
    win, posix = mod.render(WORDS, windows=True), mod.render(WORDS, windows=False)
    for name, text in win.items():
        assert "\r" not in text.replace('sub(/\\r$/', ""), name
        assert "/bin/sh -c" not in text and "-- sh -c" in text
        assert 'sub(/\\r$/, "")' in text  # a CRLF tirith.toml must not leak into commands
        assert text.startswith("#!/bin/bash\n# FORBIDDEN_WORDS_GUARD\n")
    assert all('sub(/\\r$/' not in t for t in posix.values())
    assert mod.render(WORDS) == posix  # this machine is not Windows: unchanged


def test_windows_install_writes_lf_locks_read_only_and_verifies(tmp_path: Path, windows_guard) -> None:
    repo = make_repo(tmp_path / "repo")
    try:
        assert windows_guard.install(repo, WORDS) == []
        hooks = windows_guard.hooks_dir(repo)
        for name in windows_guard.HOOKS:
            raw = (hooks / name).read_bytes()
            assert b"\r" not in raw.replace(b"\\r", b""), name
            assert not (hooks / name).stat().st_mode & stat.S_IWRITE
        assert windows_guard.verify(repo, WORDS) == []
    finally:
        make_hooks_mutable(repo)
        for p in windows_guard.hooks_dir(repo).iterdir():
            p.chmod(0o644)


def test_windows_verify_reports_crlf_and_writable(tmp_path: Path, windows_guard) -> None:
    repo = make_repo(tmp_path / "repo")
    try:
        windows_guard.install(repo, WORDS)
        hook = windows_guard.hooks_dir(repo) / "pre-commit"
        hook.chmod(0o644)
        assert any("not read-only" in p for p in windows_guard.verify(repo, WORDS))
        hook.write_bytes(hook.read_bytes().replace(b"\n", b"\r\n"))
        assert any("CRLF" in p for p in windows_guard.verify(repo, WORDS))
        windows_guard._lock(hook)
        assert windows_guard._is_locked(hook)
        windows_guard._unlock(hook)
        assert not windows_guard._is_locked(hook)
    finally:
        for p in windows_guard.hooks_dir(repo).iterdir():
            p.chmod(0o644)


def test_windows_outdated_steps_use_attrib(tmp_path: Path, windows_guard) -> None:
    repo = make_repo(tmp_path / "repo")
    hooks = windows_guard.hooks_dir(repo)
    hooks.mkdir(parents=True, exist_ok=True)
    for name, text in windows_guard.render_v1(WORDS).items():
        (hooks / name).write_bytes(text.encode())
        windows_guard._lock(hooks / name)  # an intact version 1 guard is read-only
    try:
        steps = windows_guard.outdated(repo, WORDS)
        assert steps and any("attrib -r" in s for s in steps) and not any("chflags" in s for s in steps)
    finally:
        for p in hooks.iterdir():
            p.chmod(0o644)


# -- safe-git ---------------------------------------------------------------------------------


@pytest.mark.parametrize("args", [
    ["commit", "--no-verify", "-m", "x"],
    ["commit", "--no-ver", "-m", "x"],
    ["push", "--no-verify"],
    ["-c", "alias.c=commit --no-verify", "c"],
    ["commit", "-n", "-m", "x"],
    ["commit", "-an", "-m", "x"],
    ["commit", "-m", "x", "-n"],
    ["-c", "core.hooksPath=/tmp/h", "commit", "-m", "x"],
    ["-c", "CORE.HOOKSPATH=/tmp/h", "status"],
    ["--config-env=core.hooksPath=VAR", "status"],
    ["config", "core.hooksPath", "/tmp/h"],
    ["commit-tree", "HEAD^{tree}", "-m", "x"],
])
def test_safegit_refuses(args: list[str]) -> None:
    message = safegit.check(args, {})
    assert message and message.startswith("tirith: ") and "Do this instead:" in message


@pytest.mark.parametrize("args", [
    ["status"],
    ["commit", "-m", "x"],
    ["commit", "-m", "--no-verify is refused"],
    ["commit", "-m", "n"],
    ["commit", "-mn"],
    ["commit", "-am", "x"],
    ["commit", "-F", "-n"],
    ["push", "-n"],
    ["merge", "-n", "topic"],
    ["log", "--no-merges", "--oneline"],
    ["-c", "user.name=n", "commit", "-m", "x"],
    ["commit", "-m", "x", "--", "-n"],
])
def test_safegit_allows(args: list[str]) -> None:
    assert safegit.check(args, {}) is None


def test_safegit_refuses_hooks_path_in_environment() -> None:
    assert safegit.check(["status"], {"GIT_CONFIG_KEY_0": "core.hooksPath"})
    assert safegit.check(["status"], {"GIT_CONFIG_PARAMETERS": "'core.hookspath=/x'"})
    assert safegit.check(["status"], {"GIT_CONFIG_KEY_0": "user.name"}) is None


def test_safegit_main_refuses_with_exit_2_and_runs_real_git(tmp_path: Path, monkeypatch, capsys) -> None:
    assert safegit.main(["commit", "--no-verify"]) == 2
    assert "no-verify" in capsys.readouterr().err
    marker = tmp_path / "ran"
    fake = tmp_path / "fakegit"
    fake.write_text(f"#!/bin/sh\necho \"$@\" > {marker}\n", encoding="utf-8")
    fake.chmod(0o755)
    monkeypatch.setenv(safegit.REAL_ENV, str(fake))
    assert safegit.main(["status", "-s"]) == 0
    assert marker.read_text().strip() == "status -s"


def _fake_real_git(tmp_path: Path) -> Path:
    real = tmp_path / "realbin"
    real.mkdir()
    (real / "git").write_text("#!/bin/sh\n", encoding="utf-8")
    (real / "git").chmod(0o755)
    return real


def test_safegit_install_verify_uninstall(tmp_path: Path) -> None:
    real, shims = _fake_real_git(tmp_path), tmp_path / "shims"
    path = os.pathsep.join([str(shims), str(real)])
    assert safegit.install(shims, path)[0] == "installed"
    text = (shims / "git.cmd").read_bytes().decode()
    assert text.startswith("@echo off\r\nREM tirith-safegit v1\r\n") and text.count("\n") == text.count("\r\n")
    assert f'set "TIRITH_REAL_GIT={real / "git"}"' in text and "%~dp0tirith_safegit.py" in text
    assert (shims / "tirith_safegit.py").read_bytes() == Path(safegit.__file__).read_bytes()
    assert safegit.verify(shims, path) == []
    assert any("not first" in p for p in safegit.verify(shims, os.pathsep.join([str(real), str(shims)])))
    (shims / "tirith_safegit.py").write_text("# changed\n", encoding="utf-8")
    assert any("differs" in p for p in safegit.verify(shims, path))
    assert safegit.uninstall(shims)[0] == "removed"
    assert not (shims / "git.cmd").exists() and not (shims / "tirith_safegit.py").exists()


def test_safegit_never_overwrites_a_foreign_git_cmd_or_the_posix_wrapper(tmp_path: Path) -> None:
    real, shims = _fake_real_git(tmp_path), tmp_path / "shims"
    shims.mkdir()
    (shims / "git.cmd").write_text("@echo mine\r\n", encoding="utf-8")
    status, detail = safegit.install(shims, str(real))
    assert status == "skipped" and "not a Tirith" in detail
    assert (shims / "git.cmd").read_bytes() == b"@echo mine\r\n"
    assert safegit.uninstall(shims)[0] == "skipped" and (shims / "git.cmd").exists()
    assert safegit.install(tmp_path / "other", str(tmp_path / "empty"))[0] == "skipped"  # no git on PATH
    assert not (shims / "git").exists()  # only git.cmd and the core are ever written by this shim


def test_safegit_cli(home: Path, tmp_path: Path, monkeypatch, capsys) -> None:
    real = _fake_real_git(tmp_path)
    monkeypatch.setenv("PATH", str(real))
    shims = tmp_path / "shims"
    assert main(["safegit", "install", "--dir", str(shims)]) == 0
    assert "installed" in capsys.readouterr().out
    assert main(["safegit", "verify", "--dir", str(shims)]) == 0
    assert main(["safegit", "uninstall", "--dir", str(shims)]) == 0
    assert main(["safegit", "verify", "--dir", str(shims)]) == 1
