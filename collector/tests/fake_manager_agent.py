"""A fake stream-json agent for the persistent manager session tests (no credits, no network).

stdin: one JSON object per line (user messages, control_response, control_request interrupt).
stdout: system init, partial text deltas, assistant / user (tool) events, control_request questions, result.
The text of the user message steers it: ``PERM:<Tool>:<json input>`` asks a permission question and waits for
the answer, ``SLOW`` waits for an interrupt, ``CRASH`` exits with 3, ``/compact`` answers with a small context.
Env: ``FAKE_LOG`` (file: one JSON line per start and per message), ``FAKE_CTX`` (context tokens reported).
"""

import json
import os
import sys
import time


def out(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()


def log(obj):
    path = os.environ.get("FAKE_LOG")
    if path:
        with open(path, "a") as fh:
            fh.write(json.dumps(obj) + "\n")


def read():
    line = sys.stdin.readline()
    return json.loads(line) if line else None


def usage(tokens):
    return {"input_tokens": tokens, "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0}


def main():
    argv = sys.argv[1:]
    resume = argv[argv.index("--resume") + 1] if "--resume" in argv else None
    sid = resume or "sess-" + str(os.getpid())
    log({"start": argv, "pid": os.getpid(), "resume": resume})
    announced = False
    while True:
        msg = read()
        if msg is None:
            return
        if msg.get("type") != "user":
            continue
        text = "".join(b.get("text", "") for b in msg["message"]["content"])
        log({"message": text})
        if not announced:
            out({"type": "system", "subtype": "init", "session_id": sid})
            announced = True
        if "[[CRASH]]" in text:
            sys.exit(3)
        ctx = (int(os.environ.get("FAKE_COMPACT_CTX", "1000")) if text.startswith("/compact")
               else int(os.environ.get("FAKE_CTX", "1000")))
        for piece in ("Looking ", "into it"):
            out({"type": "stream_event", "session_id": sid, "event": {
                "type": "content_block_delta", "delta": {"type": "text_delta", "text": piece}}})
        if "[[SLOW]]" in text:
            while True:
                req = read()
                if req is None:
                    return
                if req.get("type") == "control_request" and req["request"].get("subtype") == "interrupt":
                    out({"type": "result", "subtype": "error_during_execution", "is_error": True,
                         "session_id": sid, "result": "", "usage": usage(ctx)})
                    break
            continue
        if "PERM:" in text:
            tool, _, raw = text.split("PERM:", 1)[1].partition(":")
            data = json.loads(raw.splitlines()[0])
            out({"type": "control_request", "request_id": "r-" + tool, "request": {
                "subtype": "can_use_tool", "tool_name": tool, "input": data}})
            while True:
                reply = read()
                if reply is None:
                    return
                if reply.get("type") == "control_response":
                    log({"permission": reply["response"]["response"]["behavior"], "tool": tool})
                    break
        out({"type": "assistant", "session_id": sid, "message": {"role": "assistant", "usage": usage(ctx), "content": [
            {"type": "text", "text": "Checking the plan."},
            {"type": "tool_use", "id": "t1", "name": "Read", "input": {"file_path": "docs/TODO.md"}}]}})
        out({"type": "user", "session_id": sid, "message": {"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": "t1", "content": "# TODO\n## 1. First"}]}})
        out({"type": "assistant", "session_id": sid, "message": {"role": "assistant", "usage": usage(ctx), "content": [
            {"type": "tool_use", "id": "t2", "name": "TodoWrite", "input": {"todos": [
                {"content": "Read plan", "status": "completed"}, {"content": "Answer", "status": "in_progress"}]}},
            {"type": "tool_use", "id": "t3", "name": "Edit", "input": {
                "file_path": "/tmp/x.py", "old_string": "a = 1", "new_string": "a = 2\nb = 3"}}]}})
        time.sleep(0.05)
        out({"type": "result", "subtype": "success", "is_error": False, "session_id": sid,
             "result": "Answer from the fake manager.", "total_cost_usd": 0.0123, "duration_ms": 1500,
             "usage": usage(ctx)})


main()
