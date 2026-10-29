"""The dispatcher and the manager's hooks: finish/ask handled by code, the manager held to the inbox."""

from __future__ import annotations

import io
import json
import os
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest

from conftest import make_repo, write_config, write_lane_manifest
from tirith import dispatch as dispatch_mod
from tirith import hooks as hooks_mod
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith.cli import main
from tirith.collector import Collector
from tirith.home import load_config, now_iso
from tirith.store import read_json, read_jsonl

NOW = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)
PLAN = "# TODO\n\n## 1. First\n\n- [ ] Open thing.\n"


def _at(seconds_ago: float) -> str:
    return (NOW - timedelta(seconds=seconds_ago)).strftime("%Y-%m-%dT%H:%M:%SZ")


class Client:
    def __init__(self) -> None:
        self.prompts: list[str] = []

    def chat(self, messages, *, max_tokens: int = 512) -> str:
        if "owner of a project asks" not in messages[-1]["content"]:
            return "{}"
        self.prompts.append(messages[-1]["content"])
        return "One open item."


@pytest.fixture
def project(home: Path, tmp_path: Path):
    repo = make_repo(tmp_path / "demo")
    write_lane_manifest(repo)
    (repo / "docs").mkdir()
    (repo / "docs" / "TODO.md").write_text(PLAN, encoding="utf-8")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}])
    return repo


@pytest.fixture
def launches(monkeypatch):
    calls: list[tuple] = []

    def fake(home, project, step, request_id=None):
        calls.append((project, step))
        return f"lane-{len(calls)}"
    monkeypatch.setattr(dispatch_mod, "_default_launcher", fake)
    return calls


def _by_id(home: Path) -> dict:
    return {r["id"]: r for r in inbox_mod.requests(home)}


def _flows(home: Path) -> list[tuple]:
    return [(f["from"], f["to"], f["kind"]) for f in read_jsonl(home / "flows.jsonl") if f["from"] != "guard"]


def _append_config(home: Path, text: str) -> None:
    path = home / "config.toml"
    path.write_text(path.read_text() + "\n" + text + "\n")


# -- finish -------------------------------------------------------------------------------------


def test_finish_launches_a_lane_once(project, home: Path, launches) -> None:
    rid = inbox_mod.add_request(home, "finish", "demo", "1", at=_at(5))["id"]
    collector = Collector(home, client=None)
    for _ in range(3):  # idempotent across cycles
        collector.build_state(NOW)
    assert launches == [("demo", "1")]
    req = _by_id(home)[rid]
    assert req["status"] == "answered" and req["manager_pending"] is False
    assert req["answers"][0]["by"] == "tirith" and req["answers"][0]["auto"] is True
    assert "lane-1" in req["answers"][0]["text"]
    assert _flows(home) == [("owner", "tirith", "request"), ("tirith", "agent:lane-1", "brief")]
    assert read_json(home / "dispatch.json", {})["handled"] == {rid: "launched:lane-1"}


def test_finish_on_a_step_with_a_working_lane_answers_with_the_lane(project, home: Path) -> None:
    lane = {"id": "first", "project": "demo", "step": "1"}
    rid = inbox_mod.add_request(home, "finish", "demo", "1", at=_at(5))["id"]
    agents = [{"id": "first", "name": "Brann", "summary": "Writing the tests"}]
    dispatch_mod.run(home, load_config(home), NOW, agents=agents, lanes=[lane],
                     status_of=lambda lane, now: "working", launcher=lambda *a: pytest.fail("launched"))
    (answer,) = _by_id(home)[rid]["answers"]
    assert "Brann" in answer["text"] and "Writing the tests" in answer["text"] and "`first`" in answer["text"]
    assert ("tirith", "agent:first", "message") in _flows(home)


def test_finish_refused_answers_with_the_fix_and_asks_the_owner(project, home: Path, monkeypatch) -> None:
    def refuse(home, project, step, request_id=None):
        raise launch_mod.LaunchError("tirith check fails for 'demo', so no lane may start.")
    monkeypatch.setattr(dispatch_mod, "_default_launcher", refuse)
    rid = inbox_mod.add_request(home, "finish", "demo", "1", at=_at(5))["id"]
    collector = Collector(home, client=None)
    collector.build_state(NOW)
    state = collector.build_state(NOW)  # the owner item is on the list from the next cycle
    (answer,) = _by_id(home)[rid]["answers"]
    assert "tirith check --project demo" in answer["text"] and "no lane may start" in answer["text"]
    (item,) = [w for w in state["waiting_on_you"] if w["id"] == f"dispatch-{rid}"]
    assert [o["label"] for o in item["options"]] == ["Fix it", "Try again"]
    assert item["options"][0]["command"] == "tirith check --project demo"
    assert len(read_jsonl(home / "waits.jsonl")) == 1


def test_a_real_refusal_is_explained_with_a_fix(project, home: Path) -> None:
    rid = inbox_mod.add_request(home, "finish", "demo", "1", at=_at(5))["id"]
    Collector(home, client=None).build_state(NOW)
    text = _by_id(home)[rid]["answers"][0]["text"]
    assert text.startswith("Could not start an agent on demo step 1:") and "Fix: `" in text


def test_refusal_fixes() -> None:
    assert dispatch_mod.refusal_fix("p", "2", "no agent command: add ...").startswith("add [launch] command")
    assert dispatch_mod.refusal_fix("p", "2", "step 2 (x) is already done").startswith("tirith inbox list")
    assert dispatch_mod.refusal_fix("p", "2", "git worktree add failed") == "tirith launch --project p --step 2"


def test_auto_launch_off_leaves_finish_to_the_manager(project, home: Path, launches) -> None:
    _append_config(home, "[dispatch]\nauto_launch = false")
    rid = inbox_mod.add_request(home, "finish", "demo", "1", at=_at(5))["id"]
    Collector(home, client=None).build_state(NOW)
    assert launches == [] and _by_id(home)[rid]["manager_pending"] is True


def test_dispatch_off_does_nothing(project, home: Path, launches) -> None:
    _append_config(home, "[dispatch]\nenabled = false")
    inbox_mod.add_request(home, "finish", "demo", "1", at=_at(5))
    Collector(home, client=None).build_state(NOW)
    assert launches == [] and not (home / "dispatch.json").exists()


# -- ask / setup ----------------------------------------------------------------------------------


def test_ask_is_answered_by_the_brain_at_once_and_stays_pending_for_the_manager(project, home: Path) -> None:
    client = Client()
    collector = Collector(home, client=client)
    rid = inbox_mod.add_request(home, "ask", "demo", "1", "What is left?", at=_at(2))["id"]
    setup = inbox_mod.add_request(home, "setup", "demo", "1",
                                  '{"action": "run-stage", "name": "test"}', at=_at(2))["id"]
    collector.build_state(NOW)
    collector.worker.drain(5)
    for _ in range(2):
        collector.build_state(NOW)
    rows = _by_id(home)
    assert len(client.prompts) == 1
    assert rows[rid]["answers"][0]["by"] == "brain" and rows[rid]["answers"][0]["auto"] is True
    assert len(rows[rid]["answers"]) == 1
    assert rows[rid]["manager_pending"] is True and rows[setup]["manager_pending"] is True
    assert rows[setup]["answers"] == []
    flows = _flows(home)
    assert ("owner", "tirith", "request") in flows and ("tirith", "brain", "request") in flows
    assert ("brain", "owner", "answer") in flows
    inbox_mod.add_answer(home, rid, "Followed up.")
    assert _by_id(home)[rid]["manager_pending"] is False


def test_watch_still_emits_a_brain_answered_ask(home: Path) -> None:
    rid = inbox_mod.add_request(home, "ask", "demo", "1", "q")["id"]
    inbox_mod.add_answer(home, rid, "auto", by="brain", auto=True)
    seen: list[dict] = []
    inbox_mod.watch(home, seen.append, should_stop=lambda: True)
    assert [r["id"] for r in seen] == [rid]


# -- manager hooks --------------------------------------------------------------------------------

MANAGER = {"TIRITH_ROLE": "manager"}


def _hook(home: Path, event: str, payload: dict, environ: dict, capsys, agent: str = "anthropic"):
    row, _ = hooks_mod.build_event(home, agent, {"hook_event_name": event, **payload}, event, environ)
    saved = dict(os.environ)
    os.environ.clear()
    os.environ.update(environ)
    try:
        code = hooks_mod.decide(agent, row, payload, home)
    finally:
        os.environ.clear()
        os.environ.update(saved)
    out = capsys.readouterr()
    return code, out.out, out.err


def _pending(home: Path, kind: str = "ask", seconds_ago: float = 300) -> str:
    at = (datetime.now(timezone.utc) - timedelta(seconds=seconds_ago)).strftime("%Y-%m-%dT%H:%M:%SZ")
    return inbox_mod.add_request(home, kind, "demo", "1", "What is left?", at=at)["id"]


@pytest.mark.parametrize("event", ["UserPromptSubmit", "SessionStart"])
def test_manager_context_lists_pending_requests(home: Path, capsys, event: str) -> None:
    rid = _pending(home)
    code, out, err = _hook(home, event, {"cwd": str(home)}, MANAGER, capsys)
    assert code == 0 and err == ""
    data = json.loads(out)["hookSpecificOutput"]
    assert data["hookEventName"] == event
    assert f'tirith inbox answer {rid} "<markdown>"' in data["additionalContext"]
    assert f"tirith inbox done {rid}" in data["additionalContext"] and "What is left?" in data["additionalContext"]
    assert [r for r in read_jsonl(home / "events.jsonl") if r["kind"] == "inbox-injection"]


def test_manager_context_is_empty_when_nothing_is_pending(home: Path, capsys) -> None:
    rid = _pending(home)
    inbox_mod.add_answer(home, rid, "done")
    assert _hook(home, "UserPromptSubmit", {"cwd": str(home)}, MANAGER, capsys)[1] == ""


def test_other_sessions_are_never_touched(home: Path, capsys) -> None:
    _pending(home)
    assert _hook(home, "Stop", {"cwd": str(home)}, {}, capsys)[0] == 0
    lane = {"TIRITH_ROLE": "manager", "TIRITH_LANE": "w1"}
    assert _hook(home, "Stop", {"cwd": str(home)}, lane, capsys) == (0, "", "")
    assert _hook(home, "UserPromptSubmit", {"cwd": str(home)}, {}, capsys)[1] == ""
    assert _hook(home, "Stop", {"cwd": str(home)}, MANAGER, capsys, agent="muse")[0] == 0


def test_session_cwd_identifies_the_manager(home: Path, tmp_path: Path, capsys) -> None:
    folder = tmp_path / "manager"
    (home / "config.toml").write_text(f'[manager]\nsession_cwd = "{folder}"\n')
    _pending(home)
    assert _hook(home, "Stop", {"cwd": str(folder / "sub")}, {}, capsys)[0] == 2
    assert _hook(home, "Stop", {"cwd": str(tmp_path / "other")}, {}, capsys)[0] == 0


def test_stop_blocks_three_times_then_records_a_violation(home: Path, capsys) -> None:
    rid = _pending(home)
    for n in (1, 2, 3):
        code, _out, err = _hook(home, "Stop", {"cwd": str(home)}, MANAGER, capsys)
        assert code == 2 and rid in err and f"block {n} of 3" in err
    for _ in range(2):  # then it may stop, with one violation only
        assert _hook(home, "Stop", {"cwd": str(home)}, MANAGER, capsys)[0] == 0
    events = read_jsonl(home / "events.jsonl")
    assert [e["rule"] for e in events if e["kind"] == "violation"] == ["inbox-ignored"]
    assert len([e for e in events if e["kind"] == "inbox-block"]) == 3
    _pending(home)  # a new set of requests starts counting again
    assert _hook(home, "Stop", {"cwd": str(home)}, MANAGER, capsys)[0] == 2


def test_stop_ignores_young_answered_and_done_requests(home: Path, capsys) -> None:
    _pending(home, seconds_ago=20)
    answered = _pending(home)
    inbox_mod.add_answer(home, answered, "manager here")
    inbox_mod.mark_done(home, _pending(home))
    assert _hook(home, "Stop", {"cwd": str(home)}, MANAGER, capsys)[0] == 0


def test_a_brain_answer_does_not_release_the_manager(home: Path, capsys) -> None:
    rid = _pending(home)
    inbox_mod.add_answer(home, rid, "auto", by="brain", auto=True)
    assert _hook(home, "Stop", {"cwd": str(home)}, MANAGER, capsys)[0] == 2


def test_run_hook_end_to_end_exit_code(home: Path, capsys, monkeypatch) -> None:
    monkeypatch.setenv("TIRITH_ROLE", "manager")
    _pending(home)
    payload = json.dumps({"hook_event_name": "Stop", "cwd": str(home), "session_id": "s"})
    assert hooks_mod.run_hook(home, "anthropic", "Stop", io.StringIO(payload)) == 2
    assert "tirith inbox answer" in capsys.readouterr().err


# -- install --------------------------------------------------------------------------------------


def test_install_with_role_writes_the_env_in_the_command(tmp_path: Path, home: Path) -> None:
    settings = tmp_path / "settings.json"
    assert main(["hook", "install", "--agent", "anthropic", "--role", "manager", "--settings", str(settings)]) == 0
    hooks = json.loads(settings.read_text())["hooks"]
    commands = [h["command"] for g in hooks["Stop"] for h in g["hooks"]]
    assert len(commands) == 1 and commands[0].startswith("TIRITH_ROLE=manager ")
    plain = ["hook", "install", "--agent", "anthropic", "--settings", str(settings)]
    assert main(plain) == 0  # the same entry is refreshed, not doubled
    hooks = json.loads(settings.read_text())["hooks"]
    assert [h["command"] for g in hooks["Stop"] for h in g["hooks"]] == [
        commands[0].removeprefix("TIRITH_ROLE=manager ")]
    assert main(["hook", "uninstall", "--agent", "anthropic", "--settings", str(settings)]) == 0
    assert "hooks" not in json.loads(settings.read_text())


# -- escalation -----------------------------------------------------------------------------------


def test_escalation_after_ten_minutes_once(project, home: Path) -> None:
    sent: list[tuple] = []
    rid = inbox_mod.add_request(home, "ask", "demo", "1", "Are you there?", at=_at(601))["id"]
    young = inbox_mod.add_request(home, "ask", "demo", "1", "Too soon", at=_at(60))["id"]
    config = load_config(home)
    for _ in range(3):
        dispatch_mod.run(home, config, NOW, agents=[], lanes=[], status_of=lambda lane, now: "working",
                         notify=lambda title, message: sent.append((title, message)))
    assert sent == [("Tirith", "The manager has not answered: Are you there?")]
    collector = Collector(home, client=None)
    collector.build_state(NOW)
    state = collector.build_state(NOW)
    (item,) = [w for w in state["waiting_on_you"] if w["id"] == f"manager-silent-{rid}"]
    assert item["text"] == "The manager has not answered: Are you there?"
    assert [o["label"] for o in item["options"]] == ["Wait", "Start an agent", "Answer it with the brain"]
    assert ("tirith", "owner", "alert") in _flows(home)
    assert not [w for w in state["waiting_on_you"] if young in w["id"]]
    inbox_mod.add_answer(home, rid, "Here now.")  # the manager caught up: the item goes away
    collector.build_state(NOW)
    state = collector.build_state(NOW)
    assert not [w for w in state["waiting_on_you"] if w["id"] == f"manager-silent-{rid}"]


def test_notify_is_a_noop_off_macos(monkeypatch) -> None:
    monkeypatch.setattr(dispatch_mod.sys, "platform", "linux")
    monkeypatch.setattr(dispatch_mod.subprocess, "run", lambda *a, **k: pytest.fail("ran"))
    dispatch_mod.macos_notify("Tirith", 'say "hi"')


def test_inbox_brain_command_answers(project, home: Path, monkeypatch) -> None:
    rid = inbox_mod.add_request(home, "ask", "demo", "1", "What is left?")["id"]
    monkeypatch.setattr("tirith.brain.make_client", lambda config, api_key=None: Client())
    assert main(["inbox", "brain", rid]) == 0
    assert _by_id(home)[rid]["answers"][0]["by"] == "brain"
