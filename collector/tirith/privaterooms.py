"""Private rooms: create, invite, join and leave, and the ``private`` rows of ``state.json`` ``rooms[]``.

See ``docs/CONTRACT-private-rooms.md``. The crypto and the network live in ``sealed.py``; the per-room record
(``<home>/rooms/<room>.json``) and the bridge thread that listens live in ``roombridge.py``. This module only
glues them to the CLI, the inbox request ``room-new`` and the state. Both are imported inside the functions so
Tirith still starts where one of them is missing.

Nothing secret leaves this module: ``state.json`` gets the key ids (public) and, for the one unused pairing code,
the invite URL the contract names. A private key never leaves ``~/.tirith/am/``.
"""

from __future__ import annotations

import re
import threading
from pathlib import Path

from tirith import inbox as inbox_mod
from tirith.home import now_iso
from tirith.rooms import MAX_TEXT

STATE_MESSAGES = 40
PREFIX = "sealed-"
ROOM_RE = re.compile(r"^sealed-[A-Za-z0-9._-]{1,120}$")
KEY_ID_RE = re.compile(r"^[0-9a-f]{8,128}$")
SEP = ":"  # invite code: <sealed-room>:<manager key id>
SHORT = 8
_started: set[str] = set()
_lock = threading.Lock()


class PrivateRoomError(RuntimeError):
    """Something the owner can read (never holds a key or a pairing code)."""


def _modules():
    from tirith import roombridge, sealed  # late: a sibling lane's modules
    return sealed, roombridge


def manager_name(home: Path) -> str:
    from tirith.home import load_config
    try:
        names = list(load_config(home).owner_names)
    except Exception:  # a broken config never blocks a room
        names = []
    return "tirith-manager" if not names else f"tirith-{names[0]}"[:40]


def valid_room(room: str) -> bool:
    return bool(ROOM_RE.match(room or ""))


def invite_code(record: dict) -> str:
    return f"{record['id']}{SEP}{record['me']['key_id']}"


def parse_invite(code: str) -> tuple[str, str]:
    room, _, key_id = str(code or "").strip().partition(SEP)
    if not valid_room(room) or (key_id and not KEY_ID_RE.match(key_id)):
        raise PrivateRoomError("that is not a room code (expected sealed-<room>:<key id>)")
    return room, key_id


def _record(home: Path, room: str) -> dict:
    _, bridge = _modules()
    record = bridge.load_room(home, room)
    if not record:
        raise PrivateRoomError(f"this Tirith is not in {room}")
    return record


def _start_bridge(home: Path) -> None:
    _, bridge = _modules()
    bridge.start(home)


def new_room(home: Path, title: str = "", project: str = "") -> dict:
    """Make a private room, join it with this Tirith's key and prepare the first QR. Returns the record."""
    sealed, bridge = _modules()
    try:
        me = sealed.load_or_create_key(home, manager_name(home))
        room = sealed.new_room()
        sealed.join(me, room)
    except sealed.SealedError as exc:
        raise PrivateRoomError(f"could not create the room: {exc}") from exc
    record = bridge.load_room(home, room) or {}
    record.update({"id": room, "private": True, "title": (title or "").strip() or "Private room",
                   "project": (project or "").strip() or None, "created_at": now_iso(), "role": "manager",
                   "me": {"name": me.name, "key_id": me.key_id}})
    for key in ("devices", "managers"):
        record.setdefault(key, [])
    bridge.save_room(home, record)
    _renew_invite(home, record)
    _start_bridge(home)
    return bridge.load_room(home, room)


def _renew_invite(home: Path, record: dict) -> str:
    """A fresh one-use code; the URL is kept in the record (mode 600) so the app can draw its QR."""
    _, bridge = _modules()
    code = bridge.new_pairing(home, record["id"])
    fresh = bridge.load_room(home, record["id"]) or record
    url = bridge.pairing_url(fresh, code)
    fresh["invite"] = {"url": url, "used": False}
    bridge.save_room(home, fresh)
    return url


def pairing_url(home: Path, room: str) -> str:
    """The QR URL of a room: the unused one if there is one, else a new code."""
    record = _record(home, room)
    invite = record.get("invite") or {}
    if invite.get("url") and not _invite_used(record):
        return invite["url"]
    return _renew_invite(home, record)


def _invite_used(record: dict) -> bool:
    invite = record.get("invite") or {}
    if not invite.get("url"):
        return True
    if invite.get("used"):
        return True
    for key in ("pairing_code", "pairing", "code"):  # the bridge burns the code by clearing it
        if key in record:
            return not record[key]
    return False


def join_room(home: Path, code: str) -> dict:
    """Join as a peer (no orders from us, none to us) using a code printed by ``tirith room invite``."""
    room, manager_key = parse_invite(code)
    sealed, bridge = _modules()
    try:
        me = sealed.load_or_create_key(home, manager_name(home))
        sealed.join(me, room)
    except sealed.SealedError as exc:
        raise PrivateRoomError(f"could not join the room: {exc}") from exc
    record = bridge.load_room(home, room) or {}
    managers = [m for m in record.get("managers") or [] if m.get("key_id") != manager_key]
    if manager_key:
        managers.append({"name": None, "key_id": manager_key, "last_seen": None})
    record.update({"id": room, "private": True, "role": "peer", "me": {"name": me.name, "key_id": me.key_id},
                   "managers": managers, "joined_at": now_iso()})
    record.setdefault("title", "Private room")
    record.setdefault("devices", [])
    record.pop("invite", None)  # a peer does not pair devices
    bridge.save_room(home, record)
    _start_bridge(home)
    return bridge.load_room(home, room)


def leave_room(home: Path, room: str) -> None:
    """Stop listening and forget the room here. The key stays (it is the identity, not the room's)."""
    if not valid_room(room):
        raise PrivateRoomError("invalid room")
    _record(home, room)
    path = home / "rooms" / f"{room}.json"
    path.unlink(missing_ok=True)
    with _lock:
        _started.discard(room)


# -- the inbox request ``room-new`` -----------------------------------------------------------


def handle_requests(home: Path) -> None:
    """Start (in a thread: it uses the network) every open ``room-new`` request not handled yet."""
    for req in inbox_mod.requests(home):
        if req["kind"] != "room-new" or req["status"] != "open" or req["answers"]:
            continue
        with _lock:
            if req["id"] in _started:
                continue
            _started.add(req["id"])
        threading.Thread(target=_run_request, args=(home, req), daemon=True, name="room-new").start()


def _run_request(home: Path, req: dict) -> None:
    try:
        record = new_room(home, req["text"], req["project"])
        text = f"Room {record['id']} is ready: scan its QR with your phone."
    except Exception as exc:  # the answer says why; never the traceback
        text = f"Could not create the room: {exc}"
    try:
        inbox_mod.add_answer(home, req["id"], text, by="tirith")
        inbox_mod.mark_done(home, req["id"])
    except Exception:
        pass


# -- state.json ---------------------------------------------------------------------------------


def _message(i: int, raw: dict) -> dict:
    text = str(raw.get("text") or "")
    return {"n": int(raw.get("n") or i + 1), "at": raw.get("at"), "from": str(raw.get("author") or raw.get("from") or ""),
            "text": text[:MAX_TEXT], "re": [], "agreed": [], "signed": "self" if raw.get("verified") else "none",
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
           "title": record.get("title") or "Private room", "project": record.get("project"),
           "summary": None, "summary_source": "none", "needs_owner": [], "message_count": len(messages),
           "last_message_at": messages[-1]["at"] if messages else None, "activity_per_hour": 0,
           "messages_last_day": 0, "polled_at": record.get("polled_at"), "error": record.get("error"),
           "participants": participants, "reply_edges": [], "messages": messages[-STATE_MESSAGES:],
           "private": True, "connected": bool(record.get("connected")),
           "me": {"name": me.get("name"), "key_id": me.get("key_id")},
           "devices": devices, "managers": managers}
    if invite.get("url") and record.get("role") != "peer":
        row["invite"] = {"url": invite["url"], "used": _invite_used(record)}
    else:
        row["invite"] = {"url": None, "used": True}
    return row


def state_rows(home: Path) -> list[dict]:
    """The private rooms this Tirith is in; ``[]`` when the bridge module is not there."""
    try:
        _, bridge = _modules()
        records = [r for r in bridge.list_rooms(home) if isinstance(r, dict) and r.get("private") and r.get("id")]
    except Exception:
        return []
    return [state_row(r) for r in records]


def ensure_bridge(home: Path) -> None:
    """Called each cycle: a Tirith that restarted listens again to the rooms it is in."""
    if "bridge" in _started or not state_rows(home):
        return
    with _lock:
        _started.add("bridge")
    try:
        _start_bridge(home)
    except Exception:
        _started.discard("bridge")
