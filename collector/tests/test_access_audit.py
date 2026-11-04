from __future__ import annotations

import io
import json
from pathlib import Path

from tirith import access_audit, hooks
from tirith.store import lane_path, write_json_atomic


def _run(home: Path, payload: dict, monkeypatch, event="PreToolUse") -> None:
    monkeypatch.setenv("TIRITH_LANE", "w1")
    hooks.run_hook(home, "anthropic", event, io.StringIO(json.dumps(payload)))


def test_pretooluse_writes_access_line_without_secrets(tmp_path, monkeypatch) -> None:
    home = tmp_path / "home"
    write_json_atomic(lane_path(home, "w1"), {"id": "w1", "item": "Audit it", "item_step": "21"})
    cmd = "curl -H 'Authorization: Bearer abc123SECRET' https://x.io -d password=hunter2\nsecond line"
    _run(home, {"hook_event_name": "PreToolUse", "session_id": "s", "cwd": "/tmp",
                "tool_name": "Bash", "tool_input": {"command": cmd}}, monkeypatch)
    files = list((home / "access").glob("*.ndjson"))
    assert len(files) == 1
    row = json.loads(files[0].read_text().splitlines()[0])
    assert row["agent"] == "anthropic" and row["lane"] == "w1" and row["system"] == "network"
    assert row["plan"] == "21: Audit it" and row["ts"]
    text = files[0].read_text()
    assert "abc123SECRET" not in text and "hunter2" not in text and "second line" not in text


def test_other_events_are_not_logged(tmp_path, monkeypatch) -> None:
    home = tmp_path / "home"
    _run(home, {"hook_event_name": "PostToolUse", "cwd": "/tmp", "tool_name": "Edit",
                "tool_input": {"file_path": "/tmp/a"}}, monkeypatch, event="PostToolUse")
    assert not (home / "access").exists()


def test_system_and_scrub() -> None:
    assert access_audit.system_of("Bash", {"command": "git status"}) == "git"
    assert access_audit.system_of("Edit", {}) == "files"
    assert "s3cr3t" not in access_audit.scrub("API_KEY=s3cr3t run")
    assert "pw" not in access_audit.scrub("git clone https://u:pw@host/r")
