#!/usr/bin/env python3
"""Manual integration check of the desktop chat protocol (not run in CI, spends a few tokens).

Starts the agent exactly as the app does (`tirith lane run --interactive ... -- <command> <chat_args>`
from `~/.tirith/config.toml`), sends one user message as a line of streaming JSON, answers any
permission question with a deny, prints every streamed JSON line until the `result` line, then closes
stdin so the agent exits.

    CHAT_CHECK_STEP=<N> CHAT_CHECK_ITEM=<open item> python3 tools/chat_check.py [alias] [project] ["message"]

An alias with `chat_protocol = "msp"` (a host that speaks MSP over stdio, e.g. `muse serve`) is driven through
the same adapter the collector uses (`collector/tirith/mspchat.py`): `initialize` -> `session/start` -> one
`turn/start`; the adapter's stream-json output is what gets printed. Add `--interrupt` as the last argument to
send `turn/interrupt` as soon as the turn is running and check that the turn ends.
"""
import json
import os
import subprocess
import sys
import time
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "collector"))
from tirith import agentrun, mspchat  # noqa: E402

INTERRUPT = "--interrupt" in sys.argv
if INTERRUPT:
    sys.argv.remove("--interrupt")
alias = sys.argv[1] if len(sys.argv) > 1 else "anthropic"
project = sys.argv[2] if len(sys.argv) > 2 else "tirith"
text = sys.argv[3] if len(sys.argv) > 3 else "reply with ok"

config = tomllib.loads((Path.home() / ".tirith" / "config.toml").read_text())

class _Cfg:  # what agentrun.agent_settings reads: the preset defaults merged with [agents.<alias>]
    agents = config.get("agents", {})


agent = agentrun.agent_settings(_Cfg, alias)
lane_id = f"chat-check-{int(time.time())}"
brief = Path.home() / ".tirith" / "chats" / f"{lane_id}.md"
brief.parent.mkdir(parents=True, exist_ok=True)
brief.write_text(f"Chat check with {alias}.\n")
# no agent starts without an open plan item: CHAT_CHECK_STEP / CHAT_CHECK_ITEM name it (item text or i-xxxxxxxx id)
plan = []
if os.environ.get("CHAT_CHECK_ITEM"):
    plan = ["--step", os.environ.get("CHAT_CHECK_STEP", "9"), "--item", os.environ["CHAT_CHECK_ITEM"]]
cmd = [str(ROOT / "bin" / "tirith"), "lane", "run", "--interactive", "--project", project,
       "--class", "chat", "--id", lane_id, "--feature", f"Chat check with {alias}",
       "--brief", str(brief), *plan, "--", agent["command"], *agent["chat_args"]]

proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, cwd=ROOT)


def write(line: str) -> None:
    proc.stdin.write(line + "\n")
    proc.stdin.flush()


def send(msg: dict) -> None:
    write(json.dumps(msg))


msp = mspchat.Msp(str(agent.get("chat_effort") or agent.get("effort") or "minimal")) \
    if agent.get("chat_protocol") == mspchat.PROTOCOL else None
if msp:
    for line in msp.hello() + msp.user(text):
        write(line)
else:
    # The documented streaming input message: one user turn, content as a list of blocks.
    send({"type": "user", "message": {"role": "user", "content": [{"type": "text", "text": text}]}})
result = None
interrupted = False
for raw in proc.stdout:
    raw = raw.rstrip("\n")
    if msp:
        print("<-", raw)
        step = msp.incoming(raw)
        for line in step.send:
            write(line)
        lines = step.ui
        if INTERRUPT and not interrupted and msp.turn:
            interrupted = True
            write(msp.interrupt())
    else:
        print(raw)
        lines = [raw]
    for line in lines:
        if msp:
            print("ui", line)
        try:
            obj = json.loads(line)
        except ValueError:
            continue
        if obj.get("type") == "control_request" and obj["request"].get("subtype") == "can_use_tool":
            if msp:
                deny = msp.answer(obj["request_id"], False, "chat_check denies tools")
                if deny:
                    write(deny)
            else:
                send({"type": "control_response", "response": {
                    "subtype": "success", "request_id": obj["request_id"],
                    "response": {"behavior": "deny", "message": "chat_check denies tools"}}})
        elif obj.get("type") == "result":
            result = obj
    if result:
        break
proc.stdin.close()
proc.wait(timeout=60)
if result is None:
    sys.exit("no result line before the agent exited")
print("\nRESULT:", result.get("subtype"), "|", result.get("result"))
sys.exit(1 if result.get("is_error") else 0)
