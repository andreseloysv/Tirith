#!/usr/bin/env python3
"""Install the supported browser MCP and a Codex profile used only by Tirith (Python 3.11+)."""
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import time
import tomllib

root = Path(__file__).resolve().parents[1]
home = Path(os.environ.get("TIRITH_HOME", Path.home() / ".tirith"))
codex = Path(os.environ.get("CODEX_HOME", Path.home() / ".codex"))
runtime = home / "browser-mcp"
config = home / "config.toml"
profile = codex / "tirith-browser.config.toml"
node, npm = shutil.which("node"), shutil.which("npm")
if not node or not npm:
    raise SystemExit("Node.js and npm must be installed before setting up browser tools.")
original = config.read_text() if config.exists() else ""
existing = tomllib.loads(original).get("agents", {}).get("codex", {}).get("profile")
if existing and existing != "tirith-browser":
    raise SystemExit(f"Codex already uses profile {existing!r}; merge its settings with the browser profile explicitly.")
runtime.mkdir(parents=True, exist_ok=True)
subprocess.run([npm, "install", "--prefix", str(runtime), "--save-exact", "--ignore-scripts", "--no-audit", "--no-fund", "@playwright/mcp@0.0.85"], check=True)
launcher = runtime / "launch.mjs"
shutil.copyfile(root / "scripts/codex-browser.mjs", launcher)
text = f'''# Tirith only: replace the unavailable desktop Browser runtime with a standard browser MCP.
[mcp_servers.node_repl]
enabled = false

[plugins."browser@openai-bundled"]
enabled = false

[mcp_servers.tirith_browser]
command = {json.dumps(node)}
args = [{json.dumps(str(launcher))}]
startup_timeout_sec = 30
tool_timeout_sec = 90

[mcp_servers.tirith_browser.env]
TIRITH_HOME = {json.dumps(str(home))}
'''
tomllib.loads(text)
codex.mkdir(parents=True, exist_ok=True)

def save(path, value):
    if path.exists() and path.read_text() == value:
        return
    if path.exists():
        backup = path.with_name(path.name + f".before-browser-{time.time_ns()}")
        shutil.copy2(path, backup)
        backup.chmod(0o600)
    temporary = path.with_name(path.name + ".browser-new")
    temporary.write_text(value)
    temporary.chmod(0o600)
    temporary.replace(path)

save(profile, text)
if not existing:
    header = re.search(r"(?m)^\[agents\.codex\][ \t]*(?:#.*)?$", original)
    if header:
        original = original[:header.end()] + '\nprofile = "tirith-browser"' + original[header.end():]
    else:
        original += '\n[agents.codex]\ncommand = "codex"\nprofile = "tirith-browser"\n'
    assert tomllib.loads(original)["agents"]["codex"]["profile"] == "tirith-browser"
    save(config, original)
print("Installed browser MCP and Tirith's Codex profile.")
print("Browser profile:", home / "browser/profile")
print("New Codex turns use it after the daemon has the profile-support update.")
