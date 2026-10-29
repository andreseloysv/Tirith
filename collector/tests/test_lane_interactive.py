"""lane run --interactive: the child keeps the terminal, signals are forwarded, the lane always ends."""

from __future__ import annotations

import os
import signal
import sys
import threading
import time
from pathlib import Path

import pytest

from conftest import make_hooks_mutable
from test_lane_run import _lane_run_setup
from tirith import asks as asks_mod
from tirith.cli import main
from tirith.store import read_json, read_jsonl


def _interactive(repo: Path, lane_id: str, code: str) -> int:
    return main(["lane", "run", "--interactive", "--project", "demo", "--class", "warden",
                 "--feature", "Chat", "--brief", str(repo / "brief.md"), "--id", lane_id,
                 "--", sys.executable, "-c", code])


def test_interactive_inherits_env_and_uses_stop_event(home: Path, tmp_path: Path, capsys,
                                                      monkeypatch: pytest.MonkeyPatch) -> None:
    repo = _lane_run_setup(home, tmp_path, monkeypatch)
    mark = tmp_path / "mark"
    monkeypatch.setenv("MARK", str(mark))
    monkeypatch.setenv("EVENTS", str(home / "events.jsonl"))
    # The child writes the Stop event itself, as the agent's Stop hook would: after the lane started.
    # Written before the lane, its whole-second ts could fall before started_at and be ignored.
    child = ("import json,os,time;open(os.environ['MARK'],'w').write(os.environ['TIRITH_LANE']);"
             "open(os.environ['EVENTS'],'a').write(json.dumps({'kind':'hook','event':'Stop',"
             "'ts':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),"
             "'lane':os.environ['TIRITH_LANE'],'summary':'All wrapped up'})+'\\n')")
    try:
        code = _interactive(repo, "chat", child)
    finally:
        make_hooks_mutable(repo)
    assert code == 0 and mark.read_text() == "chat"
    lane = read_json(home / "lanes" / "chat.json", {})
    assert lane["result"] == "done" and lane["log"] is None and lane["report"] == "All wrapped up"
    assert "token" not in lane and lane["pid"]
    assert capsys.readouterr().out.splitlines()[0].startswith("lane chat: done,")


def test_interactive_failure_ends_lane(home: Path, tmp_path: Path, capsys,
                                       monkeypatch: pytest.MonkeyPatch) -> None:
    repo = _lane_run_setup(home, tmp_path, monkeypatch)
    try:
        code = _interactive(repo, "bad", "import sys;sys.exit(4)")
    finally:
        make_hooks_mutable(repo)
    assert code == 4
    assert read_json(home / "lanes" / "bad.json", {})["result"] == "failed"
    assert not read_jsonl(home / "waits.jsonl")
    assert [q["id"] for q in asks_mod.queue_items(home, only_open=True)] == ["sq-lane-bad"]


@pytest.mark.parametrize("name,exit_code", [("SIGINT", 0), ("SIGTERM", 0), ("SIGHUP", 7)])
def test_interactive_forwards_signals(home: Path, tmp_path: Path, capsys, name: str, exit_code: int,
                                      monkeypatch: pytest.MonkeyPatch) -> None:
    repo = _lane_run_setup(home, tmp_path, monkeypatch)
    ready = tmp_path / "ready"
    child = (f"import signal,sys,time,pathlib;"
             f"signal.signal(signal.{name}, lambda *a: sys.exit({exit_code}));"
             f"pathlib.Path({str(ready)!r}).write_text('x');time.sleep(30)")

    def poke() -> None:
        for _ in range(300):
            if ready.exists():
                break
            time.sleep(0.05)
        time.sleep(0.2)
        os.kill(os.getpid(), getattr(signal, name))

    thread = threading.Thread(target=poke)
    thread.start()
    try:
        code = _interactive(repo, "sig", child)
    finally:
        thread.join()
        make_hooks_mutable(repo)
    assert code == exit_code
    lane = read_json(home / "lanes" / "sig.json", {})
    assert lane["ended_at"] and lane["result"] == ("done" if exit_code == 0 else "failed")
    assert f"lane sig: {lane['result']}" in capsys.readouterr().out


def test_interactive_child_killed_by_signal_fails(home: Path, tmp_path: Path, capsys,
                                                  monkeypatch: pytest.MonkeyPatch) -> None:
    repo = _lane_run_setup(home, tmp_path, monkeypatch)
    try:
        code = _interactive(repo, "killed", "import os,signal;os.kill(os.getpid(),signal.SIGKILL)")
    finally:
        make_hooks_mutable(repo)
    assert code == 137
    assert read_json(home / "lanes" / "killed.json", {})["result"] == "failed"
