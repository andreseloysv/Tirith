"""tirith hook: adapters, the command end to end, install/uninstall, collector attach."""

from __future__ import annotations

import io
import json
from pathlib import Path

from tests.conftest import write_config
from tirith import hooks
from tirith.cli import main
from tirith.collector import Collector
from tirith.store import read_json, read_jsonl

ANTHROPIC_PAYLOADS = {
    "SessionStart": {"hook_event_name": "SessionStart", "session_id": "s1", "cwd": "/w",
                     "source": "startup"},
    "UserPromptSubmit": {"hook_event_name": "UserPromptSubmit", "session_id": "s1", "cwd": "/w",
                         "prompt": "Fix the login bug\nit crashes on empty email"},
    "PreToolUse": {"hook_event_name": "PreToolUse", "session_id": "s1", "cwd": "/w",
                   "tool_name": "Edit", "tool_input": {"file_path": "/w/web/js/ui.js"}},
    "PostToolUse": {"hook_event_name": "PostToolUse", "session_id": "s1", "cwd": "/w",
                    "tool_name": "Bash", "tool_input": {"command": "uv run pytest\nmore"}},
    "SubagentStop": {"hook_event_name": "SubagentStop", "session_id": "s1", "cwd": "/w",
                     "last_assistant_message": "Found it.\nDetails"},
    "Stop": {"hook_event_name": "Stop", "session_id": "s1", "cwd": "/w"},
}


def test_anthropic_adapter_every_event() -> None:
    out = {name: hooks.adapt("anthropic", p) for name, p in ANTHROPIC_PAYLOADS.items()}
    assert out["SessionStart"]["summary"] == "Session startup"
    assert out["UserPromptSubmit"]["summary"] == "Fix the login bug"
    assert out["UserPromptSubmit"]["prompt"].startswith("Fix the login bug")
    assert out["PreToolUse"]["summary"] == "Edit web/js/ui.js"
    assert out["PostToolUse"]["summary"] == "Bash: uv run pytest"
    assert out["SubagentStop"]["summary"] == "Found it."
    assert out["Stop"]["summary"] == "Stop"
    assert all(o["agent_session_id"] == "s1" and o["event"] == n for n, o in out.items())


def test_anthropic_subagent_tools() -> None:
    for tool in ("Task", "Agent"):
        out = hooks.adapt("anthropic", {"hook_event_name": "PreToolUse", "tool_name": tool,
                                     "tool_input": {"description": "Audit the schema",
                                                    "subagent_type": "Explore", "prompt": "x"}})
        assert out["summary"] == "Sub-agent Explore: Audit the schema"
    out = hooks.adapt("anthropic", {"hook_event_name": "PreToolUse", "tool_name": "Task",
                                 "tool_input": {"prompt": "Do the thing"}})
    assert out["summary"] == "Sub-agent: Do the thing"


def test_generic_adapter_for_muse_and_unknown() -> None:
    payload = {"event": "PreToolUse", "sessionId": "m1", "tool": "Read",
               "input": {"path": "a.py"}, "cwd": "/w"}
    for agent in ("muse", "somebot"):
        out = hooks.adapt(agent, payload)
        assert (out["event"], out["tool"], out["summary"], out["agent_session_id"]) == \
            ("PreToolUse", "Read", "Read a.py", "m1")
    assert hooks.adapt("muse", None, "Stop")["event"] == "Stop"


def test_hook_command_end_to_end(home: Path, monkeypatch) -> None:
    monkeypatch.setenv("TIRITH_LANE", "w1")
    monkeypatch.setenv("TIRITH_SESSION", "sess")
    for payload in ANTHROPIC_PAYLOADS.values():
        monkeypatch.setattr("sys.stdin", io.StringIO(json.dumps(payload)))
        assert main(["hook", "--agent", "anthropic"]) == 0
    rows = read_jsonl(home / "events.jsonl")
    assert [r["event"] for r in rows] == list(ANTHROPIC_PAYLOADS)
    edit = rows[2]
    assert edit["lane"] == "w1" and edit["session"] == "sess" and edit["agent"] == "anthropic"
    assert edit["tool"] == "Edit" and edit["agent_session_id"] == "s1" and edit["ts"]
    request = read_json(home / "requests" / "s1.json", {})
    assert request["prompt"].startswith("Fix the login bug")


def test_hook_survives_garbage_and_missing_lane(home: Path, monkeypatch) -> None:
    monkeypatch.delenv("TIRITH_LANE", raising=False)
    for text in ("not json", "", "[1,2]", '{"tool_input": 5, "tool_name": 3}'):
        monkeypatch.setattr("sys.stdin", io.StringIO(text))
        assert main(["hook", "--agent", "anthropic", "--event", "Stop"]) == 0
    rows = read_jsonl(home / "events.jsonl")
    assert len(rows) == 4 and rows[0]["lane"] is None and rows[0]["event"] == "Stop"


def test_hook_never_fails_when_home_is_unwritable(tmp_path: Path, monkeypatch) -> None:
    blocker = tmp_path / "file"
    blocker.write_text("x")
    monkeypatch.setenv("TIRITH_HOME", str(blocker / "home"))
    monkeypatch.setattr("sys.stdin", io.StringIO("{}"))
    assert main(["hook", "--agent", "anthropic"]) == 0


def test_project_from_cwd(home: Path, tmp_path: Path) -> None:
    repo = tmp_path / "repo"
    (repo / "sub").mkdir(parents=True)
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}])
    row = hooks.record(home, "anthropic", {"hook_event_name": "Stop", "cwd": str(repo / "sub")})
    assert row["project"] == "demo"


def test_install_uninstall_merge_and_idempotent(tmp_path: Path) -> None:
    settings = tmp_path / "settings.json"
    mine = {"type": "command", "command": "echo mine"}
    original = {"model": "x", "hooks": {"Stop": [{"hooks": [mine]}],
                                        "Notification": [{"hooks": [mine]}]}}
    settings.write_text(json.dumps(original))
    for _ in range(2):
        assert main(["hook", "install", "--agent", "anthropic", "--settings", str(settings)]) == 0
    data = json.loads(settings.read_text())
    assert data["model"] == "x"
    assert set(data["hooks"]) == {"Notification", *hooks.INSTALL_EVENTS}
    stop = [h["command"] for g in data["hooks"]["Stop"] for h in g["hooks"]]
    assert stop[0] == "echo mine" and len(stop) == 2 and "hook --agent anthropic" in stop[1]
    for _ in range(2):
        assert main(["hook", "uninstall", "--agent", "anthropic", "--settings", str(settings)]) == 0
    assert json.loads(settings.read_text()) == original


def test_install_new_file_and_muse_schema_version(tmp_path: Path) -> None:
    cli_settings, muse = tmp_path / "a" / "c.json", tmp_path / "b" / "m.json"
    hooks.install("anthropic", cli_settings)
    hooks.install("muse", muse, tirith="/opt/bin/tirith")
    assert "hooks" in json.loads(cli_settings.read_text())
    data = json.loads(muse.read_text())
    assert data["schema_version"] == 1
    assert data["hooks"]["Stop"][0]["hooks"][0]["command"].startswith("/opt/bin/tirith hook")
    hooks.uninstall("muse", muse)
    assert json.loads(muse.read_text()) == {"schema_version": 1}


def test_collector_attaches_events_by_lane(home: Path, monkeypatch) -> None:
    main(["lane", "start", "--project", "demo", "--class", "stonewright",
          "--feature", "F", "--id", "w1"])
    monkeypatch.delenv("TIRITH_LANE", raising=False)
    for _ in range(3):
        hooks.record(home, "anthropic", {"hook_event_name": "Stop"})
    monkeypatch.setenv("TIRITH_LANE", "w1")
    for i in range(60):
        hooks.record(home, "anthropic", {"hook_event_name": "PreToolUse", "tool_name": "Edit",
                                      "tool_input": {"file_path": f"f{i}"}})
    monkeypatch.setenv("TIRITH_LANE", "ghost")
    hooks.record(home, "anthropic", {"hook_event_name": "Stop"})
    state = Collector(home, client=None).build_state()
    agent = state["agents"][0]
    assert len(agent["events"]) == 50 and agent["events"][-1]["summary"] == "Edit f59"
    assert len(state["loose_events"]) == 4
    assert state["pipelines"] == []
