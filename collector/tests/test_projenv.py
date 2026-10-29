"""Environment variables per project: storage, CLI, tirith:// link, and use by stage runs, lanes and actions."""

from __future__ import annotations

import json
import os
import stat
import sys
from pathlib import Path

import pytest

from conftest import make_hooks_mutable, make_repo, write_config, write_lane_manifest
from tirith import actions as actions_mod
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith import projenv, runstage
from tirith.cli import main
from tirith.store import read_json


@pytest.fixture
def demo(home: Path, tmp_path: Path) -> Path:
    repo = make_repo(tmp_path / "repo")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo), "color": "#123456"}])
    return repo


# -- storage ----------------------------------------------------------------------------------


def test_save_and_load_round_trip_with_awkward_values(home: Path) -> None:
    env = {"A": "1", "EMPTY": "", "Q": 'say "hi"\nnext\\line', "U": "ñandú ✓", "_x9": "a=b"}
    projenv.save(home, "demo", env)
    assert projenv.load(home, "demo") == env


def test_file_is_private(home: Path) -> None:
    projenv.save(home, "demo", {"A": "1"})
    path = home / "env" / "demo.toml"
    assert stat.S_IMODE(path.stat().st_mode) == 0o600
    assert path.read_text() == 'A = "1"\n'


def test_set_unset_and_empty_removes_the_file(home: Path) -> None:
    projenv.set_vars(home, "demo", {"A": "1", "B": "2"})
    projenv.set_vars(home, "demo", {"B": "3"})
    assert projenv.load(home, "demo") == {"A": "1", "B": "3"}
    projenv.unset_vars(home, "demo", ["A", "NOPE"])
    assert projenv.load(home, "demo") == {"B": "3"}
    projenv.unset_vars(home, "demo", ["B"])
    assert not (home / "env" / "demo.toml").exists()


@pytest.mark.parametrize("key", ["", "1A", "A-B", "A B", "A=B", "é"])
def test_bad_keys_are_refused(home: Path, key: str) -> None:
    with pytest.raises(projenv.EnvError):
        projenv.save(home, "demo", {key: "x"})
    assert not (home / "env").exists()


@pytest.mark.parametrize("project", ["", "../x", "a/b", ".hidden"])
def test_bad_project_ids_are_refused(home: Path, project: str) -> None:
    with pytest.raises(projenv.EnvError):
        projenv.save(home, project, {"A": "1"})


def test_a_broken_file_gives_nothing(home: Path) -> None:
    (home / "env").mkdir()
    (home / "env" / "demo.toml").write_text("not [ toml", encoding="utf-8")
    assert projenv.load(home, "demo") == {}


def test_apply_lays_the_project_over_the_inherited_env(home: Path) -> None:
    projenv.save(home, "demo", {"A": "project"})
    out = projenv.apply(home, "demo", {"A": "inherited", "B": "kept"})
    assert out == {"A": "project", "B": "kept"}
    assert projenv.apply(home, "other", {"A": "inherited"}) == {"A": "inherited"}


# -- CLI --------------------------------------------------------------------------------------


def test_cli_set_list_unset(home: Path, demo: Path, capsys) -> None:
    assert main(["project", "env", "set", "--project", "demo", "API_URL=http://x?a=b", "DEBUG=1"]) == 0
    capsys.readouterr()
    assert main(["project", "env", "list", "--project", "demo", "--json"]) == 0
    assert json.loads(capsys.readouterr().out) == {"API_URL": "http://x?a=b", "DEBUG": "1"}
    assert main(["project", "env", "list", "--project", "demo"]) == 0
    assert capsys.readouterr().out.splitlines() == ["API_URL=http://x?a=b", "DEBUG=1"]
    assert main(["project", "env", "unset", "--project", "demo", "DEBUG"]) == 0
    assert projenv.load(home, "demo") == {"API_URL": "http://x?a=b"}


def test_cli_replace_from_stdin(home: Path, demo: Path, monkeypatch, capsys) -> None:
    projenv.save(home, "demo", {"OLD": "1"})
    monkeypatch.setattr(sys, "stdin", __import__("io").StringIO('{"NEW": "2"}'))
    assert main(["project", "env", "set", "--project", "demo", "--replace-stdin"]) == 0
    assert projenv.load(home, "demo") == {"NEW": "2"}


def test_cli_refuses_bad_keys_and_unknown_projects(home: Path, demo: Path, capsys) -> None:
    assert main(["project", "env", "set", "--project", "demo", "1BAD=x"]) == 3
    assert "bad variable name" in capsys.readouterr().err
    assert projenv.load(home, "demo") == {}
    with pytest.raises(SystemExit):
        main(["project", "env", "set", "--project", "ghost", "A=1"])
    with pytest.raises(SystemExit):
        main(["project", "env", "set", "--project", "demo", "NOEQUALS"])


def test_link_needs_the_token_and_replaces_the_set(home: Path, demo: Path) -> None:
    projenv.save(home, "demo", {"OLD": "1"})
    url = "tirith://env-set?project=demo&env=%7B%22NEW%22%3A%22a%20b%22%7D&token="
    assert main(["url", "open", url + "wrong"]) == 3
    assert projenv.load(home, "demo") == {"OLD": "1"}
    assert main(["url", "open", url + launch_mod.launch_token(home)]) == 0
    assert projenv.load(home, "demo") == {"NEW": "a b"}
    bad = "tirith://env-set?project=demo&env=%7B%221X%22%3A%221%22%7D&token=" + launch_mod.launch_token(home)
    assert main(["url", "open", bad]) == 3


def test_handler_log_hides_the_values() -> None:
    assert "secret" not in launch_mod.redact_url("tirith://env-set?project=d&env=secret&token=t")


# -- use --------------------------------------------------------------------------------------


def test_a_stage_run_gets_the_variables(home: Path, tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("INHERITED", "yes")
    projenv.save(home, "demo", {"GREETING": "hola"})
    code, output = runstage.run_stage(home, "demo", "ci", "test", 'echo "$GREETING $INHERITED"', tmp_path, echo=False)
    assert code == 0 and "hola yes" in output
    code, output = runstage.run_stage(home, "other", "ci", "test", 'echo "[$GREETING]"', tmp_path, echo=False)
    assert "[]" in output


def test_tirith_run_gets_the_variables(home: Path, demo: Path, monkeypatch) -> None:
    (demo / "tirith.toml").write_text(
        '[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "test"\nkind = "test"\n'
        'run = "echo $GREETING > got.txt"\n', encoding="utf-8")
    projenv.save(home, "demo", {"GREETING": "hola"})
    monkeypatch.chdir(demo)
    monkeypatch.delenv("TIRITH_LANE", raising=False)
    assert main(["run", "--project", "demo", "--stage", "test"]) == 0
    assert (demo / "got.txt").read_text().strip() == "hola"


def test_a_lane_gets_the_variables(home: Path, demo: Path, capsys, monkeypatch) -> None:
    monkeypatch.delenv("TIRITH_LANE", raising=False)
    monkeypatch.setattr("tirith.cli.verify_guard", lambda *a, **k: [])
    write_lane_manifest(demo)
    projenv.save(home, "demo", {"LANE_VAR": "from-settings"})
    child = ("import json,os;"
             "print(json.dumps({'type':'result','result':'got '+os.environ['LANE_VAR']}))")
    try:
        code = main(["lane", "run", "--project", "demo", "--class", "warden", "--feature", "Env",
                     "--brief", str(demo / "brief.md"), "--id", "envlane", "--", sys.executable, "-c", child])
    finally:
        make_hooks_mutable(demo)
    assert code == 0
    assert read_json(home / "lanes" / "envlane.json", {})["report"] == "got from-settings"


def test_a_run_action_gets_the_variables(home: Path, demo: Path, tmp_path: Path) -> None:
    rid = inbox_mod.add_request(home, "ask", "demo", "1", "what now?")["id"]
    assert main(["inbox", "answer", rid, "Do it.", "--run", "echo $ACT_VAR", "--cwd", str(tmp_path)]) == 0
    aid = actions_mod.list_actions(home)[-1]["id"]
    projenv.save(home, "demo", {"ACT_VAR": "action-value"})
    assert main(["action", "run", aid]) == 0
    assert "action-value" in (home / "actions" / (aid + ".log")).read_text()


# -- state ------------------------------------------------------------------------------------


def test_the_state_shows_each_projects_variables_in_full(home: Path, demo: Path) -> None:
    projenv.save(home, "demo", {"TOKEN": "plain-visible-value"})
    assert main(["collect", "--once"]) == 0
    state = read_json(home / "state.json", {})
    rows = {p["id"]: p for p in state["projects"]}
    assert rows["demo"]["env"] == {"TOKEN": "plain-visible-value"}
