"""Brain guesses about working agents go to the manager's queue, never to the owner."""

from __future__ import annotations

import os
from pathlib import Path

from tirith import asks as asks_mod
from tirith import dispatch as dispatch_mod
from tirith.cli import register_lane
from tirith.collector import Collector
from tirith.store import append_jsonl, read_json, write_json_atomic

from test_brain import FakeClient

NOW = dispatch_mod.parse_iso("2026-10-29T12:00:00Z")


def _open(home: Path) -> list[dict]:
    return [i for i in asks_mod.queue_items(home, only_open=True) if i["kind"] == asks_mod.AGENT_WAITING_KIND]


def _lane(home: Path, tmp_path: Path) -> Path:
    log = tmp_path / "a.log"
    log.write_text("working on migrations\nneeds a decision soon\n", encoding="utf-8")
    register_lane(home, lane_id="a-1", project="demo", hero_class="archivist", feature="Database",
                  workspace=None, log=str(log), pid=os.getpid(), brief=None)
    return log


def _settle(collector: Collector) -> dict:
    collector.cycle()
    collector.worker.drain(timeout_s=10)
    return collector.cycle()


def test_is_owner_item_needs_a_call_to_action_and_an_allowed_source() -> None:
    option = [{"label": "Yes", "value": "yes"}]
    assert asks_mod.is_owner_item({"source": "manual", "options": option})
    assert asks_mod.is_owner_item({"source": "guard", "actions": [{"command": "x"}]})
    assert not asks_mod.is_owner_item({"source": "manual", "options": []})
    assert not asks_mod.is_owner_item({"source": "brain", "options": option})
    assert not asks_mod.is_owner_item({"options": option})


def test_brain_wait_is_queued_once_for_the_manager_and_shown_in_its_context(home: Path, tmp_path: Path) -> None:
    _lane(home, tmp_path)
    collector = Collector(home, client=FakeClient())
    state = _settle(collector)
    assert not [w for w in state["waiting_on_you"] if w["source"] == "brain"]
    (item,) = _open(home)
    assert item["lane"] == "a-1" and item["project"] == "demo" and item["text"] == "Approve the schema choice"
    for _ in range(3):  # same lane + text: still one item
        collector.cycle()
    assert len(_open(home)) == 1
    context = dispatch_mod.manager_context(home, NOW)
    assert "Approve the schema choice" in context and f"tirith queue done {item['id']}" in context


def test_item_closes_when_the_lane_ends(home: Path, tmp_path: Path) -> None:
    _lane(home, tmp_path)
    collector = Collector(home, client=FakeClient())
    _settle(collector)
    assert len(_open(home)) == 1
    lane_file = home / "lanes" / "a-1.json"
    lane = read_json(lane_file, {})
    write_json_atomic(lane_file, {**lane, "ended_at": "2026-10-29T11:00:00Z", "result": "done"})
    collector.cycle()
    assert _open(home) == []
    done = [i for i in asks_mod.queue_items(home) if i["kind"] == "agent-waiting"]
    assert len(done) == 1 and done[0]["status"] == "done" and done[0]["result"] == asks_mod.AUTO_CLOSED


def test_item_closes_when_the_text_leaves_the_brains_answer_and_reopens_if_it_returns(home: Path) -> None:
    found = [{"project": "demo", "lane": "a-1", "text": "Approve the schema"}]
    asks_mod.sync_agent_waiting(home, found, set())
    asks_mod.sync_agent_waiting(home, found, set())
    assert len(_open(home)) == 1
    asks_mod.sync_agent_waiting(home, [], {"a-1"})  # brain not answered yet: kept
    assert len(_open(home)) == 1
    asks_mod.sync_agent_waiting(home, [], set())
    assert _open(home) == []
    asks_mod.sync_agent_waiting(home, found, set())
    assert len(_open(home)) == 1


def test_a_manager_closed_item_stays_closed_while_the_text_is_there(home: Path) -> None:
    found = [{"project": "demo", "lane": "a-1", "text": "Approve the schema"}]
    asks_mod.sync_agent_waiting(home, found, set())
    asks_mod.queue_done(home, _open(home)[0]["id"], "handled")
    asks_mod.sync_agent_waiting(home, found, set())
    assert _open(home) == []


def test_every_item_in_the_owner_list_passes_is_owner_item(home: Path, tmp_path: Path) -> None:
    """A generator that slips in a non-allow-listed source is caught and queued for the manager."""
    append_jsonl(home / "waits.jsonl", {"id": "w-1", "at": "2026-10-29T00:00:00Z", "project": "demo",
                                         "text": "Ship it?", "options": [{"label": "Ship", "value": "ship"}]})
    collector = Collector(home, client=None)
    shown = collector._enforce_cta([
        {"id": "x-1", "project": "demo", "text": "Guess", "source": "brain",
         "options": [{"label": "Got it", "value": "ack"}]},
        {"id": "x-2", "project": "demo", "text": "Real", "source": "manual",
         "options": [{"label": "Yes", "value": "yes"}]}])
    assert [w["id"] for w in shown] == ["x-2"]
    (queued,) = [q for q in asks_mod.queue_items(home) if q["kind"] == asks_mod.NOT_OWNER_KIND]
    assert queued["text"] == "Guess"
    state = collector.build_state()
    assert state["waiting_on_you"] and all(asks_mod.is_owner_item(w) for w in state["waiting_on_you"])
