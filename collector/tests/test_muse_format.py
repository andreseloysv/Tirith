"""Muse Code's real ``exec --json`` stream (a captured run, ``tests/fixtures/muse``) and its preset."""

from __future__ import annotations

import shutil
from pathlib import Path

from test_activity import _info, _lane
from test_agentrun import _config
from tirith import agentrun, livesteps
from tirith.logview import parse_lines

FIXTURE = Path(__file__).parent / "fixtures" / "muse" / "exec-json.jsonl"
LINES = FIXTURE.read_text(encoding="utf-8").splitlines()
WS = "/work/lane"


def test_preset_flags_from_the_real_help() -> None:
    cfg = _config(muse={"command": "agent-m"})
    cmd = agentrun.build_command(cfg, "muse", None, "hi", model="muse-spark-1.3", effort="minimal")
    assert cmd == ["agent-m", "exec", "--json", "--model", "muse-spark-1.3",
                   "--reasoning-effort", "minimal", "hi"]


def test_long_prompt_goes_through_prompt_file() -> None:
    cfg = _config(muse={"command": "agent-m"})
    long = "# Lane\n" + "x" * agentrun.STDIN_MIN
    cmd = agentrun.build_command(cfg, "muse", None, long)
    assert long not in cmd and cmd[-2] == "--prompt-file"
    assert Path(cmd[-1]).read_text(encoding="utf-8") == long
    Path(cmd[-1]).unlink()
    dash = agentrun.build_command(cfg, "muse", None, "-x")
    assert dash[-2] == "--prompt-file"
    Path(dash[-1]).unlink()


def test_usage_from_run_usage() -> None:
    usage = agentrun.parse_usage(FIXTURE)
    assert usage["input_tokens"] == 54237 and usage["output_tokens"] == 106
    assert usage["cache_tokens"] == 26993 and usage["model"] == "muse-spark-1.3"
    assert usage["cost_usd"] is None


def test_log_entries_hide_bookkeeping_and_show_tool_and_answer() -> None:
    entries = parse_lines(LINES, WS)
    visible = [e for e in entries if e["kind"] != "system"]
    assert [e["kind"] for e in visible] == ["edit", "done"]
    assert visible[0]["text"] == "hello.txt" and visible[0]["detail"] == "created"
    assert visible[1]["text"] == "done"
    system = [e for e in entries if e["kind"] == "system"]
    assert len(system) == 2 and "tokens" in system[-1]["text"]  # command accepted, usage


def test_failed_run_is_an_error() -> None:
    line = ('{"payload_type": "run.terminal.failed", "payload": '
            '{"terminal": "failed", "text": null, "reason": "no credits"}}')
    entry = parse_lines([line], WS)[0]
    assert entry["kind"] == "error" and entry["text"] == "no credits"


def test_livesteps_shows_tool_and_final_answer() -> None:
    steps, summary = livesteps.parse(LINES, WS)
    assert [s["kind"] for s in steps] == ["tool", "result", "text"]
    assert steps[0]["summary"] == "write_file hello.txt" and steps[2]["summary"] == "done"
    assert summary["done"] and not summary["error"] and summary["steps"] == 3


def test_activity_counts_the_tool_call(tmp_path: Path) -> None:
    log = tmp_path / "l.log"
    shutil.copy(FIXTURE, log)
    info = _info(_lane(tmp_path, log))
    assert [e["summary"] for e in info["events"]] == ["write_file hello.txt", "done"]
    assert {t["name"]: t["count"] for t in info["tools"]} == {"write_file": 1}
