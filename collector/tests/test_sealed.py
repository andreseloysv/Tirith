"""Sealed rooms: interoperability with am.mjs (both ways) against a fake platform on localhost."""

import json
import os
import shutil
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlsplit

import pytest

from tirith import sealed

AM = Path(os.environ.get("AM_MJS", Path.home() / "DEVELOP/agentsmessenger/src/client/am.mjs"))
ROOM = "sealed-quiet-lake-77"
needs_node = pytest.mark.skipif(not (shutil.which("node") and AM.exists()), reason="needs node and am.mjs")


class Platform:
    """Just enough of agentsmessenger.com: register, events (sealed rule), raw reads, signed time stamps."""

    def __init__(self):
        seed = os.urandom(32)
        self.seed, self.pub = seed, sealed._public(seed).hex()
        self.agents: dict[str, dict] = {}
        self.events: list[dict] = []
        self.lock = threading.Lock()
        outer = self

        class H(BaseHTTPRequestHandler):
            def log_message(self, *a):
                pass

            def send(self, code, body, text=False):
                data = (body if text else json.dumps(body)).encode()
                self.send_response(code)
                self.send_header("content-type", "text/plain" if text else "application/json")
                self.end_headers()
                self.wfile.write(data)

            def do_GET(self):
                u = urlsplit(self.path)
                q = {k: v[0] for k, v in parse_qs(u.query).items()}
                if u.path == "/new":
                    return self.send(200, f"{ROOM}\n", text=True)
                if u.path == "/.well-known/agentsmessenger.json":
                    doc = {"platform_pub": outer.pub}
                    sig = sealed._sign(sealed._sha(sealed._canonical(doc)).encode(), outer.seed).hex()
                    return self.send(200, {"doc": doc, "sig": sig})
                if u.path.startswith("/r/"):
                    after = int(q.get("after", 0))
                    wait = float(q.get("wait", 0))
                    end = time.time() + min(wait, 3)
                    while len(outer.events) <= after and time.time() < end:
                        time.sleep(0.02)
                    with outer.lock:
                        msgs = [outer.message(i + 1, e) for i, e in enumerate(outer.events) if i >= after]
                        return self.send(200, {"room": u.path[3:], "count": len(outer.events), "messages": msgs})
                self.send(404, {"error": "no"})

            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers["content-length"])))
                if self.path == "/register":
                    outer.agents[body["pubkey"]] = body["card"]
                    return self.send(200, {"ok": True})
                if body["author"] not in outer.agents:
                    return self.send(403, {"error": "author is not registered; POST /register first"})
                if body["id"] != sealed._event_id(body) or not sealed._verify(body["id"].encode(), bytes.fromhex(body["sig"]), bytes.fromhex(body["author"])):
                    return self.send(400, {"error": "bad signature"})
                if isinstance(body["body"], str) and body["body"] != "joined":
                    return self.send(403, {"error": "this room is private: only encrypted messages can go in."})
                with outer.lock:
                    if body["kind"] == "announce" and any(e["kind"] == "announce" and e["author"] == body["author"] for e in outer.events):
                        return self.send(409, {"error": "repeated"})
                    ts = int(time.time() * 1000) + len(outer.events)
                    body["server_ts"] = ts
                    body["server_sig"] = sealed._sign(f"{body['id']}:{ts}".encode(), outer.seed).hex()
                    outer.events.append(body)
                self.send(200, {"ok": True, "n": len(outer.events)})

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), H)
        self.url = f"http://127.0.0.1:{self.server.server_port}"
        threading.Thread(target=self.server.serve_forever, daemon=True).start()

    def message(self, n, e):
        return {
            "n": n, "id": e["id"], "from": self.agents[e["author"]]["name"], "kind": e["kind"],
            "at": "2026-01-01T00:00:00.000Z", "text": e["body"] if isinstance(e["body"], str) else None,
            "signed_by": "self", "agent_id": sealed._sha(bytes.fromhex(e["author"])), "event": e,
        }


@pytest.fixture
def platform():
    p = Platform()
    yield p
    p.server.shutdown()


def am(home, name, url, *args, stdin=None):
    env = {"PATH": os.environ["PATH"], "AM_HOME": str(home), "AM_AGENT": name, "AM_URL": url}
    return subprocess.run(["node", str(AM), *args], env=env, capture_output=True, text=True, timeout=60, check=True).stdout


def test_keys_are_private_and_stable(tmp_path):
    a = sealed.load_or_create_key(tmp_path, "Manager")
    b = sealed.load_or_create_key(tmp_path, "manager")
    assert a.key_id == b.key_id and a.name == "manager"
    path = tmp_path / "am" / "manager.json"
    assert oct(path.stat().st_mode & 0o777) == "0o600"
    assert a.secret_key not in repr(a) and a.secret_key not in str(a)


@needs_node
def test_sealed_key_file_is_the_am_mjs_key(tmp_path, platform):
    me = sealed.load_or_create_key(tmp_path / "py", "tirith")
    (tmp_path / "js" / "keys").mkdir(parents=True)
    (tmp_path / "js" / "keys" / "tirith.json").write_text(json.dumps({"name": "tirith", "secretKey": me.secret_key}))
    out = am(tmp_path / "js", "tirith", platform.url, "whoami")
    assert me.key_id in out and me.public_key in out


def test_new_room(platform):
    assert sealed.new_room(platform.url) == ROOM


@needs_node
def test_sealed_python_to_am_mjs_and_back(tmp_path, platform):
    me = sealed.load_or_create_key(tmp_path / "py", "manager")
    sealed.join(me, ROOM, platform.url)
    sealed.join(me, ROOM, platform.url)  # joining twice is fine
    with pytest.raises(sealed.SealedError):  # nobody to encrypt for yet
        sealed.say(me, ROOM, "hello", base_url=platform.url)

    js = tmp_path / "js"
    am(js, "phone", platform.url, "join", ROOM, "--as", "phone")

    sealed.say(me, ROOM, "hola desde Python ñandú 🗳\nsegunda línea", base_url=platform.url)
    assert "> hola desde Python ñandú 🗳\n> segunda línea" in am(js, "phone", platform.url, "read", ROOM)

    am(js, "phone", platform.url, "say", ROOM, "respuesta de am.mjs ñ", "--anyway")
    got, cursor = sealed.read(me, ROOM, base_url=platform.url)
    texts = [(m.author, m.text, m.verified) for m in got]
    assert ("phone", "respuesta de am.mjs ñ", True) in texts
    assert ("manager", "hola desde Python ñandú 🗳\nsegunda línea", True) in texts
    assert cursor == len(platform.events)
    assert sealed.read(me, ROOM, since=cursor, base_url=platform.url)[0] == []
    assert all(m.key_id and m.at and m.id for m in got)


@needs_node
def test_read_waits_and_to_names_recipients(tmp_path, platform):
    me = sealed.load_or_create_key(tmp_path / "py", "manager")
    js = tmp_path / "js"
    for who in ("phone", "spy"):
        am(js / who, who, platform.url, "join", ROOM, "--as", who)
    sealed.join(me, ROOM, platform.url)
    cursor = len(platform.events)
    threading.Timer(0.3, lambda: am(js / "phone", "phone", platform.url, "say", ROOM, "--to", "manager", "just you", "--anyway")).start()
    got, cursor = sealed.read(me, ROOM, since=cursor, wait=5, base_url=platform.url)
    assert [m.text for m in got] == ["just you"]

    sealed.say(me, ROOM, "only phone", to="phone", base_url=platform.url)
    assert "> only phone" in am(js / "phone", "phone", platform.url, "read", ROOM)
    assert "only phone" not in am(js / "spy", "spy", platform.url, "read", ROOM)
    with pytest.raises(sealed.SealedError):
        sealed.say(me, ROOM, "x", to="nobody", base_url=platform.url)


@needs_node
def test_unreadable_or_forged_messages_are_never_readable(tmp_path, platform):
    me = sealed.load_or_create_key(tmp_path / "py", "manager")
    js = tmp_path / "js"
    for who in ("phone", "spy"):
        am(js / who, who, platform.url, "join", ROOM, "--as", who)
    sealed.join(me, ROOM, platform.url)
    # not encrypted for the manager
    am(js / "phone", "phone", platform.url, "say", ROOM, "--to", "spy", "not for you", "--anyway")
    got, _ = sealed.read(me, ROOM, since=0, base_url=platform.url)
    hidden = [m for m in got if m.text is None]
    assert len(hidden) == 1 and not hidden[0].verified

    # a tampered ciphertext, and a wrong platform time stamp
    sealed.say(me, ROOM, "genuine", to="phone", base_url=platform.url)
    good = platform.events[-1]
    forged = json.loads(json.dumps(good))
    ct = bytearray(__import__("base64").b64decode(forged["body"]["ciphertext"]))
    ct[-1] ^= 1
    forged["body"]["ciphertext"] = __import__("base64").b64encode(bytes(ct)).decode()
    platform.events.append(forged)
    me2 = sealed.load_or_create_key(tmp_path / "py", "manager")
    got, _ = sealed.read(me2, ROOM, since=0, base_url=platform.url)
    assert not got[-1].verified and got[-1].text is None
    bad_ts = json.loads(json.dumps(good))
    bad_ts["server_ts"] += 5
    platform.events[-1] = bad_ts
    got, _ = sealed.read(me, ROOM, since=0, base_url=platform.url)
    assert got[-1].text is None and not got[-1].verified


def test_not_a_sealed_room(tmp_path, platform):
    me = sealed.load_or_create_key(tmp_path, "manager")
    with pytest.raises(sealed.SealedError):
        sealed.join(me, "quiet-lake-77", platform.url)
    with pytest.raises(sealed.SealedError):
        sealed.new_room("http://127.0.0.1:1")


def test_crypto_vectors():
    # RFC 8032 test 1 and RFC 7748 / NaCl box vectors guard the primitives themselves
    seed = bytes.fromhex("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
    assert sealed._public(seed).hex() == "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
    sig = sealed._sign(b"", seed)
    assert sig.hex().startswith("e5564300c360ac729086e2cc806e828a")
    assert sealed._verify(b"", sig, sealed._public(seed)) and not sealed._verify(b"x", sig, sealed._public(seed))
    k = bytes.fromhex("a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4")
    u = bytes.fromhex("e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c")
    assert sealed._x25519(k, u).hex() == "c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552"
