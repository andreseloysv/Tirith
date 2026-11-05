"""lane_room_timeout: the manager is told when a lane has waited 30 minutes for an answer in a room."""

from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path

from tirith import asks as asks_mod
from tirith import laneroom as laneroom_mod
from tirith import managerd
from tirith.cli import register_lane
from tirith.store import read_json, write_json_atomic, write_lane

ROOM = "team-room"


def _waiting(home: Path, since: str | None) -> None:
    register_lane(home, lane_id="a-1", project="demo", hero_class="archivist", feature="Ask",
                  workspace=None, log=None, pid=None, brief=None)
    laneroom_mod.mark(home, "a-1", ROOM)
    if since:
        lane = read_json(home / "lanes" / "a-1.json", {})
        lane["waiting_room"]["since"] = since
        write_lane(home, lane)


def _run_until_stopped(home: Path, polls: int) -> None:
    n = [0]

    def sleep(_s: float) -> None:
        n[0] += 1
        if n[0] >= polls:
            write_json_atomic(home / "rooms" / f"{ROOM}.json", {"room": ROOM, "messages": [
                {"n": 1, "from": "peer", "at": "2999-01-01T00:00:00Z", "text": "x"}]})

    (home / "rooms").mkdir(exist_ok=True)
    laneroom_mod.wait_for_answer(home, "a-1", sleep=sleep, clock=lambda: 0.0)


def test_lane_room_timeout_counts_from_the_wait_not_from_the_controller(home: Path) -> None:
    _waiting(home, "2000-01-01T00:00:00Z")
    _run_until_stopped(home, 3)
    (item,) = [i for i in asks_mod.queue_items(home) if i["kind"] == laneroom_mod.QUEUE_KIND]
    assert item["lane"] == "a-1" and ROOM in item["text"]


def test_lane_room_timeout_no_notice_before_30_minutes(home: Path) -> None:
    _waiting(home, None)
    _run_until_stopped(home, 3)
    assert not [i for i in asks_mod.queue_items(home) if i["kind"] == laneroom_mod.QUEUE_KIND]


def test_lane_room_timeout_item_wakes_the_manager(home: Path) -> None:
    _waiting(home, "2000-01-01T00:00:00Z")
    _run_until_stopped(home, 3)
    items = managerd.pending_items(home, datetime.now(timezone.utc))
    assert any(q["kind"] == laneroom_mod.QUEUE_KIND for q in items["queue"])
