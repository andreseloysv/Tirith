"""The Setup card's "rules are not active" banner: one check, shared with `tirith shim verify`."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from conftest import write_config
from tirith import actions as actions_mod
from tirith import gates
from tirith.cli import main
from tirith.collector import Collector
from tirith.hooks import INSTALL_EVENTS, hook_command
from tirith.home import load_config

jsonschema = pytest.importorskip("jsonschema")
ROOT = Path(__file__).resolve().parent.parent.parent
SCHEMA = json.loads((ROOT / "collector" / "state.schema.json").read_text(encoding="utf-8"))


@pytest.fixture
def machine(home: Path, tmp_path: Path, monkeypatch) -> Path:
    """A temp owner machine: one agent 'anthropic' whose real binary is on PATH, no hooks, no shims."""
    folder = tmp_path / "proj"
    folder.mkdir()
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(folder)}],
                 '[agents.anthropic]\ncommand = "alpha-cli"\nsettings = "%s"\n' % (tmp_path / "settings.json"))
    real = tmp_path / "real"
    real.mkdir()
    exe = real / "alpha-cli"
    exe.write_text("#!/bin/sh\n", encoding="utf-8")
    exe.chmod(0o755)
    monkeypatch.setenv("HOME", str(tmp_path / "owner"))
    monkeypatch.setenv("PATH", f"{real}:/usr/bin:/bin")
    return folder


def _hooks(tmp_path: Path) -> None:
    data = {"hooks": {e: [{"hooks": [{"type": "command", "command": hook_command("anthropic", e)}]}]
                      for e in INSTALL_EVENTS}}
    (tmp_path / "settings.json").write_text(json.dumps(data), encoding="utf-8")


def _setup(home: Path) -> dict:
    state = Collector(home).build_state()
    setup = next(p for p in state["projects"] if p["id"] == "demo")["setup"]
    jsonschema.validate(setup, {**SCHEMA["$defs"]["setup"], "$defs": SCHEMA["$defs"]})
    return setup


def test_nothing_installed_says_what_is_missing_with_run_actions(machine: Path, home: Path) -> None:
    s = _setup(home)
    assert s["gates_active"] is False
    assert "hooks are not installed" in s["gates_summary"]
    assert "Agents could start other agents without Tirith seeing them" in s["gates_summary"]
    assert [a["command"] for a in s["gates_actions"]] == ["tirith hook install --agent anthropic",
                                                          "tirith shim install"]
    assert all(a["cwd"] == str(machine) for a in s["gates_actions"])
    for action in s["gates_actions"]:  # the Run path finds the stored command by id
        assert actions_mod.find_action(home, action["id"])["command"] == action["command"]


def test_missing_shims_only(machine: Path, home: Path, tmp_path: Path) -> None:
    _hooks(tmp_path)
    s = _setup(home)
    assert s["gates_active"] is False
    assert s["gates_summary"] == ("Agents could start other agents without Tirith seeing them, because the "
                                  "agent shims are not installed.")
    assert [a["command"] for a in s["gates_actions"]] == ["tirith shim install"]


def test_everything_installed_is_active_and_matches_shim_verify(machine: Path, home: Path, tmp_path: Path,
                                                                 capsys) -> None:
    _hooks(tmp_path)
    rows = gates.install_shims(load_config(home), home / "shims")
    assert [r[1] for r in rows] == ["installed"], rows
    assert main(["--home", str(home), "shim", "verify"]) == 0
    assert "all intact" in capsys.readouterr().out
    s = _setup(home)
    assert s["gates_active"] is True
    assert s["gates_summary"] == "" and s["gates_actions"] == [] and s["gates_problems"] == []


def test_old_shim_folder_is_not_what_counts(machine: Path, home: Path, tmp_path: Path) -> None:
    """Shims in ~/.local/bin (the old place) do not make the rules active; <home>/shims does."""
    _hooks(tmp_path)
    gates.install_shims(load_config(home), tmp_path / "owner" / ".local" / "bin")
    assert _setup(home)["gates_active"] is False
