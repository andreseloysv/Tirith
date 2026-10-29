"""tirith lane review: tests, then merge, for lanes that `tirith launch` started."""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

import pytest

from conftest import make_repo, write_config
from tirith import launch as launch_mod
from tirith import review as review_mod
from tirith.cli import end_lane, main, register_lane
from tirith.store import read_json, lane_path

MANIFEST = ('deploy = "none"\n[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "build"\nrun = "true"\n'
            '[[pipelines.stages]]\nname = "test"\nrun = "{test}"\n')


def git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True, check=True).stdout


@pytest.fixture
def launched(home: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    for key in ("AUTHOR", "COMMITTER"):
        monkeypatch.setenv(f"GIT_{key}_NAME", "Tirith Test")
        monkeypatch.setenv(f"GIT_{key}_EMAIL", "t@example.invalid")
    repo = make_repo(tmp_path / "demo")
    (repo / "tirith.toml").write_text(MANIFEST.format(test="test -f feature.txt"), encoding="utf-8")
    git(repo, "add", ".")
    git(repo, "commit", "-q", "-m", "manifest")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}])
    worktree = launch_mod.create_worktree(repo, "web-panel")
    register_lane(home, lane_id="web-panel", project="demo", hero_class="glasswright", feature="Web panel",
                  workspace=str(worktree), log=None, pid=None, brief=None, step="2")
    (worktree / "feature.txt").write_text("built\n", encoding="utf-8")
    end_lane(home, "web-panel", "done", "built it")
    return repo, worktree


def test_public_review_is_pending_for_a_finished_launched_lane(launched, home: Path) -> None:
    repo, _ = launched
    lane = read_json(lane_path(home, "web-panel"), None)
    assert review_mod.public(lane, repo) == {"status": "pending"}
    assert review_mod.public({**lane, "workspace": str(repo)}, repo) is None  # not a launched worktree
    assert review_mod.public({**lane, "ended_at": None}, repo) is None


def test_review_runs_tests_commits_merges_and_cleans_up(launched, home: Path, capsys) -> None:
    repo, worktree = launched
    assert main(["lane", "review", "web-panel"]) == 0
    assert "web-panel: merged" in capsys.readouterr().out
    assert (repo / "feature.txt").read_text() == "built\n"
    assert "Merge lane/web-panel: Web panel" in git(repo, "log", "-1", "--format=%s")
    assert not worktree.exists() and "lane/web-panel" not in git(repo, "branch", "--list")
    lane = read_json(lane_path(home, "web-panel"), None)
    assert lane["review"]["status"] == "merged" and lane["review"]["commit"] == git(repo, "rev-parse", "HEAD").strip()
    assert main(["lane", "review", "web-panel"]) == 3  # already merged
    assert "already merged" in capsys.readouterr().err


def test_failing_tests_do_not_merge(launched, home: Path, capsys) -> None:
    repo, worktree = launched
    (worktree / "feature.txt").unlink()
    before = git(repo, "rev-parse", "HEAD")
    assert main(["lane", "review", "web-panel"]) == 1
    assert "tests-failed" in capsys.readouterr().out
    assert git(repo, "rev-parse", "HEAD") == before and worktree.is_dir()
    assert read_json(lane_path(home, "web-panel"), None)["review"]["status"] == "tests-failed"


def test_tests_only_leaves_the_lane_unmerged(launched, home: Path) -> None:
    repo, worktree = launched
    assert main(["lane", "review", "web-panel", "--tests-only"]) == 0
    assert read_json(lane_path(home, "web-panel"), None)["review"]["status"] == "ready"
    assert not (repo / "feature.txt").exists() and worktree.is_dir()


def test_dirty_repo_blocks_the_merge(launched, home: Path) -> None:
    repo, _ = launched
    (repo / "README.md").write_text("changed\n", encoding="utf-8")
    assert main(["lane", "review", "web-panel"]) == 1
    lane = read_json(lane_path(home, "web-panel"), None)
    assert lane["review"]["status"] == "merge-failed" and "uncommitted" in lane["review"]["message"]
    assert not (repo / "feature.txt").exists()


def test_refuses_running_unknown_and_unlaunched_lanes(launched, home: Path, capsys) -> None:
    repo, _ = launched
    register_lane(home, lane_id="manual", project="demo", hero_class="warden", feature="x",
                  workspace=str(repo), log=None, pid=os.getpid(), brief=None)
    assert main(["lane", "review", "manual"]) == 3
    assert "still running" in capsys.readouterr().err
    end_lane(home, "manual", "done")
    assert main(["lane", "review", "manual"]) == 3
    assert "not started by `tirith launch`" in capsys.readouterr().err
    assert main(["lane", "review", "ghost"]) == 3
    assert "unknown lane" in capsys.readouterr().err


def test_url_review_needs_the_token_then_spawns_the_review(launched, home: Path, monkeypatch, capsys) -> None:
    started: list[list[str]] = []
    monkeypatch.setattr(launch_mod, "spawn_detached", lambda argv, home, log: started.append(argv) or 1)
    token = launch_mod.launch_token(home)
    assert main(["url", "open", launch_mod.build_review_url("web-panel", "nope")]) == 3
    assert started == []
    assert main(["url", "open", launch_mod.build_review_url("web-panel", token)]) == 0
    assert started[0][-3:] == ["lane", "review", "web-panel"]
    assert main(["url", "open", launch_mod.build_review_url("ghost", token)]) == 3
    assert len(started) == 1


def test_state_shows_the_review(launched, home: Path) -> None:
    main(["collect", "--once"])
    state = json.loads((home / "state.json").read_text(encoding="utf-8"))
    agent = next(a for a in state["agents"] if a["id"] == "web-panel")
    assert agent["review"] == {"status": "pending"}
