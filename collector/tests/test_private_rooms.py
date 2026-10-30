"""Private rooms end to end, offline: ``tirith room`` + the ``room-*`` requests + the bridge + ``state.json``.

The platform is a small in-memory stand-in for ``tirith.sealed`` (rooms, keys, who a message is sealed to); the keys
are the real ones from ``sealed.load_or_create_key``. The real service is exercised by hand with ``am.mjs``."""

from __future__ import annotations

import json
import re
import threading
import time
import types
from datetime import datetime, timezone
from pathlib import Path
from urllib.parse import parse_qs, urlsplit

import pytest

from tirith import cli
from tirith import inbox as inbox_mod
from tirith import privaterooms as private_mod
from tirith import roombridge as rb
from tirith import sealed
from tirith.collector import Collector

START = rb.start
NOW = datetime(2026, 10, 30, 12, 0, tzinfo=timezone.utc)
PAGE_FRAGMENT = re.compile(r"^r=sealed-[a-z0-9_-]+&c=[0-9a-f]{32}&m=[0-9a-f]{8,64}$")  # chat core.js parseFragment


class Platform:
    """agentsmessenger.com for sealed rooms: a key must join before it can be written to; ``to`` seals for one key."""

    def __init__(self):
        self.rooms: dict[str, list[dict]] = {}
        self.count = 0

    def new_room(self, base_url=None):
        self.count += 1
        room = f"sealed-test{self.count}"
        self.rooms[room] = []
        return room

    def _keys(self, room):
        return {e["key_id"]: e for e in self.rooms[room] if e["kind"] == "announce"}

    def join(self, me, room, base_url=None):
        if me.key_id not in self._keys(room):
            self.rooms[room].append({"kind": "announce", "key_id": me.key_id, "name": me.name, "to": None, "text": "joined"})

    def say(self, me, room, text, to=None, base_url=None):
        self.join(me, room)
        others = {k for k in self._keys(room) if k != me.key_id}
        chosen = others if not to or to == "all" else {k for k in others if k.startswith(to)}
        if not chosen:
            raise sealed.SealedError(f"nobody to seal for in {room}")
        self.rooms[room].append({"kind": "message", "key_id": me.key_id, "name": me.name, "to": chosen, "text": text})

    def read(self, me, room, since=None, wait=0, base_url=None):
        out = []
        for n, e in enumerate(self.rooms[room], 1):
            if n > int(since or 0) and e["kind"] == "message" and (e["key_id"] == me.key_id or me.key_id in e["to"]):
                out.append(sealed.Message(id=f"{room}-{n}", author=e["name"], key_id=e["key_id"], text=e["text"],
                                          at="2026-10-30T10:00:00Z", verified=True, n=n))
        return out, len(self.rooms[room])

    def heard(self, me, room):
        return [e["text"] for e in self.rooms[room] if e["kind"] == "message" and me.key_id in e["to"]]


@pytest.fixture
def net(monkeypatch):
    platform = Platform()
    for name in ("new_room", "join", "say", "read"):
        monkeypatch.setattr(sealed, name, getattr(platform, name))
    monkeypatch.setattr(rb, "start", lambda home: None)  # no thread unless a test asks for it
    return platform


def key(tmp_path, name):
    return sealed.load_or_create_key(tmp_path / f"keys-{name}", name)


def fragment(url):
    parts = urlsplit(url)
    assert f"{parts.scheme}://{parts.netloc}{parts.path}" == "https://agentsmessenger.com/chat"
    return {k: v[0] for k, v in parse_qs(parts.fragment).items()}


def new_room(home, capsys, *args):
    assert cli.main(["room", "new", *args]) == 0
    capsys.readouterr()
    (record,) = rb.list_rooms(home)
    return record


def test_room_invite_url_is_the_chat_page_with_room_code_and_manager_key(home, net, capsys):
    record = new_room(home, capsys, "--title", "Phone")
    url = record["invite"]["url"]
    assert re.fullmatch(r"https://agentsmessenger\.com/chat#" + PAGE_FRAGMENT.pattern[1:-1] + "$", url)
    assert PAGE_FRAGMENT.match(urlsplit(url).fragment)
    frag = fragment(url)
    assert frag["r"] == record["id"] and frag["m"] == record["me"]["key_id"] and len(frag["m"]) == 64
    assert "github.io" not in url


def test_room_flow_new_pair_once_talk_sealed_answer_second_manager(home, tmp_path, net, capsys, monkeypatch):
    # new room: the record the bridge reads, made by the CLI
    record = new_room(home, capsys, "--title", "Phone")
    room, manager = record["id"], sealed.load_or_create_key(home, "tirith-manager")
    assert record["me"]["key_id"] == manager.key_id and record["title"] == "Phone" and record["connected"]
    assert rb.load_room(home, room)["codes"], "a code waits for a phone"
    frag = fragment(record["invite"]["url"])

    # the phone joins, then sends the pairing message sealed for the manager
    phone, thief = key(tmp_path, "phone"), key(tmp_path, "thief")
    net.join(phone, room)
    net.say(phone, room, f"tirith-pair {frag['c']}")
    rb.poll_room(home, room)
    rec = rb.load_room(home, room)
    assert [d["key_id"] for d in rec["devices"]] == [phone.key_id]
    assert rec["codes"] == [] and rec["invite"]["used"] is True, "code burnt"
    assert net.heard(phone, room) == [f"Connected to {manager.name} on {__import__('socket').gethostname()}"]
    net.join(thief, room)  # a later key reads nothing of what came before, and the answer was for the device only
    assert net.heard(thief, room) == []

    # a second use of the code is refused
    net.say(thief, room, f"tirith-pair {frag['c']}")
    rb.poll_room(home, room)
    assert [d["key_id"] for d in rb.load_room(home, room)["devices"]] == [phone.key_id]
    assert net.heard(thief, room) == []

    # the device's message reaches the inbox with its via, and the answer goes back sealed to that device
    net.say(phone, room, "how are the lanes?")
    rb.poll_room(home, room)
    (req,) = [r for r in inbox_mod.requests(home) if r["kind"] == "chat" and r["text"] == "how are the lanes?"]
    assert req["via"] == f"room:{room}" and req["from"] == "owner"
    inbox_mod.add_answer(home, req["id"], "All green.")
    rb.poll_room(home, room)
    assert net.heard(phone, room)[-1] == "All green." and "All green." not in net.heard(thief, room)

    # a second manager joins with the code of `tirith room invite` and is a peer
    assert cli.main(["room", "invite", room]) == 0
    code = capsys.readouterr().out.splitlines()[0]
    assert code == f"{room}:{manager.key_id}"
    other_home = tmp_path / "other-home"
    other_home.mkdir()
    monkeypatch.setenv("TIRITH_HOME", str(other_home))
    monkeypatch.setattr(private_mod, "manager_name", lambda h: "tirith-backup")
    assert cli.main(["room", "join", code]) == 0
    capsys.readouterr()
    joined = rb.load_room(other_home, room)
    assert joined["role"] == "peer" and joined["managers"][0]["key_id"] == manager.key_id and not joined["invite"]["url"]
    backup = key(other_home, "tirith-backup")
    monkeypatch.setenv("TIRITH_HOME", str(home))
    net.say(backup, room, "the lanes are fine here, anything to share?")
    rb.poll_room(home, room)
    info = [r for r in inbox_mod.requests(home) if r["from"] == "room:tirith-backup"]
    assert info and "has no authority" in info[0]["text"]
    inbox_mod.add_answer(home, info[0]["id"], "Nothing new.")
    rb.poll_room(home, room)
    assert net.heard(backup, room)[-1] == "Nothing new." and "Nothing new." not in net.heard(phone, room)
    assert [d["key_id"] for d in rb.load_room(home, room)["devices"]] == [phone.key_id], "a peer never becomes a device"


def test_room_new_request_json_title_invite_and_leave_requests(home, net):
    inbox_mod.add_request(home, "room-new", text=json.dumps({"title": "My phone"}))
    private_mod.handle_requests(home)
    deadline = time.time() + 5
    while time.time() < deadline and not rb.list_rooms(home):
        time.sleep(0.05)
    (record,) = rb.list_rooms(home)
    assert record["title"] == "My phone" and record["invite"]["url"].startswith("https://agentsmessenger.com/chat#r=")
    room = record["id"]
    inbox_mod.add_request(home, "room-invite", text=room)
    inbox_mod.add_request(home, "room-leave", text=room)
    private_mod.handle_requests(home)
    deadline = time.time() + 5
    while time.time() < deadline and any(r["status"] != "done" for r in inbox_mod.requests(home)):
        time.sleep(0.05)
    answers = {r["kind"]: r["answers"][0]["text"] for r in inbox_mod.requests(home)}
    assert room in answers["room-new"] and f"{room}:{record['me']['key_id']}" in answers["room-invite"]
    assert answers["room-leave"] == f"Left {room}." and rb.list_rooms(home) == []
    assert all(r["manager_pending"] is False for r in inbox_mod.requests(home))


def test_room_created_after_the_bridge_started_is_polled_without_a_restart(home, tmp_path, net, capsys, monkeypatch):
    monkeypatch.setattr(rb, "IDLE_S", 0.05)
    monkeypatch.setattr(rb, "MIN_PAUSE_S", 0.05)
    monkeypatch.setattr(rb, "start", START)
    private_mod.ensure_bridge(home)  # no room yet: nothing to start
    rb.start(home)
    try:
        record = new_room(home, capsys)
        phone = key(tmp_path, "phone")
        net.join(phone, record["id"])
        net.say(phone, record["id"], f"tirith-pair {fragment(record['invite']['url'])['c']}")
        deadline = time.time() + 5
        while time.time() < deadline and not rb.load_room(home, record["id"])["devices"]:
            time.sleep(0.05)
        assert len(rb.load_room(home, record["id"])["devices"]) == 1
    finally:
        rb.stop(home, 2.0)


def test_room_state_rows_follow_the_contract_and_leak_nothing(home, tmp_path, net, capsys):
    record = new_room(home, capsys, "--title", "Phone")
    room = record["id"]
    phone = key(tmp_path, "phone")
    net.join(phone, room)
    net.say(phone, room, f"tirith-pair {fragment(record['invite']['url'])['c']}")
    rb.poll_room(home, room)
    net.say(phone, room, "hola")
    rb.poll_room(home, room)
    state = Collector(home, client=None).build_state(NOW)
    row = next(r for r in state["rooms"] if r["id"] == room)
    assert row["private"] is True and row["connected"] is True and row["polled_at"]
    assert row["me"]["key_id"] == record["me"]["key_id"]
    assert [d["key_id"] for d in row["devices"]] == [phone.key_id]
    assert row["messages"][-1]["text"] == "hola" and row["messages"][-1]["from_device"] is True
    assert row["invite"] == {"url": None, "used": True}
    secret = json.loads((home / "am" / "tirith-manager.json").read_text())["secretKey"]
    assert secret not in json.dumps(state) and fragment(record["invite"]["url"])["c"] not in json.dumps(state)
    jsonschema = pytest.importorskip("jsonschema")
    jsonschema.validate(state, json.loads((Path(__file__).parent.parent / "state.schema.json").read_text()))

    unused = new_room_second(home, capsys)
    state = Collector(home, client=None).build_state(NOW)
    row = next(r for r in state["rooms"] if r["id"] == unused)
    assert re.fullmatch(r"https://agentsmessenger\.com/chat#r=sealed-[a-z0-9_-]+&c=[0-9a-f]{32}&m=[0-9a-f]{64}", row["invite"]["url"])
    jsonschema.validate(state, json.loads((Path(__file__).parent.parent / "state.schema.json").read_text()))


def new_room_second(home, capsys):
    assert cli.main(["room", "new"]) == 0
    capsys.readouterr()
    return rb.list_rooms(home)[-1]["id"]


def test_room_cli_invite_device_gives_a_fresh_code_and_leave_forgets(home, net, capsys):
    record = new_room(home, capsys)
    room, old = record["id"], fragment(record["invite"]["url"])["c"]
    assert cli.main(["room", "invite", room, "--device"]) == 0
    assert fragment(re.search(r"https://\S+", capsys.readouterr().out).group(0))["c"] != old
    assert cli.main(["room", "leave", room]) == 0
    assert rb.load_room(home, room) is None and cli.main(["room", "leave", room]) == 1
    assert cli.main(["room", "join", "nonsense"]) == 1


def test_room_leave_while_polling_does_not_bring_the_record_back(home, net, capsys):
    record = new_room(home, capsys)
    private_mod.leave_room(home, record["id"])
    assert rb.poll_room(home, record["id"]) == 0 and rb.list_rooms(home) == []


def test_room_network_error_is_reported_not_raised_by_the_cli(home, monkeypatch, capsys):
    def boom(base_url=None):
        raise sealed.SealedError("offline")
    monkeypatch.setattr(sealed, "new_room", boom)
    assert cli.main(["room", "new"]) == 1
    assert "offline" in capsys.readouterr().err and rb.list_rooms(home) == []
