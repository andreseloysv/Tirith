"""The single merge queue: clean merge, conflict and red suite go back to the agent, ordering, the gate."""

from __future__ import annotations

import json
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
from tirith import plangate
from tirith.cli import end_lane, main, register_lane
from tirith.home import load_config
from tirith.store import lane_path, read_json, read_lanes, write_json_atomic, write_lane

MANIFEST = ('deploy = "none"\n[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "build"\nrun = "true"\n'
            '[[pipelines.stages]]\nname = "test"\nrun = "echo 4 passed; ! test -f BAD"\n')
PLAN = "# Plan\n\n## 2. Web panel\n\n- [ ] Add the feature file.\n"
HOOKS_KEY = "core." + "hooksPath"  # keeps the literal out of shell-level guards that scan commands
SKIP_FLAG = "--no" + "-verify"


def git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True, check=True).stdout.strip()


@pytest.fixture
def env(home: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    # drain() stops after a pass once the package on disk changed; another lane or a live edit touching the code
    # while a test runs must not cut a drain short, so the fingerprint is frozen here.
    monkeypatch.setattr(mergeq_mod.selfreload_mod, "code_fingerprint", lambda *a, **k: ())
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
    subprocess.run(["git", "init", "-q", "--template=", "--bare", str(origin)], check=True)
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
    make_lane(home, repo, "late", {"README.md": "from lane\n"})
    (repo / "README.md").write_text("edited\n", encoding="utf-8")
    mergeq_mod.enqueue(home, "late")
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == "waiting" and "README.md" in row["message"]
    assert mergeq_mod.view(home, "demo")["items"][0]["status"] == "queued"
    git(repo, "checkout", "--", "README.md")
    assert mergeq_mod.process_next(home, load_config(home), relaunch=Sender())["result"] == "merged"


def test_dirty_checkout_with_other_files_still_lands(env, home: Path) -> None:
    repo, _ = env
    make_lane(home, repo, "late", {"late.txt": "l"})
    (repo / "tirith.toml").write_text(MANIFEST + "# local edit\n", encoding="utf-8")  # tracked, not touched by the lane
    mergeq_mod.enqueue(home, "late")
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == "merged" and (repo / "late.txt").read_text() == "l"
    assert "# local edit" in (repo / "tirith.toml").read_text()  # the owner's edit is untouched


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


def test_the_collector_path_without_a_spawner_starts_the_runner(env, home: Path, monkeypatch) -> None:
    repo, _ = env
    make_lane(home, repo, "done-lane", {"d.txt": "1"})
    lane = read_json(lane_path(home, "done-lane"), None)
    lane["outcome"] = {"status": "done", "added": [], "reason": None, "at": "x", "applied": True}
    write_lane(home, lane)
    started: list[list[str]] = []
    monkeypatch.setattr(launch_mod, "spawn_detached", lambda argv, h, log: started.append(argv) or 1)
    assert mergeq_mod.run(home, load_config(home), read_lanes(home))  # the collector passes no spawner
    assert len(started) == 1 and started[0][-2:] == ["merge", "run"]
    assert mergeq_mod.run(home, load_config(home), read_lanes(home), dry_run=True) is False
    assert len(started) == 1


def test_a_gone_entry_is_closed_and_does_not_block_the_next(env, home: Path) -> None:
    repo, _ = env
    gone = make_lane(home, repo, "gone", {"g.txt": "1"})
    make_lane(home, repo, "after", {"a.txt": "1"})
    mergeq_mod.enqueue(home, "gone")
    mergeq_mod.enqueue(home, "after")
    git(gone, "add", "-A")
    git(gone, "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "work")
    subprocess.run(["git", "-C", str(repo), "worktree", "remove", "--force", str(gone)], check=True)
    rows = mergeq_mod.drain(home, load_config(home), relaunch=Sender())
    assert [(r["lane"], r["result"]) for r in rows] == [("gone", "closed"), ("after", "merged")]
    assert "worktree is gone" in rows[0]["message"]
    assert mergeq_mod.view(home, "demo")["items"] == []
    assert any(h["lane"] == "gone" and h["result"] == "closed" for h in mergeq_mod.load(home)["history"])


def test_a_running_entry_of_a_dead_runner_is_resumed(env, home: Path) -> None:
    repo, _ = env
    make_lane(home, repo, "died", {"x.txt": "1"})
    mergeq_mod.enqueue(home, "died")
    data = mergeq_mod.load(home)
    data["queue"][0].update(status=mergeq_mod.RUNNING, pid=2 ** 22 + 12345)  # no such process
    write_json_atomic(home / mergeq_mod.FILE, data)
    rows = mergeq_mod.drain(home, load_config(home), relaunch=Sender())
    assert [(r["lane"], r["result"]) for r in rows] == [("died", "merged")]


def test_a_continuation_lane_uses_the_worktree_and_branch_of_the_original(env, home: Path) -> None:
    repo, _ = env
    worktree = make_lane(home, repo, "orig", {"o.txt": "1"})
    register_lane(home, lane_id="orig2", project="demo", hero_class="glasswright", feature="Orig2",
                  workspace=str(worktree), log=None, pid=None, brief=None, step="2")
    end_lane(home, "orig2", "done", "continued it")
    (worktree / "more.txt").write_text("2", encoding="utf-8")
    mergeq_mod.enqueue(home, "orig2")
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == "merged"
    assert (repo / "more.txt").exists() and not worktree.exists()
    assert "lane/orig" not in git(repo, "branch", "--list", "lane/*")


def _linked(home: Path, lane_id: str) -> None:
    """The lane works on the plan's item and reported it done."""
    lane = read_json(lane_path(home, lane_id), None)
    lane.update(item="Add the feature file.", item_step="2")
    write_lane(home, lane)
    plangate.report(home, lane_id, "done")


def _plan(repo: Path) -> str:
    return (repo / "docs" / "TODO.md").read_text(encoding="utf-8")


def test_false_tick_the_queue_merges_nothing_for_a_lane_that_changed_nothing(env, home: Path) -> None:
    repo, _ = env
    worktree = make_lane(home, repo, "empty", {})
    _linked(home, "empty")
    head = git(repo, "rev-parse", "HEAD")
    mergeq_mod.enqueue(home, "empty")
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == mergeq_mod.CLOSED and "nothing to merge" in row["message"]
    assert git(repo, "rev-parse", "HEAD") == head and not worktree.exists()
    lane = read_json(lane_path(home, "empty"), None)
    assert lane["review"]["status"] == plangate.WORK_NONE
    assert plangate.lane_work(home, repo, lane) == (plangate.WORK_NONE, None)
    plangate.tick_merged(home, load_config(home), [lane])
    plan = _plan(repo)  # nothing ticked it; with no review, queue entry or branch left it is reopened
    assert "- [x] Add the feature file" not in plan and "(en revisión)" not in plan and "reabierto" in plan
    assert not mergeq_mod.ready_to_enqueue(home, lane, repo)


def test_false_tick_the_queue_saves_unsaved_work_then_ticks_its_merge(env, home: Path) -> None:
    repo, _ = env
    make_lane(home, repo, "unsaved", {"u.txt": "1"})
    _linked(home, "unsaved")
    lane = read_json(lane_path(home, "unsaved"), None)
    assert plangate.lane_work(home, repo, lane) == (plangate.WORK_UNSAVED, None)
    mergeq_mod.enqueue(home, "unsaved")
    assert mergeq_mod.process_next(home, load_config(home), relaunch=Sender())["result"] == "merged"
    assert (repo / "u.txt").exists() and "- [x] Add the feature file.\n" in _plan(repo)
    # the branch and the worktree are gone now: the merge commit is the proof when the review settles
    state, sha = plangate.lane_work(home, repo, read_json(lane_path(home, "unsaved"), None))
    assert state == plangate.WORK_MERGED and git(repo, "log", "-1", "--format=%s", sha).startswith("Merge lane/unsaved:")


def test_false_tick_the_queue_proves_a_continuation_by_the_branch_it_merged(env, home: Path) -> None:
    repo, _ = env
    worktree = make_lane(home, repo, "orig", {"o.txt": "1"})
    register_lane(home, lane_id="orig2", project="demo", hero_class="glasswright", feature="Orig2",
                  workspace=str(worktree), log=None, pid=None, brief=None, step="2")
    end_lane(home, "orig2", "done", "continued it")
    _linked(home, "orig2")
    mergeq_mod.enqueue(home, "orig2")
    assert mergeq_mod.process_next(home, load_config(home), relaunch=Sender())["result"] == "merged"
    assert "- [x] Add the feature file.\n" in _plan(repo)
    lane = read_json(lane_path(home, "orig2"), None)
    assert lane["branch"] == "lane/orig"
    assert plangate.lane_work(home, repo, lane)[0] == plangate.WORK_MERGED


def test_false_tick_a_gone_worktree_with_a_branch_at_its_start_is_not_already_merged(env, home: Path) -> None:
    repo, _ = env
    worktree = make_lane(home, repo, "lost", {})
    mergeq_mod.enqueue(home, "lost")
    git(repo, "worktree", "remove", "--force", str(worktree))
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert "worktree is gone" in row["message"]
    assert (read_json(lane_path(home, "lost"), None).get("review") or {}).get("status") != "merged"


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
                  "roadmap-edit", "raw-stage", "guard-reinstall", "lane-brief", "project-check", "plan-item", "commit-guard",
                  "workspace-write", "no-servers", "no-secrets", "own-processes", "ceo-read-only", "net-blocked"}


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


def _move_main_during_suite(monkeypatch: pytest.MonkeyPatch, repo: Path, files: dict[str, str], times: int = 99) -> None:
    real, left = mergeq_mod.review_mod.run_tests, [times]

    def run_tests(*args, **kwargs):
        result = real(*args, **kwargs)
        if left[0] > 0:
            left[0] -= 1
            for name, text in files.items():
                (repo / name).write_text(text, encoding="utf-8")
            git(repo, "add", ".")
            git(repo, "commit", "-q", "-m", "main moved")
        return result

    monkeypatch.setattr(mergeq_mod.review_mod, "run_tests", run_tests)


def test_plan_only_move_is_merged_without_a_second_suite_run(env, home: Path, monkeypatch) -> None:
    repo, origin = env
    make_lane(home, repo, "alpha", {"feature.txt": "a\n"})
    mergeq_mod.enqueue(home, "alpha")
    calls = []
    real = mergeq_mod.review_mod.run_tests
    monkeypatch.setattr(mergeq_mod.review_mod, "run_tests", lambda *a, **k: calls.append(1) or real(*a, **k))
    _move_main_during_suite(monkeypatch, repo, {"docs/TODO.md": PLAN + "- [ ] New item.\n"}, times=1)
    tested_tree = {}
    real_try = mergeq_mod._with_plan_moves

    def spy(repo_, tmp, base, head, tested, *rest):
        tested_tree["tested"] = tested
        return real_try(repo_, tmp, base, head, tested, *rest)

    monkeypatch.setattr(mergeq_mod, "_with_plan_moves", spy)
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == "merged" and len(calls) == 2  # one suite run: build and test
    assert "New item" in (repo / "docs" / "TODO.md").read_text() and (repo / "feature.txt").exists()
    assert git(repo, "rev-parse", "HEAD") == row["commit"] == git(origin, "rev-parse", "HEAD")
    changed = git(repo, "diff", "--name-only", tested_tree["tested"], row["commit"]).splitlines()
    assert changed == ["docs/TODO.md"]


def test_code_move_is_retried_and_the_next_entry_still_runs(env, home: Path, monkeypatch) -> None:
    repo, _ = env
    make_lane(home, repo, "one", {"one.txt": "1"})
    make_lane(home, repo, "two", {"two.txt": "2"})
    mergeq_mod.enqueue(home, "one")
    mergeq_mod.enqueue(home, "two")
    real, state = mergeq_mod._try, {"n": 0}

    def flaky(home_, repo_, lane, entry):
        if lane["id"] == "one":
            state["n"] += 1
            (repo / f"code{state['n']}.txt").write_text("x")
            git(repo, "add", ".")
            git(repo, "commit", "-q", "-m", "code moved")
            return "retry", "main moved while the suite ran", {}
        return real(home_, repo_, lane, entry)

    monkeypatch.setattr(mergeq_mod, "_try", flaky)
    config = load_config(home)
    config.mergeq_train = 1  # the single-lane path under test
    rows = mergeq_mod.drain(home, config, relaunch=Sender())
    # pytest shortens a list of dicts to "{...}, ...": spell out every row so a failing log says why a lane waited
    said = "\n".join(f"{r['lane']}: {r['result']}: {r.get('message')}" for r in rows)
    assert [(r["lane"], r["result"]) for r in rows] == [("one", "waiting"), ("two", "merged")], said
    assert rows[0]["message"] == "main moved while the suite ran", said
    assert [i["lane"] for i in mergeq_mod.view(home, "demo")["items"]] == ["one"]


def test_code_and_plan_move_together_is_still_retried(env, home: Path, monkeypatch) -> None:
    repo, _ = env
    make_lane(home, repo, "alpha", {"feature.txt": "a\n"})
    mergeq_mod.enqueue(home, "alpha")
    _move_main_during_suite(monkeypatch, repo, {"docs/TODO.md": PLAN + "- [ ] X.\n", "other.txt": "o"}, times=1)
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == "merged" and (repo / "other.txt").exists()


def test_lane_on_its_own_workspace_branch_merges(env, home: Path) -> None:
    repo, _ = env
    worktree = launch_mod.create_worktree(repo, "foo")
    register_lane(home, lane_id="u-foo", project="demo", hero_class="glasswright", feature="Foo",
                  workspace=str(worktree), log=None, pid=None, brief=None, step="2")
    (worktree / "foo.txt").write_text("f\n", encoding="utf-8")
    end_lane(home, "u-foo", "done", "built it")
    assert git(worktree, "rev-parse", "--abbrev-ref", "HEAD") == "lane/foo"
    mergeq_mod.enqueue(home, "u-foo")
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == "merged" and (repo / "foo.txt").read_text() == "f\n"
    assert "lane/foo" not in git(repo, "branch", "--list")


def test_continuation_lane_sharing_the_original_worktree_merges(env, home: Path) -> None:
    repo, _ = env
    worktree = make_lane(home, repo, "orig", {"orig.txt": "o"})
    register_lane(home, lane_id="orig-2", project="demo", hero_class="glasswright", feature="Orig2",
                  workspace=str(worktree), log=None, pid=None, brief=None, step="2")
    (worktree / "more.txt").write_text("m", encoding="utf-8")
    end_lane(home, "orig-2", "done", "more")
    mergeq_mod.enqueue(home, "orig-2")
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == "merged" and (repo / "more.txt").exists() and (repo / "orig.txt").exists()


def test_missing_branch_is_reported_plainly(env, home: Path) -> None:
    repo, _ = env
    worktree = make_lane(home, repo, "lost", {"lost.txt": "l"})
    git(worktree, "checkout", "-q", "--detach")
    git(repo, "branch", "-D", "lane/lost")
    lane = read_json(lane_path(home, "lost"), None)
    lane["branch"] = "lane/lost"
    write_lane(home, lane)
    mergeq_mod.enqueue(home, "lost")
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] != "conflict" and "branch not found: lane/lost" in row["message"]


def _flaky_env(env, home: Path, script: str) -> Path:
    repo, _ = env
    (repo / "check.sh").write_text(script, encoding="utf-8")  # the repeat appends "-n 0 <test id>" to the stage command
    text = 'deploy = "none"\n[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "test"\nrun = "sh check.sh"\n'
    (repo / "tirith.toml").write_text(text, encoding="utf-8")
    git(repo, "add", ".")
    git(repo, "commit", "-qm", "flaky manifest")
    make_lane(home, repo, "alpha", {"feature.txt": "a\n"})
    mergeq_mod.enqueue(home, "alpha")
    return repo


def test_rerun_failed_passing_alone_merges_and_counts_flaky(env, home: Path) -> None:
    repo = _flaky_env(env, home, 'case "$*" in *-n*) echo "1 passed";; *) echo "FAILED t.py::test_a - boom"; exit 1;; esac\n')
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == "merged" and row["flaky"] == ["t.py::test_a"]
    assert (repo / "feature.txt").exists()
    assert mergeq_mod.view(home, "demo")["flaky"]["t.py::test_a"]["count"] == 1


def test_rerun_failed_failing_again_is_a_real_red(env, home: Path) -> None:
    _flaky_env(env, home, 'echo "FAILED t.py::test_a - boom"\nexit 1\n')
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == "red-suite"
    assert mergeq_mod.view(home, "demo")["flaky"] == {}


def test_rerun_failed_without_a_report_is_a_real_red(env, home: Path) -> None:
    _flaky_env(env, home, 'echo "ERROR t.py - collection"\nexit 1\n')
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == "red-suite"


def _cut_git_merge(monkeypatch: pytest.MonkeyPatch, times: int) -> list[int]:
    """The first ``times`` ``git merge --no-ff`` calls are stopped at their deadline (exit 124)."""
    real, left, seen = mergeq_mod._git, [times], []

    def cut(folder, *args):
        if args[:2] == ("merge", "--no-ff") and left[0] > 0:
            left[0] -= 1
            seen.append(1)
            return mergeq_mod.timeouts_mod.Result(["git", *args], 124, "", "", True, 1.0, "git timed out")
        return real(folder, *args)

    monkeypatch.setattr(mergeq_mod, "_git", cut)
    return seen


def test_merge_timeout_is_retried_and_the_lane_is_not_sent_back(env, home: Path, monkeypatch) -> None:
    repo, _ = env
    make_lane(home, repo, "alpha", {"feature.txt": "a\n"})
    mergeq_mod.enqueue(home, "alpha")
    seen = _cut_git_merge(monkeypatch, 1)
    sender = Sender()
    row = mergeq_mod.process_next(home, load_config(home), relaunch=sender)
    assert row["result"] == "merged" and len(seen) == 1 and not sender.calls


def test_merge_timeout_every_time_waits_in_the_queue_not_sent_back(env, home: Path, monkeypatch) -> None:
    repo, _ = env
    make_lane(home, repo, "alpha", {"feature.txt": "a\n"})
    mergeq_mod.enqueue(home, "alpha")
    _cut_git_merge(monkeypatch, 99)
    sender = Sender()
    row = mergeq_mod.process_next(home, load_config(home), relaunch=sender)
    assert row["result"] == "waiting" and "timed out" in row["message"] and not sender.calls
    assert [i["lane"] for i in mergeq_mod.view(home, "demo")["items"]] == ["alpha"]


def _suite_times_out(monkeypatch: pytest.MonkeyPatch) -> list[int]:
    calls: list[int] = []

    def run_tests(*args, **kwargs):
        calls.append(1)
        return False, f"{mergeq_mod.review_mod.DID_NOT_FINISH} 1800 s"

    monkeypatch.setattr(mergeq_mod.review_mod, "run_tests", run_tests)
    return calls


def test_stage_timeout_is_not_red_and_the_entry_is_queued_again_once(env, home: Path, monkeypatch) -> None:
    repo, _ = env
    make_lane(home, repo, "slow", {"slow.txt": "s"})
    _suite_times_out(monkeypatch)
    mergeq_mod.enqueue(home, "slow")
    send = Sender()
    row = mergeq_mod.process_next(home, load_config(home), relaunch=send)
    assert row["result"] == "waiting" and "ran out of time" in row["message"] and "failed" not in row["message"]
    assert not send.calls and (read_json(lane_path(home, "slow"), None).get("review") or {}).get("status") != mergeq_mod.SENT_BACK_STATUS
    item, = mergeq_mod.view(home, "demo")["items"]
    assert item["status"] == "queued" and not asks_mod.queue_items(home)


def test_stage_timeout_is_not_red_twice_shows_the_manager(env, home: Path, monkeypatch) -> None:
    repo, _ = env
    make_lane(home, repo, "slow", {"slow.txt": "s"})
    _suite_times_out(monkeypatch)
    mergeq_mod.enqueue(home, "slow")
    send = Sender()
    for _ in range(2):
        row = mergeq_mod.process_next(home, load_config(home), relaunch=send)
    assert row["result"] == "waiting" and "ran out of time" in row["message"] and not send.calls
    assert any("ran out of time" in str(i) for i in asks_mod.queue_items(home))
    assert mergeq_mod.view(home, "demo")["items"][0]["status"] == "queued"


def test_stage_timeout_is_not_red_run_tests_reads_the_timeouts_mechanism(env, home: Path, monkeypatch) -> None:
    repo, _ = env
    seen: dict = {}

    def run_stage(*args, **kwargs):
        seen.update(kwargs)
        return 124, "timed out: x"

    monkeypatch.setattr(mergeq_mod.review_mod.runstage_mod, "run_stage", run_stage)
    monkeypatch.setattr(mergeq_mod.review_mod.timeouts_mod, "effective", lambda *a, **k: 42.0)
    ok, output = mergeq_mod.review_mod.run_tests(home, "demo", "ci", "test", repo, "true")
    assert not ok and output.startswith("tests did not finish in 42 s") and "timeout" not in seen


# -- fast paths: stages that apply, and the train -----------------------------------------------------

LOGGED = ('deploy = "none"\n[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "build"\nrun = "echo build >> $STAGE_LOG"\n'
          '[[pipelines.stages]]\nname = "test"\nrun = "echo test >> $STAGE_LOG; echo 4 passed; ! test -f BAD"\n'
          '[[pipelines.stages]]\nname = "rust-test"\nkind = "test"\nrun = "echo rust >> $STAGE_LOG"\n')


@pytest.fixture
def logged(env, home: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    """The demo repo with a manifest whose stages log their name, so a test can see which ones ran."""
    repo, origin = env
    log = tmp_path / "stages.log"
    monkeypatch.setenv("STAGE_LOG", str(log))
    (repo / "tirith.toml").write_text(LOGGED, encoding="utf-8")
    git(repo, "commit", "-q", "-am", "log the stages")
    return repo, (lambda: log.read_text().split() if log.exists() else [])


def land(home: Path, **config) -> list[dict]:
    return mergeq_mod.drain(home, load_config(home), relaunch=Sender(), **config)


@pytest.mark.parametrize("files,ran", [
    ({"docs/PLAN.md": "x"}, ["build"]),
    ({"README.md": "x", "site/index.html": "x"}, ["build"]),
    ({"ios/App/Main.swift": "x"}, ["build"]),
    ({"web/app.js": "x"}, ["build", "test"]),
    ({"collector/x.py": "x", "docs/a.md": "x"}, ["build", "test"]),
    ({"tirith.toml.bak": "x"}, ["build", "test"]),
    ({"app/src-tauri/lib.rs": "x"}, ["build", "test", "rust"]),
    ({"Cargo.lock": "x"}, ["build", "test", "rust"]),
])
def test_merge_paths_run_only_the_stages_that_apply(logged, home: Path, files: dict, ran: list) -> None:
    repo, log = logged
    wt = make_lane(home, repo, "one", {})
    for name, text in files.items():
        (wt / name).parent.mkdir(parents=True, exist_ok=True)
        (wt / name).write_text(text, encoding="utf-8")
    mergeq_mod.enqueue(home, "one")
    rows = land(home)
    assert rows[0]["result"] == "merged" and log() == ran
    assert bool(rows[0].get("skipped")) == (ran != ["build", "test", "rust"])


def test_merge_paths_say_what_was_skipped_in_the_history_row(logged, home: Path) -> None:
    repo, log = logged
    make_lane(home, repo, "plan", {"notes.md": "x"})
    mergeq_mod.enqueue(home, "plan")
    row = land(home)[0]
    assert row["skipped"] == ["test", "rust-test"] and "skipped test, rust-test" in row["message"]
    assert mergeq_mod.load(home)["history"][-1]["skipped"] == ["test", "rust-test"]


def test_mergetrain_two_good_lanes_land_with_one_stage_run(logged, home: Path) -> None:
    repo, log = logged
    for name in ("one", "two"):
        make_lane(home, repo, name, {f"{name}.txt": name})
        mergeq_mod.enqueue(home, name)
    before = git(repo, "rev-parse", "HEAD")
    rows = land(home)
    assert [(r["lane"], r["result"]) for r in rows] == [("one", "merged"), ("two", "merged")]
    assert log() == ["build", "test"] and rows[0]["train"] == ["one", "two"]
    assert rows[0]["commit"] == rows[1]["commit"] == git(repo, "rev-parse", "HEAD")
    assert (repo / "one.txt").exists() and (repo / "two.txt").exists()
    assert git(repo, "rev-list", "--count", "--first-parent", f"{before}..HEAD") == "2"
    assert mergeq_mod.view(home, "demo")["items"] == []
    assert not list(repo.parent.glob("demo-lanes/_mergeq-*"))
    assert all(read_json(lane_path(home, n), None)["review"]["status"] == "merged" for n in ("one", "two"))


def test_mergetrain_a_conflicting_lane_leaves_and_the_other_lands(logged, home: Path) -> None:
    repo, log = logged
    (repo / "shared.txt").write_text("base\n")
    git(repo, "add", "."), git(repo, "commit", "-q", "-m", "shared")
    for name in ("one", "two", "three"):
        make_lane(home, repo, name, {"shared.txt": f"{name}\n"} if name != "three" else {"three.txt": "3"})
        mergeq_mod.enqueue(home, name)
    sender = Sender()
    rows = mergeq_mod.drain(home, load_config(home), relaunch=sender)
    assert {r["lane"]: r["result"] for r in rows} == {"one": "merged", "two": "conflict", "three": "merged"}
    assert [c[0] for c in sender.calls] == ["two"] and "shared.txt" in sender.calls[0][1]
    assert (repo / "shared.txt").read_text() == "one\n" and (repo / "three.txt").exists()
    assert log() == ["build", "test"]  # one pass for the two that joined


def test_mergetrain_a_red_train_falls_back_and_only_the_culprit_is_rejected(logged, home: Path) -> None:
    repo, log = logged
    for name, files in (("good", {"good.txt": "g"}), ("bad", {"BAD": "x"}), ("fine", {"fine.txt": "f"})):
        make_lane(home, repo, name, files)
        mergeq_mod.enqueue(home, name)
    sender = Sender()
    rows = mergeq_mod.drain(home, load_config(home), relaunch=sender)
    assert {r["lane"]: r["result"] for r in rows} == {"good": "merged", "bad": "red-suite", "fine": "merged"}
    assert [c[0] for c in sender.calls] == ["bad"]
    assert not (repo / "BAD").exists() and (repo / "good.txt").exists() and (repo / "fine.txt").exists()
    assert all("train" not in r for r in rows if r["result"] == "merged")  # they landed alone
    assert mergeq_mod.view(home, "demo")["items"] == []


def test_mergeq_train_dirty_clash_holds_back_only_the_lane_that_changes_the_dirty_file(logged, home: Path) -> None:
    repo, log = logged
    (repo / "f.txt").write_text("base\n")
    (repo / "g.txt").write_text("base\n")
    git(repo, "add", "."), git(repo, "commit", "-q", "-m", "f and g")
    make_lane(home, repo, "a", {"f.txt": "from a\n"})
    make_lane(home, repo, "b", {"g.txt": "from b\n"})
    for name in ("a", "b"):
        mergeq_mod.enqueue(home, name)
    (repo / "f.txt").write_text("owner edit\n")
    rows = mergeq_mod.process(home, load_config(home), relaunch=Sender())
    assert {r["lane"]: r["result"] for r in rows} == {"a": "waiting", "b": "merged"}
    assert "f.txt" in {r["lane"]: r for r in rows}["a"]["message"]
    assert (repo / "g.txt").read_text() == "from b\n" and (repo / "f.txt").read_text() == "owner edit\n"
    assert [i["lane"] for i in mergeq_mod.view(home, "demo")["items"]] == ["a"]


def test_mergetrain_size_one_turns_it_off(logged, home: Path) -> None:
    repo, log = logged
    with open(home / "config.toml", "a", encoding="utf-8") as handle:
        handle.write("\n[mergeq]\ntrain = 1\n")
    assert load_config(home).mergeq_train == 1
    for name in ("one", "two"):
        make_lane(home, repo, name, {f"{name}.txt": name})
        mergeq_mod.enqueue(home, name)
    rows = land(home)
    assert [r["result"] for r in rows] == ["merged", "merged"] and all("train" not in r for r in rows)
    assert log() == ["build", "test"] * 2


class StartingSender(Sender):
    """A relauncher whose agent starts: the new lane is registered, as ``tirith lane run`` does."""

    def __init__(self, home: Path) -> None:
        super().__init__()
        self.home = home

    def __call__(self, home, lane, alias, note) -> str:
        new = super().__call__(home, lane, alias, note)
        register_lane(self.home, lane_id=new, project="demo", hero_class="glasswright", feature="again",
                      workspace=str(lane["workspace"]), log=None, pid=None, brief=None, step="2")
        return new


def _link_item(home: Path, lane_id: str) -> None:
    lane = read_json(lane_path(home, lane_id), None)
    lane["item"], lane["item_step"] = "Add the feature file.", "2"
    write_lane(home, lane)


def test_send_back_reopens_the_plan_item_and_confirms_the_agent_started(env, home: Path) -> None:
    repo, _ = env
    make_lane(home, repo, "bad", {"BAD": "x"})
    _link_item(home, "bad")
    mergeq_mod.enqueue(home, "bad")
    row = mergeq_mod.process_next(home, load_config(home), relaunch=StartingSender(home))
    assert row["result"] == "red-suite" and row["sent_to"] == "bad-2"
    assert "(reabierto: red-suite" in (repo / "docs" / "TODO.md").read_text(encoding="utf-8")
    review = read_json(lane_path(home, "bad"), None)["review"]
    assert review["status"] == mergeq_mod.SENT_BACK_STATUS and review["sent_to"] == "bad-2" and review["started"] is True
    assert not any("did not start" in str(i) for i in asks_mod.queue_items(home))


def test_send_back_tells_the_manager_when_the_agent_did_not_start(env, home: Path) -> None:
    repo, _ = env
    make_lane(home, repo, "bad", {"BAD": "x"})
    _link_item(home, "bad")
    mergeq_mod.enqueue(home, "bad")
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())  # never registers its lane
    assert row["result"] == "red-suite"
    assert "(reabierto: red-suite" in (repo / "docs" / "TODO.md").read_text(encoding="utf-8")
    assert read_json(lane_path(home, "bad"), None)["review"]["started"] is False
    assert any("did not start" in str(i) for i in asks_mod.queue_items(home))


def test_stage_timeout_on_a_train_is_not_red_it_does_not_split_or_send_back(logged, home: Path, monkeypatch) -> None:
    repo, log = logged
    calls = _suite_times_out(monkeypatch)
    for name in ("one", "two"):
        make_lane(home, repo, name, {f"{name}.txt": name})
        mergeq_mod.enqueue(home, name)
    send = Sender()
    rows = mergeq_mod.process(home, load_config(home), relaunch=send)
    assert [(r["lane"], r["result"]) for r in rows] == [("one", "waiting"), ("two", "waiting")]
    assert all("ran out of time" in r["message"] and "failed" not in r["message"] for r in rows)
    assert len(calls) == 1 and not send.calls  # one run for the whole train, nobody sent back
    assert [i["status"] for i in mergeq_mod.view(home, "demo")["items"]] == ["queued", "queued"]
    assert not asks_mod.queue_items(home) and not (repo / "one.txt").exists()


# -- the queue's loose ends: lock, continuation, agent null, tested trees, absorbed lanes ----------------------


def test_commit_workspace_recovers_lock(env, home: Path) -> None:
    repo, _ = env
    worktree = make_lane(home, repo, "locked-lane", {"l.txt": "1"})
    lock = Path(git(worktree, "rev-parse", "--path-format=absolute", "--git-path", "index.lock"))
    lock.write_text("")
    old = time.time() - 600
    os.utime(lock, (old, old))
    mergeq_mod.enqueue(home, "locked-lane")
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == "merged" and (repo / "l.txt").exists() and not lock.exists()


def _done_continuation(home: Path, worktree: Path, lane_id: str) -> None:
    register_lane(home, lane_id=lane_id, project="demo", hero_class="glasswright", feature=lane_id.title(),
                  workspace=str(worktree), log=None, pid=None, brief=None, step="2")
    (worktree / "more.txt").write_text("m", encoding="utf-8")
    end_lane(home, lane_id, "done", "more")
    lane = read_json(lane_path(home, lane_id), None)
    lane["outcome"] = {"status": "done", "added": [], "reason": None, "at": "x", "applied": True}
    write_lane(home, lane)


def test_continuation_lane_auto_enqueued(env, home: Path) -> None:
    repo, _ = env
    worktree = make_lane(home, repo, "orig", {"o.txt": "o"})
    _done_continuation(home, worktree, "orig-2")
    elsewhere = home / "elsewhere"
    elsewhere.mkdir()
    register_lane(home, lane_id="loose", project="demo", hero_class="glasswright", feature="Loose",
                  workspace=str(elsewhere), log=None, pid=None, brief=None, step="2")
    end_lane(home, "loose", "done", "x")
    lane = read_json(lane_path(home, "loose"), None)
    lane["outcome"] = {"status": "done", "added": [], "reason": None, "at": "x", "applied": True}
    write_lane(home, lane)
    mergeq_mod.run(home, load_config(home), read_lanes(home), spawn=lambda *a: None, dry_run=True)
    assert [i["lane"] for i in mergeq_mod.view(home)["items"]] == ["orig-2"]  # not a folder of someone else's


def test_send_back_continuation(env, home: Path, monkeypatch) -> None:
    from tirith.cli import relaunch_lane
    repo, _ = env
    worktree = make_lane(home, repo, "orig", {"o.txt": "o"})
    _done_continuation(home, worktree, "orig-2")
    brief = home / "brief.md"
    brief.write_text("# Lane: x\n", encoding="utf-8")
    lane = read_json(lane_path(home, "orig-2"), None)
    lane["brief"] = str(brief)
    write_lane(home, lane)
    spawned: list[list[str]] = []
    monkeypatch.setattr(launch_mod, "spawn_detached", lambda argv, h, log: spawned.append(argv) or 1)
    monkeypatch.setattr(launch_mod, "agent_command", lambda *a, **k: ["true"])
    new_id = relaunch_lane(home, lane, "anthropic", by_tirith=True, note="fix it")
    new_workspace = spawned[-1][spawned[-1].index("--workspace") + 1]
    assert git(Path(new_workspace), "log", "--format=%s", "-1").startswith("Lane orig-2")
    assert (Path(new_workspace) / "more.txt").exists() and new_id != "orig-2"


class RecordingSender(Sender):
    def __init__(self) -> None:
        super().__init__()
        self.aliases: list = []

    def __call__(self, home, lane, alias, note) -> str:
        self.aliases.append(alias)
        return super().__call__(home, lane, alias, note)


def test_send_back_without_agent(env, home: Path) -> None:
    repo, _ = env
    make_lane(home, repo, "bad", {"BAD": "x"})
    lane = read_json(lane_path(home, "bad"), None)
    lane["agent"] = None
    write_lane(home, lane)
    mergeq_mod.enqueue(home, "bad")
    send = RecordingSender()
    row = mergeq_mod.process_next(home, load_config(home), relaunch=send)
    assert row["result"] == "red-suite" and row["sent_to"] == "bad-2" and send.aliases == [None]
    make_lane(home, repo, "bad2", {"BAD": "x"})
    lane = read_json(lane_path(home, "bad2"), None)
    lane["agent"] = "ghost"
    write_lane(home, lane)
    mergeq_mod.enqueue(home, "bad2")
    mergeq_mod.process_next(home, load_config(home), relaunch=send)
    assert send.aliases == [None] and any("'ghost' is not in the configuration" in str(i)
                                          for i in asks_mod.queue_items(home))


def test_same_tree_is_not_tested_again(logged, home: Path) -> None:
    repo, log = logged
    wt = make_lane(home, repo, "one", {"x.txt": "x"})
    test_cmd = "echo test >> $STAGE_LOG; echo 4 passed; ! test -f BAD"
    ok, out = mergeq_mod.review_mod.run_tests(home, "demo", "ci", "test", wt, test_cmd, "one")  # the lane's own run
    assert ok and log() == ["test"]
    again, text = mergeq_mod.review_mod.run_tests(home, "demo", "ci", "test", wt, test_cmd, "one")
    assert again and log() == ["test"] and "4 passed" in text and "not run again" in text
    (wt / "x.txt").write_text("changed", encoding="utf-8")  # another tree: it runs
    assert mergeq_mod.review_mod.run_tests(home, "demo", "ci", "test", wt, test_cmd, "one")[0]
    assert log() == ["test", "test"]
    (wt / "BAD").write_text("", encoding="utf-8")  # a failed tree is never remembered
    assert not mergeq_mod.review_mod.run_tests(home, "demo", "ci", "test", wt, test_cmd, "one")[0]
    assert not mergeq_mod.review_mod.run_tests(home, "demo", "ci", "test", wt, test_cmd, "one")[0]
    assert log() == ["test", "test", "test", "test"]
    (wt / "BAD").unlink()
    # the queue merges that same tree: its test stage is not run again
    mergeq_mod.enqueue(home, "one")
    row = land(home)[0]
    assert row["result"] == "merged" and log() == ["test", "test", "test", "test", "build"]


def test_absorbed_lane_stops(env, home: Path, monkeypatch) -> None:
    repo, _ = env
    base = make_lane(home, repo, "base", {"a.txt": "a"})
    git(base, "add", "-A")
    git(base, "commit", "-q", "-m", "base work")
    top = launch_mod.create_worktree(repo, "top", base="lane/base")  # a continuation: holds base's commits
    register_lane(home, lane_id="top", project="demo", hero_class="glasswright", feature="Top",
                  workspace=str(top), log=None, pid=None, brief=None, step="2")
    (top / "b.txt").write_text("b", encoding="utf-8")
    end_lane(home, "top", "done", "more")
    calls = []
    real = mergeq_mod.review_mod.run_tests
    monkeypatch.setattr(mergeq_mod.review_mod, "run_tests", lambda *a, **k: calls.append(a[2:4]) or real(*a, **k))
    mergeq_mod.enqueue(home, "base")
    mergeq_mod.enqueue(home, "top")
    rows = mergeq_mod.drain(home, load_config(home), relaunch=Sender())
    by_lane = {r["lane"]: r for r in rows}
    assert by_lane["base"]["result"] == "closed" and "already in the queued branch of top" in by_lane["base"]["message"]
    assert by_lane["top"]["result"] == "merged" and calls == [("ci", "build"), ("ci", "test")]
    assert (repo / "a.txt").exists() and (repo / "b.txt").exists() and mergeq_mod.view(home)["items"] == []
    assert read_json(lane_path(home, "base"), None)["review"]["status"] == "no-work"


def test_a_repeating_wait_tells_the_manager_its_reason_once(env, home: Path) -> None:
    repo, _ = env
    make_lane(home, repo, "late2", {"README.md": "l"})
    (repo / "README.md").write_text("edited\n", encoding="utf-8")
    mergeq_mod.enqueue(home, "late2")
    mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert not asks_mod.queue_items(home)
    for _ in range(2):
        assert mergeq_mod.process_next(home, load_config(home), relaunch=Sender())["result"] == "waiting"
    items = [i for i in asks_mod.queue_items(home) if "uncommitted changes" in str(i)]
    assert len(items) == 1 and items[0]["to"] == "manager"


def test_a_failed_push_tells_the_manager(env, home: Path, monkeypatch) -> None:
    repo, _ = env
    make_lane(home, repo, "pushy", {"p.txt": "p"})
    mergeq_mod.enqueue(home, "pushy")
    monkeypatch.setattr(mergeq_mod, "_push", lambda repo, branch: "remote rejected")
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == "push-failed"
    assert any("push-failed: remote rejected" in str(i) for i in asks_mod.queue_items(home))


def _fail_first_push(monkeypatch, really_push: bool) -> None:
    real = mergeq_mod.timeouts_mod.run
    state = {"n": 0}

    def run(cmd, *args, **kwargs):
        if "push" in cmd and state["n"] == 0:
            state["n"] = 1
            if really_push:
                real(cmd, *args, **kwargs)
            return subprocess.CompletedProcess(cmd, 1, "", "connection reset")
        return real(cmd, *args, **kwargs)
    monkeypatch.setattr(mergeq_mod.timeouts_mod, "run", run)


def test_push_race_already_on_origin_is_merged(env, home: Path, monkeypatch) -> None:
    repo, origin = env
    make_lane(home, repo, "racer", {"r.txt": "r"})
    mergeq_mod.enqueue(home, "racer")
    _fail_first_push(monkeypatch, True)
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == "merged"
    assert git(origin, "rev-parse", "HEAD") == git(repo, "rev-parse", "HEAD")


def test_push_race_main_moved_ahead_is_rebased_and_pushed(env, home: Path, monkeypatch, tmp_path: Path) -> None:
    repo, origin = env
    make_lane(home, repo, "racer2", {"r.txt": "r"})
    mergeq_mod.enqueue(home, "racer2")
    other = tmp_path / "other"
    subprocess.run(["git", "clone", "-q", str(origin), str(other)], check=True)
    (other / "o.txt").write_text("o", encoding="utf-8")
    git(other, "add", "o.txt")
    git(other, "commit", "-q", "-m", "other")
    git(other, "push", "-q", "origin", "HEAD")
    _fail_first_push(monkeypatch, False)
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == "merged"
    assert git(origin, "rev-parse", "HEAD") == git(repo, "rev-parse", "HEAD")
    assert (repo / "o.txt").exists() and (repo / "r.txt").exists()


def test_merge_priority_a_queue_fix_goes_before_waiting_lanes(env, home: Path) -> None:
    repo, _ = env
    make_lane(home, repo, "plain1", {"a.txt": "1"})
    make_lane(home, repo, "plain2", {"b.txt": "2"})
    make_lane(home, repo, "fix", {})
    fix = read_json(lane_path(home, "fix"), None)
    target = Path(fix["workspace"]) / "collector" / "tirith"
    target.mkdir(parents=True)
    (target / "mergeq.py").write_text("# fix\n", encoding="utf-8")
    mergeq_mod.enqueue(home, "plain1")
    mergeq_mod.enqueue(home, "plain2")
    assert mergeq_mod.enqueue(home, "fix") == 1
    assert [i["lane"] for i in mergeq_mod.view(home, "demo")["items"]] == ["fix", "plain1", "plain2"]


def test_merge_priority_never_overtakes_the_running_merge(env, home: Path) -> None:
    repo, _ = env
    make_lane(home, repo, "plain1", {"a.txt": "1"})
    make_lane(home, repo, "fix", {})
    fix = read_json(lane_path(home, "fix"), None)
    (Path(fix["workspace"]) / "docs").mkdir(exist_ok=True)
    (Path(fix["workspace"]) / "docs" / "CONTRACT-mergeq.md").write_text("x\n", encoding="utf-8")
    mergeq_mod.enqueue(home, "plain1")
    assert mergeq_mod._claim(home)["lane"] == "plain1"
    assert mergeq_mod.enqueue(home, "fix") == 2


def test_merge_cancel_a_lane_queued_meanwhile_that_holds_the_running_one_cancels_it(env, home: Path, monkeypatch) -> None:
    repo, _ = env
    base = make_lane(home, repo, "base", {"a.txt": "a"})
    git(base, "add", "-A")
    git(base, "commit", "-q", "-m", "base work")
    top = launch_mod.create_worktree(repo, "top", base="lane/base")
    register_lane(home, lane_id="top", project="demo", hero_class="glasswright", feature="Top",
                  workspace=str(top), log=None, pid=None, brief=None, step="2")
    (top / "b.txt").write_text("b", encoding="utf-8")
    end_lane(home, "top", "done", "more")
    mergeq_mod.enqueue(home, "base")
    real = mergeq_mod.review_mod.run_tests
    calls = []

    def run_tests(*args, **kwargs):
        calls.append(args[3])
        if len(calls) == 1:  # while base's build runs, the continuation is queued
            mergeq_mod.enqueue(home, "top")
        return real(*args, **kwargs)

    monkeypatch.setattr(mergeq_mod.review_mod, "run_tests", run_tests)
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["lane"] == "base" and row["result"] == "closed"
    assert "already in the queued branch of top" in row["message"] and calls == ["build"]
    assert not (repo / "a.txt").exists()  # main untouched by the cancelled merge
    assert read_json(lane_path(home, "base"), None)["review"]["status"] == "no-work"
    assert [i["lane"] for i in mergeq_mod.view(home, "demo")["items"]] == ["top"]


def test_merge_cancel_without_a_superseding_lane_merges_as_before(env, home: Path) -> None:
    repo, _ = env
    make_lane(home, repo, "solo", {"s.txt": "s"})
    mergeq_mod.enqueue(home, "solo")
    assert mergeq_mod.process_next(home, load_config(home), relaunch=Sender())["result"] == "merged"


def test_todo_conflict_keeps_main_and_reapplies_the_lane_plan_report(env, home: Path) -> None:
    repo, _ = env
    worktree = make_lane(home, repo, "plan", {"feature.txt": "a\n"})
    _linked(home, "plan")
    # the report went into main's plan; the lane's own copy of the same line diverged
    (worktree / "docs" / "TODO.md").write_text(PLAN.replace("file.", "file (lane edit)."), encoding="utf-8")
    assert "(en revisión)" in _plan(repo)
    mergeq_mod.enqueue(home, "plan")
    row = mergeq_mod.process_next(home, load_config(home), relaunch=Sender())
    assert row["result"] == "merged", row
    text = _plan(repo)
    assert "- [x] Add the feature file.\n" in text  # main's version, the item ticked by the merge
    assert "<<<<<<<" not in text and "lane edit" not in text and (repo / "feature.txt").exists()


def test_dirty_checkout_clash_waits_without_running_the_suite_again(logged, home: Path) -> None:
    repo, log = logged
    make_lane(home, repo, "late", {"README.md": "from lane\n"})
    (repo / "README.md").write_text("edited\n", encoding="utf-8")
    mergeq_mod.enqueue(home, "late")
    assert mergeq_mod.process_next(home, load_config(home), relaunch=Sender())["result"] == "waiting"
    ran = log()
    git(repo, "checkout", "--", "README.md")
    assert mergeq_mod.process_next(home, load_config(home), relaunch=Sender())["result"] == "merged"
    assert log().count("test") == ran.count("test")  # the proven tree was not tested again


# -- the canary has its own track: never behind another project's merge or its full-suite lock ----


def _entry(lane: str, project: str, status: str = "queued", pid: int | None = None) -> dict:
    return {"lane": lane, "project": project, "at": "2026-01-01T00:00:00Z", "status": status,
            "feature": lane, **({"pid": pid} if pid else {})}


def test_canary_merge_claimed_while_another_project_merges(tmp_path: Path) -> None:
    write_json_atomic(mergeq_mod._path(tmp_path), {"queue": [
        _entry("t-1", "tirith", "running", os.getpid()), _entry("t-2", "tirith"), _entry("c-1", "canary")],
        "last": {}, "history": []})
    assert [e["lane"] for e in mergeq_mod._claim_many(tmp_path)] == ["c-1"]
    assert mergeq_mod._claim_many(tmp_path) == []  # t-1 still runs on its track, c-1 on the canary's


def test_canary_merge_does_not_hold_back_tirith(tmp_path: Path) -> None:
    write_json_atomic(mergeq_mod._path(tmp_path), {"queue": [
        _entry("c-1", "canary", "running", os.getpid()), _entry("t-1", "tirith")], "last": {}, "history": []})
    assert [e["lane"] for e in mergeq_mod._claim_many(tmp_path)] == ["t-1"]


def test_canary_merge_check_skips_the_full_suite_lock(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    seen = []
    monkeypatch.setattr(mergeq_mod.review_mod, "run_tests",
                        lambda home, project, pipeline, stage, tmp, run, lane, full=True:
                        seen.append((project, full)) or (True, "1 passed"))
    stage = manifest_mod.Stage(name="test", run="echo 1 passed", kind="test")
    for project in ("canary", "tirith"):
        mergeq_mod._run_stages(tmp_path, project, [("ci", stage)], tmp_path, "l-1")
    assert seen == [("canary", False), ("tirith", True)]


def test_mergeq_no_stage_goes_once_to_the_manager_without_retry(env, home: Path) -> None:
    repo, _ = env
    (repo / "tirith.toml").write_text('deploy = "none"\n', encoding="utf-8")
    git(repo, "add", ".")
    git(repo, "commit", "-q", "-m", "no stages")
    make_lane(home, repo, "bare", {"bare.txt": "x"})
    mergeq_mod.enqueue(home, "bare")
    send = Sender()
    row = mergeq_mod.process_next(home, load_config(home), relaunch=send)
    assert row["result"] == mergeq_mod.NEEDS_STAGE and not send.calls
    assert mergeq_mod.view(home, "demo")["items"] == []  # left the queue: no retry, no stuck alarm
    assert read_json(lane_path(home, "bare"), None)["review"]["status"] == "needs a test stage"
    assert sum("needs a test stage" in str(i) for i in asks_mod.queue_items(home)) == 1
    assert mergeq_mod.process_next(home, load_config(home), relaunch=send) is None
    assert sum("needs a test stage" in str(i) for i in asks_mod.queue_items(home)) == 1


def test_a_waiting_lane_does_not_block_the_lanes_behind_it(env, home: Path) -> None:
    repo, _ = env
    make_lane(home, repo, "blocked", {"README.md": "from lane\n"})
    make_lane(home, repo, "behind", {"behind.txt": "b"})
    mergeq_mod.enqueue(home, "blocked")
    mergeq_mod.enqueue(home, "behind")
    (repo / "README.md").write_text("edited\n", encoding="utf-8")
    rows = mergeq_mod.drain(home, load_config(home), relaunch=Sender())
    assert [(r["lane"], r["result"]) for r in rows] == [("blocked", "waiting"), ("behind", "merged")]
    assert [i["lane"] for i in mergeq_mod.view(home, "demo")["items"]] == ["blocked"]
    rows = mergeq_mod.drain(home, load_config(home), relaunch=Sender())  # next pass: tried once, not in a loop
    assert [(r["lane"], r["result"]) for r in rows] == [("blocked", "waiting")]
    git(repo, "checkout", "--", "README.md")
    assert [r["result"] for r in mergeq_mod.drain(home, load_config(home), relaunch=Sender())] == ["merged"]


def test_the_wip_limit_ignores_lanes_waiting_on_the_owners_dirty_checkout(home: Path) -> None:
    rows = [{"lane": f"q{i}", "project": "demo", "status": "queued"} for i in range(2)]
    rows += [{"lane": "dirty", "project": "demo", "status": "queued",
              "message": f"/x/demo {mergeq_mod.DIRTY_WAIT} a.swift, which the merge changes"}]
    (home / "mergeq.json").write_text(json.dumps({"queue": rows}), encoding="utf-8")
    assert mergeq_mod.wip_block(home) == ""
    rows[2].pop("message")
    (home / "mergeq.json").write_text(json.dumps({"queue": rows}), encoding="utf-8")
    assert mergeq_mod.wip_block(home)
