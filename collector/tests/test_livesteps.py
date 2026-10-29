"""The live transcript of a headless agent: stream-json lines to CLI-style steps, in the lane's detail file."""

from __future__ import annotations

import json
from pathlib import Path

from test_collector import _collector, _lane
from tirith import livesteps
from tirith.dispatch import load_state, save_state

TEXT = {"type": "assistant", "message": {"content": [{"type": "text", "text": "Checking the plan first.\nThen the lane."}]}}
THINK = {"type": "assistant", "message": {"content": [{"type": "thinking", "thinking": "", "signature": "xx"}]}}
READ = {"type": "assistant", "message": {"content": [
    {"type": "tool_use", "id": "t1", "name": "Read", "input": {"file_path": "/w/repo/docs/TODO.md"}}]}}
BASH = {"type": "assistant", "message": {"content": [
    {"type": "tool_use", "id": "t2", "name": "Bash", "input": {"command": "tirith inbox answer in-1 \"hi\"\necho second"}}]}}
GREP = {"type": "assistant", "message": {"content": [
    {"type": "tool_use", "id": "t3", "name": "Grep", "input": {"pattern": "^## 7", "path": "/w/repo/docs"}}]}}
EDIT = {"type": "assistant", "message": {"content": [
    {"type": "tool_use", "id": "t4", "name": "Edit", "input": {"file_path": "/w/repo/a.py", "old_string": "a", "new_string": "b"}}]}}
RES_OK = {"type": "user", "message": {"content": [
    {"type": "tool_result", "tool_use_id": "t1", "content": "l1\nl2\nl3\nl4\nl5"}]}}
RES_ERR = {"type": "user", "message": {"content": [
    {"type": "tool_result", "tool_use_id": "t2", "content": "boom: no such command", "is_error": True}]}}
RES_BLOCKS = {"type": "user", "message": {"content": [
    {"type": "tool_result", "tool_use_id": "t3", "content": [{"type": "text", "text": "found 2"}]}]}}
SYSTEM = {"type": "system", "subtype": "thinking_tokens", "estimated_tokens": 50}
RATE = {"type": "rate_limit_event", "rate_limit_info": {"status": "allowed"}}
DONE = {"type": "result", "subtype": "success", "is_error": False, "duration_ms": 48200,
        "total_cost_usd": 0.2134, "result": "All done."}


def lines(*events) -> list[str]:
    return [json.dumps(e) for e in events]


def test_steps_read_like_the_cli() -> None:
    steps, summary = livesteps.parse(
        lines(SYSTEM, THINK, TEXT, READ, RES_OK, BASH, RES_ERR, GREP, RES_BLOCKS, EDIT, RATE), "/w/repo")
    assert [(s["kind"], s["summary"]) for s in steps] == [
        ("text", "Checking the plan first.\nThen the lane."),
        ("tool", "Read docs/TODO.md"),
        ("result", "l1\nl2\nl3\n… +2 lines"),
        ("tool", 'Bash: tirith inbox answer in-1 "hi"'),
        ("result", "boom: no such command"),
        ("tool", 'Grep "^## 7" in docs'),
        ("result", "found 2"),
        ("tool", "Edit a.py"),
    ]
    assert [s.get("error") for s in steps if s["kind"] == "result"] == [False, True, False]
    assert steps[2]["name"] == "Read" and steps[4]["name"] == "Bash"
    assert summary["steps"] == 8 and summary["done"] is False


def test_result_event_gives_cost_and_duration_and_errors() -> None:
    _, summary = livesteps.parse(lines(TEXT, DONE))
    assert summary == {"steps": 1, "done": True, "error": False, "cost_usd": 0.2134, "duration_s": 48}
    _, bad = livesteps.parse(lines({**DONE, "is_error": True, "subtype": "error_during_execution"}))
    assert bad["error"] is True


def test_unparseable_lines_and_raw_json_never_show() -> None:
    steps, _ = livesteps.parse(["not json", "{broken", "", json.dumps([1, 2]), json.dumps(TEXT)])
    assert [s["kind"] for s in steps] == ["text"]
    assert all("{" not in s["summary"] for s in steps)


def test_secrets_are_masked() -> None:
    cmd = {"type": "assistant", "message": {"content": [{"type": "tool_use", "id": "x", "name": "Bash", "input": {
        "command": "curl 'tirith://inbox?kind=chat&token=abc123def&env=SECRET=1' "
                   "-H 'Authorization: Bearer abcdefghij1234567890'"}}]}}
    out = {"type": "user", "message": {"content": [{"type": "tool_result", "tool_use_id": "x",
           "content": "key sk-abcdefghijklmnopqrstu and ghp_abcdefghijklmnopqrstuvwx and GITHUB_TOKEN=zzz9"}]}}
    steps, _ = livesteps.parse(lines(cmd, out))
    text = " ".join(s["summary"] for s in steps)
    for secret in ("abc123def", "SECRET=1", "abcdefghij1234567890", "sk-abcdefghijklmnopqrstu",
                   "ghp_abcdefghijklmnopqrstuvwx", "zzz9"):
        assert secret not in text, secret
    assert "***" in text


def test_steps_for_stamps_first_seen_and_follows_the_growing_log(tmp_path: Path) -> None:
    log = tmp_path / "m.log"
    log.write_text("\n".join(lines(TEXT, READ)) + "\n", encoding="utf-8")
    steps, _ = livesteps.steps_for(log, None, "2026-10-29T10:00:00Z")
    assert [s["at"] for s in steps] == ["2026-10-29T10:00:00Z"] * 2   # already there: the fallback time
    log.write_text("\n".join(lines(TEXT, READ, RES_OK, DONE)) + "\n", encoding="utf-8")
    steps, summary = livesteps.steps_for(log, None, "2026-10-29T10:00:00Z")
    assert len(steps) == 3 and steps[2]["at"] != "2026-10-29T10:00:00Z" and summary["done"]
    assert livesteps.steps_for(tmp_path / "missing.log") == ([], {})


def test_steps_for_keeps_only_the_newest(tmp_path: Path) -> None:
    log = tmp_path / "m.log"
    many = [{"type": "assistant", "message": {"content": [{"type": "text", "text": f"step {i}"}]}} for i in range(200)]
    log.write_text("\n".join(lines(*many)) + "\n", encoding="utf-8")
    steps, summary = livesteps.steps_for(log)
    assert len(steps) == livesteps.STEPS_KEPT and steps[-1]["summary"] == "step 199" and summary["steps"] == 200


def test_collector_writes_a_small_steps_file_not_the_state(home: Path, tmp_path: Path) -> None:
    log = tmp_path / "manager-1.log"
    log.write_text("\n".join(lines(TEXT, READ, RES_OK)) + "\n", encoding="utf-8")
    _lane(home, "manager-1", log)
    state = _collector(home).cycle()
    row = state["agents"][0]
    assert row["live"]["steps"] == 3 and row["live"]["detail"]["path"].endswith("manager-1.steps.js")
    assert "Read" not in json.dumps(row["live"])   # the steps themselves are not in state.json
    body = Path(row["live"]["detail"]["path"]).read_text(encoding="utf-8")
    assert body.startswith("window.TIRITH_AGENT = ")
    data = json.loads(body[len("window.TIRITH_AGENT = "):].rstrip(";\n"))
    assert data["id"] == "manager-1.steps" and data["steps"][1]["summary"].startswith("Read ")


def test_manager_turn_is_linked_to_the_requests_it_answers(home: Path, tmp_path: Path) -> None:
    log = tmp_path / "manager-2.log"
    log.write_text(json.dumps(TEXT) + "\n", encoding="utf-8")
    _lane(home, "manager-2", log)
    dstate = load_state(home)
    dstate.setdefault("manager", {})["turns"] = {"manager-2": ["in-aa", "in-bb"]}
    save_state(home, dstate)
    row = _collector(home).cycle()["agents"][0]
    assert row["turn_requests"] == ["in-aa", "in-bb"]
