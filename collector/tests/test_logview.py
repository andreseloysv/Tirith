"""Structured log entries: every kind, JSON never raw, bounded output."""

from __future__ import annotations

import json
from pathlib import Path

from tirith.logview import entries_from_events, log_entries, parse_lines

WS = "/work/space"


def _use(name: str, call_id: str, **input_) -> str:
    return json.dumps({"type": "assistant", "message": {"content": [
        {"type": "tool_use", "id": call_id, "name": name, "input": input_}]}})


def _res(call_id: str, text: str, error: bool = False) -> str:
    return json.dumps({"type": "user", "message": {"content": [
        {"type": "tool_result", "tool_use_id": call_id, "content": text, "is_error": error}]}})


def test_every_kind_from_a_transcript() -> None:
    lines = [
        json.dumps({"type": "assistant", "message": {"content": [{"type": "text", "text": "Starting."}]}}),
        _use("Bash", "c1", command=f"cd {WS} && ls"),
        _res("c1", "a\nb"),
        _use("Edit", "e1", file_path=f"{WS}/web/js/ui.js", old_string="a\nb", new_string="a\nb\nc\nd"),
        _use("Write", "e2", file_path=f"{WS}/new.py", content="x"),
        _use("Read", "r1", file_path=f"{WS}/README.md"),
        _res("r1", "file contents"),
        _use("Bash", "t1", command="uv run pytest -q"),
        _res("t1", "101 passed in 3s"),
        _use("Task", "s1", description="Check the schema", subagent_type="reviewer"),
        _use("Bash", "x1", command="false"),
        _res("x1", "Exit code 1\nboom", True),
        json.dumps({"type": "result", "result": "All done. More.",
                    "usage": {"input_tokens": 12000, "output_tokens": 400}, "duration_ms": 38000}),
    ]
    entries = parse_lines(lines, WS)
    assert [e["kind"] for e in entries] == [
        "say", "command", "output", "edit", "edit", "read", "test", "spawn", "command", "error", "done"]
    edit, write = entries[3], entries[4]
    assert (edit["text"], edit["detail"], edit["path"]) == ("web/js/ui.js", "+4 −2", f"{WS}/web/js/ui.js")
    assert write["detail"] == "created"
    assert entries[1]["text"] == "cd . && ls"
    test = entries[6]
    assert test["detail"] == "passed" and test["fields"] == {"passed": 101, "failed": 0}
    assert entries[7]["text"] == "Check the schema" and entries[7]["detail"] == "reviewer"
    assert entries[-1]["fields"] == {"usage": "12.4k tokens, 38 s"}


def test_json_is_never_shown_raw() -> None:
    lines = [
        json.dumps({"type": "system", "subtype": "init", "cwd": WS}),
        json.dumps({"type": "rate_limit_event", "status": "ok"}),
        json.dumps({"exit_code": 0, "ok": True, "finished_at": "2026-10-29T12:00:00Z"}),
        json.dumps({"passed": 7, "failed": 1}),
        json.dumps({"error": {"message": "disk full"}}),
        json.dumps({"input_tokens": 9000, "duration_ms": 4000}),
        "$ echo hi",
        '{"not": "closed"',
    ]
    entries = parse_lines(lines, WS)
    assert [e["kind"] for e in entries] == [
        "system", "system", "say", "test", "error", "system", "command", "say"]
    assert entries[2]["fields"]["exit_code"] == 0 and "exit_code" in entries[2]["detail"]
    assert entries[3]["detail"] == "failed" and entries[3]["fields"] == {"passed": 7, "failed": 1}
    assert entries[4]["text"] == "Error: disk full"
    assert entries[5]["text"] == "9.0k tokens, 4 s"


def test_json_output_of_a_command_becomes_a_card() -> None:
    lines = [_use("Bash", "c1", command="tirith status --json"),
             _res("c1", json.dumps({"agents": [1, 2, 3], "ok": True}))]
    entries = parse_lines(lines, WS)
    assert entries[1]["kind"] == "say" and entries[1]["fields"]["agents"] == [1, 2, 3]


def test_long_output_is_split_and_bounded() -> None:
    body = "\n".join(f"line {i} " + "x" * 400 for i in range(2000))
    entries = parse_lines([_use("Bash", "c1", command="cat big"), _res("c1", body)], WS)
    out = entries[1]
    assert out["kind"] == "output" and len(out["text"].splitlines()) == 6
    assert len(out["detail"]) <= 4096 and all(len(x) <= 300 for x in out["text"].splitlines())


def test_traceback_is_one_error_and_plain_text_is_say() -> None:
    lines = ["hello there", "Traceback (most recent call last):", '  File "a.py", line 1, in <module>',
             "    boom()", "ValueError: nope", "back to normal"]
    entries = parse_lines(lines, WS)
    assert [e["kind"] for e in entries] == ["say", "error", "say"]
    assert entries[1]["text"] == "ValueError: nope" and "File" in entries[1]["detail"]


def test_codex_style_items_and_muse_terminal() -> None:
    lines = [
        json.dumps({"type": "item.started", "item": {"id": "1", "type": "command_execution",
                                                     "command": "pytest"}}),
        json.dumps({"type": "item.completed", "item": {"id": "1", "type": "command_execution",
                                                       "command": "pytest", "exit_code": 1,
                                                       "aggregated_output": "2 failed"}}),
        json.dumps({"type": "item.completed", "item": {"id": "2", "type": "file_change",
                                                       "changes": [{"path": f"{WS}/a.py", "kind": "add"}]}}),
        json.dumps({"payload_type": "runtime.heartbeat", "payload": {}}),
        json.dumps({"payload_type": "run.terminal.completed", "payload": {"text": "Done."}}),
    ]
    entries = parse_lines(lines, WS)
    assert [e["kind"] for e in entries] == ["test", "error", "edit", "system", "done"]
    assert entries[0]["detail"] == "failed"
    assert entries[2]["text"] == "a.py" and entries[2]["detail"] == "created"


def test_log_entries_newest_first_and_missing(tmp_path: Path) -> None:
    log = tmp_path / "x.log"
    log.write_text("first\nsecond\n", encoding="utf-8")
    assert [e["text"] for e in log_entries(log)] == ["second", "first"]
    assert log_entries(tmp_path / "none.log") == [] and log_entries(None) == []


def test_entries_from_hook_events() -> None:
    events = [{"ts": "t1", "event": "PreToolUse", "tool": "Bash", "summary": "Bash: uv run pytest -q"},
              {"ts": "t2", "event": "PreToolUse", "tool": "Edit", "summary": "Edit web/js/ui.js"}]
    entries = entries_from_events(events, WS)
    assert [e["kind"] for e in entries] == ["edit", "test"]
