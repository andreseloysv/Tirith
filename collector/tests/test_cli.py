"""CLI behaviour: lanes, stage runs, notes, waits, steward, status."""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

from tirith.cli import main
from tirith.store import read_json, read_jsonl


def test_lane_start_end_and_files(home: Path, capsys) -> None:
    assert main(["lane", "start", "--project", "demo", "--class", "stonewright",
                 "--feature", "Room database", "--id", "w1-data",
                 "--pid", str(os.getpid())]) == 0
    assert capsys.readouterr().out.strip() == "w1-data"
    lane = read_json(home / "lanes" / "w1-data.json", {})
    assert lane["project"] == "demo"
    assert lane["ended_at"] is None and lane["result"] is None
    assert main(["lane", "end", "w1-data", "--result", "done",
                 "--report", "All migrations green."]) == 0
    lane = read_json(home / "lanes" / "w1-data.json", {})
    assert lane["result"] == "done" and lane["ended_at"]
    assert lane["report"] == "All migrations green."


def test_lane_start_auto_id(home: Path, capsys) -> None:
    assert main(["lane", "start", "--project", "demo", "--class", "warden",
                 "--feature", "Check the gates"]) == 0
    lane_id = capsys.readouterr().out.strip()
    assert lane_id and (home / "lanes" / f"{lane_id}.json").exists()


def test_note_wait_steward(home: Path, capsys) -> None:
    assert main(["note", "Merged the data lane", "--project", "demo"]) == 0
    notes = read_jsonl(home / "notes.jsonl")
    assert notes[0]["project"] == "demo" and "Merged" in notes[0]["text"]
    assert main(["wait", "add", "Tick the declaration", "--project", "demo"]) == 0
    wait_id = capsys.readouterr().out.strip().splitlines()[-1]
    assert wait_id == "w-1"
    assert main(["wait", "done", wait_id]) == 0
    rows = read_jsonl(home / "waits.jsonl")
    assert rows[-1]["done_at"] and rows[-1]["text"] == "Tick the declaration"
    assert main(["steward", "Reviewing the data lane"]) == 0
    steward = read_json(home / "orchestrator.json", {})
    assert steward["activity"] == "Reviewing the data lane" and steward["since"]


def test_pipeline_declare_and_run_events(home: Path, capsys) -> None:
    assert main(["pipeline", "declare", "release", "--project", "demo",
                 "--stages", "build,test"]) == 0
    capsys.readouterr()
    code = main(["run", "--project", "demo", "--pipeline", "release",
                 "--stage", "build", "--", sys.executable, "-c",
                 "print('build ok')"])
    assert code == 0
    assert "build ok" in capsys.readouterr().out
    events = read_jsonl(home / "events.jsonl")
    assert [e["event"] for e in events] == ["start", "end"]
    assert events[0]["run"] == "release#1" and events[1]["status"] == "passed"
    assert Path(events[0]["log"]).read_text(encoding="utf-8").strip() == "build ok"
    failing = main(["run", "--project", "demo", "--pipeline", "release",
                    "--stage", "test", "--", sys.executable, "-c",
                    "raise SystemExit(7)"])
    assert failing == 7
    events = read_jsonl(home / "events.jsonl")
    assert events[-1]["status"] == "failed" and events[-1]["run"] == "release#1"


def test_collect_once_and_status(home: Path, capsys) -> None:
    main(["lane", "start", "--project", "demo", "--class", "loremaster",
          "--feature", "Map the code", "--id", "map-1", "--pid", str(os.getpid())])
    capsys.readouterr()
    assert main(["collect", "--once"]) == 0
    state = json.loads((home / "state.json").read_text(encoding="utf-8"))
    assert state["version"] == 1
    assert state["agents"][0]["id"] == "map-1"
    capsys.readouterr()
    assert main(["status"]) == 0
    assert "Map the code" in capsys.readouterr().out
