"""The MSP chat adapter (``muse serve`` over stdio) and a manager Session driving a fake MSP host."""

from __future__ import annotations

import json
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

from tirith import msession as msession_mod
from tirith import mspchat as mspchat_mod
from tirith.home import load_config

HOST = str(Path(__file__).resolve().parents[2] / "tools" / "msp_fake_host.py")


def rpc(**kw) -> str:
    return json.dumps({"jsonrpc": "2.0", **kw})


def opened(m: mspchat_mod.Msp) -> None:
    hello = json.loads(m.hello()[0])
    assert hello["method"] == "initialize" and hello["params"]["clientInfo"]["name"] == "tirith"
    sent = [json.loads(x) for x in m.incoming(rpc(id=hello["id"], result={})).send]
    assert sent[0] == {"jsonrpc": "2.0", "method": "initialized"}  # the notification muse serve requires
    start = sent[1]
    assert start["method"] == "session/start"
    assert m.incoming(rpc(id=start["id"], result={"session": {"sessionId": "s-1"}})).send == []


def test_queued_message_becomes_a_turn_after_the_handshake() -> None:
    m = mspchat_mod.Msp("minimal")
    assert m.user("hi") == []
    hello = json.loads(m.hello()[0])
    start = json.loads(m.incoming(rpc(id=hello["id"], result={})).send[1])
    turn = json.loads(m.incoming(rpc(id=start["id"], result={"session": {"sessionId": "s-9"}})).send[0])
    assert turn["method"] == "turn/start" and turn["params"]["sessionId"] == "s-9"
    assert turn["params"]["reasoningEffort"] == "minimal"
    assert turn["params"]["input"] == [{"type": "text", "text": "hi"}]
    assert json.loads(m.user("again")[0])["method"] == "turn/start"


def test_text_streams_once_tools_show_once_and_the_turn_ends() -> None:
    m = mspchat_mod.Msp()
    opened(m)
    out = m.incoming(rpc(method="item/delta", params={"itemId": "i1", "field": "text", "delta": "he"}))
    assert json.loads(out.ui[0])["event"]["delta"] == {"type": "text_delta", "text": "he"}
    done = lambda i: rpc(method="item/completed", params={"item": {"itemId": i, "kind": "agentMessage", "text": "hello"}})
    assert m.incoming(done("i1")).ui == []  # already streamed
    assert json.loads(m.incoming(done("i2")).ui[0])["message"]["content"][0]["text"] == "hello"
    assert m.incoming(rpc(method="item/delta", params={"itemId": "r", "field": "summary.0", "delta": "x"})).ui == []
    tool = lambda meth: rpc(method=meth, params={"item": {"itemId": "t1", "kind": "toolCall", "tool": "shell",
                                                          "args": "{\"command\":\"ls\"}"}})
    block = json.loads(m.incoming(tool("item/started")).ui[0])["message"]["content"][0]
    assert block["name"] == "shell" and block["input"] == {"command": "ls"}
    assert m.incoming(tool("item/completed")).ui == []
    end = json.loads(m.incoming(rpc(method="turn/completed", params={"terminal": "completed"})).ui[0])
    assert end["type"] == "result" and end["is_error"] is False and end["result"] == "hehello"


def test_failed_turn_and_errors_reach_the_session() -> None:
    m = mspchat_mod.Msp()
    opened(m)
    out = m.incoming(rpc(method="turn/completed", params={
        "terminal": "failed", "error": {"message": "log in"}}))
    assert "Muse error: log in" in out.ui[0] and json.loads(out.ui[1])["is_error"] is True
    assert "nope" in m.incoming(rpc(id=99, error={"code": -1, "message": "nope"})).ui[0]
    assert m.incoming("not json").ui == []


def test_approval_is_acked_asked_as_the_cli_tool_and_decided() -> None:
    m = mspchat_mod.Msp()
    opened(m)
    req = rpc(id="srv-1", method="approval/request", params={
        "approvalId": "ap-1", "currentRequirementId": {"approvalId": "ap-1", "sourceIndex": 0},
        "subject": {"kind": "shell", "toolName": "shell", "command": "rm x"},
        "availableChoices": [
            {"choiceId": "c-s", "decision": "approvedForSession", "scope": "session"},
            {"choiceId": "c-y", "decision": "approved", "scope": "once"},
            {"choiceId": "c-n", "decision": "denied", "scope": "once"}]})
    out = m.incoming(req)
    assert json.loads(out.send[0]) == {"jsonrpc": "2.0", "id": "srv-1", "result": {}}
    ask = json.loads(out.ui[0])
    assert ask["request_id"] == "ap-1" and ask["request"]["tool_name"] == "Bash"
    assert ask["request"]["input"]["command"] == "rm x"
    d = json.loads(m.answer("ap-1", True))
    assert d["method"] == "approval/decide" and d["params"]["choiceId"] == "c-y"
    assert d["params"]["requirementId"]["sourceIndex"] == 0 and d["params"]["sessionId"] == "s-1"
    assert m.answer("ap-1", True) is None  # answered once
    m.incoming(req)
    assert json.loads(m.answer("ap-1", False, "no"))["params"]["choiceId"] == "c-n"


def test_muse_chat_command_is_serve_without_effort_or_resume(home: Path) -> None:
    (home / "config.toml").write_text('[agents.muse]\ncommand = "muse"\nmodel = "m1"\neffort = "low"\n', encoding="utf-8")
    config = load_config(home)
    cmd = msession_mod.chat_command(config, "muse", None, None, "old-session")
    assert cmd[:2] == ["muse", "serve"] and "--disable-request-user-input" in cmd
    assert "--reasoning-effort" not in cmd and "--resume" not in cmd and cmd[-2:] == ["--model", "m1"]
    adapter = msession_mod.chat_adapter(config, "muse", "low")
    assert adapter is not None and adapter.effort == "low"


def run_session(home: Path, text: str, allow: bool) -> tuple[dict, list[str]]:
    """A Session over the fake host: one message, permission decided by ``allow``. Returns (turn view, calls)."""
    proc = subprocess.Popen([sys.executable, HOST], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    calls: list[str] = []

    def decide(tool, data):
        calls.append(f"{tool}:{data.get('command')}")
        return ("allow", "") if allow else ("deny", "not here")

    sess = msession_mod.Session(home, "lane-x", proc, decide, home, adapter=mspchat_mod.Msp("minimal"))
    try:
        assert sess.send(text, ["r1"], datetime.now(timezone.utc))
        end = time.monotonic() + 20
        while sess.busy and time.monotonic() < end:
            time.sleep(0.02)
        assert not sess.busy, "the turn never ended"
        return sess.views()[-1], calls
    finally:
        sess.terminate()


def test_session_chats_with_the_fake_host(home: Path) -> None:
    view, calls = run_session(home, "hello", True)
    assert view["answer"] == "echo: hello" and view["summary"]["done"] and not view["error"] and calls == []


def test_session_tool_permission_is_decided_by_code(home: Path) -> None:
    view, calls = run_session(home, "use a tool", True)
    assert calls == ["Bash:echo hi"] and view["answer"] == "tool ran"
    assert any(s["kind"] == "tool" for s in view["steps"])
    view, calls = run_session(home, "use a tool", False)
    assert calls == ["Bash:echo hi"] and view["answer"] == "tool denied"


def test_interrupt_and_resume_use_the_schema_params() -> None:
    m = mspchat_mod.Msp()
    assert m.interrupt() is None
    m.resume("s-old")
    hello = json.loads(m.hello()[0])
    res = json.loads(m.incoming(rpc(id=hello["id"], result={})).send[1])
    assert res["method"] == "session/resume"
    assert set(res["params"]) == {"commandId", "sessionId"} and res["params"]["sessionId"] == "s-old"
    m.incoming(rpc(id=res["id"], result={"session": {"sessionId": "s-old"}}))
    turn = json.loads(m.user("x")[0])
    m.incoming(rpc(id=turn["id"], result={"turnId": "t7"}))
    stop = json.loads(m.interrupt(retract=True))
    assert stop["method"] == "turn/interrupt"
    assert stop["params"]["sessionId"] == "s-old" and stop["params"]["turnId"] == "t7"
    assert stop["params"]["retract"] is True and stop["params"]["commandId"]
    m.incoming(rpc(method="turn/completed", params={"terminal": "interrupted"}))
    assert "turnId" not in json.loads(m.interrupt())["params"]


def test_resume_falls_back_to_a_new_session_when_refused() -> None:
    m = mspchat_mod.Msp()
    m.resume("old")
    hello = json.loads(m.hello()[0])
    res = json.loads(m.incoming(rpc(id=hello["id"], result={})).send[1])
    out = m.incoming(rpc(id=res["id"], error={"code": 1, "message": "gone"}))
    start = json.loads(out.send[0])
    assert start["method"] == "session/start" and out.ui == []
    out = m.incoming(rpc(id=start["id"], result={"session": {"sessionId": "new"}}))
    assert json.loads(out.ui[0])["session_id"] == "new"


def test_interrupt_ends_the_running_turn(home: Path) -> None:
    proc = subprocess.Popen([sys.executable, HOST], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    sess = msession_mod.Session(home, "lane-x", proc, lambda t, d: ("allow", ""), home, adapter=mspchat_mod.Msp())
    try:
        assert not sess.interrupt()  # idle
        assert sess.send("slow one", ["r1"], datetime.now(timezone.utc))
        end = time.monotonic() + 20
        while sess.busy and time.monotonic() < end:
            sess.interrupt()
            time.sleep(0.1)
        assert not sess.busy and sess.views()[-1]["interrupted"]
        assert sess.send("hello", ["r2"], datetime.now(timezone.utc))  # the session lives on
        end = time.monotonic() + 20
        while sess.busy and time.monotonic() < end:
            time.sleep(0.02)
        assert sess.views()[-1]["answer"] == "echo: hello"
    finally:
        sess.terminate()


def test_session_resumes_against_the_fake_host(home: Path) -> None:
    proc = subprocess.Popen([sys.executable, HOST], stdin=subprocess.PIPE, stdout=subprocess.PIPE)
    sess = msession_mod.Session(home, "lane-x", proc, lambda t, d: ("allow", ""), home,
                                adapter=msession_mod.mspchat_mod.Msp())
    sess.adapter.resume("ses-fake")
    try:
        assert sess.send("hello", ["r1"], datetime.now(timezone.utc))
        end = time.monotonic() + 20
        while sess.busy and time.monotonic() < end:
            time.sleep(0.02)
        assert sess.views()[-1]["answer"] == "echo: hello" and sess.session_id == "ses-fake"
    finally:
        sess.terminate()


def test_fake_host_refuses_requests_before_initialized() -> None:
    proc = subprocess.Popen([sys.executable, HOST], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
    try:
        proc.stdin.write(rpc(id=1, method="initialize", params={}) + "\n" + rpc(id=2, method="session/start", params={}) + "\n")
        proc.stdin.flush()
        proc.stdout.readline()
        err = json.loads(proc.stdout.readline())
        assert err["id"] == 2 and err["error"]["message"] == "Not initialized"
    finally:
        proc.kill()
