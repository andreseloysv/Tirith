"""Every build and test run is a recorded pipeline run: review, `tirith run` in a lane worktree, the gate."""

from __future__ import annotations

import io
import json
import os
import subprocess
from pathlib import Path

import pytest

from conftest import make_repo, write_config
from tirith import launch as launch_mod
from tirith import runstage
from tirith.cli import end_lane, main, register_lane
from tirith.collector import Collector
from tirith.store import read_json, read_jsonl

MANIFEST = ('[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "build"\nrun = "true"\n'
            '[[pipelines.stages]]\nname = "test"\nrun = "{test}"\n')


def git(repo: Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True, check=True).stdout


@pytest.fixture
def lane(home: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    for key in ("AUTHOR", "COMMITTER"):
        monkeypatch.setenv(f"GIT_{key}_NAME", "Tirith Test")
        monkeypatch.setenv(f"GIT_{key}_EMAIL", "t@example.invalid")
    repo = make_repo(tmp_path / "demo")
    (repo / "tirith.toml").write_text(MANIFEST.format(test="pwd > where.txt && test -f feature.txt"),
                                      encoding="utf-8")
    git(repo, "add", ".")
    git(repo, "commit", "-q", "-m", "manifest")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}])
    worktree = launch_mod.create_worktree(repo, "web-panel")
    register_lane(home, lane_id="web-panel", project="demo", hero_class="glasswright", feature="Web panel",
                  workspace=str(worktree), log=None, pid=None, brief=None, step="2")
    (worktree / "feature.txt").write_text("built\n", encoding="utf-8")
    return repo, worktree


def events(home: Path) -> list[dict]:
    return [e for e in read_jsonl(home / "events.jsonl") if e.get("event") in ("start", "end")]


# -- tirith run in a lane worktree -----------------------------------------------------------


def test_run_in_a_worktree_uses_the_worktree_and_tags_the_lane(lane, home, monkeypatch) -> None:
    repo, worktree = lane
    monkeypatch.chdir(worktree)
    assert main(["run", "--project", "demo", "--stage", "test", "--full"]) == 0  # a lane's plain test run is changed-only
    assert os.path.realpath((worktree / "where.txt").read_text().strip()) == os.path.realpath(worktree)
    assert not (repo / "where.txt").exists()
    start, end = events(home)
    for row in (start, end):
        assert row["lane"] == "web-panel" and row["branch"] == "lane/web-panel"
        assert os.path.realpath(row["worktree"]) == os.path.realpath(worktree)
        assert row["project"] == "demo" and row["stage"] == "test"
    assert (start["event"], end["event"], end["status"]) == ("start", "end", "passed")


def test_run_in_the_main_repo_is_not_tagged(lane, home, monkeypatch) -> None:
    repo, worktree = lane
    (repo / "feature.txt").write_text("x\n", encoding="utf-8")
    monkeypatch.chdir(repo)
    monkeypatch.delenv("TIRITH_LANE", raising=False)
    assert main(["run", "--project", "demo", "--stage", "test"]) == 0
    assert os.path.realpath((repo / "where.txt").read_text().strip()) == os.path.realpath(repo)
    assert all("lane" not in row and "branch" not in row for row in events(home))


def test_collector_exposes_the_lane_of_a_run(lane, home, monkeypatch) -> None:
    _, worktree = lane
    monkeypatch.chdir(worktree)
    main(["run", "--project", "demo", "--stage", "test"])
    assert main(["collect", "--once"]) == 0
    state = read_json(home / "state.json", {})
    run = next(p for p in state["pipelines"] if p.get("lane"))
    assert [s["status"] for s in run["stages"] if s["name"] == "test"] == ["passed"]
    assert run["lane"] == "web-panel" and run["branch"] == "lane/web-panel"


def test_run_stage_records_a_failure_and_returns_the_output(home, tmp_path) -> None:
    code, output = runstage.run_stage(home, "p", "ci", "test", "echo boom; exit 3", tmp_path, echo=False)
    assert code == 3 and "boom" in output
    assert [e["status"] for e in events(home)] == ["running", "failed"]
    assert (home / "logs").is_dir()


def test_run_stage_timeout_ends_the_run_as_failed(home, tmp_path) -> None:
    code, output = runstage.run_stage(home, "p", "ci", "test", "sleep 5", tmp_path, echo=False, timeout=0.3)
    assert code != 0 and "timed out after" in output
    assert events(home)[-1]["status"] == "failed"


# -- tirith lane review ----------------------------------------------------------------------


def test_review_is_a_visible_pipeline_run_tagged_with_the_lane(lane, home) -> None:
    repo, worktree = lane
    end_lane(home, "web-panel", "done", "built it")
    assert main(["lane", "review", "web-panel", "--tests-only"]) == 0
    start, end = events(home)
    assert (start["event"], end["event"], end["status"]) == ("start", "end", "passed")
    assert end["project"] == "demo" and end["pipeline"] == "ci" and end["stage"] == "test"
    assert end["lane"] == "web-panel" and end["branch"] == "lane/web-panel"
    assert os.path.realpath(end["cwd"]) == os.path.realpath(worktree)
    assert Path(end["log"]).is_file()
    main(["collect", "--once"])
    runs = [p for p in read_json(home / "state.json", {})["pipelines"] if p.get("lane") == "web-panel"]
    assert len(runs) == 1
    assert [s["status"] for s in runs[0]["stages"] if s["name"] == "test"] == ["passed"]


def test_failed_review_run_shows_as_failed(lane, home) -> None:
    _, worktree = lane
    (worktree / "feature.txt").unlink()
    end_lane(home, "web-panel", "done", "built it")
    assert main(["lane", "review", "web-panel", "--tests-only"]) == 1
    assert events(home)[-1]["status"] == "failed"


# -- the gate --------------------------------------------------------------------------------


@pytest.fixture
def proj(home: Path, tmp_path: Path) -> Path:
    folder = tmp_path / "proj"
    folder.mkdir()
    (folder / "tirith.toml").write_text(
        '[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "compile"\nkind = "build"\nrun = "echo c"\n'
        '[[pipelines.stages]]\nname = "check"\nkind = "test"\nrun = "echo t"\n', encoding="utf-8")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(folder)}])
    return folder


def bash(monkeypatch, cwd: Path, command: str) -> int:
    payload = {"hook_event_name": "PreToolUse", "session_id": "s1", "cwd": str(cwd),
               "tool_name": "Bash", "tool_input": {"command": command}}
    monkeypatch.setattr("sys.stdin", io.StringIO(json.dumps(payload)))
    return main(["hook", "--agent", "anthropic"])


@pytest.mark.parametrize("command,stage", [
    ("pytest -q", "check"), ("uv run -q pytest tests/test_x.py", "check"), ("cd x && npm test", "check"),
    ("pnpm run test:unit", "check"), ("yarn test", "check"), ("vitest run", "check"), ("npx jest", "check"),
    ("go test ./...", "check"), ("cargo test", "check"), ("./gradlew testDebug", "check"),
    ("make check", "check"), ("xcodebuild test -scheme A", "check"), ("swift test", "check"),
    ("cargo build --release", "compile"), ("go build ./...", "compile"), ("swift build", "compile"),
    ("make build", "compile"), ("./gradlew assembleRelease", "compile"), ("npm run build", "compile"),
    ("xcodebuild -scheme A build", "compile"), ("node --check web/js/ui.js", "compile"),
])
def test_raw_runners_are_blocked_with_the_matching_stage(proj, monkeypatch, capsys, command, stage) -> None:
    assert bash(monkeypatch, proj, command) == 2
    err = capsys.readouterr().err.strip()
    assert err.startswith("tirith: raw-stage: ") and "must run as a pipeline stage so the owner sees it" in err
    assert err.endswith(f"Do this instead: tirith run --project demo --stage {stage} [-- <cmd>]")


def test_runner_inside_a_subfolder_is_blocked(proj, monkeypatch) -> None:
    (proj / "sub").mkdir()
    assert bash(monkeypatch, proj / "sub", "pytest") == 2


@pytest.mark.parametrize("command", [
    "tirith run --project demo --stage check", "tirith run --project demo --stage check -- pytest -q",
    "tirith run --stage compile -- cargo build", "ls", "echo pytest", "cat > t.sh <<'EOF'\npytest\nnpm test\nEOF",
    "git status", "python3 script.py", "make buildall", "cargo fmt",
])
def test_wrapped_or_unrelated_commands_pass(proj, monkeypatch, capsys, command) -> None:
    assert bash(monkeypatch, proj, command) == 0
    assert capsys.readouterr().err == ""


def test_command_after_a_heredoc_is_still_checked(proj, monkeypatch) -> None:
    assert bash(monkeypatch, proj, "cat > t.sh <<'EOF'\nnpm test\nEOF\npytest -q") == 2


def test_no_matching_stage_suggests_adding_one(home, tmp_path, monkeypatch, capsys) -> None:
    folder = tmp_path / "bare"
    folder.mkdir()
    (folder / "tirith.toml").write_text('[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "docs"\n'
                                        'run = "echo l"\n', encoding="utf-8")
    write_config(home, [{"id": "bare", "name": "Bare", "repo": str(folder)}])
    assert bash(monkeypatch, folder, "cargo test") == 2
    err = capsys.readouterr().err
    assert "tirith: raw-stage: cargo test must run as a pipeline stage" in err
    assert "tirith setup add-stage" in err


def test_folders_without_a_manifest_are_not_gated(home, tmp_path, monkeypatch) -> None:
    folder = tmp_path / "plain"
    folder.mkdir()
    assert bash(monkeypatch, folder, "pytest -q") == 0


def test_exact_stage_match_keeps_its_own_message(home, tmp_path, monkeypatch, capsys) -> None:
    folder = tmp_path / "exact"
    folder.mkdir()
    (folder / "tirith.toml").write_text('[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "test"\n'
                                        'run = "pytest -q"\n', encoding="utf-8")
    write_config(home, [{"id": "exact", "name": "Exact", "repo": str(folder)}])
    assert bash(monkeypatch, folder, "pytest -q") == 2
    err = capsys.readouterr().err
    assert err.count("tirith: ") == 1 and err.strip().endswith("tirith run --project exact --stage test")
