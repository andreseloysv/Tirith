"""lane_room_wait: a lane waiting for an answer in a room is marked and resumed when it arrives."""

from __future__ import annotations

from pathlib import Path

from tirith import asks as asks_mod
from tirith import laneroom as laneroom_mod
from tirith import lanestop as lanestop_mod
from tirith import roombridge as rb
from tirith.cli import main, register_lane
from tirith.store import read_json, write_json_atomic

ROOM = "team-room"


def _lane(home: Path) -> str:
    register_lane(home, lane_id="a-1", project="demo", hero_class="archivist", feature="Ask",
                  workspace=None, log=None, pid=None, brief=None)
    return "a-1"


def _cache(home: Path, messages: list[dict]) -> None:
    (home / "rooms").mkdir(exist_ok=True)
    write_json_atomic(home / "rooms" / f"{ROOM}.json", {"room": ROOM, "messages": messages})


def _msg(n: int, who: str, at: str) -> dict:
    return {"n": n, "from": who, "at": at, "text": "SECRET TEXT"}


def _lane_file(home: Path) -> dict:
    return read_json(home / "lanes" / "a-1.json", {})


def test_lane_room_wait_command_marks_the_lane_and_the_state_label(home: Path) -> None:
    _lane(home)
    assert main(["--home", str(home), "lane", "await-room", ROOM, "--lane", "a-1", "--ignore", "me"]) == 0
    wait = _lane_file(home)["waiting_room"]
    assert wait["room"] == ROOM and wait["ignore"] == ["me"]
    assert laneroom_mod.label(_lane_file(home)) == f"esperando en la sala {ROOM}"
    assert main(["--home", str(home), "lane", "await-room", "bad room!", "--lane", "a-1"]) == 3


def test_lane_room_wait_resumes_on_an_answer_without_its_text(home: Path) -> None:
    _lane(home)
    laneroom_mod.mark(home, "a-1", ROOM, ["me"])
    since = _lane_file(home)["waiting_room"]["since"]
    _cache(home, [_msg(1, "other", "2000-01-01T00:00:00Z"), _msg(2, "me", "2999-01-01T00:00:00Z")])
    assert laneroom_mod.answer(home, _lane_file(home)["waiting_room"]) is None  # old and own do not count
    _cache(home, [_msg(1, "other", "2000-01-01T00:00:00Z"), _msg(2, "me", "2999-01-01T00:00:00Z"),
                  _msg(3, "peer", "2999-01-01T00:00:00Z")])
    text = laneroom_mod.wait_for_answer(home, "a-1", sleep=lambda s: None)
    assert text and ROOM in text and "number 3" in text and "peer" in text
    assert "SECRET TEXT" not in text
    assert "waiting_room" not in _lane_file(home) and since


def test_lane_room_wait_sealed_room_ignores_the_managers_own_answers(home: Path) -> None:
    room = "sealed-abc"
    record = rb.new_record(room, {"name": "boss", "key_id": "a" * 64})
    record["messages"] = [{"id": "m1", "at": "2999-01-01T00:00:00Z", "from": "boss", "text": "x"}]
    rb.save_room(home, record)
    _lane(home)
    laneroom_mod.mark(home, "a-1", room)
    assert laneroom_mod.answer(home, _lane_file(home)["waiting_room"]) is None
    record["messages"].append({"id": "m2", "at": "2999-01-01T00:00:00Z", "from": "phone", "text": "y"})
    rb.save_room(home, record)
    assert laneroom_mod.answer(home, _lane_file(home)["waiting_room"])["id"] == "m2"


def test_lane_room_wait_asks_the_manager_after_30_minutes_once_and_keeps_waiting(home: Path) -> None:
    _lane(home)
    laneroom_mod.mark(home, "a-1", ROOM)
    now = [0.0]
    ticks = []

    def sleep(s: float) -> None:
        now[0] += 600
        ticks.append(now[0])
        if len(ticks) == 8:
            _cache(home, [_msg(1, "peer", "2999-01-01T00:00:00Z")])

    text = laneroom_mod.wait_for_answer(home, "a-1", sleep=sleep, clock=lambda: now[0])
    assert text
    items = [i for i in asks_mod.queue_items(home) if i["kind"] == laneroom_mod.QUEUE_KIND]
    assert len(items) == 1 and items[0]["lane"] == "a-1"


def test_lane_room_wait_ends_when_the_lane_is_stopped_or_not_waiting(home: Path) -> None:
    _lane(home)
    assert laneroom_mod.wait_for_answer(home, "a-1", sleep=lambda s: None) is None  # no wait marked
    laneroom_mod.mark(home, "a-1", ROOM)
    lanestop_mod.stop_lane(home, "a-1", "owner", "test")
    assert laneroom_mod.wait_for_answer(home, "a-1", sleep=lambda s: None) is None
