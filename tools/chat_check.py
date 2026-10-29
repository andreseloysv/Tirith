#!/usr/bin/env python3
"""Manual integration check of the desktop chat protocol (not run in CI, spends a few tokens).

Starts the agent exactly as the app does (`tirith lane run --interactive ... -- <command> <chat_args>`
from `~/.tirith/config.toml`), sends one user message as a line of streaming JSON, answers any
permission question with a deny, prints every streamed JSON line until the `result` line, then closes
stdin so the agent exits.

    python3 tools/chat_check.py [alias] [project] ["message"]
"""
import json
import subprocess
import sys
import time
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
alias = sys.argv[1] if len(sys.argv) > 1 else "anthropic"
project = sys.argv[2] if len(sys.argv) > 2 else "tirith"
text = sys.argv[3] if len(sys.argv) > 3 else "reply with ok"

config = tomllib.loads((Path.home() / ".tirith" / "config.toml").read_text())
agent = config["agents"][alias]
lane_id = f"chat-check-{int(time.time())}"
brief = Path.home() / ".tirith" / "chats" / f"{lane_id}.md"
brief.parent.mkdir(parents=True, exist_ok=True)
brief.write_text(f"Chat check with {alias}.\n")
cmd = [str(ROOT / "bin" / "tirith"), "lane", "run", "--interactive", "--project", project,
       "--class", "chat", "--id", lane_id, "--feature", f"Chat check with {alias}",
       "--brief", str(brief), "--", agent["command"], *agent["chat_args"]]

proc = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, cwd=ROOT)


def send(msg: dict) -> None:
    proc.stdin.write(json.dumps(msg) + "\n")
    proc.stdin.flush()


# The documented streaming input message: one user turn, content as a list of blocks.
send({"type": "user", "message": {"role": "user", "content": [{"type": "text", "text": text}]}})
result = None
for line in proc.stdout:
    line = line.rstrip("\n")
    print(line)
    try:
        obj = json.loads(line)
    except ValueError:
        continue
    if obj.get("type") == "control_request" and obj["request"].get("subtype") == "can_use_tool":
        send({"type": "control_response", "response": {
            "subtype": "success", "request_id": obj["request_id"],
            "response": {"behavior": "deny", "message": "chat_check denies tools"}}})
    elif obj.get("type") == "result":
        result = obj
        break
proc.stdin.close()
proc.wait(timeout=60)
if result is None:
    sys.exit("no result line before the agent exited")
print("\nRESULT:", result.get("subtype"), "|", result.get("result"))
sys.exit(1 if result.get("is_error") else 0)
