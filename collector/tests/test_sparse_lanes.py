"""A project's ``lane_exclude`` keeps heavy paths out of lane worktrees without losing them on merge."""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

from conftest import make_repo
from tirith import launch as launch_mod
from tirith import mergeq as mergeq_mod


def git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(repo), "-c", "user.email=t@example.invalid", "-c", "user.name=T",
                           *args], capture_output=True, text=True, check=True).stdout


@pytest.fixture
def heavy(tmp_path: Path) -> Path:
    repo = tmp_path / "big"
    (repo / "Bundles" / "a" / "audio").mkdir(parents=True)
    (repo / "Bundles" / "a" / "audio" / "x.wav").write_text("sound")
    (repo / "Bundles" / "a" / "data.json").write_text("{}")
    (repo / "tirith.toml").write_text('lane_exclude = ["Bundles/*/audio"]\n')
    make_repo(repo)
    return repo


def test_lane_worktree_lacks_the_heavy_path(heavy: Path) -> None:
    lane = launch_mod.create_worktree(heavy, "one")
    assert not (lane / "Bundles" / "a" / "audio").exists()
    assert (lane / "Bundles" / "a" / "data.json").is_file()


def test_commit_in_sparse_lane_does_not_delete_and_merge_keeps_it(heavy: Path) -> None:
    lane = launch_mod.create_worktree(heavy, "one")
    (lane / "new.txt").write_text("hi")
    launch_mod.stage_all(lane)
    git(lane, "commit", "-qm", "work")
    assert git(lane, "diff", "--name-status", "HEAD~1", "HEAD").split() == ["A", "new.txt"]
    git(heavy, "merge", "--no-ff", "-q", "lane/one", "-m", "merge")
    assert (heavy / "Bundles" / "a" / "audio" / "x.wav").read_text() == "sound"
    assert "Bundles/a/audio/x.wav" in git(heavy, "ls-files")


def test_without_the_key_lanes_are_full(tmp_path: Path) -> None:
    repo = tmp_path / "plain"
    (repo / "audio").mkdir(parents=True)
    (repo / "audio" / "x.wav").write_text("s")
    make_repo(repo)
    assert (launch_mod.create_worktree(repo, "one") / "audio" / "x.wav").is_file()


def test_mergeq_worktree_is_full_unless_opted_in(heavy: Path) -> None:
    base = git(heavy, "rev-parse", "HEAD").strip()
    tmp, error = mergeq_mod._make_tmp(heavy, "_mergeq-a", base)
    assert error is None and (tmp / "Bundles" / "a" / "audio" / "x.wav").is_file()
    mergeq_mod._drop_tmp(heavy, tmp)
    (heavy / "tirith.toml").write_text('lane_exclude = ["Bundles/*/audio"]\nmergeq_exclude = true\n')
    git(heavy, "commit", "-qam", "opt in")
    base = git(heavy, "rev-parse", "HEAD").strip()
    tmp, error = mergeq_mod._make_tmp(heavy, "_mergeq-b", base)
    assert error is None and not (tmp / "Bundles" / "a" / "audio").exists()
    mergeq_mod._drop_tmp(heavy, tmp)


def test_failed_add_leaves_no_folder_or_worktree(heavy: Path) -> None:
    path = launch_mod.worktree_path(heavy, "bad")
    error = launch_mod.worktree_add(heavy, path, branch="lane/bad", base="no-such-ref",
                                    exclude=["Bundles/*/audio"])
    assert error
    assert not path.exists()
    assert "bad" not in git(heavy, "worktree", "list")
