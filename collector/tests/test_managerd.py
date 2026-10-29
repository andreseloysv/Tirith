"""The manager inside Tirith: presence, the headless manager on a pending message, one at a time, detached lanes."""

from __future__ import annotations

import os
import subprocess
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest

from conftest import make_repo, write_config, write_lane_manifest
from tirith import dispatch as dispatch_mod
from tirith import hooks as hooks_mod
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith import managerd as managerd_mod
from tirith.cli import main
from tirith.collector import Collector
from tirith.home import load_config
from tirith.store import read_json

NOW = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)
AGENTS = '[agents.anthropic]\ncommand = "agent-bin"\n'


@pytest.fixture
def project(home: Path, tmp_path: Path) -> Path:
    repo = make_repo(tmp_path / "demo")
    write_lane_manifest(repo)
    (repo / "docs").mkdir()
    (repo / "docs" / "TODO.md").write_text("# TODO\n\n## 1. First\n\n- [ ] Open thing.\n", encoding="utf-8")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}], AGENTS)
    return repo


@pytest.fixture
def spawned() -> list[list[str]]:
    return []


def cycle(home: Path, spawned: list, now: datetime = NOW, lanes: list | None = None) -> None:
    dispatch_mod.run(home, load_config(home), now, agents=[], lanes=lanes or [],
                     status_of=lambda lane, n: "working",
                     manager_spawn=lambda argv, h, log: spawned.append(argv) or 1)


def chat(home: Path, text: str = "hello", at: datetime = NOW) -> str:
    return inbox_mod.add_request(home, "chat", text=text, at=at.strftime("%Y-%m-%dT%H:%M:%SZ"))["id"]


def brief_of(argv: list[str]) -> str:
    return Path(argv[argv.index("--brief") + 1]).read_text(encoding="utf-8")


# -- presence ---------------------------------------------------------------------------------


def test_attached_needs_a_fresh_heartbeat_and_a_live_pid(home: Path) -> None:
    assert not managerd_mod.attached(home, NOW)
    managerd_mod.heartbeat(home, "s1", NOW, pid=os.getpid())
    assert read_json(home / "manager.json", {})["session"] == "s1"
    assert managerd_mod.attached(home, NOW + timedelta(seconds=119))
    assert not managerd_mod.attached(home, NOW + timedelta(seconds=121))
    managerd_mod.heartbeat(home, "s1", NOW, pid=99999999)  # no such process
    assert not managerd_mod.attached(home, NOW)


def test_manager_hooks_write_the_heartbeat_and_lanes_do_not(home: Path) -> None:
    ev = {"event": "PostToolUse", "cwd": "/x", "agent_session_id": "abc"}
    lane_env = {"TIRITH_LANE": "w1", "TIRITH_ROLE": "manager"}
    assert hooks_mod._manager_inbox("anthropic", ev, home, environ=lane_env) == 0
    assert not (home / "manager.json").exists()
    assert hooks_mod._manager_inbox("anthropic", ev, home, environ={"TIRITH_ROLE": "manager"}) == 0
    assert read_json(home / "manager.json", {})["session"] == "abc"


# -- the headless manager ---------------------------------------------------------------------


def test_pending_message_starts_one_headless_manager(project, home: Path, spawned: list) -> None:
    rid = chat(home, "How is the plan?")
    cycle(home, spawned)
    assert len(spawned) == 1
    argv = spawned[0]
    assert argv[argv.index("--class") + 1] == "steward" and argv[argv.index("--feature") + 1] == "Manager"
    assert argv[argv.index("--agent") + 1] == "anthropic"
    brief = brief_of(argv)
    assert rid in brief and "How is the plan?" in brief and "tirith inbox answer" in brief
    assert "demo: 0/1 items done" in brief
    assert read_json(home / "dispatch.json", {})["manager"]["lane"] == argv[argv.index("--id") + 1]


def test_attached_manager_means_no_headless_one(project, home: Path, spawned: list) -> None:
    managerd_mod.heartbeat(home, "s1", NOW, pid=os.getpid())
    chat(home)
    cycle(home, spawned)
    assert spawned == []


def test_nothing_pending_starts_nothing(project, home: Path, spawned: list) -> None:
    cycle(home, spawned)
    assert spawned == []


def test_one_at_a_time_then_the_next_pending_starts_another(project, home: Path, spawned: list) -> None:
    chat(home, "one")
    cycle(home, spawned)
    lane_id = spawned[0][spawned[0].index("--id") + 1]
    chat(home, "two", NOW + timedelta(seconds=5))
    running = [{"id": lane_id, "pid": os.getpid(), "ended_at": None}]
    cycle(home, spawned, NOW + timedelta(seconds=90), running)  # still running: no second one
    assert len(spawned) == 1
    ended = [{"id": lane_id, "pid": os.getpid(), "ended_at": "2026-10-29T12:03:00Z", "result": "done"}]
    cycle(home, spawned, NOW + timedelta(seconds=200), ended)  # ended: the new message starts another
    assert len(spawned) == 2
    assert "two" in brief_of(spawned[1])


def test_a_lane_not_registered_yet_still_counts_as_running(project, home: Path, spawned: list) -> None:
    chat(home)
    cycle(home, spawned)
    chat(home, "again", NOW + timedelta(seconds=10))
    cycle(home, spawned, NOW + timedelta(seconds=70))
    assert len(spawned) == 1


def test_answered_requests_are_not_pending(project, home: Path, spawned: list) -> None:
    rid = chat(home)
    inbox_mod.add_answer(home, rid, "done", by="manager")
    cycle(home, spawned)
    assert spawned == []


def test_the_same_open_items_do_not_start_a_manager_forever(project, home: Path, spawned: list) -> None:
    chat(home)
    for n in range(6):
        cycle(home, spawned, NOW + timedelta(seconds=300 * n))
    assert len(spawned) == managerd_mod.MAX_TRIES


def test_queue_items_wake_the_manager_but_waiting_guesses_do_not(project, home: Path, spawned: list) -> None:
    from tirith import asks as asks_mod
    asks_mod.queue_add(home, "demo", "w1", "waiting?", kind=asks_mod.AGENT_WAITING_KIND)
    cycle(home, spawned)
    assert spawned == []
    asks_mod.queue_add(home, "demo", "w1", "Lane w1 failed", kind="failed")
    cycle(home, spawned)
    assert len(spawned) == 1
    assert "Lane w1 failed" in brief_of(spawned[0])


def test_auto_launch_off_disables_it(project, home: Path, spawned: list) -> None:
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(project)}],
                 AGENTS + "[dispatch]\nauto_launch = false\n")
    chat(home)
    cycle(home, spawned)
    assert spawned == []


def test_no_agent_command_is_recorded_not_raised(project, home: Path, spawned: list) -> None:
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(project)}])
    chat(home)
    cycle(home, spawned)
    assert spawned == []
    assert "no command" in read_json(home / "dispatch.json", {})["manager"]["error"]


def test_notes_roll_and_come_back_in_the_next_brief(project, home: Path, spawned: list) -> None:
    assert main(["manager-note", "launched the auditor"]) == 0
    chat(home)
    cycle(home, spawned)
    assert "launched the auditor" in brief_of(spawned[0])
    for _ in range(400):
        managerd_mod.add_note(home, "x" * 100, NOW)
    assert (home / "manager-notes.md").stat().st_size <= managerd_mod.NOTES_KEPT + 200


def test_state_presence(project, home: Path) -> None:
    state = Collector(home).build_state()
    assert state["manager"]["presence"] == {"mode": "on_call", "working": False, "lane": None}
    managerd_mod.heartbeat(home, "s", datetime.now(timezone.utc), pid=os.getpid())
    assert Collector(home).build_state()["manager"]["presence"]["mode"] == "live"


# -- detached lanes ---------------------------------------------------------------------------


def test_spawn_detached_is_its_own_session_with_no_stdin(home: Path, monkeypatch) -> None:
    seen: dict = {}

    class Proc:
        pid = 4242

    def fake(argv, **kw):
        seen.update(kw)
        return Proc()
    monkeypatch.setattr(subprocess, "Popen", fake)
    assert launch_mod.spawn_detached(["x"], home, home / "logs" / "l.log") == 4242
    assert seen["stdin"] == subprocess.DEVNULL and seen["start_new_session"] is True


@pytest.mark.skipif(os.name != "posix", reason="sessions are POSIX")
def test_spawn_detached_child_leads_its_own_session(home: Path) -> None:
    out = home / "sid.txt"
    code = "import os,sys;open(sys.argv[1],'w').write(str(os.getsid(0)==os.getpid()))"
    pid = launch_mod.spawn_detached([sys.executable, "-c", code, str(out)], home, home / "l.log")
    os.waitpid(pid, 0)
    assert out.read_text() == "True"
