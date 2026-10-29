"""Lanes stop filling the disk: one shared cargo target per project, merged worktrees removed, a free-space guard."""

from __future__ import annotations

import subprocess
from datetime import datetime, timezone
from pathlib import Path

import pytest

from conftest import make_repo, write_config
from tirith import asks as asks_mod
from tirith import diskdiet as diskdiet_mod
from tirith import dispatch as dispatch_mod
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith import runstage as runstage_mod
from tirith.cli import end_lane, register_lane
from tirith.home import load_config
from tirith.store import read_json, read_lanes

GB = 1024 ** 3


def git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(repo), "-c", "user.email=t@example.invalid", "-c", "user.name=T",
                           *args], capture_output=True, text=True, check=True).stdout


def now() -> datetime:
    return datetime.now(timezone.utc)


@pytest.fixture
def repo(home: Path, tmp_path: Path) -> Path:
    repo = make_repo(tmp_path / "demo")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}])
    return repo


def stage_output(home: Path, cwd: Path, env: dict | None = None) -> str:
    code, output = runstage_mod.run_stage(home, "demo", "ci", "rust-test", 'printf "%s" "$CARGO_TARGET_DIR"', cwd,
                                          echo=False, env=env)
    assert code == 0
    return output


# -- 1. one cargo target per project ----------------------------------------------------------


def test_cargo_stages_share_one_target_dir_across_worktrees(home: Path, repo: Path, monkeypatch) -> None:
    monkeypatch.delenv("CARGO_TARGET_DIR", raising=False)
    expected = str(repo.resolve().parent / "demo-lanes" / ".cargo-target")
    first = launch_mod.create_worktree(repo, "one")
    second = launch_mod.create_worktree(repo, "two")
    assert stage_output(home, repo) == stage_output(home, first) == stage_output(home, second) == expected


def test_cargo_target_dir_comes_from_the_config_and_an_existing_value_wins(home: Path, repo: Path, tmp_path: Path,
                                                                           monkeypatch) -> None:
    monkeypatch.delenv("CARGO_TARGET_DIR", raising=False)
    shared = tmp_path / "elsewhere" / "target"
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo), "cargo_target_dir": str(shared)}])
    assert load_config(home).project("demo").cargo_target_dir == shared
    assert stage_output(home, repo) == str(shared)
    assert stage_output(home, repo, env={"PATH": "/usr/bin:/bin", "CARGO_TARGET_DIR": "/mine"}) == "/mine"


def test_without_a_project_repo_the_worktrees_git_repo_decides(home: Path, tmp_path: Path, monkeypatch) -> None:
    monkeypatch.delenv("CARGO_TARGET_DIR", raising=False)
    other = make_repo(tmp_path / "other")
    assert stage_output(home, other) == str(other.resolve().parent / "other-lanes" / ".cargo-target")
    assert stage_output(home, tmp_path) == ""  # not a repo: nothing to share


# -- 2. merged worktrees are removed ----------------------------------------------------------


def lane(home: Path, repo: Path, lane_id: str, work: bool = True, end: bool = True) -> Path:
    worktree = launch_mod.create_worktree(repo, lane_id)
    register_lane(home, lane_id=lane_id, project="demo", hero_class="glasswright", feature=lane_id,
                  workspace=str(worktree), log=None, pid=None, brief=None, step=None, request_id=None)
    if work:
        (worktree / f"{lane_id}.txt").write_text("built\n", encoding="utf-8")
        git(worktree, "add", "-A")
        git(worktree, "commit", "-q", "-m", f"work of {lane_id}")
    if end:
        end_lane(home, lane_id, "done", "## What changed\n- x\n")
    return worktree


def record(home: Path, lane_id: str) -> dict:
    return next(r for r in read_lanes(home) if r["id"] == lane_id)


def test_a_merged_clean_lane_loses_worktree_and_branch_but_keeps_its_record(home: Path, repo: Path) -> None:
    worktree = lane(home, repo, "done-lane")
    (home / "logs").mkdir()
    (home / "logs" / "done-lane.log").write_text("log\n", encoding="utf-8")
    assert not diskdiet_mod.cleanup(home, load_config(home), now())  # unmerged: untouched
    assert worktree.is_dir()
    git(repo, "merge", "--no-ff", "-q", "-m", "merge", "lane/done-lane")
    assert diskdiet_mod.cleanup(home, load_config(home), now())
    assert not worktree.exists()
    assert "lane/done-lane" not in git(repo, "branch", "--list")
    assert record(home, "done-lane")["cleanup"]["status"] == "removed"
    assert (home / "logs" / "done-lane.log").is_file()
    assert not diskdiet_mod.cleanup(home, load_config(home), now())  # once


def test_a_dirty_unmerged_running_or_foreign_worktree_is_left_alone(home: Path, repo: Path) -> None:
    dirty = lane(home, repo, "dirty")
    (dirty / "scratch.txt").write_text("not committed\n", encoding="utf-8")
    running = lane(home, repo, "running", end=False)
    unmerged = lane(home, repo, "unmerged")
    for name in ("dirty", "running"):
        git(repo, "merge", "--no-ff", "-q", "-m", name, f"lane/{name}")
    manual = repo.parent / "manual-tree"
    git(repo, "worktree", "add", "-q", "-b", "lane/manual", str(manual))
    register_lane(home, lane_id="manual", project="demo", hero_class="glasswright", feature="manual",
                  workspace=str(manual), log=None, pid=None, brief=None, step=None, request_id=None)
    end_lane(home, "manual", "done", "")
    diskdiet_mod.cleanup(home, load_config(home), now())
    assert dirty.is_dir() and running.is_dir() and unmerged.is_dir() and manual.is_dir()  # never --force
    assert record(home, "dirty")["cleanup"]["status"] == "dirty"
    assert "cleanup" not in record(home, "unmerged") and "cleanup" not in record(home, "running")
    assert "lane/dirty" in git(repo, "branch", "--list")


def test_the_dispatcher_cycle_cleans_up(home: Path, repo: Path) -> None:
    worktree = lane(home, repo, "cycle")
    git(repo, "merge", "--no-ff", "-q", "-m", "merge", "lane/cycle")
    dispatch_mod.run(home, load_config(home), now(), agents=[], lanes=read_lanes(home), status_of=lambda *a: "done")
    assert not worktree.exists()


# -- 3. the disk guard ------------------------------------------------------------------------


def cycle(home: Path, free_gb: float, launched: list) -> None:
    dispatch_mod.run(home, load_config(home), now(), agents=[], lanes=[], status_of=lambda *a: "done",
                     launcher=lambda h, p, s, r: launched.append((p, s)) or "new-lane",
                     free=lambda folder: int(free_gb * GB))


def test_low_disk_starts_no_lane_and_tells_the_manager_once(home: Path, repo: Path) -> None:
    request = inbox_mod.add_request(home, "finish", "demo", "2")
    launched: list = []
    for _ in range(3):
        cycle(home, 1.0, launched)
    assert launched == []
    assert inbox_mod.find_request(home, request["id"])["answers"] == []  # waits, is not refused
    state = read_json(home / "dispatch.json", {})
    assert state["disk"]["low"] and state["disk"]["free_bytes"] == GB
    (item,) = asks_mod.queue_items(home, only_open=True)
    assert "almost full" in item["text"] and item["to"] == "manager" and item["id"] == state["disk"]["queue"]
    assert not (home / "waits.jsonl").exists()  # not an owner item
    cycle(home, 50.0, launched)  # room again: the item closes and the request is handled
    assert asks_mod.queue_items(home, only_open=True) == []
    assert launched == [("demo", "2")]
    assert not read_json(home / "dispatch.json", {})["disk"]["low"]


def test_the_limit_is_configurable_and_the_autopilot_gets_no_room(home: Path, repo: Path, monkeypatch) -> None:
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}], extra="[disk]\nmin_free_gb = 0.5\n")
    assert load_config(home).disk_min_free_gb == 0.5
    seen: list = []
    from tirith import autopilot as autopilot_mod
    monkeypatch.setattr(autopilot_mod, "run", lambda *a, **kw: seen.append(kw.get("launch")) or False)
    cycle(home, 1.0, [])
    cycle(home, 0.2, [])
    assert seen == [True, False]
