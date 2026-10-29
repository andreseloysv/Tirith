"""Decisions: `tirith decide`, extraction from lane reports, and violations in state."""

from __future__ import annotations

import os
from datetime import datetime, timezone
from pathlib import Path

import pytest

from conftest import write_config
from tirith import cli
from tirith.collector import Collector
from tirith.flows import extract_decisions
from tirith.store import append_jsonl, read_jsonl

NOW = datetime(2026, 10, 28, 20, 0, tzinfo=timezone.utc)


def _start(lane_id: str = "w-1") -> None:
    cli.main(["lane", "start", "--project", "demo", "--class", "warden", "--feature", "Tests",
              "--id", lane_id, "--pid", str(os.getpid())])


def test_decide_command_records_agent_decision(home: Path, monkeypatch) -> None:
    _start()
    assert cli.main(["decide", "Use SQLite", "--why", "one file, no server", "--lane", "w-1"]) == 0
    monkeypatch.setenv("TIRITH_LANE", "w-1")
    assert cli.main(["decide", "Keep the schema flat", "--why", "easier to migrate"]) == 0
    monkeypatch.delenv("TIRITH_LANE")
    assert cli.main(["decide", "Ship Friday", "--why", "owner asked"]) == 0
    rows = [r for r in read_jsonl(home / "flows.jsonl") if r["kind"] == "decision"]
    assert [(r["from"], r["to"], r["text"], r["why"]) for r in rows] == [
        ("agent:w-1", "steward", "Use SQLite", "one file, no server"),
        ("agent:w-1", "steward", "Keep the schema flat", "easier to migrate"),
        ("steward", "owner", "Ship Friday", "owner asked")]
    assert rows[0]["project"] == "demo"
    with pytest.raises(SystemExit):
        cli.main(["decide", "x", "--why", "y", "--lane", "nope"])
    with pytest.raises(SystemExit):
        cli.main(["decide", "x", "--why", " "])
    state = Collector(home, client=None).build_state(NOW)
    flow = next(f for f in state["flows"] if f["text"] == "Use SQLite")
    assert flow["why"] == "one file, no server" and flow["kind"] == "decision"


def test_extract_decisions_only_explicit_lines() -> None:
    report = """Done. Summary of the work.

- Decision: use SQLite — because one file, no server
**Decided:** keep the schema flat (why: easier to migrate)
Decision: drop the cache
Why: it hid stale rows
Decision needed on the logo: ask the owner
We decided to skip it.

## Left open
- Decision: pick a cloud
Decided: later

## Notes
Decided: back in a normal section
"""
    assert extract_decisions(report) == [
        ("use SQLite", "one file, no server"),
        ("keep the schema flat", "easier to migrate"),
        ("drop the cache", "it hid stale rows"),
        ("back in a normal section", "")]
    assert extract_decisions(None) == [] and extract_decisions("Nothing here") == []


def test_collector_extracts_decisions_from_reports_once(home: Path) -> None:
    write_config(home, [{"id": "demo", "name": "Demo"}])
    _start()
    cli.main(["decide", "Use SQLite", "--why", "one file", "--lane", "w-1"])
    cli.main(["lane", "end", "w-1", "--result", "done", "--report",
              "Done\nDecision: Use SQLite - because one file\nDecision: Flat schema - because simple\n"
              "Left open:\nDecision: cloud?"])
    Collector(home, client=None).build_state(NOW)
    state = Collector(home, client=None).build_state(NOW)   # a second cycle adds nothing
    decisions = [(f["text"], f.get("why")) for f in state["flows"] if f["kind"] == "decision"]
    assert decisions == [("Use SQLite", "one file"), ("Flat schema", "simple")]
    row = next(f for f in state["flows"] if f["text"] == "Flat schema")
    assert row["from"] == "agent:w-1" and row["to"] == "steward" and row["project"] == "demo"


def test_state_violations_split_blocked_and_instead(home: Path) -> None:
    write_config(home, [{"id": "demo", "name": "Demo"}])
    append_jsonl(home / "events.jsonl", {
        "kind": "violation", "ts": "2026-10-28T19:59:00Z", "rule": "subagent-tool",
        "agent": "anthropic", "lane": None, "project": "demo",
        "message": "tirith: subagent-tool: the Task tool is not allowed. "
                   "Do this instead: run tirith lane run"})
    row = Collector(home, client=None).build_state(NOW)["violations"][0]
    assert row["blocked"] == "the Task tool is not allowed"
    assert row["instead"] == "run tirith lane run"
