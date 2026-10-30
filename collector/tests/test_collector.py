"""Collector merge: statuses, stalled detection, renown, git stats, drops."""

from __future__ import annotations

import json
import os
import subprocess
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path

from conftest import GIT, make_repo
from tirith.cli import end_lane, register_lane
from tirith.collector import Collector, parse_shortstat
from tirith.home import now_iso
from tirith.store import append_jsonl, read_json, write_lane


def _collector(home: Path) -> Collector:
    return Collector(home, client=None)  # brain disabled: documented fallbacks


def _lane(home: Path, lane_id: str, log: Path, **overrides) -> dict:
    lane = register_lane(
        home, lane_id=lane_id, project="demo", hero_class="stonewright",
        feature="Feature work", workspace=None, log=str(log),
        pid=os.getpid(), brief=None)
    lane.update(overrides)
    write_lane(home, lane)
    return lane


def test_working_agent_without_structured_log_uses_fallback(home: Path, tmp_path: Path) -> None:
    log = tmp_path / "a.log"
    log.write_text("starting\n\nwriting migration tests\n", encoding="utf-8")
    _lane(home, "a-1", log)
    state = _collector(home).cycle()
    agent = state["agents"][0]
    assert agent["status"] == "working"
    assert agent["summary"] == "Working on Feature work."
    assert agent["log_tail"][-1] == "writing migration tests"
    assert agent["name"] and agent["epithet"] and agent["rank"] == "recruit"
    assert state["brain"] == {"enabled": False, "model": state["brain"]["model"],
                               "last_error": None, "usage": state["brain"]["usage"]}
    assert state["brain"]["usage"]["total"] == 0 and not state["brain"]["usage"]["budget_reached"]


def test_summary_is_the_agents_words_not_tool_output(home: Path, tmp_path: Path) -> None:
    log = tmp_path / "a.log"
    said = {"type": "assistant", "timestamp": "2026-10-29T11:00:00Z", "message": {"content": [{"type": "text", "text": "Now wiring the kanban."}]}}
    call = {"type": "assistant", "message": {"content": [
        {"type": "tool_use", "id": "b1", "name": "Bash", "input": {"command": "grep -n x f.py"}}]}}
    out = {"type": "user", "message": {"content": [
        {"type": "tool_result", "tool_use_id": "b1", "content": "593: agent_rows = []"}]}}
    log.write_text("\n".join(json.dumps(r) for r in (said, call, out)) + "\n", encoding="utf-8")
    _lane(home, "a-1", log)
    assert _collector(home).cycle()["agents"][0]["summary"] == "Now wiring the kanban."


def test_summary_without_say_is_not_raw_output(home: Path, tmp_path: Path) -> None:
    log = tmp_path / "a.log"
    call = {"type": "assistant", "message": {"content": [
        {"type": "tool_use", "id": "b1", "name": "Bash", "input": {"command": "grep -n x f.py"}}]}}
    out = {"type": "user", "message": {"content": [
        {"type": "tool_result", "tool_use_id": "b1", "content": "593: agent_rows = []"}]}}
    log.write_text("\n".join(json.dumps(r) for r in (call, out)) + "\n", encoding="utf-8")
    _lane(home, "a-1", log)
    assert _collector(home).cycle()["agents"][0]["summary"] == "Working on Feature work."


def test_ended_lane_summary_is_first_report_paragraph(home: Path, tmp_path: Path) -> None:
    log = tmp_path / "a.log"
    log.write_text("noise\n", encoding="utf-8")
    _lane(home, "a-1", log, ended_at=now_iso(datetime.now(timezone.utc) - timedelta(minutes=30)), result="done",
          report="Shipped the **kanban**.\n\nDetails follow.")
    assert _collector(home).cycle()["agents"][0]["summary"] == "Shipped the **kanban**."


def test_stalled_after_ten_minutes_of_silence(home: Path, tmp_path: Path) -> None:
    log = tmp_path / "a.log"
    log.write_text("retrying model turn\n", encoding="utf-8")
    old = time.time() - 20 * 60
    os.utime(log, (old, old))
    _lane(home, "a-1", log)
    state = _collector(home).cycle()
    assert state["agents"][0]["status"] == "stalled"
    assert "no log output" in state["agents"][0]["summary"]


def test_dead_pid_without_end_is_failed(home: Path, tmp_path: Path) -> None:
    proc = subprocess.Popen(["/usr/bin/true"])
    proc.wait()
    log = tmp_path / "a.log"
    log.write_text("boom\n", encoding="utf-8")
    _lane(home, "a-1", log, pid=proc.pid)
    state = _collector(home).cycle()
    assert state["agents"][0]["status"] == "failed"


def test_renown_done_failed_and_stage_credit(home: Path, tmp_path: Path) -> None:
    log = tmp_path / "a.log"
    log.write_text("done\n", encoding="utf-8")
    _lane(home, "a-1", log)
    _lane(home, "a-2", log)
    end_lane(home, "a-1", "done")
    end_lane(home, "a-2", "failed")
    append_jsonl(home / "events.jsonl", {
        "at": now_iso(), "run": "release#1", "project": "demo",
        "pipeline": "release", "stage": "build", "event": "end",
        "status": "passed", "log": str(log), "agent": "a-1"})
    collector = _collector(home)
    state = collector.cycle()
    by_id = {a["id"]: a for a in state["agents"]}
    assert by_id["a-1"]["renown"] == 120 and by_id["a-1"]["rank"] == "sentinel"
    assert by_id["a-2"]["renown"] == 0  # -30 floored at zero
    state = collector.cycle()  # second cycle must not double-count
    assert {a["id"]: a for a in state["agents"]}["a-1"]["renown"] == 120


def test_old_ended_lanes_are_dropped(home: Path, tmp_path: Path) -> None:
    log = tmp_path / "a.log"
    log.write_text("old\n", encoding="utf-8")
    lane = _lane(home, "a-1", log)
    long_ago = datetime.now(timezone.utc) - timedelta(hours=25)
    lane["started_at"] = now_iso(long_ago - timedelta(hours=1))
    lane["ended_at"] = now_iso(long_ago)
    lane["result"] = "done"
    write_lane(home, lane)
    assert _collector(home).cycle()["agents"] == []


def test_parse_shortstat() -> None:
    assert parse_shortstat(" 3 files changed, 10 insertions(+), 2 deletions(-)") == (3, 10, 2)
    assert parse_shortstat(" 1 file changed, 1 insertion(+)") == (1, 1, 0)
    assert parse_shortstat("") == (0, 0, 0)


def test_git_stats_against_start_commit(home: Path, tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    log = tmp_path / "a.log"
    log.write_text("editing\n", encoding="utf-8")
    lane = register_lane(
        home, lane_id="a-1", project="demo", hero_class="stonewright",
        feature="Edit readme", workspace=str(repo), log=str(log),
        pid=os.getpid(), brief=None)
    assert lane["start_commit"]
    (repo / "README.md").write_text("# demo repo\nnew line\n", encoding="utf-8")
    state = _collector(home).cycle()
    agent = state["agents"][0]
    assert agent["files_changed"] == 1 and agent["insertions"] == 1
    head = subprocess.run([GIT, "-C", str(repo), "rev-parse", "HEAD"],
                          capture_output=True, text=True, check=True).stdout.strip()
    assert lane["start_commit"] == head


def test_pipeline_stage_order_pending_and_skipped(home: Path, tmp_path: Path) -> None:
    from tirith.cli import main
    log = tmp_path / "s.log"
    log.write_text("stage output\n", encoding="utf-8")
    main(["pipeline", "declare", "release", "--project", "demo",
          "--stages", "build,test,ship"])
    base = {"run": "release#1", "project": "demo", "pipeline": "release", "log": str(log)}
    append_jsonl(home / "events.jsonl",
                 {"at": "2026-10-28T10:00:00Z", **base, "stage": "build",
                  "event": "start", "status": "running"})
    append_jsonl(home / "events.jsonl",
                 {"at": "2026-10-28T10:01:00Z", **base, "stage": "build",
                  "event": "end", "status": "failed"})
    state = _collector(home).cycle()
    pipe = state["pipelines"][0]
    assert pipe["status"] == "failed" and pipe["ended_at"] == "2026-10-28T10:01:00Z"
    assert [(s["name"], s["status"], s["duration_s"]) for s in pipe["stages"]] == [
        ("build", "failed", 60), ("test", "skipped", None), ("ship", "skipped", None)]
    assert read_json(home / "state.json", {})["pipelines"][0]["run"] == "release#1"


def test_manager_live_follows_manager_json(home: Path) -> None:
    assert _collector(home).cycle()["manager"]["live"] is None
    (home / "manager.json").write_text(json.dumps({"heartbeat": "2026-10-29T09:00:00"}), encoding="utf-8")
    assert _collector(home).cycle()["manager"]["live"] == {"heartbeat": "2026-10-29T09:00:00Z"}
    (home / "manager.json").write_text("not json", encoding="utf-8")
    assert _collector(home).cycle()["manager"]["live"] is None
