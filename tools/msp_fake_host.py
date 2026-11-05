#!/usr/bin/env python3
"""A fake MSP host (what `muse serve` is on the wire): JSON-RPC 2.0, one JSON object per line on stdio.

Used by the desktop chat adapter's test (app/src-tauri/src/msp.rs). It answers `initialize`,
`session/start`, `session/resume`, `turn/start`, `turn/interrupt` and `approval/decide`. A turn whose text contains "tool" runs a tool call
that first needs an approval (`approval/request`); any other turn just streams "echo: <text>".
No model, no network, no ports.

If MSP_SCHEMA points at the `msp.schema.json` written by `muse schema generate-json-schema --out DIR`,
every request it receives is checked against the schema's params for that method (types, required keys,
enums through the `jsonschema` package when installed, else required keys only) and a bad one is
answered with a JSON-RPC error, so a test fails on a malformed frame.
"""
import json
import os
import sys

SCHEMA = None
if os.environ.get("MSP_SCHEMA"):
    with open(os.environ["MSP_SCHEMA"]) as f:
        SCHEMA = json.load(f)
try:
    import jsonschema
except ImportError:
    jsonschema = None


def out(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()


def notify(method, params):
    out({"jsonrpc": "2.0", "method": method, "params": params})


def check(method, params):
    """Returns an error string for params that do not fit the schema, else None."""
    if SCHEMA is None:
        return None
    spec = SCHEMA["methods"].get(method)
    if spec is None:
        return f"unknown method {method}"
    schema = dict(spec["params"])
    schema["$defs"] = SCHEMA["$defs"]
    if jsonschema is not None:
        try:
            jsonschema.validate(params, schema)
        except jsonschema.ValidationError as e:
            return e.message
        return None
    ref = schema.get("$ref", "").split("/")[-1]
    body = SCHEMA["$defs"].get(ref, schema)
    missing = [k for k in body.get("required", []) if k not in params]
    return f"missing {missing}" if missing else None


def item(item_id, kind, **kw):
    return {"itemId": item_id, "kind": kind, "revision": 1, "status": kw.pop("status", "completed"), **kw}


def main():
    session = "ses-fake"
    turn_no = 0
    waiting = None  # (approvalId, tool item) while a tool call waits for approve/deny
    for line in sys.stdin:
        msg = json.loads(line)
        method, mid, params = msg.get("method"), msg.get("id"), msg.get("params", {})
        if method is None:
            continue  # a receipt for approval/request
        bad = check(method, params)
        if bad:
            out({"jsonrpc": "2.0", "id": mid, "error": {"code": -32602, "message": f"{method}: {bad}"}})
            continue
        if method == "initialize":
            out({"jsonrpc": "2.0", "id": mid, "result": {
                "experimentalApi": False, "grantedCapabilities": [], "museHome": "/fake",
                "platformFamily": "unix", "platformOs": "macos",
                "schema": {"fingerprint": "sha256:" + "0" * 64, "version": 1},
                "serverInfo": {"name": "fake", "version": "0"}, "userAgent": "fake/0"}})
        elif method == "session/start":
            out({"jsonrpc": "2.0", "id": mid, "result": {"session": {"sessionId": session}}})
        elif method == "session/resume":
            if params["sessionId"] != session:
                out({"jsonrpc": "2.0", "id": mid, "error": {"code": -32004, "message": "no such session"}})
                continue
            out({"jsonrpc": "2.0", "id": mid, "result": {"session": {"sessionId": session}}})
        elif method == "turn/interrupt":
            out({"jsonrpc": "2.0", "id": mid, "result": {
                "commandId": params["commandId"], "status": "accepted", "turnId": params.get("turnId") or f"t{turn_no}"}})
            if waiting:
                waiting = None
                notify("turn/completed", {"sessionId": session, "turnId": f"t{turn_no}", "terminal": "interrupted",
                                          "viewCursor": f"c{turn_no}"})
        elif method == "turn/start":
            turn_no += 1
            text = params["input"][0].get("text", "")
            out({"jsonrpc": "2.0", "id": mid, "result": {
                "commandId": params["commandId"], "disposition": "started", "startedNewTurn": True,
                "status": "accepted", "turnId": f"t{turn_no}"}})
            if "tool" in text:
                tool = item(f"tool{turn_no}", "toolCall", status="inProgress", tool="shell",
                            args=json.dumps({"command": "echo hi"}))
                notify("item/started", {"sessionId": session, "item": tool})
                out({"jsonrpc": "2.0", "id": f"ap-req-{turn_no}", "method": "approval/request", "params": {
                    "approvalId": f"ap{turn_no}", "sessionId": session,
                    "currentRequirementId": {"approvalId": f"ap{turn_no}", "sourceIndex": 0},
                    "subject": {"kind": "shell", "toolName": "shell", "command": "echo hi"},
                    "availableChoices": [
                        {"choiceId": "yes", "decision": "approved", "label": "Allow", "scope": "once"},
                        {"choiceId": "no", "decision": "denied", "label": "Deny", "scope": "once"}]}})
                waiting = (f"ap{turn_no}", tool)
            else:
                reply(session, turn_no, f"echo: {text}")
        elif method == "approval/decide":
            out({"jsonrpc": "2.0", "id": mid, "result": {
                "approvalId": params["approvalId"], "commandId": params["commandId"],
                "status": "accepted", "terminal": True}})
            if waiting and waiting[0] == params["approvalId"]:
                done = dict(waiting[1], status="completed")
                notify("item/completed", {"sessionId": session, "item": done})
                reply(session, turn_no, "tool ran" if params["choiceId"] == "yes" else "tool denied")
                waiting = None
        else:
            out({"jsonrpc": "2.0", "id": mid, "error": {"code": -32601, "message": f"no {method}"}})


def reply(session, n, text):
    iid = f"msg{n}"
    notify("item/started", {"sessionId": session, "item": item(iid, "agentMessage", status="inProgress", text="")})
    for part in (text[: len(text) // 2], text[len(text) // 2:]):
        notify("item/delta", {"sessionId": session, "itemId": iid, "field": "text", "delta": part,
                              "viewCursor": f"c{n}"})
    notify("item/completed", {"sessionId": session, "item": item(iid, "agentMessage", text=text)})
    notify("turn/completed", {"sessionId": session, "turnId": f"t{n}", "terminal": "completed",
                              "viewCursor": f"c{n}", "sourceRange": {
                                  "first": {"id": "a", "sequence": 1}, "last": {"id": "b", "sequence": 2},
                                  "stream": {"id": "s", "kind": "session"}},
                              "usage": {"cachedTokens": 0, "inputTokens": 1, "outputTokens": 1, "reasoningTokens": 0}})


if __name__ == "__main__":
    main()
