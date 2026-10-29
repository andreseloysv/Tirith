"""Every configured agent is guarded for real: codex and copilot get hooks, rules are Active only when all are."""

from __future__ import annotations

import io
import json
from pathlib import Path

import pytest

from conftest import write_config
from tirith import gates, hooks
from tirith import setup as setup_mod
from tirith.cli import main
from tirith.collector import Collector
from tirith.home import load_config

CONFIG = ('[agents.anthropic]\ncommand = "alpha-cli"\nsettings = "%s"\n\n'
          '[agents.codex]\ncommand = "beta-cli"\n\n[agents.copilot]\ncommand = "gamma-cli"\n')


@pytest.fixture
def machine(home: Path, tmp_path: Path, monkeypatch) -> Path:
    owner = tmp_path / "owner"
    owner.mkdir()
    folder = tmp_path / "proj"
    folder.mkdir()
    real = tmp_path / "real"
    real.mkdir()
    for name in ("alpha-cli", "beta-cli", "gamma-cli"):
        (real / name).write_text("#!/bin/sh\n", encoding="utf-8")
        (real / name).chmod(0o755)
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(folder)}],
                 CONFIG % (owner / "alpha" / "settings.json"))
    monkeypatch.setenv("HOME", str(owner))
    monkeypatch.delenv("COPILOT_HOME", raising=False)
    monkeypatch.setenv("PATH", f"{real}:/usr/bin:/bin")
    return owner


def _setup(home: Path) -> dict:
    return next(p for p in Collector(home).build_state()["projects"] if p["id"] == "demo")["setup"]


def test_default_locations(machine: Path) -> None:
    assert hooks.default_settings("codex") == machine / ".codex" / "hooks.json"
    assert hooks.default_settings("copilot") == machine / ".copilot" / "hooks" / "tirith.json"


def test_copilot_home_moves_the_file(machine: Path, monkeypatch) -> None:
    monkeypatch.setenv("COPILOT_HOME", str(machine / "cp"))
    assert hooks.default_settings("copilot") == machine / "cp" / "hooks" / "tirith.json"


def test_install_codex_keeps_other_hooks_backs_up_and_is_idempotent(machine: Path) -> None:
    path = machine / ".codex" / "hooks.json"
    path.parent.mkdir()
    mine = {"hooks": {"PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "echo mine"}]}]}}
    path.write_text(json.dumps(mine), encoding="utf-8")
    hooks.install("codex")
    first = path.read_text(encoding="utf-8")
    hooks.install("codex")
    assert path.read_text(encoding="utf-8") == first
    assert json.loads((machine / ".codex" / "hooks.json.tirith-bak").read_text(encoding="utf-8")) == mine
    data = json.loads(first)
    assert data["hooks"]["PreToolUse"][0]["hooks"][0]["command"] == "echo mine"
    assert hooks.has_hooks(path, "codex") and not hooks.has_hooks(path, "copilot")
    assert set(data["hooks"]) == set(hooks.INSTALL_EVENTS)


def test_install_copilot_writes_its_own_flat_file(machine: Path) -> None:
    path = hooks.install("copilot")
    data = json.loads(path.read_text(encoding="utf-8"))
    assert data["version"] == 1
    entry = data["hooks"]["PreToolUse"][0]
    assert entry["type"] == "command" and "hook --agent copilot --event PreToolUse" in entry["bash"]
    assert hooks.has_hooks(path, "copilot")
    hooks.install("copilot")
    assert len(json.loads(path.read_text(encoding="utf-8"))["hooks"]["PreToolUse"]) == 1
    hooks.uninstall("copilot")
    assert not hooks.has_hooks(path, "copilot")
    assert "hooks" not in json.loads(path.read_text(encoding="utf-8"))


def test_hook_install_records_settings_in_config(machine: Path, home: Path) -> None:
    assert main(["--home", str(home), "hook", "install", "--agent", "codex"]) == 0
    text = (home / "config.toml").read_text(encoding="utf-8")
    assert '[agents.codex]\ncommand = "beta-cli"\nsettings = "~/.codex/hooks.json"' in text
    assert "settings" not in text.split("[agents.copilot]")[1]
    assert (home / "config.toml.tirith-bak").exists()
    assert load_config(home).agents["codex"]["settings"] == "~/.codex/hooks.json"


def test_agents_install_guards_everything_and_is_idempotent(machine: Path, home: Path) -> None:
    assert main(["--home", str(home), "agents", "install"]) == 0
    once = (home / "config.toml").read_text(encoding="utf-8")
    assert main(["--home", str(home), "agents", "install"]) == 0
    assert (home / "config.toml").read_text(encoding="utf-8") == once
    config = load_config(home)
    for alias in ("anthropic", "codex", "copilot"):
        assert hooks.has_hooks(hooks.default_settings(alias, config.agents), alias)
    assert config.agents["copilot"]["settings"] == "~/.copilot/hooks/tirith.json"


def test_rules_active_only_when_every_agent_is_guarded(machine: Path, home: Path) -> None:
    assert main(["--home", str(home), "shim", "install"]) == 0
    hooks.install("anthropic", None, agents=load_config(home).agents)
    s = _setup(home)
    assert s["gates_active"] is False
    assert any("codex" in p for p in s["gates_problems"]) and any("copilot" in p for p in s["gates_problems"])
    assert all(not r["active"] for r in s["rules"] if r["enforced_by"] == ["hook"])
    hooks.install("codex")
    assert _setup(home)["gates_active"] is False  # copilot still open
    hooks.install("copilot")
    s = _setup(home)
    assert s["gates_active"] is True and s["gates_problems"] == [] and s["gates_summary"] == ""
    assert all(r["active"] for r in s["rules"] if "git" not in r["enforced_by"] or "hook" in r["enforced_by"])
    hooks.uninstall("copilot")  # the collector would put it back (protect); the read-only check sees it gone
    assert setup_mod.machine_state(load_config(home), home=home).active is False


def test_hook_verify_for_codex(machine: Path, home: Path) -> None:
    assert main(["--home", str(home), "hook", "verify", "--agent", "codex"]) == 1
    hooks.install("codex")
    assert main(["--home", str(home), "hook", "verify", "--agent", "codex"]) == 0


def test_protect_heals_codex_and_copilot(machine: Path, home: Path) -> None:
    config = load_config(home)
    hooks.install("codex")
    hooks.install("copilot")
    rows = {r["agent"]: r for r in hooks.protect(home, config)}
    assert rows["codex"]["ok"] and rows["copilot"]["ok"]
    hooks.uninstall("copilot")
    rows = {r["agent"]: r for r in hooks.protect(home, config)}
    assert rows["copilot"]["ok"] and hooks.has_hooks(hooks.default_settings("copilot"), "copilot")


@pytest.mark.parametrize("agent,tool", [("codex", "Bash"), ("copilot", "bash")])
def test_pretooluse_of_codex_and_copilot_is_gated(machine: Path, home: Path, agent: str, tool: str) -> None:
    payload = {"hook_event_name": "PreToolUse", "session_id": "s1", "cwd": str(machine),
               "tool_name": tool, "tool_input": {"command": "git commit --no-verify -m x"}}
    assert hooks.run_hook(home, agent, None, io.StringIO(json.dumps(payload))) == 2
    payload["tool_input"] = {"command": "ls"}
    assert hooks.run_hook(home, agent, None, io.StringIO(json.dumps(payload))) == 0


def test_agent_settings_files_are_protected_from_agents(machine: Path, home: Path) -> None:
    paths = gates.protected_paths(load_config(home), home)
    assert machine / ".codex" / "hooks.json" in paths
    assert machine / ".copilot" / "hooks" / "tirith.json" in paths
