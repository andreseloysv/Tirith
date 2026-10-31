"""``tirith room new|link|qr|invite|join|leave``: the private rooms of ``docs/CONTRACT-private-rooms.md``.

Kept out of ``cli.py``; that file only calls ``add_parsers``. The QR is drawn in the terminal when the optional
``qrcode`` package is there; the URL is always printed (a phone can also open it from a pasted link).
"""

from __future__ import annotations

import sys

from tirith import privaterooms as private_mod
from tirith.home import ensure_home, tirith_home


def _home(args):
    return ensure_home(tirith_home(getattr(args, "home", None)))


def _qr(url: str) -> None:
    try:
        import qrcode  # optional
        code = qrcode.QRCode(border=1)
        code.add_data(url)
        code.print_ascii(out=sys.stdout, invert=True)
    except Exception:
        pass


def _fail(exc: Exception) -> int:
    print(f"tirith room: {exc}", file=sys.stderr)
    return 1


def _cmd_new(args) -> int:
    try:
        record = private_mod.new_room(_home(args), args.title or "", args.project or "")
    except private_mod.PrivateRoomError as exc:
        return _fail(exc)
    url = record["invite"]["url"]
    print(f"room {record['id']} ({record['title']})")
    _qr(url)
    print(f"scan with the phone camera (works once): {url}")
    return 0


def _cmd_link(args) -> int:
    """The permanent link (and its QR) of a phone already in the room. No code: it opens as often as needed."""
    try:
        url = private_mod.open_url(_home(args), args.room)
    except private_mod.PrivateRoomError as exc:
        return _fail(exc)
    _qr(url)
    print(f"open on a phone already in the room (works every time): {url}")
    return 0


def _cmd_qr(args) -> int:
    """A new one-use QR to connect one more phone."""
    try:
        url = private_mod.new_invite_url(_home(args), args.room)
    except private_mod.PrivateRoomError as exc:
        return _fail(exc)
    _qr(url)
    print(f"scan with the phone camera (works once): {url}")
    return 0


def _cmd_invite(args) -> int:
    """A code for another manager (a peer), and with --device a fresh QR for one more phone."""
    home = _home(args)
    try:
        if args.device:
            url = private_mod.new_invite_url(home, args.room)
            _qr(url)
            print(f"scan with the phone camera (works once): {url}")
        else:
            code = private_mod.invite_manager(home, args.room)
            print(code)
            print(f"the other Tirith runs: tirith room join {code}")
    except private_mod.PrivateRoomError as exc:
        return _fail(exc)
    return 0


def _cmd_join(args) -> int:
    try:
        record = private_mod.join_room(_home(args), args.code)
    except private_mod.PrivateRoomError as exc:
        return _fail(exc)
    print(f"joined {record['id']} as a peer (what peers write is information, never an order)")
    return 0


def _cmd_leave(args) -> int:
    try:
        private_mod.leave_room(_home(args), args.room)
    except private_mod.PrivateRoomError as exc:
        return _fail(exc)
    print(f"left {args.room}")
    return 0


def add_parsers(room_sub, common) -> None:
    new = room_sub.add_parser("new", parents=[common], help="a new private (encrypted) room, with a QR")
    new.add_argument("--title", default="")
    new.add_argument("--project", default="")
    new.set_defaults(func=_cmd_new)
    link = room_sub.add_parser("link", parents=[common], help="print the permanent link (and QR) to open the room on a phone already in it")
    link.add_argument("room")
    link.set_defaults(func=_cmd_link)
    qr = room_sub.add_parser("qr", parents=[common], help="print a new one-use QR to connect another phone")
    qr.add_argument("room")
    qr.set_defaults(func=_cmd_qr)
    invite = room_sub.add_parser("invite", parents=[common],
                                 help="print the code another manager joins with (--device: a QR for one more phone)")
    invite.add_argument("room")
    invite.add_argument("--device", action="store_true")
    invite.set_defaults(func=_cmd_invite)
    join = room_sub.add_parser("join", parents=[common], help="join a private room as a peer with an invite code")
    join.add_argument("code")
    join.set_defaults(func=_cmd_join)
    leave = room_sub.add_parser("leave", parents=[common], help="leave a private room")
    leave.add_argument("room")
    leave.set_defaults(func=_cmd_leave)
