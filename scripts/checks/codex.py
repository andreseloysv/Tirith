#!/usr/bin/env python3
"""Exercise the real daemon with a fake Codex and a local encrypted phone service.

Build: cargo build -p tirith. Run in a venv with PyNaCl installed:
  python scripts/checks/codex.py [--keep]
No live models, real accounts or existing Tirith state are used.
"""
import base64
import hashlib
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse
from urllib.request import Request, urlopen

from nacl.public import Box
from nacl.secret import SecretBox
from nacl.signing import SigningKey, VerifyKey

ROOT = Path(__file__).resolve().parents[2]
HOME = Path(tempfile.mkdtemp(prefix="tirith-codex-check-"))
platform = SigningKey.generate()
phone = SigningKey.generate()
events = {}
lock = threading.Lock()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def digest(value):
    return hashlib.sha256(value.encode()).hexdigest()


def signed(key, value):
    return key.sign(value.encode()).signature.hex()


def publish(e, name):
    e = dict(e)
    e["server_ts"] = int(time.time() * 1000)
    e["server_sig"] = signed(platform, f'{e["id"]}:{e["server_ts"]}')
    with lock:
        rows = events.setdefault(e["tag"][1:], [])
        rows.append(dict(n=len(rows) + 1, event=e, signed_by="self", at="2026-11-15T12:00:00Z", **{"from": name}))


def send_phone(room, public, text, key=phone):
    author = key.verify_key.encode().hex()
    tag = "#" + room
    inner = canonical(dict(text=text, sig=signed(key, digest(canonical(dict(author=author, tag=tag, text=text))))))
    secret = os.urandom(32)
    cipher = SecretBox(secret).encrypt(inner.encode())
    recipients = []
    for pub in {public, author}:
        wrapped = Box(key.to_curve25519_private_key(), VerifyKey(bytes.fromhex(pub)).to_curve25519_public_key()).encrypt(secret)
        recipients.append(dict(pub=pub, wrapped_key=base64.b64encode(wrapped).decode()))
    body = dict(v=1, alg="x25519-xsalsa20-poly1305", sender_pub=author, recipients=recipients,
                nonce=base64.b64encode(cipher.nonce).decode(), ciphertext=base64.b64encode(cipher.ciphertext).decode())
    e = dict(author=author, kind="message", tag=tag, refs=[], body=body, ts=int(time.time() * 1000))
    e["id"] = digest(canonical(e))
    e["sig"] = signed(key, e["id"])
    publish(e, "test-phone" if key is phone else "other-agent")


def phone_replies(room):
    result = []
    with lock:
        rows = list(events.get(room, []))
    for row in rows:
        e = row["event"]
        if e["author"] == phone.verify_key.encode().hex() or not isinstance(e["body"], dict):
            continue
        b = e["body"]
        for r in b.get("recipients", []):
            if r["pub"] == phone.verify_key.encode().hex():
                box = Box(phone.to_curve25519_private_key(), VerifyKey(bytes.fromhex(e["author"])).to_curve25519_public_key())
                secret = box.decrypt(base64.b64decode(r["wrapped_key"]))
                plain = SecretBox(secret).decrypt(base64.b64decode(b["ciphertext"]), base64.b64decode(b["nonce"]))
                result.append(json.loads(plain)["text"])
    return result


class Platform(BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def reply(self, value, raw=False):
        data = (value if raw else json.dumps(value)).encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        u = urlparse(self.path)
        if u.path == "/.well-known/agentsmessenger.json":
            doc = dict(platform_pub=platform.verify_key.encode().hex())
            self.reply(dict(doc=doc, sig=signed(platform, digest(canonical(doc)))))
        elif u.path == "/new":
            with lock:
                room = f"sealed-test-{len(events)}"
                events[room] = []
            self.reply("/r/" + room, True)
        else:
            after = int(parse_qs(u.query).get("after", [0])[0])
            with lock:
                rows = list(events.get(u.path.split("/")[-1], []))
            self.reply(dict(messages=[r for r in rows if r["n"] > after], count=len(rows), here=[]))

    def do_POST(self):
        value = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        if self.path == "/events":
            publish(value, "tirith-manager")
        self.reply({})


def wait(check, timeout=15):
    until = time.monotonic() + timeout
    while time.monotonic() < until:
        try:
            value = check()
            if value:
                return value
        except (ConnectionError, OSError):
            pass
        time.sleep(.1)
    raise AssertionError("Timed out: " + repr(check))


FAKE = r'''#!/usr/bin/env python3
import json, os, subprocess, sys, time
from pathlib import Path
a=sys.argv[1:]; home=Path(os.environ['TIRITH_HOME']); role=os.environ.get('TIRITH_ROLE','workers')
def emit(v): print(json.dumps(v), flush=True)
if '--input-format' in a:
 emit(dict(type='system',subtype='init',session_id='stream-thread'))
 for line in sys.stdin:
  v=json.loads(line)
  if v['type'] != 'user': continue
  record=dict(role=role,argv=a,input=json.dumps(v),project=os.environ.get('TIRITH_PROJECT',''),protocol='stream-json')
  with (home/'calls.jsonl').open('a') as f: f.write(json.dumps(record)+'\n')
  emit(dict(type='assistant',message=dict(content=[dict(type='text',text='answer-stream')])))
  emit(dict(type='result'))
 sys.exit(0)
text=sys.stdin.read() if a[-1:] == ['-'] else 'worker'
model=a[a.index('--model')+1] if '--model' in a else 'default'
record=dict(role=role, argv=a, input=text, project=os.environ.get('TIRITH_PROJECT',''))
with (home/'calls.jsonl').open('a') as f: f.write(json.dumps(record)+'\n')
emit(dict(type='thread.started',thread_id='thread-'+role))
if text.rstrip().endswith('WAIT'):
 while not (home/'release').exists(): time.sleep(.05)
if text.rstrip().endswith('FAIL'):
 emit(dict(type='turn.failed',error=dict(message='test quota failure')));sys.exit(1)
if role == 'workers':
 Path('result.txt').write_text('done\n')
 subprocess.run(['git','add','result.txt'],check=True,stdout=sys.stderr)
 subprocess.run(['git','commit','-m','Complete sample item'],check=True,stdout=sys.stderr)
emit(dict(type='item.completed',item=dict(type='agent_message',text='answer-'+role+'-'+model)))
emit(dict(type='turn.completed',usage={}))
'''


service = ThreadingHTTPServer(("127.0.0.1", 0), Platform)
threading.Thread(target=service.serve_forever, daemon=True).start()
with socket.socket() as s:
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
base = f"http://127.0.0.1:{port}"


def api(path, data=None, method=None):
    req = Request(base + path, data=None if data is None else json.dumps(data).encode(),
                  headers={"Content-Type": "application/json", "X-Tirith-Token": (HOME / "owner.token").read_text()}, method=method)
    with urlopen(req, timeout=5) as r:
        return json.load(r)


def chat(text, project=""):
    return api("/api/chat", dict(text=text, project=project))


def calls():
    p = HOME / "calls.jsonl"
    return [json.loads(l) for l in p.read_text().splitlines()] if p.exists() else []


def roles():
    return api("/api/models")["roles"]


daemon = None
try:
    mock = HOME / "codex"
    mock.write_text(FAKE)
    mock.chmod(0o755)
    catalog = HOME / "catalog"
    catalog.mkdir()
    catalog.joinpath("models_cache.json").write_text(json.dumps(dict(models=[
        dict(slug="gpt-test", display_name="Test", visibility="list", supported_reasoning_levels=[dict(effort="high"), dict(effort="low")]),
        dict(slug="gpt-hidden", visibility="hide")])) )
    repo = HOME / "project"
    repo.mkdir()
    for args in [["init", "-b", "main"], ["config", "user.name", "Test"], ["config", "user.email", "test@example.test"]]:
        subprocess.run(["git", *args], cwd=repo, check=True, capture_output=True)
    (repo / "docs").mkdir()
    (repo / "docs/TODO.md").write_text("## 1. Sample\n- [ ] Create result.txt\n")
    (repo / "tirith.toml").write_text('guard = false\ndeploy = "none"\n')
    subprocess.run(["git", "add", "."], cwd=repo, check=True)
    subprocess.run(["git", "commit", "-qm", "Initial sample"], cwd=repo, check=True)
    (HOME / "config.toml").write_text(f'''
[agents.codex]
command = {json.dumps(str(mock))}
profile = "test-browser"
[agents.anthropic]
command = {json.dumps(str(mock))}
[models.manager]
agent = "codex"
[models.workers]
agent = "codex"
[models.ceo]
agent = "codex"
[network]
sandbox = false
proxy_port = 0
[autopilot]
enabled = false
[merge]
push = false
[messenger]
url = "http://127.0.0.1:{service.server_port}"
[ceo]
review_after_minutes = 0
[ceo.targets]
main_red = -1
[[projects]]
id = "sample"
repo = {json.dumps(str(repo))}
''')
    env = dict(os.environ, TIRITH_HOME=str(HOME), TIRITH_PORT=str(port), CODEX_HOME=str(catalog))
    log = (HOME / "daemon.log").open("w")
    daemon = subprocess.Popen([ROOT / "target/debug/neobot", "serve"], env=env, stdout=log, stderr=log, start_new_session=True)
    wait(lambda: api("/api/models"))
    assert [m["slug"] for m in api("/api/models")["choices"]["codex"]] == ["gpt-test"]
    chat("/model all codex:gpt-test high")
    assert all(r["agent"] == "codex" and r["model"] == "gpt-test" and r["effort"] == "high" for r in roles().values())
    before = roles()
    chat("/model all codex:gpt-test ultra")
    assert roles() == before, "invalid batch must not change any role"
    chat("/model all unknown:model")
    assert roles() == before
    chat("/models")
    assert not calls(), "model commands must not call an agent"
    chat("hello", "sample")
    wait(lambda: calls())
    wait(lambda: not api("/api/state")["busy"])
    assert calls()[-1]["argv"][-1] == "-"
    assert '--config=model_reasoning_effort="high"' in calls()[-1]["argv"]
    assert calls()[-1]["argv"][:3] == ["--profile", "test-browser", "exec"]
    chat("second", "sample")
    wait(lambda: len(calls()) >= 2)
    wait(lambda: not api("/api/state")["busy"])
    assert "resume" in calls()[-1]["argv"]
    chat("WAIT", "sample")
    wait(lambda: "sample" in api("/api/state")["busy"])
    wait(lambda: calls()[-1]["input"].rstrip().endswith("WAIT"))
    count = len(calls())
    chat("/model manager codex:gpt-test low", "sample")
    chat("queued after change", "sample")
    time.sleep(.3)
    assert len(calls()) == count
    (HOME / "release").touch()
    wait(lambda: len(calls()) > count)
    wait(lambda: not api("/api/state")["busy"])
    assert '--config=model_reasoning_effort="low"' in calls()[-1]["argv"]
    (HOME / "release").unlink()
    chat("WAIT", "sample")
    wait(lambda: "sample" in api("/api/state")["busy"])
    wait(lambda: calls()[-1]["input"].rstrip().endswith("WAIT"))
    api("/api/manager/interrupt", dict(project="sample"))
    wait(lambda: not api("/api/state")["busy"])
    chat("FAIL", "sample")
    wait(lambda: any(c["input"].rstrip().endswith("FAIL") for c in calls()))
    wait(lambda: not api("/api/state")["busy"])
    chat("recover", "sample")
    wait(lambda: calls()[-1]["input"].rstrip().endswith("recover"))
    wait(lambda: not api("/api/state")["busy"])
    assert "resume" not in calls()[-1]["argv"]
    chat("/model manager anthropic:sonnet", "sample")
    chat("after provider switch", "sample")
    wait(lambda: calls()[-1].get("protocol") == "stream-json")
    wait(lambda: not api("/api/state")["busy"])
    assert "hello" in calls()[-1]["input"] and "--resume" not in calls()[-1]["argv"]
    chat("/model manager codex:gpt-test low", "sample")
    chat("back to Codex", "sample")
    wait(lambda: calls()[-1]["input"].rstrip().endswith("back to Codex"))
    wait(lambda: not api("/api/state")["busy"])
    assert "after provider switch" in calls()[-1]["input"] and "resume" not in calls()[-1]["argv"]
    # A lane exercises model/effort flags and the actual merge queue.
    lane = api("/api/lanes", dict(project="sample", step="1", item="Create result.txt"))
    wait(lambda: any(c["role"] == "workers" for c in calls()), 30)
    wait(lambda: (repo / "result.txt").exists(), 30)
    saved_lane = next(l for l in api("/api/state")["lanes"] if l["id"] == lane["id"])
    assert saved_lane["model"] == "gpt-test" and saved_lane["session"] == "thread-workers"
    worker = next(c for c in calls() if c["role"] == "workers")
    assert worker["argv"][worker["argv"].index("--model")+1] == "gpt-test"
    assert '--config=model_reasoning_effort="high"' in worker["argv"]
    assert worker["argv"][:3] == ["--profile", "test-browser", "exec"]
    # The merge also schedules a plan-finished notification for the manager after two seconds.
    wait(lambda: any("The plan has no open item left" in c["input"] for c in calls()), 10)
    wait(lambda: not api("/api/state")["busy"])
    print("PASS: model commands, manager turns and lane merge", flush=True)
    # Pair through the real signed and encrypted room protocol, then change settings.
    room = api("/api/rooms", dict(project="sample", title="Phone check"))
    rid = room["id"]
    code = parse_qs(urlparse(room["invite"]).fragment)["c"][0]
    identity = json.loads((HOME / "am/tirith-manager.json").read_text())
    public = identity["secretKey"][64:]
    send_phone(rid, public, "tirith-pair " + code)
    wait(lambda: api("/api/rooms/" + rid)["devices"], 15)
    count = len(calls())
    send_phone(rid, public, "/model workers codex:gpt-test low")
    wait(lambda: roles()["workers"]["effort"] == "low", 15)
    wait(lambda: any("Saved for all projects" in t for t in phone_replies(rid)), 15)
    assert len(calls()) == count, "paired phone command must bypass the manager"
    send_phone(rid, public, "/model workers codex:gpt-test high", SigningKey.generate())
    wait(lambda: len(calls()) > count, 15)
    assert roles()["workers"]["effort"] == "low", "another agent cannot change models through the command path"
    # Visual controls use signed requests, never a manager turn, and retries cannot reapply an old save.
    def control(id, action="set", effort="high", key=phone):
        request = dict(v=1, id=id, action=action, role="workers", setting=dict(agent="codex", model="gpt-test", effort=effort))
        send_phone(rid, public, "tirith-models " + json.dumps(request), key)

    def response(id):
        return [json.loads(t[20:]) for t in phone_replies(rid) if t.startswith("tirith-models-state ") and json.loads(t[20:])["id"] == id]

    control("first")
    wait(lambda: response("first"))
    assert response("first")[-1]["ok"] and roles()["workers"]["effort"] == "high"
    control("second", effort="low")
    wait(lambda: response("second"))
    control("first")
    wait(lambda: len(response("first")) == 2)
    assert roles()["workers"]["effort"] == "low", "a retried old request must not overwrite newer settings"
    control("invalid", effort="ultra")
    wait(lambda: response("invalid"))
    assert not response("invalid")[-1]["ok"] and roles()["workers"]["effort"] == "low"
    control("stranger", key=SigningKey.generate())
    control("readback", action="get")
    wait(lambda: response("readback"))
    assert not response("stranger") and roles()["workers"]["effort"] == "low"
    assert response("readback")[-1]["state"]["choices"]["codex"][0]["slug"] == "gpt-test"
    print("PASS: encrypted phone commands and controls, retry ordering and peer isolation; waiting for CEO health tick", flush=True)
    # A forced red metric schedules the CEO on the second health tick.
    wait(lambda: any(c["role"] == "ceo" for c in calls()), 75)
    ceo = next(c for c in calls() if c["role"] == "ceo")
    assert "read-only" in ceo["argv"] and "--dangerously-bypass-approvals-and-sandbox" not in ceo["argv"]
    assert ceo["argv"][:3] == ["--profile", "test-browser", "exec"]
    before = roles()
    daemon.terminate()
    daemon.wait(10)
    daemon = subprocess.Popen([ROOT / "target/debug/neobot", "serve"], env=env, stdout=log, stderr=log, start_new_session=True)
    wait(lambda: roles() == before)
    chat("/model workers reset")
    assert roles()["workers"]["source"] == "config"
    print("PASS: catalog, all roles, validation, queue, resume, stop, failure, lane merge, encrypted phone, peer isolation, CEO, persistence, reset", flush=True)
    if "--keep" in sys.argv:
        print(f"Preview: {base} — artifacts: {HOME}", flush=True)
        while not (HOME / "stop").exists():
            time.sleep(.5)
finally:
    if daemon is not None:
        os.killpg(daemon.pid, signal.SIGTERM)
        daemon.wait(10)
    service.shutdown()
    print(f"Check artifacts: {HOME}", flush=True)
