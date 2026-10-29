"""CLI behaviour: lanes, stage runs, notes, waits, manager, status."""

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


def test_note_wait_manager(home: Path, capsys) -> None:
    assert main(["note", "Merged the data lane", "--project", "demo"]) == 0
    notes = read_jsonl(home / "notes.jsonl")
    assert notes[0]["project"] == "demo" and "Merged" in notes[0]["text"]
    assert main(["wait", "add", "--why-owner", "owner-decision", "Tick the declaration", "--project", "demo", "--option", "Done|done"]) == 0
    wait_id = capsys.readouterr().out.strip().splitlines()[-1]
    assert wait_id == "w-1"
    assert main(["wait", "done", wait_id]) == 0
    rows = read_jsonl(home / "waits.jsonl")
    assert rows[-1]["done_at"] and rows[-1]["text"] == "Tick the declaration"
    assert main(["manager", "Reviewing the data lane"]) == 0
    manager = read_json(home / "orchestrator.json", {})
    assert manager["activity"] == "Reviewing the data lane" and manager["since"]


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


def test_lane_request_links_to_owner_prompt(tmp_path: Path) -> None:
    from datetime import datetime, timedelta, timezone
    from tirith.cli import bound_text, owner_request
    from tirith.home import now_iso

    def put(name: str, prompt: str, age_h: float) -> None:
        at = now_iso(datetime.now(timezone.utc) - timedelta(hours=age_h))
        (tmp_path / "requests").mkdir(exist_ok=True)
        (tmp_path / "requests" / f"{name}.json").write_text(
            json.dumps({"agent_session_id": name, "prompt": prompt, "at": at}), encoding="utf-8")

    assert owner_request(tmp_path, {}) is None
    put("old", "stale", 9)
    assert owner_request(tmp_path, {}) is None
    put("s1", "first", 2)
    put("s2", "x" * 5000, 1)
    got = owner_request(tmp_path, {})
    assert got["agent_session_id"] == "s2" and len(got["text"]) <= 2048
    assert owner_request(tmp_path, {"TIRITH_AGENT_SESSION": "s1"})["text"] == "first"
    assert bound_text("a\nb\n" * 10000, 100).endswith("…") and len(bound_text("a\nb", 100)) == 3
