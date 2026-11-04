"""The test suite stays cheap to run: count the processes and files a fixture makes instead of timing them."""

from __future__ import annotations

import subprocess
from pathlib import Path

from conftest import UNLOCK_ROOT_BUDGET_S, make_repo


def test_suite_budget_a_throwaway_repo_is_a_copy_and_no_sample_hooks(tmp_path: Path, monkeypatch) -> None:
    real = subprocess.Popen.__init__
    started: list[object] = []

    def spy(self, args, *a, **k):
        started.append(args)
        real(self, args, *a, **k)

    monkeypatch.setattr(subprocess.Popen, "__init__", spy)
    make_repo(tmp_path / "first")  # the process's template repo: init, add, commit once
    assert len(started) <= 3
    started.clear()
    repo = make_repo(tmp_path / "demo")
    assert started == []  # every later repo is a copy of the template: no git process
    (tmp_path / "filled").mkdir()
    (tmp_path / "filled" / "a.txt").write_text("a", encoding="utf-8")
    make_repo(tmp_path / "filled")
    assert len(started) == 3  # a folder with files gets its own init, add, commit
    git_dir = repo / ".git"
    assert not list((git_dir / "hooks").glob("*.sample"))  # the default template writes ~14 files nobody reads
    assert sum(1 for _ in git_dir.rglob("*")) < 25
    log = subprocess.run(["git", "-C", str(repo), "log", "--format=%s"], capture_output=True, text=True).stdout
    assert log.split() == ["initial"]


def test_suite_budget_the_shared_temp_root_walk_is_short() -> None:
    assert UNLOCK_ROOT_BUDGET_S <= 1.0
