"""The on-call manager never blocks the chat; lanes get their own worktree and a detached start."""

from __future__ import annotations

import json
import os
import subprocess
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest

from conftest import GIT, make_hooks_mutable, make_repo, write_config, write_lane_manifest, write_plan, PLAN_ARGS
from tirith import dispatch as dispatch_mod
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith import managerd as managerd_mod
from tirith.cli import in_manager_session, main
from tirith.collector import close_dead_lanes
from tirith.home import load_config
from tirith.store import read_json, write_lane

NOW = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)
AGENTS = '[agents.anthropic]\ncommand = "agent-bin"\n'


@pytest.fixture
def project(home: Path, tmp_path: Path) -> Path:
    repo = make_repo(tmp_path / "demo")
    write_lane_manifest(repo)
    write_plan(repo)
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}], AGENTS)
    return repo


def chat(home: Path, text: str, at: datetime) -> None:
    inbox_mod.add_request(home, "chat", text=text, at=at.strftime("%Y-%m-%dT%H:%M:%SZ"))


def cycle(home: Path, spawned: list, killed: list, now: datetime, lanes: list) -> None:
    dispatch_mod.run(home, load_config(home), now, agents=[], lanes=lanes,
                     status_of=lambda lane, n: "working",
                     manager_spawn=lambda argv, h, log: spawned.append(argv) or 1,
                     manager_kill=lambda lane: killed.append(lane["id"]))


def manager_log(tmp_path: Path, finished: bool, written: datetime) -> Path:
    log = tmp_path / "manager.log"
    lines = [{"type": "assistant", "message": "working"}]
    if finished:
        lines.append({"type": "result", "result": "answered"})
    log.write_text("\n".join(json.dumps(l) for l in lines) + "\n", encoding="utf-8")
    os.utime(log, (written.timestamp(), written.timestamp()))
    return log


# -- 1. a manager turn ends with its result --------------------------------------------------


def test_result_event_ends_the_turn_and_the_next_message_starts_at_once(project, home, tmp_path) -> None:
    spawned: list = []
    killed: list = []
    chat(home, "one", NOW)
    cycle(home, spawned, killed, NOW, [])
    lane_id = spawned[0][spawned[0].index("--id") + 1]
    chat(home, "two", NOW + timedelta(seconds=5))
    log = manager_log(tmp_path, True, NOW + timedelta(seconds=8))
    alive = [{"id": lane_id, "pid": os.getpid(), "ended_at": None, "log": str(log)}]
    cycle(home, spawned, killed, NOW + timedelta(seconds=10), alive)  # alive, but its turn is over
    assert len(spawned) == 2 and "two" in Path(spawned[1][spawned[1].index("--brief") + 1]).read_text()
    assert killed == []  # only 2 s after the result: it still gets its time to exit


def test_a_manager_without_a_result_is_still_working(project, home, tmp_path) -> None:
    spawned: list = []
    chat(home, "one", NOW)
    cycle(home, spawned, [], NOW, [])
    lane_id = spawned[0][spawned[0].index("--id") + 1]
    chat(home, "two", NOW + timedelta(seconds=5))
    log = manager_log(tmp_path, False, NOW)
    alive = [{"id": lane_id, "pid": os.getpid(), "ended_at": None, "log": str(log)}]
    cycle(home, spawned, [], NOW + timedelta(seconds=90), alive)
    assert len(spawned) == 1


def test_manager_alive_30s_after_its_result_is_terminated(project, home, tmp_path) -> None:
    spawned: list = []
    killed: list = []
    chat(home, "one", NOW)
    cycle(home, spawned, killed, NOW, [])
    lane_id = spawned[0][spawned[0].index("--id") + 1]
    log = manager_log(tmp_path, True, NOW)
    alive = [{"id": lane_id, "pid": os.getpid(), "pgid": 2 ** 30, "ended_at": None, "log": str(log)}]
    cycle(home, spawned, killed, NOW + timedelta(seconds=29), alive)
    assert killed == []
    cycle(home, spawned, killed, NOW + timedelta(seconds=31), alive)
    assert killed == [lane_id]


def test_terminate_group_signals_the_process_group_only(monkeypatch) -> None:
    sent = []
    monkeypatch.setattr(os, "killpg", lambda pgid, sig: sent.append(("group", pgid)))
    monkeypatch.setattr(os, "kill", lambda pid, sig: sent.append(("pid", pid)))
    managerd_mod.terminate_group({"pid": 4242, "pgid": 4242})
    managerd_mod.terminate_group({"pid": 4343})
    assert sent == [("group", 4242), ("pid", 4343)]


def test_a_detached_lane_survives_the_manager_group(tmp_path) -> None:
    """The manager's group is terminated; a lane started with start_new_session is not in it."""
    manager = subprocess.Popen(["sleep", "30"], start_new_session=True)
    lane = subprocess.Popen(["sleep", "30"], start_new_session=True)
    try:
        managerd_mod.terminate_group({"pid": manager.pid, "pgid": manager.pid})
        manager.wait(timeout=5)
        assert lane.poll() is None
    finally:
        lane.kill()
        manager.kill()
        lane.wait()


# -- 4. a dead lane is closed ------------------------------------------------------------------


def test_dead_lane_without_ended_at_is_closed_as_stopped(home) -> None:
    dead = subprocess.Popen([sys.executable, "-c", "pass"])
    dead.wait()
    old = (NOW - timedelta(minutes=5)).strftime("%Y-%m-%dT%H:%M:%SZ")
    young = (NOW - timedelta(seconds=3)).strftime("%Y-%m-%dT%H:%M:%SZ")
    lanes = [
        {"id": "gone", "pid": dead.pid, "pgid": dead.pid, "started_at": old, "ended_at": None, "token": "t"},
        {"id": "alive", "pid": os.getpid(), "started_at": old, "ended_at": None},
        {"id": "fresh", "pid": dead.pid, "started_at": young, "ended_at": None},
        {"id": "nopid", "pid": None, "started_at": old, "ended_at": None},
    ]
    for lane in lanes:
        write_lane(home, lane)
    assert close_dead_lanes(home, lanes, NOW) == ["gone"]
    stored = read_json(home / "lanes" / "gone.json", {})
    assert stored["result"] == "stopped" and stored["ended_at"] == "2026-10-29T12:00:00Z"
    assert "token" not in stored
    assert read_json(home / "lanes" / "alive.json", {})["ended_at"] is None
    assert read_json(home / "lanes" / "fresh.json", {})["ended_at"] is None


# -- 2, 3, 5. lane run ------------------------------------------------------------------------


def _setup(home: Path, tmp_path: Path, monkeypatch) -> Path:
    repo = make_repo(tmp_path / "repo")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo), "color": "#123456"}])
    monkeypatch.delenv("TIRITH_LANE", raising=False)
    monkeypatch.delenv("TIRITH_ROLE", raising=False)
    monkeypatch.setattr("tirith.cli.verify_guard", lambda *a, **k: [])
    write_lane_manifest(repo)
    write_plan(repo)
    return repo


def test_in_manager_session() -> None:
    assert in_manager_session({"TIRITH_LANE": "manager-ab12"})
    assert in_manager_session({"TIRITH_ROLE": "manager"})
    assert not in_manager_session({"TIRITH_LANE": "w1"})
    assert not in_manager_session({})


def test_lane_run_in_a_manager_session_detaches_in_a_worktree(home, tmp_path, monkeypatch, capsys) -> None:
    repo = _setup(home, tmp_path, monkeypatch)
    monkeypatch.setenv("TIRITH_LANE", "manager-ab12")
    started: list = []
    monkeypatch.setattr(launch_mod, "spawn_detached", lambda argv, h, log: started.append(argv) or 1)
    try:
        code = main(["lane", "run", "--project", "demo", "--class", "stonewright", "--feature", "Fix it",
                     "--brief", str(repo / "brief.md"), *PLAN_ARGS, "--", "agent-bin", "go"])
    finally:
        make_hooks_mutable(repo)
    assert code == 0
    lane_id = capsys.readouterr().out.splitlines()[0]
    assert lane_id == "fix-it"
    argv = started[0]
    workspace = launch_mod.worktree_path(repo, lane_id)
    assert argv[argv.index("--workspace") + 1] == str(workspace) and "--no-detach" in argv
    assert argv[argv.index("--id") + 1] == lane_id and argv[-3:] == ["--", "agent-bin", "go"]
    assert workspace.is_dir() and workspace != repo
    branch = subprocess.run([GIT, "-C", str(workspace), "branch", "--show-current"],
                            capture_output=True, text=True).stdout.strip()
    assert branch == f"lane/{lane_id}"


def test_lane_run_by_tirith_or_no_detach_runs_in_the_foreground(home, tmp_path, monkeypatch, capsys) -> None:
    repo = _setup(home, tmp_path, monkeypatch)
    monkeypatch.setenv("TIRITH_LANE", "manager-ab12")
    started: list = []
    monkeypatch.setattr(launch_mod, "spawn_detached", lambda argv, h, log: started.append(argv) or 1)
    child = "import json;print(json.dumps({'type':'result','result':'ok'}))"
    try:
        code = main(["lane", "run", "--project", "demo", "--class", "stonewright", "--feature", "Fg",
                     "--brief", str(repo / "brief.md"), "--id", "fg", "--by-tirith", *PLAN_ARGS, "--", sys.executable, "-c", child])
    finally:
        make_hooks_mutable(repo)
    assert code == 0 and started == []
    assert "lane fg: done" in capsys.readouterr().out


def test_lane_run_outside_the_manager_blocks_unless_detach_is_asked(home, tmp_path, monkeypatch, capsys) -> None:
    repo = _setup(home, tmp_path, monkeypatch)
    started: list = []
    monkeypatch.setattr(launch_mod, "spawn_detached", lambda argv, h, log: started.append(argv) or 1)
    try:
        code = main(["lane", "run", "--project", "demo", "--class", "stonewright", "--feature", "Later",
                     "--brief", str(repo / "brief.md"), "--detach", *PLAN_ARGS, "--", "agent-bin"])
    finally:
        make_hooks_mutable(repo)
    assert code == 0 and len(started) == 1
    assert capsys.readouterr().out.splitlines()[0] == "later"


def test_lane_without_workspace_never_runs_in_the_main_checkout(home, tmp_path, monkeypatch, capsys) -> None:
    repo = _setup(home, tmp_path, monkeypatch)
    child = "import os,json;print(json.dumps({'type':'result','result':os.getcwd()}))"
    try:
        code = main(["lane", "run", "--project", "demo", "--class", "warden", "--feature", "Echo",
                     "--brief", str(repo / "brief.md"), "--id", "echo", *PLAN_ARGS, "--", sys.executable, "-c", child])
    finally:
        make_hooks_mutable(repo)
    assert code == 0
    lane = read_json(home / "lanes" / "echo.json", {})
    worktree = launch_mod.worktree_path(repo, "echo")
    assert lane["workspace"] == str(worktree.resolve())
    assert Path(lane["report"]).resolve() == worktree.resolve()
    assert (worktree / ".git").exists()


def test_the_manager_lane_itself_runs_in_the_main_checkout(home, tmp_path, monkeypatch) -> None:
    repo = _setup(home, tmp_path, monkeypatch)
    try:
        code = main(["lane", "run", "--project", "demo", "--class", "steward", "--feature", "Manager",
                     "--brief", str(repo / "brief.md"), "--id", "manager-1", "--by-tirith", *PLAN_ARGS, "--",
                     sys.executable, "-c", "pass"])
    finally:
        make_hooks_mutable(repo)
    assert code == 0
    assert read_json(home / "lanes" / "manager-1.json", {})["workspace"] == str(repo.resolve())
    assert not launch_mod.worktree_path(repo, "manager-1").exists()


def test_a_lane_stores_its_own_brief_as_its_request(home, tmp_path, monkeypatch) -> None:
    repo = _setup(home, tmp_path, monkeypatch)
    (home / "requests").mkdir()
    (home / "requests" / "s1.json").write_text(
        json.dumps({"prompt": "# Lane: Manager (on call)", "at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")}),
        encoding="utf-8")
    brief = tmp_path / "own-brief.md"
    brief.write_text("# Lane: Clickable refs\n\nMake paths clickable.\n", encoding="utf-8")
    try:
        main(["lane", "run", "--project", "demo", "--class", "warden", "--feature", "Refs", "--brief", str(brief),
              "--id", "refs", *PLAN_ARGS, "--", sys.executable, "-c", "pass"])
    finally:
        make_hooks_mutable(repo)
    request = read_json(home / "lanes" / "refs.json", {})["request"]
    assert request["text"].startswith("# Lane: Clickable refs") and "Manager" not in request["text"]


def test_the_manager_brief_says_to_detach_and_not_to_close_early() -> None:
    rules = " ".join(managerd_mod.RULES)
    assert "detached" in rules and "background task" in rules
    assert "settled only when the lane reports" in rules
