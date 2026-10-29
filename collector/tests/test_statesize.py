"""state.json size budget and the per-agent detail files (see docs/CONTRACT-agents.md)."""

from __future__ import annotations

import json
import os
from datetime import datetime, timedelta, timezone
from pathlib import Path

from tirith.cli import register_lane
from tirith.collector import (
    Collector, STATE_AGENTS_MAX, STATE_EVENTS, STATE_LOG_ENTRIES, STATE_REPORT_MAX,
)
from tirith.home import now_iso
from tirith.store import agent_file_name, write_lane

BUDGET = 800 * 1024


def _transcript(path: Path, turns: int) -> None:
    rows = []
    for i in range(turns):
        rows.append({"type": "assistant", "message": {"content": [
            {"type": "text", "text": f"Step {i}: reading the module and thinking about the next change."},
            {"type": "tool_use", "id": f"c{i}", "name": "Bash", "input": {"command": f"grep -n thing{i} src/file.py"}}]}})
        rows.append({"type": "user", "message": {"content": [
            {"type": "tool_result", "tool_use_id": f"c{i}", "content": "line\n" * 30}]}})
    path.write_text("\n".join(json.dumps(r) for r in rows) + "\n", encoding="utf-8")


def _fleet(home: Path, tmp_path: Path, working: int = 5, recent: int = 15, old: int = 100) -> None:
    now = datetime.now(timezone.utc)
    log = tmp_path / "big.log"
    _transcript(log, 150)
    report = "First paragraph of the report.\n\n" + "\n\n".join(f"Section {i}: " + "word " * 120 for i in range(40))
    for n in range(working + recent + old):
        lane = register_lane(home, lane_id=f"a-{n:03d}", project="demo", hero_class="stonewright",
                             feature=f"Feature {n}", workspace=None, log=str(log), pid=os.getpid(), brief=None)
        if n >= working:
            age = timedelta(minutes=5 + n) if n < working + recent else timedelta(hours=3 + (n % 15))
            lane["started_at"] = now_iso(now - age - timedelta(minutes=10))
            lane["ended_at"] = now_iso(now - age)
            lane["result"] = "done"
            lane["report"] = report
        write_lane(home, lane)


def test_state_json_stays_under_budget_with_120_agents(home: Path, tmp_path: Path) -> None:
    _fleet(home, tmp_path)
    collector = Collector(home, client=None)
    collector.cycle()
    size = (home / "state.json").stat().st_size
    assert size < BUDGET, f"state.json is {size} bytes"
    assert (home / "state.js").stat().st_size < BUDGET
    state = json.loads((home / "state.json").read_text(encoding="utf-8"))
    assert len(state["agents"]) == STATE_AGENTS_MAX
    # a second cycle with nothing new rewrites no detail file
    before = {p.name: p.stat().st_mtime_ns for p in (home / "agents").glob("*.js")}
    assert len(before) == 120
    collector.cycle()
    assert {p.name: p.stat().st_mtime_ns for p in (home / "agents").glob("*.js")} == before


def test_full_detail_is_capped_and_old_agents_are_summaries(home: Path, tmp_path: Path) -> None:
    _fleet(home, tmp_path)
    state = Collector(home, client=None).cycle()
    by_id = {a["id"]: a for a in state["agents"]}
    working, recent = by_id["a-000"], by_id["a-006"]
    for row in (working, recent):
        assert 0 < len(row["log_entries"]) <= STATE_LOG_ENTRIES
        assert len(row["events"]) <= STATE_EVENTS
        assert row["detail_more"] is True
    assert working["status"] == "working"
    assert recent["report_clipped"] is True and len(recent["report"]) <= STATE_REPORT_MAX + 4
    old = [a for a in state["agents"] if a["id"] >= "a-020"]
    assert old, "older ended agents are listed too"
    for row in old:
        assert row["log_entries"] == [] and row["events"] == [] and row["files"] == [] and row["tests"] == []
        assert row["report"] == "First paragraph of the report."
        assert row["status"] == "done" and row["feature"]
        assert row["detail"]["path"].endswith(agent_file_name(row["id"]))
    # working and recent agents come first in the cut: none of them is missing
    assert {f"a-{n:03d}" for n in range(20)} <= set(by_id)


def test_detail_file_has_the_full_log_and_report(home: Path, tmp_path: Path) -> None:
    _fleet(home, tmp_path, working=1, recent=1, old=1)
    state = Collector(home, client=None).cycle()
    old = {a["id"]: a for a in state["agents"]}["a-002"]
    text = Path(old["detail"]["path"]).read_text(encoding="utf-8")
    assert text.startswith("window.TIRITH_AGENT = ") and text.endswith(";\n") and "</" not in text
    detail = json.loads(text[len("window.TIRITH_AGENT = "):-2])
    assert detail["id"] == "a-002"
    assert len(detail["log_entries"]) > STATE_LOG_ENTRIES
    assert len(detail["report"]) > STATE_REPORT_MAX  # the detail file keeps the (16 KB bounded) report
    assert old["detail"]["hash"]


def test_agent_file_name_never_leaves_the_folder() -> None:
    assert agent_file_name("a-1") == "a-1.js"
    for bad in ("../x", "a/b", "..", ".hidden", "", "a\\b"):
        name = agent_file_name(bad)
        assert "/" not in name and "\\" not in name and not name.startswith(".") and name.endswith(".js")
