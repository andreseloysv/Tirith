"""A hook call starts fast: the console script loads only the hook path, not the whole collector."""

from __future__ import annotations

import json
import subprocess
import sys
import time
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
HEAVY = ("tirith.cli", "tirith.dispatch", "tirith.managerd", "tirith.mergeq", "tirith.autopilot",
         "tirith.collector", "tirith.review", "tirith.roadmap", "tirith.inbox")
PROBE = """
import json, sys
from tirith import entry
try:
    code = entry.main(sys.argv[1:])
except SystemExit as stop:
    code = str(stop.code)
print(json.dumps({"code": code, "heavy": sorted(m for m in sys.modules if m in %r)}))
""" % (HEAVY,)
PAYLOAD = json.dumps({"hook_event_name": "PostToolUse", "session_id": "s-1", "cwd": "/tmp",
                      "tool_name": "Edit", "tool_input": {"file_path": "/x/y"}})


def _call(home: Path, *argv: str) -> tuple[dict, float]:
    began = time.monotonic()
    done = subprocess.run([sys.executable, "-c", PROBE, *argv], input=PAYLOAD, text=True, capture_output=True,
                          cwd=ROOT, env={"PATH": "/usr/bin:/bin", "TIRITH_HOME": str(home), "HOME": str(home)},
                          timeout=60)
    return json.loads(done.stdout.strip().splitlines()[-1]), time.monotonic() - began


def test_hook_imports_little(tmp_path: Path) -> None:
    home = tmp_path / "home"
    home.mkdir()
    out, _ = _call(home, "hook", "--agent", "anthropic", "--event", "PostToolUse")
    assert out["code"] == 0
    assert out["heavy"] == [], f"the hook path imported heavy modules: {out['heavy']}"
    rows = [json.loads(line) for line in (home / "events.jsonl").read_text().splitlines()]
    assert rows and rows[-1]["kind"] == "hook" and rows[-1]["summary"] == "Edit /x/y"


def test_hook_imports_little_other_commands_still_use_the_cli(tmp_path: Path) -> None:
    home = tmp_path / "home"
    home.mkdir()
    out, _ = _call(home, "hook", "verify", "--agent", "nosuchagent")
    assert "tirith.cli" in out["heavy"]


def test_hook_imports_little_console_script_points_at_the_slim_entry() -> None:
    scripts = tomllib.loads((ROOT / "pyproject.toml").read_text())["project"]["scripts"]
    assert scripts["tirith"] == "tirith.entry:main"
