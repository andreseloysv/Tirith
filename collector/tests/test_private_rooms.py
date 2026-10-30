"""Private rooms (CLI, room-new request, state rooms[]) against fakes of sealed.py and roombridge.py."""

from __future__ import annotations

import json
import sys
import time
import types
from datetime import datetime, timezone
from pathlib import Path

import pytest

from tirith import cli
from tirith import inbox as inbox_mod
from tirith import privaterooms as private_mod
from tirith.collector import Collector

NOW = datetime(2026, 10, 30, 12, 0, tzinfo=timezone.utc)
SECRET = "f" * 64          # a private key: must never be seen outside the fake
CODE = "c0de" * 8          # a pairing code: only inside the invite URL
KEY_ID = "ab12cd34" * 8


class Identity:
    def __init__(self, name):
        self.name, self.key_id, self.secret = name, KEY_ID, SECRET


@pytest.fixture
def fakes(home: Path, monkeypatch: pytest.MonkeyPatch):
    calls = {"joined": [], "started": 0, "codes": 0}
    sealed = types.ModuleType("tirith.sealed")

    class SealedError(RuntimeError):
        pass

    sealed.SealedError = SealedError
    sealed.load_or_create_key = lambda h, name: Identity(name)
    sealed.new_room = lambda base_url=None: "sealed-abc123"
    sealed.join = lambda ident, room, base_url=None: calls["joined"].append((ident.name, room))
    bridge = types.ModuleType("tirith.roombridge")

    def path(h, room):
        return Path(h) / "rooms" / f"{room}.json"

    def load_room(h, room):
        p = path(h, room)
        return json.loads(p.read_text()) if p.exists() else None

    def save_room(h, record):
        p = path(h, record["id"])
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(json.dumps(record))
        p.chmod(0o600)

    def list_rooms(h):
        return [json.loads(p.read_text()) for p in sorted((Path(h) / "rooms").glob("sealed-*.json"))]

    def new_pairing(h, room):
        calls["codes"] += 1
        record = load_room(h, room)
        record["pairing_code"] = f"{CODE}{calls['codes']}"
        save_room(h, record)
        return record["pairing_code"]

    bridge.load_room, bridge.save_room, bridge.list_rooms = load_room, save_room, list_rooms
    bridge.new_pairing = new_pairing
    bridge.pairing_url = lambda record, code: (
        f"https://andreseloysv.github.io/Tirith/chat/#r={record['id']}&c={code}&m={record['me']['key_id']}")
    bridge.start = lambda h: calls.__setitem__("started", calls["started"] + 1)
    monkeypatch.setitem(sys.modules, "tirith.sealed", sealed)
    monkeypatch.setitem(sys.modules, "tirith.roombridge", bridge)
    monkeypatch.setattr(private_mod, "_started", set())
    return types.SimpleNamespace(calls=calls, bridge=bridge, sealed=sealed)


def test_room_new_prints_url_and_saves_record(home, fakes, capsys) -> None:
    assert cli.main(["room", "new", "--title", "Phone"]) == 0
    out = capsys.readouterr().out
    assert f"#r=sealed-abc123&c={CODE}1&m={KEY_ID}" in out
    record = fakes.bridge.load_room(home, "sealed-abc123")
    assert record["private"] and record["title"] == "Phone" and record["me"]["key_id"] == KEY_ID
    assert fakes.calls["joined"] and fakes.calls["started"] == 1


def test_invite_code_joins_as_peer_and_is_not_the_pairing_code(home, fakes, capsys) -> None:
    cli.main(["room", "new"])
    capsys.readouterr()
    assert cli.main(["room", "invite", "sealed-abc123"]) == 0
    out = capsys.readouterr().out
    assert CODE not in out and f"sealed-abc123:{KEY_ID}" in out
    assert cli.main(["room", "join", f"sealed-abc123:{KEY_ID}"]) == 0
    record = fakes.bridge.load_room(home, "sealed-abc123")
    assert record["role"] == "peer" and "invite" not in record
    assert cli.main(["room", "join", "nonsense"]) == 1


def test_invite_device_gives_a_fresh_code(home, fakes, capsys) -> None:
    cli.main(["room", "new"])
    capsys.readouterr()
    cli.main(["room", "invite", "sealed-abc123", "--device"])
    assert f"c={CODE}2" in capsys.readouterr().out


def test_leave_forgets_the_room(home, fakes, capsys) -> None:
    cli.main(["room", "new"])
    assert cli.main(["room", "leave", "sealed-abc123"]) == 0
    assert fakes.bridge.load_room(home, "sealed-abc123") is None
    assert cli.main(["room", "leave", "sealed-abc123"]) == 1
    assert cli.main(["room", "leave", "../x"]) == 1


def test_state_rows_follow_the_contract_and_leak_nothing(home, fakes) -> None:
    cli.main(["room", "new", "--title", "Phone"])
    record = fakes.bridge.load_room(home, "sealed-abc123")
    record.update({"connected": True, "secret": SECRET, "private_key": SECRET,
                   "devices": [{"name": "iphone", "key_id": "d" * 64, "paired_at": "2026-10-30T11:00:00Z",
                                "last_seen": "2026-10-30T11:59:00Z", "secret": SECRET}],
                   "managers": [{"name": "other", "key_id": "e" * 64, "last_seen": None}],
                   "messages": [{"id": "1", "author": "iphone", "text": "hola", "at": "2026-10-30T11:59:00Z",
                                 "from_device": True, "verified": True}]})
    fakes.bridge.save_room(home, record)
    state = Collector(home, client=None).build_state(NOW)
    row = next(r for r in state["rooms"] if r["id"] == "sealed-abc123")
    assert row["private"] is True and row["connected"] is True
    assert row["me"] == {"name": "tirith-manager", "key_id": KEY_ID}
    assert row["devices"][0]["key_id"] == "d" * 64 and row["managers"][0]["name"] == "other"
    assert row["messages"][0]["from_device"] is True
    assert row["invite"] == {
        "url": f"https://andreseloysv.github.io/Tirith/chat/#r=sealed-abc123&c={CODE}1&m={KEY_ID}", "used": False}
    text = json.dumps(state)
    assert SECRET not in text
    assert text.count(CODE) == 1  # only inside the invite URL
    jsonschema = pytest.importorskip("jsonschema")
    schema = json.loads((Path(__file__).parent.parent / "state.schema.json").read_text())
    jsonschema.validate(state, schema)


def test_burned_code_shows_invite_used(home, fakes) -> None:
    cli.main(["room", "new"])
    record = fakes.bridge.load_room(home, "sealed-abc123")
    record["pairing_code"] = None
    fakes.bridge.save_room(home, record)
    assert private_mod.state_rows(home)[0]["invite"]["used"] is True


def test_missing_bridge_keeps_old_rooms_working(home, monkeypatch) -> None:
    monkeypatch.setitem(sys.modules, "tirith.roombridge", None)  # import fails
    assert private_mod.state_rows(home) == []
    assert Collector(home, client=None).build_state(NOW)["rooms"] == []


def test_room_new_request_creates_a_room_and_answers(home, fakes) -> None:
    req = inbox_mod.add_request(home, "room-new", text="Phone")
    assert req["step"] == "room-new"
    private_mod.handle_requests(home)
    private_mod.handle_requests(home)  # not twice
    for _ in range(100):
        got = inbox_mod.find_request(home, req["id"])
        if got and got["status"] == "done":
            break
        time.sleep(0.05)
    got = inbox_mod.find_request(home, req["id"])
    assert got["status"] == "done" and "sealed-abc123" in got["answers"][0]["text"]
    assert fakes.calls["codes"] == 1
    assert not inbox_mod.manager_pending(got)


def test_nothing_secret_in_logs_or_inbox(home, fakes, capsys) -> None:
    cli.main(["room", "new"])
    inbox_mod.add_request(home, "room-new", text="x")
    private_mod.handle_requests(home)
    time.sleep(0.3)
    captured = capsys.readouterr()
    for path in home.rglob("*"):
        if path.is_file():
            assert SECRET not in path.read_text(errors="ignore"), path
    assert SECRET not in captured.out + captured.err
    assert (home / "rooms" / "sealed-abc123.json").stat().st_mode & 0o777 == 0o600


def test_sealed_error_is_reported_not_raised(home, fakes, capsys) -> None:
    def boom(base_url=None):
        raise fakes.sealed.SealedError("network down")
    fakes.sealed.new_room = boom
    assert cli.main(["room", "new"]) == 1
    assert "network down" in capsys.readouterr().err
