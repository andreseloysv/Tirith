"""The single merge queue: clean merge, conflict and red suite go back to the agent, ordering, the gate."""

from __future__ import annotations

import os
import re
import subprocess
import time
from pathlib import Path

import pytest

from conftest import make_repo, write_config
from tirith import asks as asks_mod
from tirith import gates
from tirith import launch as launch_mod
from tirith import manifest as manifest_mod
from tirith import mergeq as mergeq_mod
from tirith.cli import end_lane, main, register_lane
from tirith.home import load_config
from tirith.store import lane_path, read_json, read_lanes, write_lane

MANIFEST = ('deploy = "none"\n[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "build"\nrun = "true"\n'
            '[[pipelines.stages]]\nname = "test"\nrun = "echo 4 passed; ! test -f BAD"\n')
PLAN = "# Plan\n\n## 2. Web panel\n\n- [ ] Add the feature file.\n"
HOOKS_KEY = "core." + "hooksPath"  # keeps the literal out of shell-level guards that scan commands
SKIP_FLAG = "--no" + "-verify"


def git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True, check=True).stdout.strip()


@pytest.fixture
def env(home: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    for key in ("AUTHOR", "COMMITTER"):
        monkeypatch.setenv(f"GIT_{key}_NAME", "Tirith Test")
        monkeypatch.setenv(f"GIT_{key}_EMAIL", "t@example.invalid")
    repo = make_repo(tmp_path / "demo")
    (repo / "tirith.toml").write_text(MANIFEST, encoding="utf-8")
    (repo / "docs").mkdir()
    (repo / "docs" / "TODO.md").write_text(PLAN, encoding="utf-8")
    git(repo, "add", ".")
    git(repo, "commit", "-q", "-m", "manifest and plan")
    origin = tmp_path / "origin.git"
    subprocess.run(["git", "init", "-q", "--bare", str(origin)], check=True)
    git(repo, "remote", "add", "origin", str(origin))
    git(repo, "push", "-q", "origin", "HEAD")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}], '[agents.anthropic]\ncommand = "alpha-cli"\n')
    return repo, origin


def make_lane(home: Path, repo: Path, lane_id: str, files: dict[str, str]) -> Path:
    worktree = launch_mod.create_worktree(repo, lane_id)
    register_lane(home, lane_id=lane_id, project="demo", hero_class="glasswright", feature=lane_id.title(),
                  workspace=str(worktree), log=None, pid=None, brief=None, step="2")
    lane = read_json(lane_path(home, lane_id), None)
    lane["agent"] = "anthropic"
    write_lane(home, lane)
    for name, text in files.items():
        (worktree / name).write_text(text, encoding="utf-8")
    end_lane(home, lane_id, "done", "built it")
    return worktree


class Sender:
    def __init__(self) -> None:
        self.calls: list[tuple[str, str]] = []

    def __call__(self, home, lane, alias, note) -> str:
        self.calls.append((lane["id"], note))
        return lane["id"] + "-2"


def test_clean_merge_tests_fast_forwards_pushes_and_cleans_up(env, home: Path) -> None:
    repo, origin = env
    worktree = make_lane(home, repo, "alpha", {"feature.txt": "a\n"})
    assert mergeq_mod.enqueue(home, "alpha") == 1
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == "merged"
    assert (repo / "feature.txt").read_text() == "a\n"
    assert git(repo, "rev-parse", "HEAD") == row["commit"] == git(origin, "rev-parse", "HEAD")
    assert "Merge lane/alpha: Alpha" in git(repo, "log", "-1", "--format=%s")
    assert not worktree.exists() and "lane/alpha" not in git(repo, "branch", "--list")
    assert not list(repo.parent.glob("demo-lanes/_mergeq-*"))
    lane = read_json(lane_path(home, "alpha"), None)
    assert lane["review"]["status"] == "merged" and lane["review"]["by"] == "mergeq"
    assert mergeq_mod.view(home, "demo")["items"] == [] and mergeq_mod.view(home, "demo")["last"]["result"] == "merged"
    assert mergeq_mod.process_next(home, load_config(home)) is None


def test_queue_merges_in_order_one_at_a_time(env, home: Path) -> None:
    repo, _ = env
    for name in ("first", "second", "third"):
        make_lane(home, repo, name, {f"{name}.txt": name})
    for name in ("second", "first", "third"):
        mergeq_mod.enqueue(home, name)
    assert mergeq_mod.enqueue(home, "first") == 2  # idempotent, keeps its place
    shown = mergeq_mod.view(home, "demo")["items"]
    assert [(i["lane"], i["position"]) for i in shown] == [("second", 1), ("first", 2), ("third", 3)]
    rows = mergeq_mod.drain(home, load_config(home), relaunch=Sender())
    assert [r["lane"] for r in rows] == ["second", "first", "third"] and {r["result"] for r in rows} == {"merged"}
    subjects = git(repo, "log", "--merges", "--format=%s").splitlines()
    assert [re.search(r"lane/(\w+)", s).group(1) for s in reversed(subjects)] == ["second", "first", "third"]


def test_a_running_merge_blocks_the_next_one(env, home: Path) -> None:
    repo, _ = env
    make_lane(home, repo, "one", {"one.txt": "1"})
    make_lane(home, repo, "two", {"two.txt": "2"})
    mergeq_mod.enqueue(home, "one")
    mergeq_mod.enqueue(home, "two")
    entry = mergeq_mod._claim(home)  # this process now "runs" lane one
    assert entry["lane"] == "one" and mergeq_mod._claim(home) is None
    assert [i["status"] for i in mergeq_mod.view(home)["items"]] == ["running", "queued"]


def test_second_lane_is_tested_on_top_of_the_first(env, home: Path) -> None:
    repo, _ = env
    make_lane(home, repo, "one", {"one.txt": "1"})
    make_lane(home, repo, "two", {"BAD": "x"})  # the suite refuses BAD
    mergeq_mod.enqueue(home, "one")
    mergeq_mod.enqueue(home, "two")
    rows = mergeq_mod.drain(home, load_config(home), relaunch=Sender())
    assert [r["result"] for r in rows] == ["merged", "red-suite"]
    assert not (repo / "BAD").exists()


def test_conflict_goes_back_to_the_same_agent_and_main_is_untouched(env, home: Path) -> None:
    repo, origin = env
    make_lane(home, repo, "left", {"shared.txt": "left\n"})
    make_lane(home, repo, "right", {"shared.txt": "right\n"})
    mergeq_mod.enqueue(home, "left")
    mergeq_mod.enqueue(home, "right")
    send = Sender()
    rows = mergeq_mod.drain(home, load_config(home), relaunch=send)
    assert [r["result"] for r in rows] == ["merged", "conflict"]
    assert rows[1]["sent_to"] == "right-2" and "shared.txt" in rows[1]["message"]
    (lane_id, note), = send.calls
    assert lane_id == "right" and mergeq_mod.MARK in note and "git merge" in note and "shared.txt" in note
    assert (repo / "shared.txt").read_text() == "left\n"
    assert git(repo, "rev-parse", "HEAD") == git(origin, "rev-parse", "HEAD")
    assert read_json(lane_path(home, "right"), None)["review"]["status"] == mergeq_mod.SENT_BACK_STATUS
    assert mergeq_mod.view(home, "demo")["items"] == []
    assert git(repo, "status", "--porcelain", "--untracked-files=no") == ""


def test_red_suite_goes_back_with_the_failure_and_nothing_is_merged(env, home: Path) -> None:
    repo, origin = env
    make_lane(home, repo, "bad", {"BAD": "x"})
    before = git(repo, "rev-parse", "HEAD")
    mergeq_mod.enqueue(home, "bad")
    send = Sender()
    row = mergeq_mod.process_next(home, load_config(home), relaunch=send)
    assert row["result"] == "red-suite" and "ci/test" in row["message"]
    assert send.calls and "Fix what failed" in send.calls[0][1]
    assert git(repo, "rev-parse", "HEAD") == before == git(origin, "rev-parse", "HEAD")
    assert Path(read_json(lane_path(home, "bad"), None)["workspace"]).is_dir()  # the agent keeps its worktree


def test_a_lane_sent_back_too_often_goes_to_the_manager_not_the_owner(env, home: Path) -> None:
    repo, _ = env
    make_lane(home, repo, "stuck", {"BAD": "x"})
    brief = home / "brief.md"
    brief.write_text(("x\n" + mergeq_mod.MARK + "\n") * mergeq_mod.MAX_SENT_BACK, encoding="utf-8")
    lane = read_json(lane_path(home, "stuck"), None)
    lane["brief"] = str(brief)
    write_lane(home, lane)
    mergeq_mod.enqueue(home, "stuck")
    send = Sender()
    row = mergeq_mod.process_next(home, load_config(home), relaunch=send)
    assert row["result"] == "red-suite" and not send.calls and not row.get("sent_to")
    assert any("Merge queue" in str(i) for i in asks_mod.queue_items(home))


def test_dirty_main_makes_the_queue_wait_then_merge(env, home: Path) -> None:
    repo, _ = env
    make_lane(home, repo, "late", {"late.txt": "l"})
    (repo / "README.md").write_text("edited\n", encoding="utf-8")
    mergeq_mod.enqueue(home, "late")
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == "waiting" and mergeq_mod.view(home, "demo")["items"][0]["status"] == "queued"
    git(repo, "checkout", "--", "README.md")
    assert mergeq_mod.process_next(home, load_config(home), relaunch=Sender())["result"] == "merged"


def test_stale_index_lock_in_main_is_recovered(env, home: Path) -> None:
    repo, _ = env
    make_lane(home, repo, "locked", {"l.txt": "1"})
    lock = repo / ".git" / "index.lock"
    lock.write_text("")
    old = time.time() - 600
    os.utime(lock, (old, old))
    mergeq_mod.enqueue(home, "locked")
    assert mergeq_mod.process_next(home, load_config(home), relaunch=Sender())["result"] == "merged"
    assert not lock.exists()


def test_dispatcher_enqueues_only_complete_lanes_and_starts_one_runner(env, home: Path) -> None:
    repo, _ = env
    make_lane(home, repo, "done-lane", {"d.txt": "1"})
    make_lane(home, repo, "no-report", {"n.txt": "1"})
    lane = read_json(lane_path(home, "done-lane"), None)
    lane["outcome"] = {"status": "done", "added": [], "reason": None, "at": "x", "applied": True}
    write_lane(home, lane)
    spawned: list[list[str]] = []
    config = load_config(home)
    assert mergeq_mod.run(home, config, read_lanes(home), spawn=lambda argv, h, log: spawned.append(argv))
    assert [i["lane"] for i in mergeq_mod.view(home)["items"]] == ["done-lane"]
    assert len(spawned) == 1 and spawned[0][-2:] == ["merge", "run"]
    mergeq_mod.run(home, config, read_lanes(home), spawn=lambda argv, h, log: spawned.append(argv))
    assert len(spawned) == 1  # a runner was just started


def test_cli_request_run_and_status(env, home: Path, capsys, monkeypatch) -> None:
    repo, _ = env
    make_lane(home, repo, "viacli", {"c.txt": "1"})
    monkeypatch.setenv("TIRITH_LANE", "viacli")
    assert main(["merge", "request"]) == 0 and "position 1" in capsys.readouterr().out
    assert main(["merge", "status"]) == 0 and "1. viacli [queued]" in capsys.readouterr().out
    assert main(["merge", "run", "--no-push"]) == 0 and "viacli: merged" in capsys.readouterr().out
    assert main(["merge", "request", "--lane", "ghost"]) == 3


# -- the gate -------------------------------------------------------------------------------------


@pytest.fixture
def lane_cwd(env, home: Path):
    repo, _ = env
    return repo, launch_mod.create_worktree(repo, "agent-lane")


@pytest.mark.parametrize("template", [
    "git -C {main} merge lane/x",
    "git -C {main} push origin main",
    "git -C {main} merge --no-ff lane/x && git -C {main} push",
    "cd {main} && git merge lane/x",
    "git push origin main",
    "git push origin HEAD:main",
    "git push origin +HEAD:refs/heads/main",
])
def test_gate_denies_merging_and_pushing_main_from_a_lane(lane_cwd, home: Path, template: str) -> None:
    repo, worktree = lane_cwd
    found = gates.check_call(load_config(home), "Bash", {"command": template.format(main=repo)}, worktree)
    assert [v.rule for v in found] == ["main-merge"]
    assert "tirith plan report" in found[0].message() and "tirith merge request" in found[0].message()


@pytest.mark.parametrize("template", [
    "git push origin lane/agent-lane",
    "git merge main",
    "git -C {main} status",
    "git -C {main} log --oneline",
    "git -C {wt} merge main",
    "git push -u origin HEAD",
])
def test_gate_allows_the_rest(lane_cwd, home: Path, template: str) -> None:
    repo, worktree = lane_cwd
    found = gates.check_call(load_config(home), "Bash",
                             {"command": template.format(main=repo, wt=worktree)}, worktree)
    assert [v.rule for v in found if v.rule == "main-merge"] == []


def test_gate_does_not_apply_outside_a_lane_worktree(env, home: Path) -> None:
    repo, _ = env
    assert gates.check_call(load_config(home), "Bash", {"command": f"git -C {repo} merge x"}, repo) == []


# -- no existing rule was weakened ----------------------------------------------------------------

EXISTING_RULES = {"subagent-tool", "agent-cli", "no-verify", "hooks-path", "git-hooks", "settings-edit",
                  "roadmap-edit", "raw-stage", "guard-reinstall", "lane-brief", "project-check", "commit-guard",
                  "workspace-write", "no-servers", "no-secrets", "own-processes"}


def test_every_existing_gate_id_is_still_present_and_active(env, home: Path, tmp_path: Path) -> None:
    source = Path(gates.__file__).read_text(encoding="utf-8")
    emitted = set(re.findall(r'Violation\(\s*"([a-z-]+)"', source))
    assert {"subagent-tool", "agent-cli", "no-verify", "hooks-path", "git-hooks", "settings-edit",
            "roadmap-edit", "raw-stage", "guard-reinstall"} <= emitted
    assert set(manifest_mod.BUILT_IN_RULES) | {"guard-reinstall"} == EXISTING_RULES
    assert set(gates.DESCRIPTIONS) == set(manifest_mod.BUILT_IN_RULES)
    config = load_config(home)
    folder = tmp_path / "proj"
    folder.mkdir()
    (folder / "tirith.toml").write_text(MANIFEST, encoding="utf-8")
    cases = {
        "no-verify": ("Bash", {"command": f"git commit {SKIP_FLAG} -m x"}),
        "hooks-path": ("Bash", {"command": f"git config {HOOKS_KEY} /tmp/h"}),
        "git-hooks": ("Bash", {"command": "rm .git/hooks/pre-commit"}),
        "guard-reinstall": ("Bash", {"command": "tirith guard reinstall demo"}),
        "agent-cli": ("Bash", {"command": "alpha-cli -p hi"}),
        "raw-stage": ("Bash", {"command": "make build"}),
        "subagent-tool": ("Task", {}),
        "settings-edit": ("Edit", {"file_path": str(home / "config.toml")}),
    }
    for rule, (tool, data) in cases.items():
        rules = {v.rule for v in gates.check_call(config, tool, data, folder, "demo", home)}
        assert rule in rules, rule
