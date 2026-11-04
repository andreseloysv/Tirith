"""Repeat detection: the same tool call again and again is reminded, then blocked (tirith.repeats)."""

from __future__ import annotations

import io
import json
from pathlib import Path

from tirith import hooks
from tirith import repeats
from tirith.cli import end_lane
from tirith.store import read_json, write_lane

GREP = {"pattern": "needle", "path": "src"}


def _call(home: Path, monkeypatch, capsys, event: str, tool_input: dict, lane: str = "w1",
          tool: str = "Grep") -> tuple[int, str, str]:
    monkeypatch.setenv("TIRITH_LANE", lane)
    payload = {"hook_event_name": event, "tool_name": tool, "tool_input": tool_input, "session_id": "s1",
               "cwd": str(home)}
    capsys.readouterr()
    code = hooks.run_hook(home, "anthropic", event, io.StringIO(json.dumps(payload)))
    out = capsys.readouterr()
    return code, out.out, out.err


def _round(home, monkeypatch, capsys, tool_input=GREP, lane="w1", tool="Grep"):
    """One tool call as the agent CLI makes it: PreToolUse, then PostToolUse."""
    pre = _call(home, monkeypatch, capsys, "PreToolUse", tool_input, lane, tool)
    post = _call(home, monkeypatch, capsys, "PostToolUse", tool_input, lane, tool) if pre[0] == 0 else None
    return pre, post


def _context(out: str) -> str:
    return json.loads(out)["hookSpecificOutput"]["additionalContext"] if out.strip() else ""


def test_the_same_call_is_reminded_at_three_and_five_then_blocked_at_eight(home, monkeypatch, capsys) -> None:
    rounds = [_round(home, monkeypatch, capsys) for _ in range(repeats.BLOCK_AT)]
    reminders = {n: _context(post[1]) for n, (_, post) in enumerate(rounds, 1) if post and _context(post[1])}
    assert sorted(reminders) == [3, 5]
    assert "3 times in a row" in reminders[3] and "blocked from 8" in reminders[5] and "needle" in reminders[5]
    assert all(pre[0] == 0 for pre, _ in rounds[:-1])
    code, _, err = rounds[-1][0]
    assert code == 2 and "8 times in a row" in err and "change your approach" in err


def test_a_different_call_in_between_starts_a_new_chain(home, monkeypatch, capsys) -> None:
    for _ in range(repeats.BLOCK_AT - 1):
        _round(home, monkeypatch, capsys)
    _round(home, monkeypatch, capsys, tool="Edit", tool_input={"file_path": "a.py", "old_string": "x",
                                                              "new_string": "y"})
    (pre, post) = _round(home, monkeypatch, capsys)
    assert pre[0] == 0 and not _context(post[1])


def test_same_tool_with_other_input_is_not_a_repeat(home, monkeypatch, capsys) -> None:
    for n in range(repeats.BLOCK_AT + 2):
        pre, post = _round(home, monkeypatch, capsys, tool_input={"pattern": f"p{n}"})
        assert pre[0] == 0 and not _context(post[1])


def test_lanes_are_counted_apart_and_the_manager_is_exempt(home, monkeypatch, capsys) -> None:
    for _ in range(repeats.BLOCK_AT - 1):
        _round(home, monkeypatch, capsys, lane="w1")
        _round(home, monkeypatch, capsys, lane="w2")
    for _ in range(repeats.BLOCK_AT + 2):
        pre, post = _round(home, monkeypatch, capsys, lane="manager-abc")
        assert pre[0] == 0 and not _context(post[1])
    assert _round(home, monkeypatch, capsys, lane="w2")[0][0] == 2


def test_gates_off_switches_the_rule_off(home, monkeypatch, capsys) -> None:
    (home / "config.toml").write_text("[gates]\nenabled = false\n", encoding="utf-8")
    for _ in range(repeats.BLOCK_AT + 1):
        assert _round(home, monkeypatch, capsys)[0][0] == 0


def test_ending_the_lane_keeps_the_totals_and_removes_the_chain(home, monkeypatch, capsys) -> None:
    write_lane(home, {"id": "w1", "project": "demo", "feature": "F"})
    for _ in range(repeats.BLOCK_AT + 1):
        _round(home, monkeypatch, capsys)
    end_lane(home, "w1", "failed", "stuck")
    lane = read_json(home / "lanes" / "w1.json", {})
    assert lane["repeats"] == {"nudged": 2, "blocked": 2, "longest": repeats.BLOCK_AT + 1}
    assert not (home / repeats.DIR / "w1.json").exists()


def test_a_lane_without_repeats_gets_no_totals(home, monkeypatch, capsys) -> None:
    write_lane(home, {"id": "w1", "project": "demo", "feature": "F"})
    _round(home, monkeypatch, capsys)
    end_lane(home, "w1", "done", "ok")
    assert "repeats" not in read_json(home / "lanes" / "w1.json", {})
