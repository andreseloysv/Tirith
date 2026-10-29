"""lane run: final answer extraction, the delegation tree and the end-to-end summary."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

from conftest import make_hooks_mutable, make_repo, write_config, write_lane_manifest, write_plan, REPORT_DONE
from tirith import asks as asks_mod
from tirith.cli import main, register_lane
from tirith.store import final_answer, read_json, read_jsonl


def _log(tmp_path: Path, lines: list) -> Path:
    path = tmp_path / "agent.log"
    path.write_text("\n".join(l if isinstance(l, str) else json.dumps(l) for l in lines) + "\n",
                    encoding="utf-8")
    return path


def test_answer_from_result_event(tmp_path: Path) -> None:
    log = _log(tmp_path, [{"type": "assistant", "message": "thinking"},
                          {"type": "result", "result": "first"},
                          {"type": "result", "result": "All done."}])
    assert final_answer(log) == ("All done.", True)


def test_answer_from_agent_message_item(tmp_path: Path) -> None:
    log = _log(tmp_path, [
        {"type": "item.completed", "item": {"type": "agent_message", "text": "early"}},
        {"type": "item.completed", "item": {"type": "command_execution", "text": "ls"}},
        {"type": "item.completed", "item": {"type": "agent_message", "text": "Fixed it."}}])
    assert final_answer(log) == ("Fixed it.", True)


def test_answer_from_copilot_final_answer_message(tmp_path: Path) -> None:
    log = _log(tmp_path, [
        {"type": "assistant.message", "data": {"phase": "commentary", "content": "working"}},
        {"type": "assistant.message_delta", "data": {"deltaContent": "do"}},
        {"type": "assistant.message", "data": {"phase": "final_answer", "content": "done"}},
        {"type": "result", "exitCode": 0, "usage": {"premiumRequests": 1}}])
    assert final_answer(log) == ("done", True)


def test_answer_from_terminal_payload(tmp_path: Path) -> None:
    log = _log(tmp_path, [
        "banner line",
        {"payload_type": "run.terminal.completed", "payload": {"text": "echo: hola"}}])
    assert final_answer(log) == ("echo: hola", True)


def test_answer_falls_back_to_log_tail(tmp_path: Path) -> None:
    log = _log(tmp_path, [f"line {n}" for n in range(60)])
    text, structured = final_answer(log)
    assert not structured
    assert text.splitlines()[-1] == "line 59" and len(text.splitlines()) == 40


def test_answer_missing_log(tmp_path: Path) -> None:
    assert final_answer(tmp_path / "nope.log") == ("", False)
    assert final_answer(None) == ("", False)


def _register(home: Path, lane_id: str) -> dict:
    return register_lane(home, lane_id=lane_id, project="demo", hero_class="warden",
                         feature=lane_id, workspace=None, log=None, pid=None, brief=None)


def test_parent_defaults_to_manager(home: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("TIRITH_LANE", raising=False)
    assert _register(home, "top")["parent"] == "manager"
    assert read_json(home / "lanes" / "top.json", {})["parent"] == "manager"


def test_nested_lanes_get_correct_parent(home: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.delenv("TIRITH_LANE", raising=False)
    _register(home, "l1")
    monkeypatch.setenv("TIRITH_LANE", "l1")
    _register(home, "l2")
    monkeypatch.setenv("TIRITH_LANE", "l2")
    _register(home, "l3")
    parents = {i: read_json(home / "lanes" / f"{i}.json", {})["parent"] for i in ("l1", "l2", "l3")}
    assert parents == {"l1": "manager", "l2": "l1", "l3": "l2"}
    flows = read_jsonl(home / "flows.jsonl")
    assert any(f.get("from") == "agent:l2" and f.get("to") == "agent:l3" for f in flows)


def _lane_run_setup(home: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    repo = make_repo(tmp_path / "repo")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo), "color": "#123456"}])
    monkeypatch.delenv("TIRITH_LANE", raising=False)
    monkeypatch.setattr("tirith.cli.verify_guard", lambda *a, **k: [])
    write_lane_manifest(repo)
    write_plan(repo, "".join(f"## {n}. Step {n}\n\n- [ ] Do the thing.\n\n" for n in (1, 14, 15, 16)))
    return repo


def test_lane_run_prints_summary_and_answer(home: Path, tmp_path: Path, capsys,
                                            monkeypatch: pytest.MonkeyPatch) -> None:
    repo = _lane_run_setup(home, tmp_path, monkeypatch)
    child = (REPORT_DONE + "import json,os;"
             "print(json.dumps({'type':'result','result':'Answer: '+os.environ['TIRITH_LANE']}))")
    try:
        code = main(["lane", "run", "--project", "demo", "--class", "warden", "--feature", "Echo", "--brief", str(repo / "brief.md"), "--step", "1", "--item", "Do the thing.",
                     "--id", "echo", "--", sys.executable, "-c", child])
    finally:
        make_hooks_mutable(repo)
    assert code == 0
    lines = capsys.readouterr().out.splitlines()
    assert lines[0].startswith("lane echo: done, 0 files +0 -0, ")
    assert lines[1] == "" and lines[2] == "Answer: echo"
    lane = read_json(home / "lanes" / "echo.json", {})
    assert lane["report"] == "Answer: echo" and lane["result"] == "done"
    assert not read_jsonl(home / "waits.jsonl")


def test_lane_run_failure_adds_wait(home: Path, tmp_path: Path, capsys,
                                    monkeypatch: pytest.MonkeyPatch) -> None:
    repo = _lane_run_setup(home, tmp_path, monkeypatch)
    try:
        code = main(["lane", "run", "--project", "demo", "--class", "warden", "--feature", "Boom", "--brief", str(repo / "brief.md"), "--step", "1", "--item", "Do the thing.",
                     "--id", "boom", "--", sys.executable, "-c", "import sys;sys.exit(3)"])
    finally:
        make_hooks_mutable(repo)
    assert code == 3
    assert "lane boom: failed" in capsys.readouterr().out
    assert not read_jsonl(home / "waits.jsonl")  # retrying is the manager's, not the owner's
    (item,) = asks_mod.queue_items(home, only_open=True)
    assert item["id"] == "sq-lane-boom" and "boom" in item["text"]


def test_lane_run_without_answer_adds_wait(home: Path, tmp_path: Path, capsys,
                                           monkeypatch: pytest.MonkeyPatch) -> None:
    repo = _lane_run_setup(home, tmp_path, monkeypatch)
    try:
        code = main(["lane", "run", "--project", "demo", "--class", "warden", "--feature", "Mute", "--brief", str(repo / "brief.md"), "--step", "1", "--item", "Do the thing.",
                     "--id", "mute", "--", sys.executable, "-c", "pass"])
    finally:
        make_hooks_mutable(repo)
    assert code == 0
    assert "lane mute: done" in capsys.readouterr().out
    assert not read_jsonl(home / "waits.jsonl")
    (item,) = asks_mod.queue_items(home, only_open=True)
    assert item["id"] == "sq-lane-mute" and "ended without an answer" in item["text"]


def _step_lane(home: Path, lane_id: str, step: str, ended: bool = False) -> None:
    register_lane(home, lane_id=lane_id, project="demo", hero_class="warden", feature=lane_id,
                  workspace=None, log=None, pid=None, brief=None, step=step)
    if ended:
        from tirith.cli import end_lane
        end_lane(home, lane_id, "done", "ok")


def _run_step(repo: Path, lane_id: str, step: str, *extra: str) -> int:
    try:
        return main(["lane", "run", "--project", "demo", "--class", "warden", "--feature", lane_id,
                     "--brief", str(repo / "brief.md"), "--id", lane_id, "--step", step, "--item", "Do the thing.", *extra,
                     "--", sys.executable, "-c", "print('x')"])
    finally:
        make_hooks_mutable(repo)


def test_lane_run_refuses_a_step_another_lane_is_working_on(home: Path, tmp_path: Path, capsys,
                                                            monkeypatch: pytest.MonkeyPatch) -> None:
    repo = _lane_run_setup(home, tmp_path, monkeypatch)
    _step_lane(home, "u-rustbase", "15")
    assert _run_step(repo, "collector-y-cli", "15") == 3
    assert "u-rustbase is already working on item \"Do the thing.\" of step 15" in capsys.readouterr().err
    assert not (home / "lanes" / "collector-y-cli.json").exists()


def test_lane_run_allows_another_step_a_finished_lane_and_a_continuation(
        home: Path, tmp_path: Path, capsys, monkeypatch: pytest.MonkeyPatch) -> None:
    repo = _lane_run_setup(home, tmp_path, monkeypatch)
    _step_lane(home, "other", "14")
    _step_lane(home, "finished", "15", ended=True)
    assert _run_step(repo, "mine", "15") == 0
    _step_lane(home, "busy", "16")
    assert _run_step(repo, "retry", "16", "--fallback-from", "busy") == 0


def test_lane_run_allows_another_item_of_a_step_a_lane_is_working_on(
        home: Path, tmp_path: Path, capsys, monkeypatch: pytest.MonkeyPatch) -> None:
    repo = _lane_run_setup(home, tmp_path, monkeypatch)
    write_plan(repo, "## 15. Step 15\n\n- [ ] Do the thing.\n- [ ] Do the other thing.\n\n")
    register_lane(home, lane_id="first", project="demo", hero_class="warden", feature="first", workspace=None,
                  log=None, pid=None, brief=None, step="15", item="Do the other thing.")
    assert _run_step(repo, "second", "15") == 0  # a different item of the same step
    capsys.readouterr()
    assert main(["lane", "run", "--project", "demo", "--class", "warden", "--feature", "third",
                 "--brief", str(repo / "brief.md"), "--id", "third", "--step", "15", "--item", "Do the other thing.",
                 "--", sys.executable, "-c", "print('x')"]) == 3
    assert "first is already working" in capsys.readouterr().err
