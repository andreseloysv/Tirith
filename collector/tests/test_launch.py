"""tirith launch: brief, refusals, url parsing, the token check and the handler bundle."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

from conftest import make_hooks_mutable, make_repo, write_config, write_lane_manifest
from tirith import launch as launch_mod
from tirith.cli import launch_step, main, register_lane
from tirith.store import read_jsonl

PLAN = """# TODO

Goal: see everything the agents do. Second line of the intro.

Another paragraph that is not the intro.

## 1. Done step

- [x] Finished.

## 2. Web panel

- [x] Already built.
- [ ] Add the button.
- [ ] Write the docs.

## 3. Empty step
"""


@pytest.fixture
def project(home: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    repo = make_repo(tmp_path / "demo")
    write_lane_manifest(repo)
    (repo / "docs").mkdir()
    (repo / "docs" / "TODO.md").write_text(PLAN, encoding="utf-8")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}],
                 extra='[launch]\ncommand = "agentcmd --brief {brief} --cwd {workspace}"\n')
    assert main(["guard", "install", "demo"]) == 0
    started: list[list[str]] = []
    monkeypatch.setattr(launch_mod, "spawn_detached", lambda argv, home, log: started.append(argv) or 4242)
    yield repo, started
    make_hooks_mutable(repo)
    make_hooks_mutable(tmp_path / "demo-lanes")


def test_intro_is_first_paragraph() -> None:
    from tirith import roadmap
    assert roadmap.intro(PLAN) == "Goal: see everything the agents do. Second line of the intro."


def test_brief_has_title_why_items_and_rules() -> None:
    brief = launch_mod.build_brief("Web panel", "The intro.", ["Add the button.", "Write the docs."], "2")
    assert brief.startswith("# Lane: Web panel\n")
    assert "## Why" in brief and "The intro." in brief and "Web panel" in brief
    assert "## What to build\n- Add the button.\n- Write the docs." in brief
    for rule in ("Do NOT commit", "alias", "No servers", "tests must pass", "what changed"):
        assert rule in brief
    assert "docs/TODO.md" in brief and "docs/CONTRACT" in brief


def test_class_by_step_title() -> None:
    assert launch_mod.pick_class("Plan visible en el Overview") == "glasswright"
    assert launch_mod.pick_class("Reglas de código") == "warden"
    assert launch_mod.pick_class("Something else") == "stonewright"


def test_dry_run_starts_nothing(project, home: Path, capsys) -> None:
    repo, started = project
    assert main(["launch", "--project", "demo", "--step", "2", "--dry-run"]) == 0
    out = capsys.readouterr().out
    assert "# Lane: Web panel" in out and "Add the button." in out and "Already built." not in out
    assert "agentcmd --brief" in out and "demo-lanes" in out
    assert started == [] and not (home / "briefs").exists() and not (repo.parent / "demo-lanes").exists()


def test_launch_creates_worktree_brief_and_runs_lane(project, home: Path, capsys) -> None:
    repo, started = project
    assert main(["launch", "--project", "demo", "--step", "2"]) == 0
    lane_id = capsys.readouterr().out.strip().removeprefix("lane ")
    worktree = repo.parent / "demo-lanes" / lane_id
    assert (worktree / "README.md").is_file()
    branches = launch_mod.subprocess.run(["git", "-C", str(repo), "branch", "--list", f"lane/{lane_id}"],
                                         capture_output=True, text=True).stdout
    assert f"lane/{lane_id}" in branches
    brief = (home / "briefs" / f"{lane_id}.md").read_text(encoding="utf-8")
    assert brief.startswith("# Lane: Web panel")
    argv = started[0]
    assert argv[:3] == [sys.executable, "-m", "tirith"] and argv[3:5] == ["lane", "run"]
    assert argv[argv.index("--step") + 1] == "2"
    assert argv[argv.index("--class") + 1] == "glasswright"
    assert argv[argv.index("--workspace") + 1] == str(worktree)
    assert argv[argv.index("--feature") + 1] == "Web panel"
    tail = argv[argv.index("--") + 1:]
    assert tail == ["agentcmd", "--brief", str(home / "briefs" / f"{lane_id}.md"), "--cwd", str(worktree)]


def test_launch_passes_request_to_the_lane(project, home: Path, capsys) -> None:
    _, started = project
    assert main(["launch", "--project", "demo", "--step", "2", "--request", "in-abc12345"]) == 0
    assert started[0][started[0].index("--request") + 1] == "in-abc12345"


def test_refuses_done_empty_and_unknown_steps(project, capsys) -> None:
    for step, text in (("1", "already done"), ("3", "no items"), ("9", "not in")):
        assert main(["launch", "--project", "demo", "--step", step]) == 3
        assert text in capsys.readouterr().err


def test_refuses_when_a_working_lane_has_the_step(project, home: Path, capsys) -> None:
    import os
    register_lane(home, lane_id="busy", project="demo", hero_class="warden", feature="x",
                  workspace=None, log=None, pid=os.getpid(), brief=None, step="2")
    assert main(["launch", "--project", "demo", "--step", "2"]) == 3
    assert "busy" in capsys.readouterr().err


def test_refuses_when_check_fails(project, capsys) -> None:
    repo, _ = project
    (repo / "tirith.toml").unlink()
    assert main(["launch", "--project", "demo", "--step", "2"]) == 3
    assert "tirith check fails" in capsys.readouterr().err


def test_refuses_without_agent_command(project, home: Path, capsys) -> None:
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(project[0])}])
    assert main(["launch", "--project", "demo", "--step", "2", "--dry-run"]) == 3
    assert "[launch] command" in capsys.readouterr().err


def test_agent_alias_uses_its_launch_template(project, home: Path, capsys) -> None:
    repo, _ = project
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}],
                 extra='[agents.muse]\ncommand = "musecli"\nlaunch = "musecli run {brief}"\n')
    launch_step(home, "demo", "2", agent="muse", dry_run=True)
    assert "musecli run" in capsys.readouterr().out
    with pytest.raises(launch_mod.LaunchError):
        launch_step(home, "demo", "2", agent="ghost", dry_run=True)


def test_parse_url() -> None:
    link = launch_mod.parse_url("tirith://launch?project=demo&step=2&token=abc%2Fd")
    assert link == {"action": "launch", "project": "demo", "step": "2", "token": "abc/d", "agent": None}
    assert launch_mod.parse_url("TIRITH://launch?project=a&step=1&agent=muse")["agent"] == "muse"
    with pytest.raises(launch_mod.LaunchError):
        launch_mod.parse_url("https://example.com/launch?project=a")


def test_url_build_roundtrip() -> None:
    link = launch_mod.parse_url(launch_mod.build_url("my proj", "3", "t0k&en"))
    assert (link["project"], link["step"], link["token"]) == ("my proj", "3", "t0k&en")


def test_token_is_created_once_and_private(home: Path) -> None:
    token = launch_mod.launch_token(home)
    assert token and launch_mod.launch_token(home) == token
    assert json.loads((home / "launch.json").read_text())["launch_token"] == token
    assert launch_mod.token_ok(home, token)
    assert not launch_mod.token_ok(home, "") and not launch_mod.token_ok(home, token + "x")


def test_url_open_with_right_token_launches(project, home: Path, capsys) -> None:
    _, started = project
    token = launch_mod.launch_token(home)
    url = launch_mod.build_url("demo", "2", token)
    assert main(["url", "open", url]) == 0
    assert len(started) == 1 and "lane " in capsys.readouterr().out


@pytest.mark.parametrize("token", ["wrong-token", ""])
def test_url_open_with_bad_token_is_refused_and_recorded(project, home: Path, capsys, token: str) -> None:
    _, started = project
    launch_mod.launch_token(home)
    assert main(["url", "open", launch_mod.build_url("demo", "2", token)]) == 3
    assert "token" in capsys.readouterr().err and started == []
    rows = [r for r in read_jsonl(home / "events.jsonl") if r.get("kind") == "violation"]
    assert rows and rows[-1]["rule"] == "launch-token" and rows[-1]["project"] == "demo"


def test_url_open_without_a_token_file_is_refused(project, home: Path, capsys) -> None:
    _, started = project
    assert main(["url", "open", "tirith://launch?project=demo&step=2"]) == 3
    assert started == []


def test_state_has_launch_token_and_availability(home: Path) -> None:
    main(["collect", "--once"])
    state = json.loads((home / "state.json").read_text(encoding="utf-8"))
    assert state["launch"]["token"] == launch_mod.launch_token(home, create=False)
    assert isinstance(state["launch"]["available"], bool)


def test_handler_bundle_in_a_temp_dir(tmp_path: Path) -> None:
    apps = tmp_path / "Applications"
    assert not launch_mod.handler_installed(apps)
    assert launch_mod.verify_handler(apps)
    script = launch_mod.applescript("/some/home")
    assert "on open location theURL" in script and "url open" in script and "TIRITH_HOME=/some/home" in script
    if sys.platform != "darwin":
        pytest.skip("osacompile is macOS only")
    launch_mod.install_handler(apps, register=False)
    assert launch_mod.handler_installed(apps) and launch_mod.verify_handler(apps) == []
    assert launch_mod.uninstall_handler(apps, register=False)
    assert not launch_mod.handler_installed(apps)


def test_url_commands_use_apps_dir(tmp_path: Path, capsys) -> None:
    apps = tmp_path / "apps"
    assert main(["url", "verify", "--apps-dir", str(apps)]) == 1
    assert main(["url", "uninstall", "--apps-dir", str(apps)]) == 0
    assert "was not installed" in capsys.readouterr().out
