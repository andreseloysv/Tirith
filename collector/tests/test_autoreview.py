"""Automatic review: a finished launched lane is tested and merged by the dispatcher, never forgotten."""

from __future__ import annotations

import subprocess
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest

from conftest import make_repo, write_config
from tirith import asks as asks_mod
from tirith import autoreview as autoreview_mod
from tirith import dispatch as dispatch_mod
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith import review as review_mod
from tirith import roadmap as roadmap_mod
from tirith.cli import end_lane, main, register_lane
from tirith.home import load_config
from tirith.store import lane_path, read_json, read_jsonl

PLAN = "# Plan\n\n## 2. Web panel\n\n- [ ] Add the feature file.\n- [ ] Document the panel.\n"
MANIFEST = ('deploy = "none"\n[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "build"\nrun = "true"\n'
            '[[pipelines.stages]]\nname = "test"\nrun = "echo 3 passed; test -f feature.txt"\n')
REPORT = "## What changed\n- Added the feature file `feature.txt`\n\n## Tests\n3 passed\n"


def git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True, check=True).stdout


class Spawner:
    """Stands in for the detached ``tirith lane review``: runs it in-process and remembers the argv."""

    def __init__(self, home: Path, repo: Path, run: bool = True) -> None:
        self.home, self.repo, self.run, self.calls = home, repo, run, []

    def __call__(self, argv, home, log) -> int:
        self.calls.append(list(argv))
        if self.run:
            review_mod.review_lane(self.home, argv[argv.index("review") + 1], self.repo,
                                   merge="--tests-only" not in argv)
        return 1


def _lane(home: Path, repo: Path, ok: bool = True, request_id: str | None = None):
    worktree = launch_mod.create_worktree(repo, "web-panel")
    brief = home / "brief.md"
    brief.write_text(launch_mod.build_brief("Web panel", "", ["Add the feature file.", "Document the panel."], "2"),
                     encoding="utf-8")
    register_lane(home, lane_id="web-panel", project="demo", hero_class="glasswright", feature="Web panel",
                  workspace=str(worktree), log=None, pid=None, brief=str(brief), step="2", request_id=request_id)
    if ok:
        (worktree / "feature.txt").write_text("built\n", encoding="utf-8")
    else:  # some work, but not what the tests look for
        (worktree / "other.txt").write_text("wip\n", encoding="utf-8")
    return end_lane(home, "web-panel", "done", REPORT)


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
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}])
    return repo


def cycle(home: Path, spawn, now: datetime | None = None, notified: list | None = None) -> None:
    dispatch_mod.run(home, load_config(home), now or datetime.now(timezone.utc), agents=[], lanes=_lanes(home),
                     status_of=lambda lane, when: "done", spawn=spawn,
                     notify=(lambda title, text: notified.append(text)) if notified is not None else (lambda *a: None))


def _lanes(home: Path) -> list[dict]:
    from tirith.store import read_lanes
    return read_lanes(home)


def owner_items(home: Path) -> list[dict]:
    return read_jsonl(home / "waits.jsonl")


# -- 1. the automatic review -------------------------------------------------------------------


def test_a_passing_lane_is_merged_once_and_the_owner_is_told(env, home: Path) -> None:
    repo = env
    rid = inbox_mod.add_request(home, "finish", "demo", "2")["id"]
    _lane(home, repo, request_id=rid)
    spawn = Spawner(home, repo)
    for _ in range(3):  # idempotent across cycles
        cycle(home, spawn)
    assert len(spawn.calls) == 1 and spawn.calls[0][-3:] == ["lane", "review", "web-panel"]
    assert (repo / "feature.txt").is_file() and not (launch_mod.worktree_path(repo, "web-panel")).exists()
    answers = inbox_mod.find_request(home, rid)["answers"]
    text = [a["text"] for a in answers if a["by"] == "tirith"][-1]
    branch = git(repo, "rev-parse", "--abbrev-ref", "HEAD").strip()
    assert text.startswith(f"Merged web-panel into {branch}: ") and "files, tests 3 passed" in text
    assert [q for q in asks_mod.queue_items(home, only_open=True) if q["kind"] != "launch-refused"] == []
    (done,) = [i for i in asks_mod.queue_items(home) if i["status"] == "done"]
    assert "Merged web-panel" in done["result"]
    assert read_json(home / "dispatch.json", {})["reviews"]["web-panel"]["settled"]


def test_the_review_is_a_visible_pipeline_run_tagged_with_the_lane(env, home: Path) -> None:
    _lane(home, env)
    cycle(home, Spawner(home, env))
    lane = read_json(lane_path(home, "web-panel"), None)
    runs = [e for e in read_jsonl(home / "events.jsonl") if e.get("lane") == "web-panel" and e.get("run")]
    assert runs and {e["run"] for e in runs} == {lane["review"]["run"]}
    assert review_mod.public(lane, env)["run"] == lane["review"]["run"]


def test_failing_tests_do_not_merge_and_go_to_the_manager(env, home: Path) -> None:
    repo = env
    _lane(home, repo, ok=False)
    spawn = Spawner(home, repo)
    cycle(home, spawn)
    cycle(home, spawn)
    assert not (repo / "feature.txt").exists() and launch_mod.worktree_path(repo, "web-panel").is_dir()
    assert not [i for i in owner_items(home) if i["id"].startswith("review-web-panel")]  # not the owner's job
    assert len(spawn.calls) == 1
    (queued,) = asks_mod.queue_items(home, only_open=True)
    assert queued["lane"] == "web-panel"
    assert "tirith lane review web-panel" in queued["text"] and "tirith lane drop web-panel" in queued["text"]


def test_a_retry_that_passes_is_settled_too(env, home: Path) -> None:
    repo = env
    _lane(home, repo, ok=False)
    spawn = Spawner(home, repo)
    cycle(home, spawn)
    cycle(home, spawn)
    (launch_mod.worktree_path(repo, "web-panel") / "feature.txt").write_text("fixed\n", encoding="utf-8")
    assert main(["lane", "review", "web-panel"]) == 0  # the owner's "Retry the review"
    cycle(home, spawn)
    assert asks_mod.queue_items(home, only_open=True) == []
    assert not [r for r in owner_items(home) if r["id"].startswith("review-web-panel")]


def test_merging_is_never_an_owner_question(env, home: Path) -> None:
    repo = env
    with (home / "config.toml").open("a") as handle:
        handle.write("\n[review]\nauto_merge = false\n")  # no longer a switch: a passing lane always merges
    _lane(home, repo)
    spawn = Spawner(home, repo)
    cycle(home, spawn)
    cycle(home, spawn)
    assert "--tests-only" not in spawn.calls[0]
    assert (repo / "feature.txt").exists()
    assert not [i for i in owner_items(home) if i["id"].startswith("review-web-panel")]


def test_a_lane_left_ready_by_a_tests_only_run_is_merged(env, home: Path) -> None:
    repo = env
    _lane(home, repo)
    assert main(["lane", "review", "web-panel", "--tests-only"]) == 0
    spawn = Spawner(home, repo)
    cycle(home, spawn)
    cycle(home, spawn)
    assert (repo / "feature.txt").exists() and "--tests-only" not in spawn.calls[0]
    assert owner_items(home) == []


def test_a_ready_result_that_settles_goes_to_the_manager_queue(env, home: Path) -> None:
    repo = env
    _lane(home, repo)
    assert main(["lane", "review", "web-panel", "--tests-only"]) == 0
    lane = read_json(lane_path(home, "web-panel"), None)
    review = review_mod.public(lane, repo)
    autoreview_mod.settle(home, load_config(home), lane, review, {}, datetime.now(timezone.utc), repo)
    assert owner_items(home) == []
    (queued,) = asks_mod.queue_items(home, only_open=True)
    assert "not merged" in queued["text"] and "tirith lane review web-panel" in queued["text"]


def test_drop_removes_worktree_and_branch(env, home: Path, capsys) -> None:
    repo = env
    _lane(home, repo, ok=False)
    assert main(["lane", "drop", "web-panel"]) == 0
    assert not launch_mod.worktree_path(repo, "web-panel").exists()
    assert "lane/web-panel" not in git(repo, "branch", "--list")
    assert read_json(lane_path(home, "web-panel"), None)["review"]["status"] == "dropped"


def test_lanes_that_were_not_launched_or_failed_are_ignored(env, home: Path) -> None:
    register_lane(home, lane_id="plain", project="demo", hero_class="x", feature="Plain",
                  workspace=str(env), log=None, pid=None, brief=None)
    end_lane(home, "plain", "done", "ok")
    spawn = Spawner(home, env)
    cycle(home, spawn)
    assert spawn.calls == []


# -- 2. the roadmap ----------------------------------------------------------------------------


def test_a_merge_ticks_what_the_report_claims_and_commits_the_plan(env, home: Path) -> None:
    repo = env
    _lane(home, repo)
    cycle(home, Spawner(home, env))
    cycle(home, Spawner(home, env))
    todo = (repo / "docs" / "TODO.md").read_text()
    assert "- [x] Add the feature file." in todo and "- [ ] Document the panel." in todo
    assert git(repo, "log", "-1", "--format=%s").strip() == "Roadmap: step 2 progress from web-panel"
    assert git(repo, "status", "--porcelain").strip() == ""
    assert "Roadmap" not in " ".join(i["text"] for i in asks_mod.queue_items(home))


def test_an_unsure_report_ticks_nothing_and_asks_the_steward(env, home: Path) -> None:
    repo = env
    _lane(home, repo)
    lane = read_json(lane_path(home, "web-panel"), None)
    lane["report"] = "Done, everything works."
    (home / "lanes" / "web-panel.json").write_text(__import__("json").dumps(lane), encoding="utf-8")
    cycle(home, Spawner(home, env))
    cycle(home, Spawner(home, env))
    assert "- [ ] Add the feature file." in (repo / "docs" / "TODO.md").read_text()
    (plan,) = [i for i in asks_mod.queue_items(home, only_open=True) if i["kind"] == "plan"]
    assert plan["text"].startswith("update the plan for step 2") and "Add the feature file." in plan["text"]


def test_brief_items_and_matching() -> None:
    brief = launch_mod.build_brief("T", "intro", ["Add the feature file.", "Document the panel."], "2")
    assert brief_items_of(brief) == ["Add the feature file.", "Document the panel."]
    report = "Summary\n\n**What changed**\n- feature file added\n- panel docs\n\n**Open**\nnothing\n"
    assert "feature file added" in autoreview_mod.changed_lines(report) and "nothing" not in autoreview_mod.changed_lines(report)
    ticked, candidates = autoreview_mod.match_items(["Add the feature file.", "Document the panel.", "Ship it"], report)
    assert ticked == ["Add the feature file."] and candidates == ["Document the panel."]
    assert autoreview_mod.match_items(["Add the feature file."], "no section here") == ([], ["Add the feature file."])
    assert autoreview_mod.changed_lines("What changed: the feature file\nOpen: none") == "the feature file"


def brief_items_of(brief: str) -> list[str]:
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        path = Path(tmp) / "brief.md"
        path.write_text(brief, encoding="utf-8")
        return autoreview_mod.brief_items(str(path))


def test_tick_changes_only_the_open_items_of_that_step() -> None:
    text = ("# P\n\n## 1. One\n\n- [ ] Same text.\n\n## 2. Two\n\n- [ ] Same text.\n- [ ] Wrapped item\n  continues here.\n"
            "- [x] Done already.\n")
    new, done = roadmap_mod.tick(text, "2", {"Same text.", "Wrapped item continues here."})
    assert done == ["Same text.", "Wrapped item continues here."]
    assert new == text.replace("## 2. Two\n\n- [ ] Same text.\n- [ ] Wrapped", "## 2. Two\n\n- [x] Same text.\n- [x] Wrapped")


# -- 3. the steward's Stop ---------------------------------------------------------------------


def _queue(home: Path, at: datetime) -> dict:
    row = asks_mod.queue_add(home, "demo", "web-panel", "Review lane web-panel")
    rows = read_jsonl(asks_mod.queue_path(home))
    rows[-1]["at"] = at.strftime("%Y-%m-%dT%H:%M:%SZ")
    asks_mod.queue_path(home).write_text("".join(__import__("json").dumps(r) + "\n" for r in rows), encoding="utf-8")
    return row


def test_stop_is_blocked_by_an_old_review_item_three_times_then_a_violation(home: Path) -> None:
    now = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)
    item = _queue(home, now - timedelta(seconds=90))
    for block in (1, 2, 3):
        reason = dispatch_mod.manager_stop(home, now)
        assert f"block {block} of 3" in reason
        assert "tirith lane review web-panel" in reason and f"tirith queue done {item['id']}" in reason
    assert dispatch_mod.manager_stop(home, now) is None
    assert [e["rule"] for e in read_jsonl(home / "events.jsonl") if e.get("kind") == "violation"] == ["inbox-ignored"]


def test_a_young_or_closed_review_item_does_not_block(home: Path) -> None:
    now = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)
    item = _queue(home, now - timedelta(seconds=20))
    assert dispatch_mod.manager_stop(home, now) is None
    later = now + timedelta(seconds=120)
    asks_mod.queue_done(home, item["id"], "merged")
    assert dispatch_mod.manager_stop(home, later) is None


# -- 4. the deadline ---------------------------------------------------------------------------


def test_a_lane_unreviewed_for_half_an_hour_is_a_violation_once(env, home: Path) -> None:
    repo = env
    _lane(home, repo)
    dead = Spawner(home, repo, run=False)  # the detached review never produced a result
    notified: list[str] = []
    cycle(home, dead, notified=notified)
    assert notified == [] and dead.calls
    late = datetime.now(timezone.utc) + timedelta(minutes=31)
    for _ in range(2):
        cycle(home, dead, now=late, notified=notified)
    events = [e for e in read_jsonl(home / "events.jsonl") if e.get("rule") == "review-overdue"]
    assert len(events) == 1 and events[0]["lane"] == "web-panel"
    assert len(notified) == 1 and "web-panel" in notified[0]
    assert not [i for i in owner_items(home) if i["id"] == "review-overdue-web-panel"]  # the manager's job
    assert [i for i in asks_mod.queue_items(home, only_open=True) if "unmerged work" in i["text"]]


def test_a_merged_lane_is_never_overdue(env, home: Path) -> None:
    _lane(home, env)
    cycle(home, Spawner(home, env))
    notified: list[str] = []
    cycle(home, Spawner(home, env), now=datetime.now(timezone.utc) + timedelta(hours=2), notified=notified)
    assert notified == [] and not [e for e in read_jsonl(home / "events.jsonl") if e.get("rule") == "review-overdue"]


def test_a_lane_already_merged_by_hand_is_settled_not_reviewed(env, home: Path) -> None:
    repo = env
    worktree = launch_mod.create_worktree(repo, "web-panel")
    register_lane(home, lane_id="web-panel", project="demo", hero_class="glasswright", feature="Web panel",
                  workspace=str(worktree), log=None, pid=None, brief=None, step="2")
    (worktree / "feature.txt").write_text("built\n", encoding="utf-8")
    git(worktree, "add", ".")
    git(worktree, "commit", "-q", "-m", "feature")
    git(repo, "merge", "-q", "--ff-only", "lane/web-panel")  # the owner merged it by hand
    end_lane(home, "web-panel", "done", REPORT)
    spawn = Spawner(home, repo)
    for _ in range(3):
        cycle(home, spawn)
    assert spawn.calls == [] and asks_mod.queue_items(home) == []
    assert read_json(home / "dispatch.json", {})["reviews"]["web-panel"]["settled"] == "merged"
