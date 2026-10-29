"""The Setup card tells the truth when the agents' real binaries share a PATH folder with the git wrapper,
and temporary folders never become projects."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from conftest import write_config
from tirith import gates, projects
from tirith import setup as setup_mod
from tirith.cli import main
from tirith.collector import Collector
from tirith.hooks import INSTALL_EVENTS, hook_command
from tirith.home import load_config

WEB = Path(__file__).resolve().parent.parent.parent / "web"


def _hooks(path: Path) -> None:
    data = {"hooks": {e: [{"hooks": [{"type": "command", "command": hook_command("anthropic", e)}]}]
                      for e in INSTALL_EVENTS}}
    path.write_text(json.dumps(data), encoding="utf-8")


def test_real_binary_in_local_bin_and_shims_in_home_is_active(home: Path, tmp_path: Path, monkeypatch,
                                                              capsys) -> None:
    owner = tmp_path / "owner"
    local_bin = owner / ".local" / "bin"
    local_bin.mkdir(parents=True)
    exe = local_bin / "alpha-cli"
    exe.write_text("#!/bin/sh\n", encoding="utf-8")
    exe.chmod(0o755)
    monkeypatch.setenv("HOME", str(owner))
    monkeypatch.setenv("PATH", f"{local_bin}:/usr/bin:/bin")
    write_config(home, [], '[agents.anthropic]\ncommand = "alpha-cli"\nsettings = "%s"\n' % (tmp_path / "s.json"))
    _hooks(tmp_path / "s.json")
    config = load_config(home)
    assert [r[1] for r in gates.install_shims(config, home / "shims")] == ["installed"]
    machine = setup_mod.machine_state(config, home=home)
    assert machine.active and machine.problems == [] and machine.summary == ""
    assert main(["--home", str(home), "shim", "verify"]) == 0
    assert "all intact" in capsys.readouterr().out
    folder = tmp_path / "proj"
    folder.mkdir()
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(folder)}],
                 '[agents.anthropic]\ncommand = "alpha-cli"\nsettings = "%s"\n' % (tmp_path / "s.json"))
    setup = next(p for p in Collector(home).build_state()["projects"] if p["id"] == "demo")["setup"]
    assert setup["gates_active"] is True and setup["gates_problems"] == []


def test_temp_folders_are_never_registered(home: Path, tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setattr(projects, "ALLOW_TEMP", False)
    repo = tmp_path / "repo"
    repo.mkdir()
    (repo / ".git").mkdir()
    write_config(home, [])
    for folder in (repo, tmp_path, Path("/tmp"), Path("/private/var/folders")):
        entries = projects.discover(home, load_config(home), [folder])
        assert entries == []
    assert projects.load_auto(home)["projects"] == []
    assert projects.is_temp_folder(repo) and projects.is_temp_folder("/private/var/folders/xx/T/repo")
    assert not projects.is_temp_folder(Path.home() / "DEVELOP" / "Tirith")


def test_stored_temp_projects_are_not_listed(home: Path, tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setattr(projects, "ALLOW_TEMP", False)
    projects.save_auto(home, {"projects": [{"id": "repo", "name": "repo", "folder": str(tmp_path / "repo")},
                                           {"id": "real", "name": "real", "folder": "/Users/x/DEVELOP/real"}],
                              "ignored": []})
    assert [p["id"] for p in projects.load_auto(home)["projects"]] == ["real"]


def test_web_has_no_add_stage_or_rule_buttons() -> None:
    text = (WEB / "js" / "v-setup.js").read_text(encoding="utf-8")
    for gone in ("+ Stage", "+ Rule", "sopen", "ssend", "add-stage", "add-rule"):
        assert gone not in text, gone


def test_overview_scopes_to_one_project() -> None:
    text = (WEB / "js" / "ui.js").read_text(encoding="utf-8")
    assert "T.scopeProject(s, T.effProject(s))" in text
