"""Shared fixtures: an isolated Tirith home and throwaway git helpers."""

from __future__ import annotations

import os
import shutil
import stat
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

GIT = "/usr/bin/git" if Path("/usr/bin/git").exists() else (shutil.which("git") or "git")


@pytest.fixture
def home(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """A fresh TIRITH_HOME with the brain key removed from the env."""
    folder = tmp_path / "tirith-home"
    folder.mkdir()
    monkeypatch.setenv("TIRITH_HOME", str(folder))
    monkeypatch.delenv("META_API_KEY", raising=False)
    monkeypatch.delenv("META_SI_API_KEY", raising=False)
    return folder


def make_repo(path: Path) -> Path:
    """Create a throwaway git repo with one commit (tmp dirs only)."""
    path.mkdir(parents=True, exist_ok=True)
    subprocess.run([GIT, "init", "-q", str(path)], check=True)
    (path / "README.md").write_text("# demo repo\n", encoding="utf-8")
    subprocess.run([GIT, "-C", str(path), "add", "."], check=True)
    subprocess.run(
        [GIT, "-C", str(path), "-c", "user.email=t@example.invalid",
         "-c", "user.name=Tirith Test", "commit", "-q", "-m", "initial"],
        check=True)
    return path


def make_hooks_mutable(repo: Path) -> None:
    """Clear immutable flags the guard sets, so tmp cleanup can proceed."""
    try:
        out = subprocess.run(
            [GIT, "-C", str(repo), "rev-parse", "--path-format=absolute", "--git-common-dir"],
            capture_output=True, text=True, check=True).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return
    hooks = Path(out) / "hooks"
    if not hooks.is_dir():
        return
    for hook in hooks.iterdir():
        try:
            flags = hook.stat().st_flags  # type: ignore[attr-defined]
            if flags & stat.UF_IMMUTABLE:
                os.chflags(hook, flags & ~stat.UF_IMMUTABLE)  # type: ignore[attr-defined]
        except (OSError, AttributeError):
            pass


def write_config(home: Path, projects: list[dict], extra: str = "") -> Path:
    """Write a minimal config.toml with ``[[projects]]`` rows."""
    lines: list[str] = []
    for proj in projects:
        lines.append("[[projects]]")
        for key, value in proj.items():
            if isinstance(value, list):
                rendered = "[" + ", ".join(f'"{item}"' for item in value) + "]"
                lines.append(f"{key} = {rendered}")
            else:
                lines.append(f'{key} = "{value}"')
        lines.append("")
    if extra:
        lines.append(extra)
    target = home / "config.toml"
    target.write_text("\n".join(lines), encoding="utf-8")
    return target


@pytest.fixture(autouse=True)
def _unlock_guard_hooks(tmp_path):
    """Guard hooks installed by a test are immutable; clear the flag so pytest can delete tmp_path."""
    yield
    if not hasattr(os, "chflags"):
        return
    for path in tmp_path.rglob("*"):
        try:
            if not path.is_symlink() and path.stat().st_flags & stat.UF_IMMUTABLE:
                os.chflags(path, path.stat().st_flags & ~stat.UF_IMMUTABLE)
        except OSError:
            pass


@pytest.fixture(autouse=True)
def _no_outer_lane(monkeypatch):
    """Tests run the same inside a lane (TIRITH_LANE / TIRITH_SESSION set by lane run) as outside."""
    monkeypatch.delenv("TIRITH_LANE", raising=False)
    monkeypatch.delenv("TIRITH_SESSION", raising=False)
