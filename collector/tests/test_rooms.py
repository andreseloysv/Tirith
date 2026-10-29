"""Rooms: incremental polling, back-off, aliases, brain jobs, CLI and schema (fake transport)."""

from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from pathlib import Path

import pytest

from conftest import write_config
from tirith import cli
from tirith import rooms as rooms_mod
from tirith.collector import Collector
from tirith.home import load_config
from tirith.rooms import RoomError, RoomPoller
from tirith.store import read_json

NOW = datetime(2026, 10, 28, 20, 0, tzinfo=timezone.utc)


def msg(n: int, sender: str, text: str = "hello", re=(), agreed=(), signed="platform",
        at: str | None = None) -> dict:
    return {"from": sender, "n": n, "id": f"id{n}", "signed_by": signed, "agent_id": "x",
            "kind": "message", "at": at or f"2026-10-28T19:{n % 60:02d}:00.000Z", "text": text,
            "sealed": False, "size": len(text), "truncated": False, "refs": [],
            "re": list(re), "agreed": list(agreed)}


class FakeTransport:
    """An in-memory room service; records every call, supports failures."""

    def __init__(self, messages: list[dict] | None = None):
        self.messages = list(messages or [])
        self.here: list[dict] = []
        self.gets: list[tuple[str, int | None]] = []
        self.posts: list[tuple[str, dict]] = []
        self.fail = False
        self.page_size = 1000

    def get(self, room_id: str, after: int | None) -> dict:
        self.gets.append((room_id, after))
        if self.fail:
            raise RoomError("agentsmessenger returned HTTP 503")
        fresh = [m for m in self.messages if m["n"] > (after or 0)][: self.page_size]
        return {"room": room_id, "count": len(self.messages), "here": self.here,
                "messages": fresh, "next": "unused"}

    def post(self, room_id: str, payload: dict) -> dict:
        self.posts.append((room_id, payload))
        return {"ok": True, "n": len(self.messages) + 1}


class Clock:
    def __init__(self) -> None:
        self.now = 1000.0

    def __call__(self) -> float:
        return self.now


def test_poll_is_incremental(home: Path) -> None:
    fake = FakeTransport([msg(1, "a"), msg(2, "b", re=[1])])
    poller = RoomPoller(home, fake)
    assert poller.poll_room("r1")
    fake.messages.append(msg(3, "a", agreed=["b"]))
    assert poller.poll_room("r1")
    assert fake.gets == [("r1", None), ("r1", 2)]
    cache = rooms_mod.read_room_cache(home, "r1")
    assert [m["n"] for m in cache["messages"]] == [1, 2, 3]
    assert cache["messages"][1]["re"] == [1]
    assert cache["messages"][2]["agreed"] == ["b"]
    assert cache["after"] == 3 and cache["error"] is None


def test_keeps_only_last_hundred(home: Path) -> None:
    fake = FakeTransport([msg(n, "a") for n in range(1, 131)])
    RoomPoller(home, fake).poll_room("big")
    kept = rooms_mod.read_room_cache(home, "big")["messages"]
    assert len(kept) == 100 and kept[0]["n"] == 31 and kept[-1]["n"] == 130


def test_follows_pages_of_long_rooms(home: Path) -> None:
    fake = FakeTransport([msg(n, "a") for n in range(1, 8)])
    fake.page_size = 3
    RoomPoller(home, fake).poll_room("paged")
    assert [m["n"] for m in rooms_mod.read_room_cache(home, "paged")["messages"]] == list(range(1, 8))
    assert [g[1] for g in fake.gets] == [None, 3, 6]


def test_reset_room_is_refetched_from_start(home: Path) -> None:
    fake = FakeTransport([msg(n, "a") for n in range(1, 6)])
    poller = RoomPoller(home, fake)
    poller.poll_room("r")
    fake.messages = [msg(1, "z", "fresh start")]
    assert poller.poll_room("r")
    kept = rooms_mod.read_room_cache(home, "r")["messages"]
    assert [(m["n"], m["from"]) for m in kept] == [(1, "z")]


def test_backoff_and_recovery(home: Path) -> None:
    fake = FakeTransport([msg(1, "a")])
    clock = Clock()
    poller = RoomPoller(home, fake, interval_s=15, clock=clock)
    assert poller.poll_due(["r"]) == ["r"]
    assert poller.poll_due(["r"]) == []          # not due yet
    clock.now += 14
    assert poller.poll_due(["r"]) == []
    clock.now += 2
    fake.fail = True
    assert poller.poll_due(["r"]) == ["r"]       # fails: next in 30 s
    cache = rooms_mod.read_room_cache(home, "r")
    assert cache["error"] == "agentsmessenger returned HTTP 503" and cache["messages"]
    clock.now += 29
    assert poller.poll_due(["r"]) == []
    clock.now += 2
    assert poller.poll_due(["r"]) == ["r"]       # fails again: next in 60 s
    assert poller.next_delay(1) == 30 and poller.next_delay(2) == 60
    assert poller.next_delay(20) == rooms_mod.MAX_BACKOFF_S
    clock.now += 61
    fake.fail = False
    assert poller.poll_due(["r"]) == ["r"]
    assert rooms_mod.read_room_cache(home, "r")["error"] is None
    clock.now += 14
    assert poller.poll_due(["r"]) == []          # back to the plain interval


def test_interval_never_below_ten_seconds(home: Path) -> None:
    assert RoomPoller(home, FakeTransport(), interval_s=1).interval_s == 10.0


def test_broken_transport_does_not_raise(home: Path) -> None:
    class Boom(FakeTransport):
        def get(self, room_id, after):  # noqa: ANN001
            raise ValueError("boom")

    assert RoomPoller(home, Boom()).poll_room("r") is False
    assert rooms_mod.read_room_cache(home, "r")["error"] == "ValueError"


def _state_with_room(home: Path, fake: FakeTransport, extra: str = "", lane: bool = True):
    write_config(home, [{"id": "demo", "name": "Demo"}],
                 extra='[owner]\nnames = ["andres"]\n\n'
                       '[[rooms]]\nid = "talk"\nproject = "demo"\n'
                       'aliases = { "muse" = "w6-perf" }\n' + extra)
    if lane:
        from tirith.cli import register_lane
        register_lane(home, lane_id="w6-perf", project="demo", hero_class="swiftrider",
                      feature="Speed", workspace=None, log=None, pid=os.getpid(), brief=None)
    RoomPoller(home, fake).poll_room("talk")
    return Collector(home, client=None, rooms_transport=fake).build_state(NOW)


def test_state_links_participants_and_replies(home: Path) -> None:
    fake = FakeTransport([
        msg(1, "visitor-bot", "Anyone seen the build?", at="2026-10-28T19:50:00.000Z"),
        msg(2, "muse", "Yes, it is green.", re=[1], at="2026-10-28T19:55:00.000Z"),
        msg(3, "andres", "thanks", re=[2], agreed=["muse"], signed="self",
            at="2026-10-28T19:58:00.000Z"),
        msg(4, "visitor-bot", "Hey andres, can you approve the release?", re=[3],
            at="2026-10-28T19:59:00.000Z"),
    ])
    fake.here = [{"name": "lurker", "waiting": True, "read_at": "2026-10-28T19:59:30.000Z"}]
    state = _state_with_room(home, fake)
    room = state["rooms"][0]
    assert room["id"] == "talk" and room["url"] == "https://agentsmessenger.com/r/talk"
    assert room["title"] == "Talk" and room["project"] == "demo"
    people = {p["name"]: p for p in room["participants"]}
    assert people["muse"]["kind"] == "tirith-agent" and people["muse"]["hero_id"] == "w6-perf"
    assert people["muse"]["hero_name"] == state["agents"][0]["name"]
    assert people["andres"]["kind"] == "owner"
    assert people["visitor-bot"]["kind"] == "external" and people["visitor-bot"]["messages"] == 2
    assert people["lurker"]["present"] is True and people["lurker"]["messages"] == 0
    edges = {(e["from"], e["to"]): e["count"] for e in room["reply_edges"]}
    assert edges[("muse", "visitor-bot")] == 1 and edges[("andres", "muse")] == 1
    assert room["messages"][2]["agreed"] == ["muse"] and room["messages"][2]["signed"] == "self"
    assert room["messages"][1]["re"] == [1]
    assert room["activity_per_hour"] == 4 and room["message_count"] == 4
    assert room["summary_source"] == "fallback" and "visitor-bot" in room["summary"]
    # No brain: the message naming the owner (and not answered by them) becomes a wait.
    assert [n["n"] for n in room["needs_owner"]] == [4]
    wait = next(w for w in state["waiting_on_you"] if w["id"] == "room-talk-4")
    assert wait["source"] == "room" and wait["link"] == room["url"] and wait["project"] == "demo"


def test_alias_to_a_lane_that_is_not_running(home: Path) -> None:
    fake = FakeTransport([msg(1, "muse", "hi")])
    state = _state_with_room(home, fake, lane=False)
    muse = state["rooms"][0]["participants"][0]
    assert muse["kind"] == "tirith-agent" and muse["hero_id"] == "w6-perf"
    assert muse["hero_name"] is None


def test_hero_name_links_without_alias(home: Path) -> None:
    from tirith.cli import register_lane
    write_config(home, [{"id": "demo", "name": "Demo"}], extra='[[rooms]]\nid = "talk"\n')
    register_lane(home, lane_id="w1", project="demo", hero_class="warden", feature="x",
                  workspace=None, log=None, pid=os.getpid(), brief=None)
    fake = FakeTransport()
    collector = Collector(home, client=None, rooms_transport=fake)
    hero = collector.build_state(NOW)["agents"][0]["name"]
    fake.messages = [msg(1, hero, "reporting"), msg(2, "w1", "same hero by lane id")]
    collector.rooms.poller.poll_room("talk")
    people = collector.build_state(NOW)["rooms"][0]["participants"]
    assert {p["hero_id"] for p in people} == {"w1"}


class ScriptedBrain:
    """A chat client answering the two room jobs."""

    def __init__(self) -> None:
        self.calls: list[str] = []

    def chat(self, messages, *, max_tokens: int = 512) -> str:  # noqa: ANN001
        prompt = messages[-1]["content"]
        self.calls.append(prompt)
        if "ONE plain sentence" in prompt:
            return "The agents are checking a release build and want a sign-off."
        return '[{"n": 4, "reason": "asks the owner to approve the release"}, {"n": 99, "reason": "x"}]'


def test_brain_summary_and_owner_detection_are_cached(home: Path) -> None:
    fake = FakeTransport([msg(1, "a", "build?"), msg(2, "b", "green"),
                          msg(3, "a", "owner, please approve the release")])
    fake.messages.append(msg(4, "a", "ping owner again"))
    write_config(home, [{"id": "demo", "name": "Demo"}],
                 extra='[owner]\nnames = ["andres"]\n[[rooms]]\nid = "talk"\n')
    RoomPoller(home, fake).poll_room("talk")
    brain = ScriptedBrain()
    collector = Collector(home, client=brain, rooms_transport=fake)
    first = collector.build_state(NOW)
    assert first["rooms"][0]["summary_source"] == "fallback"  # job still queued
    collector.worker.drain(10)
    state = collector.build_state(NOW)
    room = state["rooms"][0]
    assert room["summary_source"] == "muse" and "release build" in room["summary"]
    assert [n["n"] for n in room["needs_owner"]] == [4]       # unknown #99 dropped
    assert room["needs_owner"][0]["source"] == "brain"
    wait = next(w for w in state["waiting_on_you"] if w["id"] == "room-talk-4")
    assert wait["source"] == "brain"
    calls = len(brain.calls)
    collector.build_state(NOW)
    collector.worker.drain(10)
    assert len(brain.calls) == calls == 2                    # same messages: no new calls
    fake.messages.append(msg(5, "b", "done"))
    RoomPoller(home, fake).poll_room("talk")
    collector.build_state(NOW)
    collector.worker.drain(10)
    assert len(brain.calls) == 4                             # new message: both jobs rerun


def test_no_rooms_means_empty_section_and_no_network(home: Path) -> None:
    state = Collector(home, client=None).build_state(NOW)
    assert state["rooms"] == []


def test_collect_once_polls_rooms_synchronously(home: Path) -> None:
    fake = FakeTransport([msg(1, "a"), msg(2, "b")])
    write_config(home, [], extra='[[rooms]]\nid = "talk"\n')
    state = Collector(home, client=None, rooms_transport=fake).run(once=True)
    assert [m["n"] for m in state["rooms"][0]["messages"]] == [1, 2]
    assert (home / "state.json").exists()


# -- config + CLI ------------------------------------------------------------------------


def test_cli_add_list_remove_roundtrip(home: Path, capsys: pytest.CaptureFixture) -> None:
    write_config(home, [{"id": "demo", "name": "Demo"}], extra='[manager]\nname = "Me"\n')
    assert cli.main(["room", "add", "talk", "--project", "demo", "--alias", "muse=w6-perf"]) == 0
    assert cli.main(["room", "add", "second-room"]) == 0
    assert cli.main(["room", "add", "talk"]) == 0
    assert "already" in capsys.readouterr().out
    config = load_config(home)
    assert [r.id for r in config.rooms] == ["talk", "second-room"]
    assert config.rooms[0].project == "demo" and config.rooms[0].aliases == {"muse": "w6-perf"}
    assert config.manager_name == "Me" and config.projects[0].id == "demo"
    assert cli.main(["room", "list"]) == 0
    assert "talk" in capsys.readouterr().out
    assert cli.main(["room", "remove", "talk"]) == 0
    assert [r.id for r in load_config(home).rooms] == ["second-room"]
    assert cli.main(["room", "remove", "talk"]) == 1
    assert 'name = "Me"' in (home / "config.toml").read_text()


def test_cli_add_rejects_bad_room_id(home: Path) -> None:
    with pytest.raises(SystemExit):
        cli.main(["room", "add", "../etc/passwd"])


def test_cli_tail_and_say_use_the_transport(home: Path, monkeypatch: pytest.MonkeyPatch,
                                            capsys: pytest.CaptureFixture) -> None:
    fake = FakeTransport([msg(1, "a", "first"), msg(2, "b", "second", re=[1])])
    monkeypatch.setattr(cli, "_transport", lambda args: fake)
    assert cli.main(["room", "tail", "talk", "-n", "5"]) == 0
    out = capsys.readouterr().out
    assert "#2" in out and "re #1" in out and "first" in out
    assert fake.posts == []                                   # reading never posts
    assert cli.main(["room", "say", "talk", "--as", "my name!", "--re", "2", "--re", "1",
                     "Hello there"]) == 0
    assert fake.posts == [("talk", {"from": "my-name-", "text": "Hello there", "re": [1, 2]})]
    with pytest.raises(SystemExit):
        cli.main(["room", "say", "talk", "--as", "x", "  "])
    with pytest.raises(SystemExit):
        cli.main(["room", "say", "talk", "--as", "x", "y" * 2001])
    assert len(fake.posts) == 1


def test_collector_and_poller_never_post(home: Path) -> None:
    fake = FakeTransport([msg(1, "a", "ping owner andres")])
    write_config(home, [], extra='[owner]\nnames = ["andres"]\n[[rooms]]\nid = "talk"\n')
    Collector(home, client=None, rooms_transport=fake).run(once=True)
    assert fake.posts == []


def test_transport_only_talks_to_agentsmessenger(monkeypatch: pytest.MonkeyPatch) -> None:
    seen: list[str] = []

    class Resp:
        def __enter__(self):
            return self

        def __exit__(self, *a):
            return False

        def read(self, n=-1):
            return b'{"room": "r", "count": 0, "messages": []}'

    def fake_open(request, timeout=None):  # noqa: ANN001
        seen.append(request.full_url)
        return Resp()

    monkeypatch.setattr(rooms_mod.urllib.request, "urlopen", fake_open)
    rooms_mod.HttpTransport().get("meta-hackathon-ideas", 7)
    assert seen[0].startswith("https://agentsmessenger.com/r/meta-hackathon-ideas?format=json")
    assert "after=7" in seen[0]
    with pytest.raises(RoomError):
        rooms_mod.HttpTransport().get("bad/room", None)


def test_rooms_state_validates_against_schema(home: Path) -> None:
    jsonschema = pytest.importorskip("jsonschema")
    schema = json.loads((Path(__file__).resolve().parents[1] / "state.schema.json").read_text())
    fake = FakeTransport([msg(1, "muse", "hi"), msg(2, "andres", "yo", re=[1], signed="self"),
                          msg(3, "x", "andres?", agreed=["andres"])])
    state = _state_with_room(home, fake)
    jsonschema.validate(state, schema)
    assert state["rooms"][0]["messages"]
    assert read_json(home / "state.json", None) is None   # build_state does not write


def test_rooms_fixture_validates() -> None:
    jsonschema = pytest.importorskip("jsonschema")
    root = Path(__file__).resolve().parents[2]
    schema = json.loads((root / "collector" / "state.schema.json").read_text())
    fixture = json.loads((root / "fixtures" / "state.rooms.example.json").read_text())
    jsonschema.validate(fixture, schema)
    assert len(fixture["rooms"]) == 2
