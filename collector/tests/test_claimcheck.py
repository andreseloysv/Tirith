"""claimcheck: a lane's report is verified against its real change (temp git repos, no model calls)."""

from __future__ import annotations

import json
import os
import subprocess
import time
from pathlib import Path

import pytest

from conftest import make_repo, write_config
from tirith import autoreview as autoreview_mod
from tirith import claimcheck as cc
from tirith import launch as launch_mod
from tirith import review as review_mod
from tirith.cli import end_lane, main, register_lane
from tirith.home import load_config, now_iso
from tirith.store import lane_path, read_json, write_lane


def git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True, check=True).stdout


@pytest.fixture
def work(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    for key in ("AUTHOR", "COMMITTER"):
        monkeypatch.setenv(f"GIT_{key}_NAME", "Tirith Test")
        monkeypatch.setenv(f"GIT_{key}_EMAIL", "t@example.invalid")
    repo = make_repo(tmp_path / "demo")
    (repo / "tests").mkdir()
    (repo / "tests" / "test_old.py").write_text("def test_old():\n    pass\n", encoding="utf-8")
    git(repo, "add", ".")
    git(repo, "commit", "-q", "-m", "tests")
    return repo


def lane_of(repo: Path, **extra) -> dict:
    return {"id": "x", "workspace": str(repo), "start_commit": git(repo, "rev-parse", "HEAD").strip(),
            "started_at": "2000-01-01T00:00:00+00:00", **extra}


def add_test(repo: Path, name: str = "test_new", file: str = "tests/test_old.py") -> None:
    with (repo / file).open("a", encoding="utf-8") as fh:
        fh.write(f"\n\ndef {name}():\n    pass\n")


def bad(result: cc.Result) -> str:
    return "\n".join(result.mismatches)


def record_run(home: Path, repo: Path, passed: int, status: str = "passed", at: float | None = None) -> None:
    log = home / "logs" / "r.log"
    log.parent.mkdir(parents=True, exist_ok=True)
    log.write_text(f"{passed} passed in 1s\n", encoding="utf-8")
    stamp = time.strftime("%Y-%m-%dT%H:%M:%S+00:00", time.gmtime(at or time.time() + 5))
    base = {"run": "ci#1", "stage": "test", "worktree": str(repo), "cwd": str(repo), "log": str(log), "at": stamp}
    with (home / "events.jsonl").open("a", encoding="utf-8") as fh:
        fh.write(json.dumps({**base, "event": "start"}) + "\n")
        fh.write(json.dumps({**base, "event": "end", "status": status}) + "\n")


# -- files ---------------------------------------------------------------------------------------


def test_changed_file_true_and_false(work, home: Path) -> None:
    add_test(work)
    lane = lane_of(work)
    assert cc.verify(home, lane, report="Changed `tests/test_old.py`.").ok
    result = cc.verify(home, lane, report="Added a test in test_repo_bin.py and changed `tests/test_old.py`.")
    assert "test_repo_bin.py" in bad(result) and "tests/test_old.py" not in bad(result)


def test_new_untracked_file_counts_and_negations_are_skipped(work, home: Path) -> None:
    (work / "claimcheck.py").write_text("x = 1\n", encoding="utf-8")
    lane = lane_of(work)
    assert cc.verify(home, lane, report="Created collector/tirith/claimcheck.py").ok
    assert cc.verify(home, lane, report="I did not change docs/TODO.md.").ok


# -- tests added ---------------------------------------------------------------------------------


def test_added_test_names_and_counts(work, home: Path) -> None:
    add_test(work, "test_new")
    lane = lane_of(work)
    assert cc.verify(home, lane, report="Added 1 test: test_new in tests/test_old.py").ok
    result = cc.verify(home, lane, report="Added test_missing in tests/test_old.py")
    assert "test_missing" in bad(result) and "no such test" in bad(result)
    result = cc.verify(home, lane, report="Added three new tests")
    assert "added 3 test(s)" in bad(result) and "1 test function" in bad(result)
    assert not cc.verify(home, lane, report="Added test_new in tests/test_other.py").ok  # wrong file


def test_added_test_js_and_rust(work, home: Path) -> None:
    (work / "a.test.mjs").write_text("test('shows the panel', () => {});\n", encoding="utf-8")
    (work / "lib.rs").write_text("#[test]\nfn parses() {}\nfn helper() {}\n", encoding="utf-8")
    result = cc.verify(home, lane_of(work), report="Added 2 tests")
    assert result.ok
    assert not cc.verify(home, lane_of(work), report="Added 3 tests").ok


# -- tests pass ----------------------------------------------------------------------------------


def test_tests_pass_needs_a_recorded_run_with_that_count(work, home: Path) -> None:
    add_test(work)
    lane = lane_of(work)
    assert "no passing test run" in bad(cc.verify(home, lane, report="All tests pass."))
    record_run(home, work, 12)
    assert cc.verify(home, lane, report="Tests: 12 passed").ok
    assert "13 passed" in bad(cc.verify(home, lane, report="13 passed")) and "12" in bad(
        cc.verify(home, lane, report="13 passed"))


def test_a_run_before_the_last_change_or_a_failed_run_does_not_count(work, home: Path) -> None:
    add_test(work)
    lane = lane_of(work)
    record_run(home, work, 5, at=time.time() - 3600)
    record_run(home, work, 5, status="failed")
    assert not cc.verify(home, lane, report="tests pass").ok


# -- git -----------------------------------------------------------------------------------------


def test_commit_sha(work, home: Path) -> None:
    add_test(work)
    git(work, "commit", "-qam", "more")
    head = git(work, "rev-parse", "--short", "HEAD").strip()
    lane = lane_of(work)
    assert cc.verify(home, lane, report=f"Committed as {head}").ok
    assert "d3adbee" in bad(cc.verify(home, lane, report="Committed as d3adbee"))


def test_merged_and_pushed(work, home: Path, tmp_path: Path) -> None:
    repo = work
    lane_dir = launch_mod.create_worktree(repo, "feat")
    (lane_dir / "f.txt").write_text("x\n", encoding="utf-8")
    git(lane_dir, "add", "-A")
    git(lane_dir, "commit", "-qm", "f")
    lane = lane_of(lane_dir, id="feat", start_commit=git(repo, "rev-parse", "HEAD").strip())
    assert "merged" in bad(cc.verify(home, lane, repo, report="Merged to main."))
    assert cc.verify(home, lane, repo, report="Merged main into my branch; ready to merge.").ok
    git(repo, "merge", "--no-ff", "-qm", "m", "lane/feat")
    assert cc.verify(home, lane, repo, report="Merged to main.").ok
    assert "pushed" in bad(cc.verify(home, lane, repo, report="Pushed to origin."))
    remote = tmp_path / "remote.git"
    subprocess.run(["git", "init", "-q", "--template=", "--bare", str(remote)], check=True)
    git(repo, "remote", "add", "origin", str(remote))
    git(repo, "push", "-q", "origin", "HEAD")
    git(repo, "fetch", "-q", "origin")
    assert cc.verify(home, lane, repo, report="Pushed to origin.").ok


# -- screenshots ---------------------------------------------------------------------------------


def test_screenshot_exists_and_is_newer_than_the_lane(work, home: Path, tmp_path: Path) -> None:
    shot = tmp_path / "shot.png"
    shot.write_bytes(b"png")
    lane = lane_of(work)
    assert cc.verify(home, lane, report=f"Looked at the screenshot {shot}").ok
    assert "does not exist" in bad(cc.verify(home, lane, report=f"Looked at screenshot {tmp_path}/nope.png"))
    os.utime(shot, (1, 1))
    assert "older" in bad(cc.verify(home, lane, report=f"Screenshot looked at {shot}"))


# -- the brief's fix items -----------------------------------------------------------------------


def test_brief_fix_with_docs_only_diff_is_flagged(work, home: Path) -> None:
    (work / "docs.md").write_text("explained\n", encoding="utf-8")
    lane = lane_of(work)
    brief = "## Build\n- Fix the crash when the lane has no report\n- Do not touch docs/TODO.md\n"
    result = cc.verify(home, lane, report="Explained why it crashes.", brief=brief)
    assert "explaining or documenting is not fixing" in bad(result)
    (work / "code.py").write_text("x = 1\n", encoding="utf-8")
    assert cc.verify(home, lane, report="Fixed it.", brief=brief).ok
    assert cc.verify(home, lane_of(work), report="x", brief="- Fix the typo in the docs\n").ok  # a docs item


# -- review and autoreview -----------------------------------------------------------------------

MANIFEST = ('deploy = "none"\n[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "build"\nrun = "true"\n'
            '[[pipelines.stages]]\nname = "test"\nrun = "test -f feature.txt"\n')


@pytest.fixture
def launched(home: Path, work: Path):
    repo = work
    (repo / "tirith.toml").write_text(MANIFEST, encoding="utf-8")
    git(repo, "add", ".")
    git(repo, "commit", "-q", "-m", "manifest")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}],
                 extra='[agents.worker]\nlaunch = "true {brief}"\n')
    brief = home / "brief.md"
    brief.write_text("## Build\n- Do the thing\n", encoding="utf-8")
    tree = launch_mod.create_worktree(repo, "panel")
    register_lane(home, lane_id="panel", project="demo", hero_class="glasswright", feature="Panel",
                  workspace=str(tree), log=None, pid=None, brief=str(brief), step="2")
    lane = read_json(lane_path(home, "panel"), None)
    lane["agent"] = "worker"
    write_lane(home, lane)
    (tree / "feature.txt").write_text("built\n", encoding="utf-8")
    return repo, tree


def test_review_with_matching_report_merges_and_records_claims(launched, home: Path, capsys) -> None:
    repo, tree = launched
    end_lane(home, "panel", "done", "Added feature.txt. Tests: 1 passed.")
    assert main(["lane", "review", "panel"]) == 0
    lane = read_json(lane_path(home, "panel"), None)
    assert lane["review"]["status"] == "merged"
    assert lane["review"]["claims"]["ok"] is True and lane["review"]["claims"]["checked"] >= 2


def test_review_with_a_false_report_is_claims_failed_and_not_merged(launched, home: Path, capsys) -> None:
    repo, tree = launched
    end_lane(home, "panel", "done", "Changed `test_repo_bin.py` and added test_x in test_repo_bin.py.")
    assert main(["lane", "review", "panel"]) == 1
    out = capsys.readouterr().out
    assert "claims-failed" in out and "no such test in the diff" in out
    lane = read_json(lane_path(home, "panel"), None)
    assert lane["review"]["status"] == "claims-failed" and not lane["review"]["claims"]["ok"]
    assert "feature.txt" not in git(repo, "ls-files")
    public = review_mod.public(lane, repo)
    assert public["claims"]["mismatches"] and public["status"] == "claims-failed"


def test_claims_failed_goes_back_once_then_to_the_manager(launched, home: Path, monkeypatch) -> None:
    repo, tree = launched
    end_lane(home, "panel", "done", "Added test_x in test_repo_bin.py.")
    main(["lane", "review", "panel"])
    sent: list = []

    def fake(home_, lane, alias, note):
        sent.append((lane["id"], alias, note))
        return "panel-2"

    monkeypatch.setattr(autoreview_mod, "relauncher", fake)
    config = load_config(home)
    lane = read_json(lane_path(home, "panel"), None)
    rec: dict = {}
    autoreview_mod.settle(home, config, lane, lane["review"], rec, __import__("datetime").datetime.now().astimezone(),
                          repo)
    assert sent and sent[0][1] == "worker" and "test_x" in sent[0][2] and rec["claims_sent"] == "panel-2"
    from tirith import asks as asks_mod
    assert not [i for i in asks_mod.queue_items(home, only_open=True) if "does not match" in i["text"]]
    # the sent-back brief carries the mark: a second failure is the manager's
    (home / "brief.md").write_text("## Build\n" + cc.MARK + "\n- x\n", encoding="utf-8")
    sent.clear()
    autoreview_mod.settle(home, config, lane, lane["review"], {}, __import__("datetime").datetime.now().astimezone(),
                          repo)
    assert not sent
    assert [i for i in asks_mod.queue_items(home, only_open=True) if "does not match" in i["text"]]


def test_relaunch_with_a_note_continues_the_work(launched, home: Path, monkeypatch) -> None:
    repo, tree = launched
    end_lane(home, "panel", "done", "report")
    from tirith import cli as cli_mod
    spawned: list = []
    monkeypatch.setattr(launch_mod, "spawn_detached", lambda argv, h, log: spawned.append(argv) or 1)
    lane = read_json(lane_path(home, "panel"), None)
    new_id = cli_mod.relaunch_lane(home, lane, "worker", by_tirith=True, note=cc.MARK + "\n- x\n")
    new_tree = launch_mod.worktree_path(repo, new_id)
    assert (new_tree / "feature.txt").exists()  # the old work is the new lane's starting point
    brief = argv_value(spawned[0], "--brief")
    assert cc.MARK in Path(brief).read_text(encoding="utf-8") and "Do the thing" in Path(brief).read_text()
    assert argv_value(spawned[0], "--fallback-from") == "panel"


def argv_value(argv: list[str], flag: str) -> str:
    return argv[argv.index(flag) + 1]


def test_verify_follows_the_chain_to_the_first_lane(launched, home: Path) -> None:
    repo, tree = launched
    lane = read_json(lane_path(home, "panel"), None)
    nxt = {"id": "panel-2", "fallback_lane": "panel", "workspace": str(tree), "start_commit": "HEAD",
           "started_at": now_iso()}
    assert [l["id"] for l in cc.chain(home, nxt)] == ["panel-2", "panel"]
    assert lane["start_commit"]
