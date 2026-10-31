"""Private rooms: create, invite, join and leave, and the ``private`` rows of ``state.json`` ``rooms[]``.

See ``docs/CONTRACT-private-rooms.md``. The crypto and the network live in ``sealed.py``; the per-room record
(``<home>/rooms/<room>.json``), the pairing and the thread that listens live in ``roombridge.py``. This module
only glues them to the CLI, the inbox requests ``room-new|room-invite|room-qr|room-leave`` and the state, and keeps no
copy of what the bridge owns (record format, ``ROOM_RE``, ``pairing_url``).

Nothing secret leaves this module: ``state.json`` gets the key ids (public) and, for the one unused pairing
code, the invite URL the contract names. A private key never leaves ``~/.tirith/am/``.
"""

from __future__ import annotations

import json
import threading
from pathlib import Path

from tirith import inbox as inbox_mod
from tirith import roombridge as bridge
from tirith import sealed
from tirith.home import now_iso
from tirith.rooms import MAX_TEXT

STATE_MESSAGES = 40
SEP = ":"  # peer code: <sealed-room>:<manager key id>
SHORT = 8
_handled: set[str] = set()
_lock = threading.Lock()


class PrivateRoomError(RuntimeError):
    """Something the owner can read (never holds a key or a pairing code)."""


def manager_name(home: Path) -> str:
    from tirith.home import load_config
    try:
        names = list(load_config(home).owner_names)
    except Exception:  # a broken config never blocks a room
        names = []
    return "tirith-manager" if not names else f"tirith-{names[0]}"[:40]


def peer_code(record: dict) -> str:
    return f"{record['id']}{SEP}{record['me']['key_id']}"


def parse_peer_code(code: str) -> tuple[str, str]:
    room, _, key_id = str(code or "").strip().partition(SEP)
    if not bridge.ROOM_RE.match(room) or (key_id and not bridge.KEY_ID_RE.match(key_id)):
        raise PrivateRoomError("that is not a room code (expected sealed-<room>:<key id>)")
    return room, key_id


def record_of(home: Path, room: str) -> dict:
    record = bridge.load_room(home, room)
    if not record:
        raise PrivateRoomError(f"this Tirith is not in {room}")
    return record


def new_room(home: Path, title: str = "", project: str = "") -> dict:
    """Make a private room, join it with this Tirith's key and prepare the first QR. Returns the record."""
    try:
        me = sealed.load_or_create_key(home, manager_name(home))
        room = sealed.new_room()
        sealed.join(me, room)
    except sealed.SealedError as exc:
        raise PrivateRoomError(f"could not create the room: {exc}") from exc
    record = bridge.new_record(room, {"name": me.name, "key_id": me.key_id}, (title or "").strip()[:80] or "Private room",
                               (project or "").strip())
    record.update({"role": "manager", "created_at": now_iso()})
    bridge.add_code(record)  # saved once, with its first QR: the room is never seen without it
    bridge.save_room(home, record)
    bridge.start(home)
    return record_of(home, room)


def new_invite_url(home: Path, room: str) -> str:
    """A fresh QR URL for one more phone."""
    record = record_of(home, room)
    if record.get("role") == "peer":
        raise PrivateRoomError("this Tirith joined as a peer; only the manager who made the room pairs phones")
    return bridge.renew_invite(home, room)


def open_url(home: Path, room: str) -> str:
    """The permanent link for a phone already in the room: no code, so it works as often as needed."""
    return bridge.chat_url(record_of(home, room))


def invite_manager(home: Path, room: str) -> str:
    """The code another Tirith joins with; kept in the record so the app can show it."""
    with bridge.record_lock:
        record = record_of(home, room)
        code = record["peer_code"] = peer_code(record)
        bridge.save_room(home, record)
    return code


def join_room(home: Path, code: str) -> dict:
    """Join as a peer (no orders from us, none to us) using a code printed by ``tirith room invite``."""
    room, manager_key = parse_peer_code(code)
    try:
        me = sealed.load_or_create_key(home, manager_name(home))
        sealed.join(me, room)
    except sealed.SealedError as exc:
        raise PrivateRoomError(f"could not join the room: {exc}") from exc
    with bridge.record_lock:
        record = bridge.load_room(home, room) or bridge.new_record(room, {"name": me.name, "key_id": me.key_id}, "Private room")
        if manager_key and not any(m.get("key_id") == manager_key for m in record["managers"]):
            record["managers"].append({"name": None, "key_id": manager_key, "last_seen": None})
        record.update({"role": "peer", "joined_at": now_iso(), "invite": {"url": "", "used": True}, "codes": []})
        bridge.save_room(home, record)
    bridge.start(home)
    return record_of(home, room)


def leave_room(home: Path, room: str) -> None:
    """Stop listening and forget the room here. The key stays (it is the identity, not the room's)."""
    if not bridge.ROOM_RE.match(room or ""):
        raise PrivateRoomError("invalid room")
    with bridge.record_lock:
        record_of(home, room)
        bridge.room_path(home, room).unlink(missing_ok=True)


# -- the inbox requests ``room-new``, ``room-invite``, ``room-qr`` and ``room-leave`` -------------------------------


def handle_requests(home: Path) -> None:
    """Do (in one thread, in order: it may use the network) every open ``room-*`` request not done yet."""
    todo = []
    for req in inbox_mod.requests(home):
        if req["kind"] not in inbox_mod.ROOM_KINDS or req["status"] != "open" or req["answers"]:
            continue
        with _lock:
            if req["id"] in _handled:
                continue
            _handled.add(req["id"])
        todo.append(req)
    if todo:
        threading.Thread(target=lambda: [_run_request(home, req) for req in todo], daemon=True, name="room-requests").start()


def _title(text: str) -> str:
    """The page sends ``{"title": ...}``; the CLI and ``tirith inbox add`` may send the title itself."""
    try:
        data = json.loads(text)
    except ValueError:
        return text
    return str(data.get("title") or "") if isinstance(data, dict) else text


def _run_request(home: Path, req: dict) -> None:
    try:
        if req["kind"] == "room-new":
            record = new_room(home, _title(req["text"]), "" if req["project"] == "all" else req["project"])
            text = f"Room {record['id']} is ready: scan its QR with your phone."
        elif req["kind"] == "room-qr":
            new_invite_url(home, req["text"])
            text = "A new one-use QR is ready for another phone."
        elif req["kind"] == "room-invite":
            text = f"Code for another manager (it runs `tirith room join <code>`): {invite_manager(home, req['text'])}"
        else:
            leave_room(home, req["text"])
            text = f"Left {req['text']}."
    except Exception as exc:  # the answer says why; never the traceback
        text = f"Could not do that: {exc}"
    try:
        inbox_mod.add_answer(home, req["id"], text, by="tirith")
        inbox_mod.mark_done(home, req["id"])
    except Exception:
        pass


# -- state.json ---------------------------------------------------------------------------------


def _message(i: int, raw: dict) -> dict:
    text = str(raw.get("text") or "")
    return {"n": int(raw.get("n") or i + 1), "at": raw.get("at"), "from": str(raw.get("author") or raw.get("from") or ""),
            "text": text[:MAX_TEXT], "re": [], "agreed": [], "signed": "self",  # the bridge keeps only what verified
            "kind": "message", "truncated": len(text) > MAX_TEXT, "from_device": bool(raw.get("from_device"))}


def state_row(record: dict) -> dict:
    """One ``rooms[]`` row of a private room. Built field by field, so no secret in the record can leak."""
    messages = [_message(i, m) for i, m in enumerate(record.get("messages") or []) if isinstance(m, dict)]
    devices = [{"name": d.get("name"), "key_id": d.get("key_id"), "paired_at": d.get("paired_at"),
                "last_seen": d.get("last_seen")} for d in record.get("devices") or [] if isinstance(d, dict)]
    managers = [{"name": m.get("name"), "key_id": m.get("key_id"), "last_seen": m.get("last_seen")}
                for m in record.get("managers") or [] if isinstance(m, dict)]
    me = record.get("me") or {}
    participants = [{"name": str(p.get("name") or (p.get("key_id") or "")[:SHORT]),
                     "kind": "owner" if p in devices else "external", "hero_id": None, "hero_name": None,
                     "last_seen": p.get("last_seen"), "messages": 0, "present": bool(p.get("last_seen"))}
                    for p in devices + managers]
    invite = record.get("invite") or {}
    row = {"id": record["id"], "url": f"https://agentsmessenger.com/r/{record['id']}",
           "title": record.get("title") or bridge.DEFAULT_TITLE, "project": record.get("project"),
           "summary": None, "summary_source": "none", "needs_owner": [], "message_count": len(messages),
           "last_message_at": messages[-1]["at"] if messages else None, "activity_per_hour": 0,
           "messages_last_day": 0, "polled_at": record.get("polled_at"), "error": record.get("last_error"),
           "participants": participants, "reply_edges": [], "messages": messages[-STATE_MESSAGES:],
           "private": True, "connected": bool(record.get("connected")) and not record.get("last_error"),
           "me": {"name": me.get("name"), "key_id": me.get("key_id")},
           "devices": devices, "managers": managers, "open_url": bridge.chat_url(record) if me.get("key_id") else None}
    if invite.get("url") and not invite.get("used") and record.get("role") != "peer":
        row["invite"] = {"url": invite["url"], "used": False}
    else:
        row["invite"] = {"url": None, "used": True}
    if record.get("peer_code"):
        row["peer_code"] = record["peer_code"]
    return row


def state_rows(home: Path) -> list[dict]:
    """The private rooms this Tirith is in."""
    return [state_row(r) for r in bridge.list_rooms(home) if r.get("private")]


def ensure_bridge(home: Path) -> None:
    """Called each cycle: a Tirith that restarted listens again to the rooms it is in (``start`` is idempotent;
    the thread lists the rooms on every round, so a room made later is polled without a restart)."""
    if bridge.list_rooms(home):
        bridge.start(home)
