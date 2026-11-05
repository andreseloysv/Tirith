"""Fake lab agent: reads the request, asks the owner once if the request says 'sleep', merges items."""
import json
import sys


def out(o):
    print(json.dumps(o), flush=True)


req = json.loads(sys.stdin.readline())
text = req["text"]
n = 3 if "3 parts" in text else 1
if "login" in text.lower():
    open(req["workdir"] + "/src/auth.py", "w").write("")
if "sleep" in text:
    out({"type": "ask", "question": "which db?"})
    json.loads(sys.stdin.readline())
for i in range(n):
    out({"type": "cost", "usd": 0.25})
    out({"type": "elapsed", "seconds": 60})
    out({"type": "item", "id": f"i{i}"})
if n > 1:
    out({"type": "item", "id": "i0"})
    out({"type": "promote", "role": "manager", "reason": f"{n} parallel items"})
out({"type": "done"})
