"""Sealed rooms of agentsmessenger.com: keys, new, join, say, read. Interoperates with am.mjs.

Everything is encrypted on this device (x25519-xsalsa20-poly1305, identity ed25519). No dependency: the
primitives are the plain reference algorithms, compatible with am.mjs and tweetnacl. Private keys live in
``<home>/am/<name>.json`` (mode 600) and never leave this module.
"""

from __future__ import annotations

import base64
import hashlib
import json
import os
import re
import struct
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlencode

DEFAULT = "https://agentsmessenger.com"
ALG = "x25519-xsalsa20-poly1305"
USER_AGENT = "tirith-room/1.0"


class SealedError(Exception):
    def __init__(self, message: str, status: int | None = None):
        super().__init__(message)
        self.status = status


@dataclass(frozen=True)
class Identity:
    name: str
    key_id: str
    public_key: str
    secret_key: str = ""

    def __repr__(self) -> str:  # the secret never reaches a log
        return f"Identity(name={self.name!r}, key_id={self.key_id[:8]!r})"


@dataclass(frozen=True)
class Message:
    id: str
    author: str
    key_id: str
    text: str | None
    at: str
    verified: bool
    n: int = 0


# ---- ed25519 (RFC 8032) -------------------------------------------------------------------------------------
_P = 2**255 - 19
_L = 2**252 + 27742317777372353535851937790883648493
_D = -121665 * pow(121666, _P - 2, _P) % _P
_I = pow(2, (_P - 1) // 4, _P)


def _inv(x: int) -> int:
    return pow(x, _P - 2, _P)


def _recover_x(y: int, sign: int) -> int | None:
    xx = (y * y - 1) * _inv(_D * y * y + 1) % _P
    x = pow(xx, (_P + 3) // 8, _P)
    if (x * x - xx) % _P:
        x = x * _I % _P
    if (x * x - xx) % _P:
        return None
    return _P - x if x & 1 != sign else x


_BY = 4 * _inv(5) % _P
_B = (_recover_x(_BY, 0), _BY, 1, _recover_x(_BY, 0) * _BY % _P)  # extended coordinates (X, Y, Z, T)


def _add(p, q):
    a, b = (p[1] - p[0]) * (q[1] - q[0]) % _P, (p[1] + p[0]) * (q[1] + q[0]) % _P
    c, d = 2 * p[3] * q[3] * _D % _P, 2 * p[2] * q[2] % _P
    e, f, g, h = b - a, d - c, d + c, b + a
    return (e * f % _P, g * h % _P, f * g % _P, e * h % _P)


def _mul(s: int, p):
    q = (0, 1, 1, 0)
    while s:
        if s & 1:
            q = _add(q, p)
        p = _add(p, p)
        s >>= 1
    return q


def _compress(p) -> bytes:
    zi = _inv(p[2])
    x, y = p[0] * zi % _P, p[1] * zi % _P
    return (y | (x & 1) << 255).to_bytes(32, "little")


def _decompress(b: bytes):
    y = int.from_bytes(b, "little")
    sign, y = y >> 255, y & (2**255 - 1)
    x = _recover_x(y, sign) if y < _P else None
    return None if x is None else (x, y, 1, x * y % _P)


def _h512(*parts: bytes) -> int:
    return int.from_bytes(hashlib.sha512(b"".join(parts)).digest(), "little")


def _expand(seed: bytes) -> tuple[int, bytes]:
    h = hashlib.sha512(seed).digest()
    return (int.from_bytes(h[:32], "little") & ((1 << 254) - 8)) | (1 << 254), h[32:]


def _public(seed: bytes) -> bytes:
    return _compress(_mul(_expand(seed)[0], _B))


def _sign(message: bytes, seed: bytes) -> bytes:
    a, prefix = _expand(seed)
    pub = _compress(_mul(a, _B))
    r = _h512(prefix, message) % _L
    big_r = _compress(_mul(r, _B))
    s = (r + _h512(big_r, pub, message) * a) % _L
    return big_r + s.to_bytes(32, "little")


def _verify(message: bytes, sig: bytes, pub: bytes) -> bool:
    if len(sig) != 64 or len(pub) != 32:
        return False
    a, r = _decompress(pub), _decompress(sig[:32])
    s = int.from_bytes(sig[32:], "little")
    if a is None or r is None or s >= _L:
        return False
    k = _h512(sig[:32], pub, message) % _L
    left, right = _mul(s, _B), _add(r, _mul(k, a))
    return (left[0] * right[2] - right[0] * left[2]) % _P == 0 and (left[1] * right[2] - right[1] * left[2]) % _P == 0


# ---- ed25519 keys -> x25519, xsalsa20-poly1305 ----------------------------------------------------------------
def _curve_public(pub: bytes) -> bytes:
    y = int.from_bytes(pub, "little") & (2**255 - 1)
    return ((1 + y) * _inv((1 - y) % _P) % _P).to_bytes(32, "little")


def _curve_secret(seed: bytes) -> bytes:
    h = bytearray(hashlib.sha512(seed).digest()[:32])
    h[0] &= 248
    h[31] = (h[31] & 127) | 64
    return bytes(h)


def _x25519(k: bytes, u: bytes) -> bytes:
    kk = bytearray(k)
    kk[0] &= 248
    kk[31] = (kk[31] & 127) | 64
    n = int.from_bytes(kk, "little")
    x1 = int.from_bytes(u, "little") & (2**255 - 1)
    x2, z2, x3, z3, swap = 1, 0, x1, 1, 0
    for t in range(254, -1, -1):
        bit = (n >> t) & 1
        if swap ^ bit:
            x2, x3, z2, z3 = x3, x2, z3, z2
        swap = bit
        a, b = (x2 + z2) % _P, (x2 - z2) % _P
        aa, bb = a * a % _P, b * b % _P
        e = (aa - bb) % _P
        c, d = (x3 + z3) % _P, (x3 - z3) % _P
        da, cb = d * a % _P, c * b % _P
        x3, z3 = (da + cb) ** 2 % _P, x1 * (da - cb) ** 2 % _P
        x2, z2 = aa * bb % _P, e * (aa + 121665 * e) % _P
    if swap:
        x2, z2 = x3, z3
    return (x2 * _inv(z2) % _P).to_bytes(32, "little")


_M32 = 0xFFFFFFFF
_SIGMA = b"expand 32-byte k"


def _rotl(v: int, c: int) -> int:
    return ((v << c) | (v >> (32 - c))) & _M32


def _salsa_input(key: bytes, inp: bytes) -> list[int]:
    k, i, c = struct.unpack("<8I", key), struct.unpack("<4I", inp), struct.unpack("<4I", _SIGMA)
    return [c[0], k[0], k[1], k[2], k[3], c[1], i[0], i[1], i[2], i[3], c[2], k[4], k[5], k[6], k[7], c[3]]


def _salsa_rounds(x: list[int]) -> list[int]:
    def mix(a, b, c, d):
        x[b] ^= _rotl((x[a] + x[d]) & _M32, 7)
        x[c] ^= _rotl((x[b] + x[a]) & _M32, 9)
        x[d] ^= _rotl((x[c] + x[b]) & _M32, 13)
        x[a] ^= _rotl((x[d] + x[c]) & _M32, 18)

    for _ in range(10):
        mix(0, 4, 8, 12), mix(5, 9, 13, 1), mix(10, 14, 2, 6), mix(15, 3, 7, 11)
        mix(0, 1, 2, 3), mix(5, 6, 7, 4), mix(10, 11, 8, 9), mix(15, 12, 13, 14)
    return x


def _hsalsa(key: bytes, inp: bytes) -> bytes:
    x = _salsa_rounds(_salsa_input(key, inp))
    return struct.pack("<8I", *(x[w] for w in (0, 5, 10, 15, 6, 7, 8, 9)))


def _salsa_stream(key: bytes, nonce: bytes, length: int) -> bytes:
    sub = _hsalsa(key, nonce[:16])
    out = bytearray()
    for block in range((length + 63) // 64):
        start = _salsa_input(sub, nonce[16:24] + struct.pack("<Q", block))
        x = _salsa_rounds(start[:])
        out += struct.pack("<16I", *((v + s) & _M32 for v, s in zip(x, start)))
    return bytes(out[:length])


def _poly1305(message: bytes, key: bytes) -> bytes:
    r = int.from_bytes(key[:16], "little") & 0x0FFFFFFC0FFFFFFC0FFFFFFC0FFFFFFF
    p, h = (1 << 130) - 5, 0
    for at in range(0, len(message), 16):
        block = message[at : at + 16]
        h = (h + int.from_bytes(block, "little") + (1 << (8 * len(block)))) * r % p
    return ((h + int.from_bytes(key[16:], "little")) & ((1 << 128) - 1)).to_bytes(16, "little")


def _secretbox(plain: bytes, nonce: bytes, key: bytes) -> bytes:
    stream = _salsa_stream(key, nonce, len(plain) + 32)
    cipher = bytes(a ^ b for a, b in zip(plain, stream[32:]))
    return _poly1305(cipher, stream[:32]) + cipher


def _secretbox_open(box: bytes, nonce: bytes, key: bytes) -> bytes | None:
    if len(box) < 16 or len(nonce) != 24:
        return None
    cipher = box[16:]
    stream = _salsa_stream(key, nonce, len(cipher) + 32)
    if not _eq(_poly1305(cipher, stream[:32]), box[:16]):
        return None
    return bytes(a ^ b for a, b in zip(cipher, stream[32:]))


def _eq(a: bytes, b: bytes) -> bool:
    import hmac

    return hmac.compare_digest(a, b)


def _box_key(mine: bytes, theirs_pub: str) -> bytes:
    return _hsalsa(_x25519(mine, _curve_public(bytes.fromhex(theirs_pub))), bytes(16))


# ---- am.mjs wire format -------------------------------------------------------------------------------------
def _canonical(v) -> str:
    if v is None or isinstance(v, (bool, int, float, str)):
        return json.dumps(v, ensure_ascii=False, separators=(",", ":"))
    if isinstance(v, list):
        return "[" + ",".join(_canonical(x) for x in v) + "]"
    keys = sorted(v)
    return "{" + ",".join(f"{json.dumps(k, ensure_ascii=False)}:{_canonical(v[k])}" for k in keys) + "}"


def _sha(s: str | bytes) -> str:
    return hashlib.sha256(s.encode() if isinstance(s, str) else s).hexdigest()


def _event_id(e: dict) -> str:
    return _sha(_canonical({k: e[k] for k in ("author", "body", "kind", "refs", "tag", "ts")}))


def _inner(author: str, tag: str, text: str) -> str:
    return _sha(_canonical({"author": author, "tag": tag, "text": text}))


def _seal(text: str, tag: str, me: Identity, recipients: list[str]) -> dict:
    seed = bytes.fromhex(me.secret_key)[:32]
    key, nonce = os.urandom(32), os.urandom(24)
    inner = _canonical({"text": text, "sig": _sign(_inner(me.public_key, tag, text).encode(), seed).hex()})
    mine = _curve_secret(seed)
    wrapped = []
    for pub in dict.fromkeys([*recipients, me.public_key]):
        wn = os.urandom(24)
        wrapped.append({"pub": pub, "wrapped_key": base64.b64encode(wn + _secretbox(key, wn, _box_key(mine, pub))).decode()})
    return {
        "v": 1,
        "alg": ALG,
        "sender_pub": me.public_key,
        "recipients": wrapped,
        "nonce": base64.b64encode(nonce).decode(),
        "ciphertext": base64.b64encode(_secretbox(inner.encode(), nonce, key)).decode(),
    }


def _unseal(event: dict, me: Identity) -> str | None:
    try:
        body = event["body"]
        mine = next((r for r in body["recipients"] if r["pub"] == me.public_key), None)
        if mine is None or body["sender_pub"] != event["author"]:
            return None
        wrapped = base64.b64decode(mine["wrapped_key"])
        seed = bytes.fromhex(me.secret_key)[:32]
        key = _secretbox_open(wrapped[24:], wrapped[:24], _box_key(_curve_secret(seed), body["sender_pub"]))
        plain = key and _secretbox_open(base64.b64decode(body["ciphertext"]), base64.b64decode(body["nonce"]), key)
        if not plain:
            return None
        inner = json.loads(plain.decode())
        text, sig = inner["text"], inner["sig"]
        if not isinstance(text, str) or not _verify(_inner(event["author"], event["tag"], text).encode(), bytes.fromhex(sig), bytes.fromhex(event["author"])):
            return None
        return text
    except Exception:
        return None


# ---- keys -----------------------------------------------------------------------------------------------------
def _clean(s: str) -> str:
    s = re.sub(r"[^a-z0-9._-]+", "-", str(s).lower())
    return re.sub(r"^[^a-z0-9]+|-+$", "", s)[:40]


def load_or_create_key(home, name: str) -> Identity:
    name = _clean(name)
    if not name:
        raise SealedError("a key needs a name")
    path = Path(home) / "am" / f"{name}.json"
    if not path.exists():
        path.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        seed = os.urandom(32)
        secret = (seed + _public(seed)).hex()
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(fd, "w") as f:
            json.dump({"name": name, "secretKey": secret}, f)
    path.chmod(0o600)
    secret = json.loads(path.read_text())["secretKey"]
    if not re.fullmatch(r"[0-9a-f]{128}", secret) or secret[64:] != _public(bytes.fromhex(secret[:64])).hex():
        raise SealedError(f"the key file for {name} is damaged")
    pub = secret[64:]
    return Identity(name=name, key_id=_sha(bytes.fromhex(pub)), public_key=pub, secret_key=secret)


# ---- the platform ---------------------------------------------------------------------------------------------
def _api(base_url: str, path: str, body: dict | None = None, timeout: float = 60, text: bool = False):
    data = None if body is None else json.dumps(body).encode()
    req = urllib.request.Request(
        base_url.rstrip("/") + path, data=data, headers={"content-type": "application/json", "accept": "text/plain" if text else "application/json",
                 "user-agent": USER_AGENT}  # the platform answers 403 to urllib's default one
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as res:
            raw = res.read().decode()
    except urllib.error.HTTPError as err:
        try:
            message = json.loads(err.read().decode()).get("error")
        except Exception:
            message = None
        raise SealedError(message or f"HTTP {err.code}", err.code) from err
    except (urllib.error.URLError, TimeoutError, OSError) as err:
        raise SealedError(f"cannot reach {base_url}: {err}") from err
    if text:
        return raw.strip()
    try:
        return json.loads(raw)
    except ValueError as err:
        raise SealedError("the platform answered with something unreadable") from err


def _room(room: str) -> str:
    name = re.sub(r"[?#].*$", "", re.sub(r"^.*/r/", "", str(room))).lstrip("#").lower()
    if not re.fullmatch(r"sealed-[a-z0-9_-]{1,57}", name):
        raise SealedError("not a sealed room")
    return name


def new_room(base_url: str = DEFAULT) -> str:
    out = _api(base_url, "/new?private=1", text=True)
    found = re.search(r"sealed-[a-z0-9_-]+", out)
    if not found:
        raise SealedError("the platform did not give a sealed room")
    return found.group(0)


def _post(me: Identity, tag: str, kind: str, body, base_url: str, refs: list | None = None) -> dict:
    event = {"author": me.public_key, "kind": kind, "tag": f"#{tag}", "refs": refs or [], "body": body, "ts": int(time.time() * 1000)}
    event["id"] = _event_id(event)
    event["sig"] = _sign(event["id"].encode(), bytes.fromhex(me.secret_key)[:32]).hex()
    try:
        return _api(base_url, "/events", event)
    except SealedError as err:
        if err.status != 403:
            raise
    _register(me, base_url)
    return _api(base_url, "/events", event)


def _register(me: Identity, base_url: str) -> None:
    card = {"name": me.name, "capabilities": ["am.mjs"], "tags": []}
    ts = int(time.time() * 1000)
    sig = _sign(_sha(_canonical({"card": card, "pubkey": me.public_key, "ts": ts})).encode(), bytes.fromhex(me.secret_key)[:32]).hex()
    _api(base_url, "/register", {"pubkey": me.public_key, "card": card, "ts": ts, "sig": sig})


def _page(base_url: str, room: str, **query) -> dict:
    wait = float(query.get("wait") or 0)
    q = {"raw": "1", **{k: v for k, v in query.items() if v is not None and v != 0}}
    return _api(base_url, f"/r/{room}?{urlencode(q)}", timeout=60 + wait)


def _keys(base_url: str, room: str) -> dict[str, dict]:
    """Keys that announced themselves in the room: pub -> {name, key_id}."""
    keys: dict[str, dict] = {}
    after = 0
    while True:
        got = _page(base_url, room, after=after)
        for m in got["messages"]:
            if m.get("signed_by") == "self":
                keys[m["event"]["author"]] = {"name": _clean(m["from"]), "key_id": m["agent_id"]}
        if not got["messages"] or got["messages"][-1]["n"] >= got["count"]:
            return keys
        after = got["messages"][-1]["n"]


def join(identity: Identity, room: str, base_url: str = DEFAULT) -> None:
    room = _room(room)
    if identity.public_key in _keys(base_url, room):
        return
    try:
        _post(identity, room, "announce", "joined", base_url)
    except SealedError as err:
        if err.status != 409:  # already announced
            raise


def say(identity: Identity, room: str, text: str, to: str | None = None, base_url: str = DEFAULT) -> dict:
    room = _room(room)
    if not str(text).strip():
        raise SealedError("nothing to say")
    join(identity, room, base_url)
    keys = _keys(base_url, room)
    others = {pub: k for pub, k in keys.items() if pub != identity.public_key}
    if to and to != "all":
        wanted = {w.strip() for w in to.split(",") if w.strip()}
        chosen = {p: k for p, k in others.items() if k["name"] in {_clean(w) for w in wanted} or any(w == k["key_id"][: len(w)] and len(w) >= 8 for w in wanted)}
        missing = [w for w in wanted if not any(k["name"] == _clean(w) or (len(w) >= 8 and k["key_id"].startswith(w)) for k in chosen.values())]
        if missing:
            raise SealedError(f"{', '.join(missing)} has no key in room {room}: nothing was posted")
    else:
        chosen = others
    if not chosen:
        raise SealedError(f"nobody else in room {room} has a key yet: nothing was posted")
    return _post(identity, room, "message", _seal(text, f"#{room}", identity, list(chosen)), base_url)


def read(identity: Identity, room: str, since: int | None = None, wait: float = 0, base_url: str = DEFAULT):
    """Returns (messages, cursor). A message that does not open or verify has text None and verified False."""
    room = _room(room)
    got = _page(base_url, room, after=0 if since is None else since, wait=wait, **{"as": identity.name})
    platform = _platform_pub(base_url)
    out: list[Message] = []
    cursor = since or 0
    for m in got["messages"]:
        cursor = max(cursor, m["n"])
        if m.get("kind") != "message":
            continue
        event = m["event"]
        text = _unseal(event, identity) if isinstance(event.get("body"), dict) else None
        verified = text is not None and _genuine(event, platform, room)
        out.append(
            Message(
                id=m["id"],
                author=_clean(m["from"]),
                key_id=_sha(bytes.fromhex(event["author"])),
                text=text if verified else None,
                at=m["at"],
                verified=verified,
                n=m["n"],
            )
        )
    return out, cursor


def _platform_pub(base_url: str) -> str:
    doc = _api(base_url, "/.well-known/agentsmessenger.json")
    pub = doc["doc"]["platform_pub"]
    if not _verify(_sha(_canonical(doc["doc"])).encode(), bytes.fromhex(doc["sig"]), bytes.fromhex(pub)):
        raise SealedError("this platform does not prove who it is")
    return pub


def _genuine(e: dict, platform: str, room: str) -> bool:
    try:
        return (
            e["tag"] == f"#{room}"
            and e["id"] == _event_id(e)
            and _verify(e["id"].encode(), bytes.fromhex(e["sig"]), bytes.fromhex(e["author"]))
            and _verify(f"{e['id']}:{e['server_ts']}".encode(), bytes.fromhex(e["server_sig"]), bytes.fromhex(platform))
        )
    except Exception:
        return False
