"""claimcheck: main merged into the lane, or the word 'merged' in quotes, is no claim that the lane merged."""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

from tirith import claimcheck as cc


def _setup(tmp_path: Path):
    repo = tmp_path / "r"
    repo.mkdir()
    env = ["-c", "user.name=t", "-c", "user.email=t@example.invalid"]
    subprocess.run(["git", "-C", str(repo), "init", "-q"], check=True)
    (repo / "a.txt").write_text("a", encoding="utf-8")
    subprocess.run(["git", "-C", str(repo), "add", "."], check=True)
    subprocess.run(["git", "-C", str(repo), *env, "commit", "-q", "-m", "x"], check=True)
    other = tmp_path / "w"
    subprocess.run(["git", "-C", str(repo), "worktree", "add", "-q", "-b", "lane", str(other)], check=True)
    (other / "b.txt").write_text("b", encoding="utf-8")
    subprocess.run(["git", "-C", str(other), "add", "."], check=True)
    subprocess.run(["git", "-C", str(other), *env, "commit", "-q", "-m", "y"], check=True)
    return repo, other


@pytest.mark.parametrize("line", [
    "main merged into the lane.",
    "Main was merged into the lane before I started.",
    "The fix arrived through lane 21's merge.",
    "The code arrived through lane 21's merge, so it was already there.",
    "The word 'merged' in the report is a quote from the brief.",
    'The log said "merged" for the other lane.',
    "The status `merged` is set by the queue.",
])
def test_not_a_merge_claim(tmp_path, line):
    repo, other = _setup(tmp_path)
    result = cc.Result()
    cc.check_git(line, other, repo, result)
    assert result.mismatches == [] and result.claims == []


def test_real_claim_still_checked(tmp_path):
    repo, other = _setup(tmp_path)
    result = cc.Result()
    cc.check_git("Merged.", other, repo, result)
    assert len(result.mismatches) == 1
