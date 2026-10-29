"""Activity: brief goal, test detection, log events, git files, collector fields."""

from __future__ import annotations

import json
import os
import subprocess
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest

from tests.conftest import GIT, make_repo, write_config
from tirith import activity, hooks
from tirith.activity import ActivityTracker, git_files, goal_from_text, is_test_command, readable_command, run_outcome
from tirith.collector import Collector, git_head
from tirith.store import write_lane

DATA = Path(__file__).parent / "data"
NOW = datetime(2026, 10, 29, 12, 0, 0, tzinfo=timezone.utc)


def _lane(tmp_path: Path, log: Path | None, **extra) -> dict:
    lane = {"id": "a-1", "project": "demo", "class": "stonewright", "feature": "Feature work",
            "workspace": None, "log": str(log) if log else None, "pid": None, "brief": None,
            "started_at": "2026-10-29T11:00:00Z", "ended_at": None, "result": None}
    lane.update(extra)
    return lane


def _write(path: Path, *records) -> Path:
    path.write_text("".join(json.dumps(r) + "\n" for r in records), encoding="utf-8")
    return path


def _use(name: str, **input_) -> dict:
    return {"type": "assistant", "message": {"content": [
        {"type": "tool_use", "id": f"t-{name}-{len(input_)}", "name": name, "input": input_}]}}


def _bash(call_id: str, command: str) -> dict:
    return {"type": "assistant", "message": {"content": [
        {"type": "tool_use", "id": call_id, "name": "Bash", "input": {"command": command}}]}}


def _result(call_id: str, text: str = "ok", error: bool = False) -> dict:
    return {"type": "user", "message": {"content": [
        {"type": "tool_result", "tool_use_id": call_id, "content": text, "is_error": error}]}}


def _info(lane: dict, tracker: ActivityTracker | None = None, rows=(), status="working") -> dict:
    return (tracker or ActivityTracker()).lane(lane, status, list(rows), NOW)


# -- goal ---------------------------------------------------------------------------------------


def test_goal_prefers_why_heading_and_strips_markdown() -> None:
    text = ("# Lane: thing\n\nWorkspace: here\n\n## Why\nThe **owner** wants to see `goals`, "
            "see [docs](http://x.invalid/a).\nSecond line. Third one. Fourth gone.\n\n## What\nx\n")
    assert goal_from_text(text) == "The owner wants to see goals, see docs. Second line. Third one."


def test_goal_falls_back_to_first_heading_then_first_paragraph() -> None:
    assert goal_from_text("# Title\n\nFirst para here.\n\n## Other\nNo.") == "First para here."
    assert goal_from_text("Just text\nacross lines.\n\nMore.") == "Just text across lines."
    assert goal_from_text("# Only a title\n") is None
    assert goal_from_text("```\ncode\n```\n\nReal text.") == "Real text."


def test_goal_is_clipped_and_lane_falls_back_to_feature(tmp_path: Path) -> None:
    assert len(goal_from_text("## Goal\n" + "word " * 200)) == activity.GOAL_MAX
    brief = tmp_path / "brief.md"
    brief.write_text("## Goal\nShip the thing.\n", encoding="utf-8")
    tracker = ActivityTracker()
    assert tracker.goal(_lane(tmp_path, None, brief=str(brief))) == "Ship the thing."
    assert tracker.goal(_lane(tmp_path, None, brief=str(tmp_path / "gone.md"))) == "Feature work"
    assert tracker.goal(_lane(tmp_path, None, feature="")) is None


# -- test detection -----------------------------------------------------------------------------


@pytest.mark.parametrize("command", [
    "pytest -q", "cd collector && uv run -q pytest -q", "uv run pytest tests/x.py", "npm test",
    "npm run test -- --watch", "pnpm test", "yarn test", "npx vitest run", "jest --ci",
    "go test ./...", "cargo test", "./gradlew testDebugUnitTest", "gradle :app:test",
    "make test", "make check", "xcodebuild -scheme App test", "swift test",
    "tirith run --project p --pipeline x --stage test -- make",
    "cd x && uv run -q pytest -q 2>&1 | tail -30",
])
def test_test_commands_detected(command: str) -> None:
    assert is_test_command(command)


@pytest.mark.parametrize("command", [
    "cd /Users/x/DEVELOP/Tirith/hooks && ls", "sed -i 's/a/b/' tests/test_x.py",
    "cd tests && ls", "FOO=1 python tests/helper.py", "python scripts/make_tests.py",
    "echo pytest && true", "git commit -m 'fix pytest run'", "open tests/",
    "grep -n pytest README.md", "cat tests/test_x.py", "git log --grep test", "ls tests",
    "npm install", "make build", "echo go test", "uv run python script.py", "", None,
])
def test_non_test_commands_ignored(command) -> None:
    assert not is_test_command(command)


def test_test_commands_extra_shapes() -> None:
    assert is_test_command("FOO=1 uv run --project c -q pytest -q 2>&1 | tail -1")
    assert is_test_command("cd a && echo hi; python3 -m pytest x")
    assert is_test_command("npm run test:unit")
    assert not is_test_command("tail -f tests/log | grep go")


def test_outcome_prefers_output_over_flag() -> None:
    assert run_outcome("166 passed in 3.1s", True) == ("passed", 166, None)
    assert run_outcome("1 failed, 4 passed in 1s") == ("failed", 4, 1)
    assert run_outcome("Tests:  1 failed, 4 passed, 5 total") == ("failed", 4, 1)
    assert run_outcome("BUILD SUCCESSFUL in 4s")[0] == "passed"
    assert run_outcome("BUILD FAILED in 4s")[0] == "failed"
    assert run_outcome("ok  \tpkg\t0.1s")[0] == "passed"
    assert run_outcome("FAIL\tpkg\t0.1s")[0] == "failed"
    assert run_outcome("boom", True)[0] == "failed" and run_outcome("")[0] == "passed"


def test_readable_command_and_counts(tmp_path: Path) -> None:
    assert readable_command("cd /w/c && uv run pytest -q /w/c/t.py", "/w") == "uv run pytest -q c/t.py"
    log = _write(tmp_path / "l.log", _bash("b1", "cd /w/c && uv run -q pytest -q 2>&1 | tail -1"),
                 _result("b1", "101 passed in 2s", error=True))
    row = _info(_lane(tmp_path, log, workspace="/w"))["tests"][0]
    assert row["command"].startswith("uv run -q pytest") and row["full"].startswith("cd /w/c")
    assert (row["result"], row["passed"]) == ("passed", 101)


# -- log events ---------------------------------------------------------------------------------


def test_real_sample_logs_become_events(tmp_path: Path) -> None:
    for name, calls in (("u-steps12", 8), ("u-hooks", 6)):
        info = _info(_lane(tmp_path, DATA / f"{name}.jsonl", workspace="/Users/x/DEVELOP/Tirith"))
        assert sum(t["count"] for t in info["tools"]) == calls
        assert info["tools"][0]["count"] >= info["tools"][-1]["count"]
        assert all(e["source"] == "log" and e["lane"] == "a-1" for e in info["events"])
        assert [e["event"] for e in info["events"]].count("tool") == calls
        assert info["events"][-1]["event"] == "done"
        assert info["events"][0]["summary"].startswith("Read ")
        assert info["last_activity_at"]


def test_assistant_shapes_tests_and_results(tmp_path: Path) -> None:
    log = _write(tmp_path / "l.log",
                 _use("Edit", file_path="/w/web/js/ui.js"), _bash("b1", "uv run pytest -q"),
                 _result("b1", "5 passed"), _bash("b2", "cd c && pytest"),
                 _result("b2", "boom", error=True), _bash("b3", "pytest | tail -3"),
                 _result("b3", "1 failed, 4 passed"), _bash("b4", "npm test"),
                 {"type": "result", "subtype": "success", "result": "All done\nmore"})
    info = _info(_lane(tmp_path, log, workspace="/w"))
    assert [t["result"] for t in info["tests"]] == ["passed", "failed", "failed", "running"]
    assert info["tests"][0]["command"] == "uv run pytest -q"
    assert {t["name"]: t["count"] for t in info["tools"]} == {"Bash": 4, "Edit": 1}
    assert info["events"][0]["summary"] == "Edit web/js/ui.js"
    assert info["events"][1]["summary"] == "Bash: uv run pytest -q"
    assert info["events"][-1]["event"] == "done" and info["events"][-1]["summary"] == "All done"
    ended = _info(_lane(tmp_path, log, workspace="/w"), status="done")
    assert ended["tests"][-1]["result"] is None and ended["active"] is False


def test_item_shapes(tmp_path: Path) -> None:
    log = _write(tmp_path / "l.log",
                 {"type": "item.started", "item": {"id": "i1", "type": "command_execution",
                                                   "command": "go test ./..."}},
                 {"type": "item.completed", "item": {"id": "i1", "type": "command_execution",
                                                     "command": "go test ./...", "exit_code": 1}},
                 {"type": "item.completed", "item": {"id": "i2", "type": "command_execution",
                                                     "command": "cargo test", "exit_code": 0}},
                 {"type": "item.completed", "item": {"id": "i3", "type": "file_change", "changes": [
                     {"path": "a.py", "kind": "update"}, {"path": "b.py", "kind": "add"}]}},
                 {"type": "item.completed", "item": {"id": "i4", "type": "reasoning"}})
    info = _info(_lane(tmp_path, log))
    assert [(t["command"], t["result"]) for t in info["tests"]] == [
        ("go test ./...", "failed"), ("cargo test", "passed")]
    assert {t["name"]: t["count"] for t in info["tools"]} == {"Bash": 2, "file_change": 1}
    assert [e["summary"] for e in info["events"]] == [
        "Bash: go test ./...", "Bash: cargo test", "file_change a.py, b.py"]


def test_muse_records(tmp_path: Path) -> None:
    def rec(kind: str, **payload) -> dict:
        return {"schema_version": 1, "payload_type": kind, "payload": payload}
    log = _write(tmp_path / "l.log",
                 rec("runtime.command.accepted", command_kind="turn.submit"),
                 rec("tool.call.started", tool="shell", command="pytest -q", call_id="c1"),
                 rec("tool.call.completed", tool="shell", call_id="c1", exit_code=0),
                 rec("file.write.completed", path="src/a.py"),
                 rec("run.output.delta", text="hi"),
                 rec("run.terminal.completed", text="echo: hi"))
    info = _info(_lane(tmp_path, log))
    assert [t["result"] for t in info["tests"]] == ["passed"]
    assert [e["summary"] for e in info["events"]] == ["Bash: pytest -q", "write src/a.py",
                                                      "echo: hi"]
    assert [e["event"] for e in info["events"]] == ["tool", "tool", "done"]
    assert {t["name"] for t in info["tools"]} == {"shell", "write"}


def test_log_is_read_incrementally_and_survives_garbage(tmp_path: Path) -> None:
    log = tmp_path / "l.log"
    first = json.dumps(_use("Read", file_path="/w/a.py"))
    log.write_text("plain text line\n{broken json\n" + first + "\n" + json.dumps(_bash("b", "pytest"))[:30],
                   encoding="utf-8")
    tracker, lane = ActivityTracker(), _lane(tmp_path, log, workspace="/w")
    assert [t["name"] for t in tracker.lane(lane, "working", [], NOW)["tools"]] == ["Read"]
    with log.open("a", encoding="utf-8") as handle:  # the cut line is completed later
        handle.write(json.dumps(_bash("b", "pytest"))[30:] + "\n")
    info = tracker.lane(lane, "working", [], NOW)
    assert {t["name"]: t["count"] for t in info["tools"]} == {"Read": 1, "Bash": 1}
    assert info["tests"][0]["result"] == "running"
    with log.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(_result("b", "1 passed")) + "\n")
    assert tracker.lane(lane, "working", [], NOW)["tests"][0]["result"] == "passed"
    log.write_text(json.dumps(_use("Grep", pattern="x")) + "\n", encoding="utf-8")  # rotated
    assert [t["name"] for t in tracker.lane(lane, "working", [], NOW)["tools"]] == ["Grep"]


def test_big_log_only_reads_the_tail(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setattr(activity, "LOG_BYTES", 2000)
    log = _write(tmp_path / "l.log", *[_use("Read", file_path=f"/w/f{i}.py") for i in range(100)])
    info = _info(_lane(tmp_path, log, workspace="/w"))
    count = info["tools"][0]["count"]
    assert 0 < count < 100 and len(info["events"]) == count
    assert info["events"][-1]["summary"] == "Read f99.py"


def test_unchanged_log_is_not_reparsed(tmp_path: Path, monkeypatch) -> None:
    log = _write(tmp_path / "l.log", _use("Read", file_path="/w/a.py"))
    tracker, lane = ActivityTracker(), _lane(tmp_path, log)
    tracker.lane(lane, "working", [], NOW)
    monkeypatch.setattr(activity, "_read_new", lambda *a: pytest.fail("re-read"))
    assert tracker.lane(lane, "working", [], NOW)["tools"][0]["count"] == 1


# -- hooks fallback and activity flag -----------------------------------------------------------


def _hook(ts: str, event: str, tool: str | None, summary: str) -> dict:
    return {"kind": "hook", "ts": ts, "agent": "anthropic", "event": event, "lane": "a-1",
            "session": None, "project": None, "tool": tool, "summary": summary,
            "agent_session_id": "s"}


def test_hook_rows_feed_tools_tests_and_events(tmp_path: Path) -> None:
    rows = [_hook("2026-10-29T11:58:00Z", "PreToolUse", "Edit", "Edit a.py"),
            _hook("2026-10-29T11:58:01Z", "PostToolUse", "Edit", "Edit a.py"),
            _hook("2026-10-29T11:59:00Z", "PreToolUse", "Bash", "Bash: uv run pytest -q"),
            _hook("2026-10-29T11:59:30Z", "Stop", None, "Stop")]
    info = _info(_lane(tmp_path, None), rows=rows)
    assert {t["name"]: t["count"] for t in info["tools"]} == {"Edit": 1, "Bash": 1}
    assert info["tests"] == [{"at": "2026-10-29T11:59:00Z", "command": "uv run pytest -q",
                              "result": None}]
    assert [e["source"] for e in info["events"]] == ["hook"] * 4
    assert info["last_activity_at"] == "2026-10-29T11:59:30Z" and info["active"] is True


def test_active_needs_working_and_recent_activity(tmp_path: Path) -> None:
    old = [_hook("2026-10-29T11:40:00Z", "Stop", None, "Stop")]
    assert _info(_lane(tmp_path, None), rows=old)["active"] is False
    recent = [_hook("2026-10-29T11:58:00Z", "Stop", None, "Stop")]
    assert _info(_lane(tmp_path, None), rows=recent, status="stalled")["active"] is False
    assert _info(_lane(tmp_path, None))["last_activity_at"] is None


def test_log_and_hook_events_merge_by_time(tmp_path: Path) -> None:
    log = _write(tmp_path / "l.log", {**_use("Read", file_path="/w/a.py"),
                                       "timestamp": "2026-10-29T11:59:00Z"})
    rows = [_hook("2026-10-29T11:58:00Z", "UserPromptSubmit", None, "Do it"),
            _hook("2026-10-29T11:59:30Z", "Stop", None, "Stop")]
    info = _info(_lane(tmp_path, log, workspace="/w"), rows=rows)
    assert [(e["source"], e["summary"]) for e in info["events"]] == [
        ("hook", "Do it"), ("log", "Read a.py"), ("hook", "Stop")]


# -- git files ----------------------------------------------------------------------------------


def _git(repo: Path, *args: str) -> None:
    subprocess.run([GIT, "-C", str(repo), "-c", "user.email=t@example.invalid",
                    "-c", "user.name=T", *args], check=True, capture_output=True)


def test_git_files_every_change_kind(tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    (repo / "mod.txt").write_text("one\ntwo\nthree\n")
    (repo / "gone.txt").write_text("bye\nbye\n")
    (repo / "old name.txt").write_text("".join(f"line {i}\n" for i in range(30)))
    _git(repo, "add", ".")
    _git(repo, "commit", "-q", "-m", "base")
    start = git_head(repo)
    (repo / "mod.txt").write_text("one\nTWO\nthree\nfour\n")
    (repo / "gone.txt").unlink()
    (repo / "new.txt").write_text("a\nb\n")
    _git(repo, "add", "new.txt")
    _git(repo, "mv", "old name.txt", "new name.txt")
    (repo / "loose.txt").write_text("x\ny\nz")
    (repo / "dir").mkdir()
    (repo / "dir" / "bin.dat").write_bytes(b"\0\1\2")
    files, totals = git_files(repo, start)
    by_path = {f["path"]: f for f in files}
    assert by_path["mod.txt"] == {"path": "mod.txt", "change": "modified", "insertions": 2,
                                  "deletions": 1}
    assert by_path["gone.txt"]["change"] == "deleted" and by_path["gone.txt"]["deletions"] == 2
    assert by_path["new.txt"]["change"] == "added" and by_path["new.txt"]["insertions"] == 2
    assert by_path["new name.txt"]["change"] == "renamed"
    assert "old name.txt" not in by_path
    assert by_path["loose.txt"] == {"path": "loose.txt", "change": "untracked", "insertions": 3,
                                    "deletions": 0}
    assert by_path["dir/bin.dat"]["insertions"] == 0
    assert totals == (4, 4, 3)  # tracked only: mod, gone, new, renamed


def test_git_files_edge_cases(tmp_path: Path) -> None:
    assert git_files(None, "abc") == ([], (0, 0, 0))
    assert git_files(tmp_path, None) == ([], (0, 0, 0))
    assert git_files(tmp_path / "missing", "abc") == ([], (0, 0, 0))
    assert git_files(tmp_path, "abc") == ([], (0, 0, 0))  # not a repo
    repo = make_repo(tmp_path / "repo")
    start = git_head(repo)
    for i in range(120):
        (repo / f"f{i:03}.txt").write_text("x\n")
    files, totals = git_files(repo, start)
    assert len(files) == 100 and totals == (0, 0, 0)
    assert {f["change"] for f in files} == {"untracked"}


def test_ended_lane_keeps_its_files(tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    lane = _lane(tmp_path, None, workspace=str(repo), start_commit=git_head(repo))
    (repo / "a.txt").write_text("x\n")
    tracker = ActivityTracker()
    lane["ended_at"] = "2026-10-29T11:30:00Z"
    first = tracker.lane(lane, "done", [], NOW)["files"]
    (repo / "b.txt").write_text("y\n")
    assert tracker.lane(lane, "done", [], NOW)["files"] == first
    lane["ended_at"] = None
    assert len(tracker.lane(lane, "working", [], NOW)["files"]) == 2


# -- collector ----------------------------------------------------------------------------------


def test_collector_fills_activity_fields(home: Path, tmp_path: Path) -> None:
    write_config(home, [{"id": "demo", "name": "Demo"}])
    repo = make_repo(tmp_path / "repo")
    (repo / "README.md").write_text("# demo repo\nmore\n")
    (repo / "new.py").write_text("print(1)\n")
    brief = tmp_path / "brief.md"
    brief.write_text("# Lane\n\n## Why\nSee every agent live.\n", encoding="utf-8")
    log = _write(tmp_path / "a.log", _use("Edit", file_path=f"{repo}/new.py"),
                 _bash("b1", "uv run pytest -q"), _result("b1", "3 passed"))
    lane = _lane(tmp_path, log, workspace=str(repo), brief=str(brief), pid=os.getpid(),
                 start_commit=git_head(repo))
    write_lane(home, lane)
    hooks.record(home, "anthropic", {"hook_event_name": "Stop"})
    state = Collector(home, client=None).cycle()
    agent = state["agents"][0]
    assert agent["goal"] == "See every agent live."
    assert agent["active"] is True and agent["last_activity_at"]
    assert {f["path"]: f["change"] for f in agent["files"]} == {
        "README.md": "modified", "new.py": "untracked"}
    assert (agent["files_changed"], agent["insertions"], agent["deletions"]) == (1, 1, 0)
    assert {t["name"]: t["count"] for t in agent["tools"]} == {"Edit": 1, "Bash": 1}
    assert [t["result"] for t in agent["tests"]] == ["passed"]
    assert all(e["source"] in ("hook", "log") for e in agent["events"])
    assert agent["events"][0]["summary"] == "Edit new.py"
    jsonschema = pytest.importorskip("jsonschema")
    schema = json.loads((Path(__file__).parent.parent / "state.schema.json").read_text())
    jsonschema.validate(state, schema)


def test_collector_without_log_or_workspace(home: Path, tmp_path: Path) -> None:
    write_config(home, [{"id": "demo", "name": "Demo"}])
    write_lane(home, _lane(tmp_path, None, pid=os.getpid()))
    agent = Collector(home, client=None).cycle()["agents"][0]
    assert agent["goal"] == "Feature work" and agent["files"] == [] and agent["tests"] == []
    assert agent["tools"] == [] and agent["events"] == []
    assert agent["last_activity_at"] is None and agent["active"] is False
