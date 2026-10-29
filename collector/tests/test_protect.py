"""Protected agent hooks: the settings-edit gate, healing removed hooks, `hook verify`."""

from __future__ import annotations

import io
import json
from pathlib import Path

import pytest

from conftest import write_config
from tirith import hooks
from tirith.cli import main
from tirith.collector import Collector
from tirith.home import load_config
from tirith.store import read_json, read_jsonl


@pytest.fixture
def setup(home: Path, tmp_path: Path):
    settings = tmp_path / "agent" / "settings.json"
    settings.parent.mkdir()
    write_config(home, [], f'[agents.anthropic]\ncommand = "alpha-cli"\nsettings = "{settings}"\n')
    return home, settings


def _call(monkeypatch, cwd, tool, tool_input) -> int:
    payload = {"hook_event_name": "PreToolUse", "session_id": "s1", "cwd": str(cwd),
               "tool_name": tool, "tool_input": tool_input}
    monkeypatch.setattr("sys.stdin", io.StringIO(json.dumps(payload)))
    return main(["hook", "--agent", "anthropic"])


@pytest.mark.parametrize("tool,tool_input", [
    ("Edit", {"file_path": "{s}"}),
    ("Write", {"file_path": "{s}"}),
    ("Edit", {"file_path": "{h}/config.toml"}),
    ("Bash", {"command": "sed -i '' 's/hook/x/' {s}"}),
    ("Bash", {"command": "echo '{{}}' > {s}"}),
    ("Bash", {"command": "python3 -c \"open('{s}','w').write('{{}}')\""}),
    ("Bash", {"command": "cd {d} && rm settings.json"}),
    ("Bash", {"command": "tirith hook uninstall --agent anthropic"}),
    ("Bash", {"command": "mv {h}/config.toml /tmp/x"}),
])
def test_gate_blocks(setup, monkeypatch, capsys, tmp_path, tool, tool_input) -> None:
    home, settings = setup
    fill = {"s": str(settings), "h": str(home), "d": str(settings.parent)}
    data = {k: v.format(**fill) for k, v in tool_input.items()}
    assert _call(monkeypatch, tmp_path, tool, data) == 2
    assert "tirith: settings-edit:" in capsys.readouterr().err
    assert read_jsonl(home / "events.jsonl")[-1]["rule"] == "settings-edit"


@pytest.mark.parametrize("tool,tool_input", [
    ("Read", {"file_path": "{s}"}),
    ("Edit", {"file_path": "{d}/other.json"}),
    ("Bash", {"command": "cat {s}"}),
    ("Bash", {"command": "grep hooks {h}/config.toml"}),
    ("Bash", {"command": "echo settings.json"}),
])
def test_gate_allows(setup, monkeypatch, tmp_path, tool, tool_input) -> None:
    home, settings = setup
    fill = {"s": str(settings), "h": str(home), "d": str(settings.parent)}
    data = {k: v.format(**fill) for k, v in tool_input.items()}
    assert _call(monkeypatch, tmp_path, tool, data) == 0


def test_gate_off_with_config(setup, monkeypatch, tmp_path) -> None:
    home, settings = setup
    (home / "config.toml").write_text((home / "config.toml").read_text() + "\n[gates]\nenabled = false\n")
    assert _call(monkeypatch, tmp_path, "Edit", {"file_path": str(settings)}) == 0


def test_removed_hooks_are_healed_and_other_keys_kept(setup) -> None:
    home, settings = setup
    settings.write_text(json.dumps({"theme": "dark", "hooks": {"Stop": [
        {"hooks": [{"type": "command", "command": "their-own-tool"}]}]}}))
    hooks.install("anthropic", settings, "/opt/tirith")
    config = load_config(home)

    first = hooks.protect(home, config)
    assert first[0]["ok"] and first[0]["healed_at"] is None
    assert not [r for r in read_jsonl(home / "events.jsonl") if r.get("kind") == "violation"]

    hooks.uninstall("anthropic", settings)
    assert not hooks.has_hooks(settings, "anthropic")
    data = json.loads(settings.read_text())
    data["theme"] = "light"
    settings.write_text(json.dumps(data))

    second = hooks.protect(home, config)
    assert second[0]["ok"] and second[0]["healed_at"]
    assert hooks.has_hooks(settings, "anthropic")
    healed = json.loads(settings.read_text())
    assert healed["theme"] == "light"
    assert any(h["command"] == "their-own-tool" for g in healed["hooks"]["Stop"] for h in g["hooks"])
    assert "/opt/tirith" in healed["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
    violations = [r for r in read_jsonl(home / "events.jsonl") if r.get("kind") == "violation"]
    assert [v["rule"] for v in violations] == ["hooks-removed"] and violations[0]["agent"] == "anthropic"

    hooks.protect(home, config)  # intact again: no second violation
    assert len([r for r in read_jsonl(home / "events.jsonl") if r.get("kind") == "violation"]) == 1


def test_deleted_settings_file_is_recreated(setup) -> None:
    home, settings = setup
    hooks.install("anthropic", settings)
    config = load_config(home)
    hooks.protect(home, config)
    settings.unlink()
    assert hooks.protect(home, config)[0]["ok"]
    assert hooks.has_hooks(settings, "anthropic")


def test_never_installed_is_reported_not_installed(setup) -> None:
    home, settings = setup
    rows = hooks.protect(home, load_config(home))
    assert rows[0]["ok"] is False and not settings.exists()


def test_collector_heals_each_cycle_and_exposes_it(setup, tmp_path) -> None:
    home, settings = setup
    hooks.install("anthropic", settings)
    collector = Collector(home)
    state = collector.cycle()
    assert read_json(home / "protect.json", {})["anthropic"]["ok"]
    hooks.uninstall("anthropic", settings)
    collector.cycle()
    assert hooks.has_hooks(settings, "anthropic")
    assert any(v["rule"] == "hooks-removed" for v in collector.cycle()["violations"])
    del state


def test_verify_command(setup, capsys) -> None:
    home, settings = setup
    assert main(["hook", "verify", "--agent", "anthropic"]) == 1
    assert "Hooks missing" in capsys.readouterr().out
    hooks.install("anthropic", settings)
    assert main(["hook", "verify", "--agent", "anthropic"]) == 0
    assert "Hooks protected · the collector has not checked yet" in capsys.readouterr().out
    hooks.protect(home, load_config(home))
    assert main(["hook", "verify", "--agent", "anthropic"]) == 0
    assert "Hooks protected · last check" in capsys.readouterr().out
