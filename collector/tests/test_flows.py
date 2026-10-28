"""Information flows: CLI, automatic events, state section and schema."""

from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from pathlib import Path

import pytest

from conftest import write_config
from tirith import cli
from tirith.collector import Collector
from tirith.store import read_jsonl

NOW = datetime(2026, 10, 28, 20, 0, tzinfo=timezone.utc)


def _flows(home: Path) -> list[dict]:
    return read_jsonl(home / "flows.jsonl")


def test_flow_command_records_and_validates(home: Path) -> None:
    assert cli.main(["flow", "--from", "steward", "--to", "agent:s-world", "--kind", "message",
                     "--text", "Owner wants a much smaller\ncompact world", "--ref", "x.md"]) == 0
    assert cli.main(["flow", "--from", "room:talk", "--to", "owner", "--text", "hi"]) == 0
    rows = _flows(home)
    assert [r["id"] for r in rows] == ["f-1", "f-2"]
    assert rows[0]["text"] == "Owner wants a much smaller compact world" and rows[0]["ref"] == "x.md"
    assert rows[1]["kind"] == "message"
    with pytest.raises(SystemExit):
        cli.main(["flow", "--from", "nobody", "--to", "owner", "--text", "x"])
    with pytest.raises(SystemExit):
        cli.main(["flow", "--from", "owner", "--to", "steward", "--kind", "gossip", "--text", "x"])
    with pytest.raises(SystemExit):
        cli.main(["flow", "--from", "owner", "--to", "steward", "--text", " "])


def test_automatic_flows(home: Path, tmp_path: Path) -> None:
    write_config(home, [{"id": "demo", "name": "Demo"}])
    brief = tmp_path / "brief.md"
    brief.write_text("do it", encoding="utf-8")
    cli.main(["lane", "start", "--project", "demo", "--class", "warden", "--feature", "Tests",
              "--id", "w-1", "--brief", str(brief), "--pid", str(os.getpid())])
    cli.main(["wait", "add", "Tick the box", "--project", "demo", "--id", "w-9"])
    cli.main(["wait", "done", "w-9"])
    cli.main(["lane", "end", "w-1", "--result", "failed", "--report", "\nTwo tests red\nmore"])
    cli.main(["pipeline", "declare", "rel", "--project", "demo", "--stages", "build"])
    cli.main(["run", "--project", "demo", "--pipeline", "rel", "--stage", "build", "--new-run",
              "--", "/usr/bin/true"])
    state = Collector(home, client=None).build_state(NOW)
    Collector(home, client=None).build_state(NOW)   # a second cycle adds nothing
    pairs = [(f["from"], f["to"], f["kind"]) for f in _flows(home) if f["kind"] != "alert"]
    assert pairs == [
        ("steward", "agent:w-1", "brief"), ("steward", "owner", "request"),
        ("owner", "steward", "answer"), ("agent:w-1", "steward", "report"),
        ("pipeline:rel#1", "steward", "report")]
    flows = state["flows"]
    hero = state["agents"][0]
    flows = [f for f in flows if f["kind"] != "alert"]
    assert flows[0]["to_name"] == f"{hero['name']} {hero['epithet']}"
    assert flows[1]["to_name"] == "You" and flows[2]["from_name"] == "You"
    assert flows[3]["text"] == "failed: Two tests red"
    assert flows[4]["from_name"] == "rel#1" and "build passed" in flows[4]["text"]
    assert flows[0]["ref"] == str(brief.resolve())


def test_guard_problem_raises_one_alert(home: Path, tmp_path: Path) -> None:
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(tmp_path / "missing")}])
    collector = Collector(home, client=None)
    collector.build_state(NOW)
    collector.build_state(NOW)
    alerts = [f for f in _flows(home) if f["kind"] == "alert"]
    assert len(alerts) == 1 and alerts[0]["from"] == "guard" and alerts[0]["to"] == "owner"


def test_state_flows_limited_and_malformed_skipped(home: Path) -> None:
    rows = [{"id": f"f-{i}", "at": "2026-10-28T19:00:00Z", "from": "steward", "to": "owner",
             "kind": "message", "text": f"m{i}", "ref": None} for i in range(1, 131)]
    rows.insert(5, {"id": "bad", "from": "nowhere", "to": "owner"})
    (home / "flows.jsonl").write_text("\n".join(json.dumps(r) for r in rows) + "\nnot json\n")
    flows = Collector(home, client=None).build_state(NOW)["flows"]
    assert len(flows) == 100 and flows[0]["id"] == "f-31" and flows[-1]["id"] == "f-130"


def test_flows_validate_against_schema(home: Path) -> None:
    jsonschema = pytest.importorskip("jsonschema")
    schema = json.loads((Path(__file__).resolve().parents[1] / "state.schema.json").read_text())
    cli.main(["flow", "--from", "steward", "--to", "owner", "--kind", "decision", "--text", "ok"])
    jsonschema.validate(Collector(home, client=None).build_state(NOW), schema)
