"""An owner item whose Run actions all passed closes by itself; the manager hears about every result."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

from tirith import actions as actions_mod
from tirith import dispatch
from tirith import inbox as inbox_mod
from tirith.cli import add_wait
from tirith.collector import Collector
from tirith.store import append_jsonl, read_jsonl

NOW = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)


def _wait(home: Path, *commands: str) -> tuple[str, list[str]]:
    actions = actions_mod.make_actions(list(commands), [f"Step {i}" for i in range(len(commands))])
    wait_id = add_wait(home, "demo", "Please run it", actions=actions)
    return wait_id, [a["id"] for a in actions]


def _run(home: Path, action_id: str, status: str, at: str) -> None:
    append_jsonl(home / "inbox.jsonl", {"kind": "action-run", "action_id": action_id, "status": status,
                                        "exit_code": 0 if status == "passed" else 2, "at": at})


def _state(home: Path) -> dict:
    return Collector(home, client=None).build_state(NOW)


def _open(state: dict) -> list[str]:
    return [w["id"] for w in state["waiting_on_you"] if w["source"] == "manual"]


def test_a_wait_closes_when_every_action_passed(home: Path) -> None:
    wait_id, (first, second) = _wait(home, "true", "true")
    _run(home, first, "passed", "2026-10-29T11:00:00Z")
    assert _open(_state(home)) == [wait_id]  # one of two: still open
    _run(home, second, "passed", "2026-10-29T11:01:00Z")
    state = _state(home)
    assert _open(state) == []
    (done,) = state["answered_waits"]
    assert done["id"] == wait_id and done["answer"]["label"] == "✓ Step 0, Step 1"
    last = read_jsonl(home / "waits.jsonl")[-1]
    assert last["done_reason"] == "action-passed" and last["done_at"] == "2026-10-29T11:01:00Z"
    again = _state(home)  # stable on the next cycle, and the row is written once
    assert [w["id"] for w in again["answered_waits"]] == [wait_id]
    assert len(read_jsonl(home / "waits.jsonl")) == 2


def test_a_failed_run_keeps_the_wait_open_until_a_rerun_passes(home: Path) -> None:
    wait_id, (action,) = _wait(home, "false")
    _run(home, action, "failed", "2026-10-29T11:00:00Z")
    state = _state(home)
    assert _open(state) == [wait_id] and state["answered_waits"] == []
    (shown,) = [w for w in state["waiting_on_you"] if w["id"] == wait_id]
    assert shown["actions"][0]["last_run"]["status"] == "failed"
    _run(home, action, "passed", "2026-10-29T11:05:00Z")
    assert _open(_state(home)) == []


def test_watch_emits_each_result_once_and_the_manager_context_lists_failures(home: Path) -> None:
    wait_id, (ok, bad) = _wait(home, "true", "false")
    emitted, ticks = [], iter(range(4))

    def stop() -> bool:
        n = next(ticks)
        if n == 0:
            _run(home, ok, "running", "2026-10-29T11:00:00Z")
            _run(home, ok, "passed", "2026-10-29T11:00:01Z")
            _run(home, bad, "failed", "2026-10-29T11:00:02Z")
        return n >= 2

    inbox_mod.watch(home, emitted.append, poll_s=0, should_stop=stop, sleep=lambda s: None)
    results = [r for r in emitted if r["kind"] == "action-result"]
    assert [(r["wait"], r["action_id"], r["status"]) for r in results] == [
        (wait_id, ok, "passed"), (wait_id, bad, "failed")]
    line = json.loads(inbox_mod.watch_line(results[1]))
    assert line == {"kind": "action-result", "wait": wait_id, "project": "demo", "action_id": bad,
                    "label": "Step 1", "status": "failed", "exit_code": 2, "at": "2026-10-29T11:00:02Z"}
    text = dispatch.manager_context(home, NOW)
    assert wait_id in text and "Step 1" in text and "Step 0" not in text  # a pass never blocks
    inbox_mod.mark_done(home, wait_id, at="2026-10-29T11:30:00Z")
    assert dispatch.manager_context(home, NOW) == ""
    again: list[dict] = []  # history is not replayed on a later start unless still owed
    inbox_mod.watch(home, again.append, sleep=lambda s: None, should_stop=lambda: True)
    assert [r for r in again if r["kind"] == "action-result"] == []
