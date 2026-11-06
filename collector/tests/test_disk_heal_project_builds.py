"""Low disk: build/ and .gradle of project checkouts and worktrees go; parked merged worktrees go; unmerged stay."""

from __future__ import annotations

import subprocess
from datetime import datetime, timezone
from pathlib import Path

import pytest

from conftest import make_repo, write_config
from tirith import diskdiet as diskdiet_mod
from tirith.home import load_config

GB = 1024 ** 3


def git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(repo), "-c", "user.email=t@example.invalid", "-c", "user.name=T",
                           *args], capture_output=True, text=True, check=True).stdout


@pytest.fixture(autouse=True)
def no_real_scans(monkeypatch) -> None:
    monkeypatch.setattr(diskdiet_mod, "_biggest", lambda count=3: [])
    monkeypatch.setattr(diskdiet_mod, "busy_folders", lambda: [])
    monkeypatch.setattr(diskdiet_mod, "process_running", lambda *names: False)


@pytest.fixture
def repo(home: Path, tmp_path: Path) -> Path:
    repo = make_repo(tmp_path / "demo")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}])
    return repo


def build_output(folder: Path) -> None:
    (folder / "build.gradle").write_text("")
    for name in ("build", ".gradle"):
        (folder / name).mkdir()
        (folder / name / "x.bin").write_bytes(b"0" * 1024)


def worktree(repo: Path, folder: Path, branch: str, commit: bool = False) -> Path:
    git(repo, "worktree", "add", "-b", branch, str(folder))
    if commit:
        (folder / "work.txt").write_text("w")
        git(folder, "add", "work.txt")
        git(folder, "commit", "-m", "work")
    return folder


def test_gradle_build_folders_of_checkout_and_worktrees_are_removed(home: Path, repo: Path) -> None:
    wt = worktree(repo, repo.parent / "demo-lanes" / "a", "lane/a")
    for folder in (repo, wt):
        build_output(folder)
    freed = diskdiet_mod.clear_build_cache(home, load_config(home))
    assert freed >= 4096
    for folder in (repo, wt):
        assert not (folder / "build").exists() and not (folder / ".gradle").exists()


def test_tracked_build_folder_and_non_gradle_folder_stay(home: Path, repo: Path) -> None:
    (repo / "build").mkdir()
    (repo / "build" / "src.txt").write_text("source")
    (repo / "build.gradle").write_text("")
    git(repo, "add", "-A")
    git(repo, "commit", "-m", "tracked build")
    other = worktree(repo, repo.parent / "demo-lanes" / "b", "lane/b")
    (other / "build.gradle").unlink()
    (other / "build").mkdir(exist_ok=True)
    (other / "build" / "keep").write_text("k")
    diskdiet_mod.clear_build_cache(home, load_config(home))
    assert (repo / "build" / "src.txt").is_file()
    assert (other / "build" / "keep").is_file()

def test_a_folder_a_process_runs_in_keeps_its_build(home: Path, repo: Path) -> None:
    build_output(repo)
    diskdiet_mod.clear_build_cache(home, load_config(home), busy=lambda: [repo / "build"])
    assert (repo / "build").is_dir()


def test_merged_parked_worktrees_go_and_unmerged_or_dirty_stay(home: Path, repo: Path) -> None:
    merged = worktree(repo, repo / ".wt" / "merged", "lane/merged")
    salvage = worktree(repo, repo / ".wt-salvage" / "unmerged", "lane/unmerged", commit=True)
    dirty = worktree(repo, repo / ".wt" / "dirty", "lane/dirty")
    (dirty / "scratch.txt").write_text("not saved")
    result = diskdiet_mod.sweep(home, load_config(home), datetime.now(timezone.utc))
    assert result["worktrees"] == 1
    assert not merged.exists()
    assert salvage.is_dir() and dirty.is_dir()
    assert "lane/unmerged" in git(repo, "branch", "--list", "lane/unmerged")
