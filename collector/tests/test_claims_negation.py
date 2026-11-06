"""claimcheck: negated or prohibiting phrases are no claim of a merge or a push."""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

from tirith import claimcheck as cc


def _repo(tmp_path: Path) -> Path:
    repo = tmp_path / "r"
    repo.mkdir()
    env = ["-c", "user.name=t", "-c", "user.email=t@example.invalid"]
    subprocess.run(["git", "-C", str(repo), "init", "-q"], check=True)
    (repo / "a.txt").write_text("a", encoding="utf-8")
    subprocess.run(["git", "-C", str(repo), "add", "."], check=True)
    subprocess.run(["git", "-C", str(repo), *env, "commit", "-q", "-m", "x"], check=True)
    return repo


@pytest.mark.parametrize("line", [
    "I did not merge.",
    "I did not merge or push.",
    "No push, no merge.",
    "I won't push, and I have not merged.",
    "Nothing was merged or pushed.",
    "Do not push it unless it is already merged.",
    "It was never pushed.",
])
def test_negated_is_no_claim(tmp_path, line):
    repo = _repo(tmp_path)
    other = tmp_path / "w"
    subprocess.run(["git", "-C", str(repo), "worktree", "add", "-q", "-b", "lane", str(other)], check=True)
    (other / "b.txt").write_text("b", encoding="utf-8")
    subprocess.run(["git", "-C", str(other), "add", "."], check=True)
    subprocess.run(["git", "-C", str(other), "-c", "user.name=t", "-c", "user.email=t@example.invalid",
                    "commit", "-q", "-m", "y"], check=True)
    result = cc.Result()
    cc.check_git(line, other, repo, result)
    assert result.mismatches == [] and result.claims == []


def test_real_claim_still_checked(tmp_path):
    repo = _repo(tmp_path)
    other = tmp_path / "w"
    subprocess.run(["git", "-C", str(repo), "worktree", "add", "-q", "-b", "lane", str(other)], check=True)
    (other / "b.txt").write_text("b", encoding="utf-8")
    subprocess.run(["git", "-C", str(other), "add", "."], check=True)
    subprocess.run(["git", "-C", str(other), "-c", "user.name=t", "-c", "user.email=t@example.invalid",
                    "commit", "-q", "-m", "y"], check=True)
    result = cc.Result()
    cc.check_git("Merged and pushed.", other, repo, result)
    assert len(result.mismatches) == 2
