"""[models]: resolution, command building, config.toml edits, the manager's settings, state."""

from __future__ import annotations

import json
import tomllib
from pathlib import Path

import pytest

from tirith import agentrun, inbox as inbox_mod, models
from tirith.cli import main
from tirith.home import Config, load_config

CONFIG = """# my tirith
[[projects]]
id = "demo"
name = "Demo"

[models]
choices = ["m-big", "m-small"]   # offered in the page

[models.manager]
agent = "anthropic"
model = "m-big"
effort = "high"

[models.workers]
agent = "anthropic"
model = "m-small"
effort = "medium"

[models.workers.projects.demo]
model = "m-mid"

[models.workers.steps."demo/3"]
effort = "low"
model = "m-tiny"

[agents.anthropic]
command = "agent-x"
"""


@pytest.fixture
def cfg_home(home: Path) -> Path:
    (home / "config.toml").write_text(CONFIG, encoding="utf-8")
    return home


# -- resolution --------------------------------------------------------------------------------


def test_workers_defaults_then_project_then_step(cfg_home: Path) -> None:
    config = load_config(cfg_home)
    assert models.resolve_workers(config).as_dict() == {"agent": "anthropic", "model": "m-small", "effort": "medium"}
    assert models.resolve_workers(config, "other", "1").model == "m-small"
    assert models.resolve_workers(config, "demo", "1").as_dict() == {
        "agent": "anthropic", "model": "m-mid", "effort": "medium"}
    assert models.resolve_workers(config, "demo", "3").as_dict() == {
        "agent": "anthropic", "model": "m-tiny", "effort": "low"}
    assert models.resolve_manager(config).model == "m-big"
    assert models.parse(config.models)["choices"] == ["m-big", "m-small"]


def test_nothing_configured_is_empty() -> None:
    assert models.resolve_workers(Config(), "p", "1") == models.Choice()
    assert models.parse(None)["projects"] == {}


# -- command building --------------------------------------------------------------------------


def test_preset_takes_model_and_effort_from_workers(cfg_home: Path) -> None:
    config = load_config(cfg_home)
    cmd = agentrun.build_command(config, "anthropic", "demo", "go", step="3")
    assert cmd[cmd.index("--model") + 1] == "m-tiny" and cmd[cmd.index("--effort") + 1] == "low"
    cmd = agentrun.build_command(config, "anthropic", "demo", "go")
    assert cmd[cmd.index("--model") + 1] == "m-mid"
    explicit = agentrun.build_command(config, "anthropic", "demo", "go", model="m-cli", effort="max")
    assert explicit[explicit.index("--model") + 1] == "m-cli" and explicit[explicit.index("--effort") + 1] == "max"


def test_other_agents_ignore_the_choice_and_effort_without_flag_is_recorded_only(cfg_home: Path) -> None:
    config = load_config(cfg_home)
    config.agents["muse"] = {"command": "agent-m"}
    assert agentrun.build_command(config, "muse", "demo", "hi") == ["agent-m", "exec", "--json", "hi"]
    config.models["workers"]["agent"] = "muse"
    cmd = agentrun.build_command(config, "muse", "demo", "hi")
    assert cmd == ["agent-m", "exec", "--json", "--model", "m-mid", "--reasoning-effort", "medium", "hi"]
    config.models["workers"]["agent"] = "other"
    config.agents["other"] = {"command": "o", "model_flag": "--model"}
    assert "--effort" not in agentrun.build_command(config, "other", "demo", "hi")


def test_launch_command_gets_the_configured_model(cfg_home: Path) -> None:
    from tirith import launch
    config = load_config(cfg_home)
    config.launch_command = "agentcmd --model old --effort high --brief {brief} --cwd {workspace}"
    brief, ws = Path("/b.md"), Path("/w")
    cmd = launch.agent_command(config, None, brief, ws, "demo", "3")
    assert cmd == ["agentcmd", "--model", "m-tiny", "--effort", "low", "--brief", "/b.md", "--cwd", "/w"]
    config.launch_command = ["agentcmd", "--model=old", "--brief", "{brief}"]
    assert launch.agent_command(config, None, brief, ws, "other", "1") == [
        "agentcmd", "--model=m-small", "--brief", "/b.md"]
    config.launch_command = "agentcmd {brief}"  # no model flag in it: nothing is added
    assert launch.agent_command(config, None, brief, ws, "demo", "3") == ["agentcmd", "/b.md"]
    config.models = {}
    config.launch_command = "agentcmd --model old {brief}"
    assert launch.agent_command(config, None, brief, ws, "demo", "3") == ["agentcmd", "--model", "old", "/b.md"]


def test_command_choice_and_ran_with() -> None:
    assert models.command_choice(["x", "--model", "m1", "--effort=high", "p"]) == {"model": "m1", "effort": "high"}
    assert models.command_choice(["x", "-p", "text"]) is None
    assert models.ran_with({"ran": {"model": "m1", "effort": "low"}}, {"model": "real"}) == {
        "model": "real", "effort": "low"}
    assert models.ran_with({"ran": {"model": "m1", "effort": None}}, None) == {"model": "m1", "effort": None}
    assert models.ran_with({}, None) is None


# -- config.toml edits -------------------------------------------------------------------------


def test_set_keeps_comments_and_other_tables(cfg_home: Path) -> None:
    models.set_models(cfg_home, "workers", model="m-new", effort="high")
    models.set_models(cfg_home, "workers", model="m-p", project="demo")
    text = (cfg_home / "config.toml").read_text(encoding="utf-8")
    assert text.startswith("# my tirith\n") and "# offered in the page" in text
    data = tomllib.loads(text)
    assert data["models"]["workers"]["model"] == "m-new" and data["models"]["workers"]["effort"] == "high"
    assert data["models"]["workers"]["projects"]["demo"]["model"] == "m-p"
    assert data["models"]["manager"]["model"] == "m-big" and data["agents"]["anthropic"]["command"] == "agent-x"
    assert text.count("[models.workers]") == 1


def test_set_creates_tables_and_quoted_step(home: Path) -> None:
    models.set_models(home, "manager", agent="anthropic", model="m1", effort="high")
    models.set_models(home, "workers", model="m2", project="my.proj", step="2")
    data = tomllib.loads((home / "config.toml").read_text(encoding="utf-8"))
    assert data["models"]["manager"] == {"agent": "anthropic", "model": "m1", "effort": "high"}
    assert data["models"]["workers"]["steps"]["my.proj/2"] == {"model": "m2"}


def test_set_clear_and_errors(cfg_home: Path) -> None:
    models.set_models(cfg_home, "workers", project="demo", step="3", clear=True)
    data = tomllib.loads((cfg_home / "config.toml").read_text(encoding="utf-8"))
    assert "steps" not in data["models"]["workers"] and data["models"]["workers"]["projects"]["demo"]
    for kwargs in ({"role": "manager", "model": "x", "project": "demo"},
                   {"role": "workers", "model": "x", "step": "1"},
                   {"role": "workers", "model": 'a"b'},
                   {"role": "workers"},
                   {"role": "manager", "clear": True},
                   {"role": "nobody", "model": "x"}):
        with pytest.raises(models.ModelsError):
            models.set_models(cfg_home, **kwargs)
    tomllib.loads((cfg_home / "config.toml").read_text(encoding="utf-8"))


def test_cli_show_and_set(cfg_home: Path, capsys: pytest.CaptureFixture) -> None:
    assert main(["models", "set", "workers", "--model", "m-z", "--effort", "max"]) == 0
    assert main(["models", "set", "manager", "--model", "m-y", "--project", "demo"]) == 3
    capsys.readouterr()
    assert main(["models", "show"]) == 0
    out = capsys.readouterr().out
    assert "workers: agent=anthropic  model=m-z  effort=max" in out
    assert "workers project demo: model=m-mid" in out and "workers step demo/3:" in out
    assert "choices: m-big, m-small" in out


# -- the manager's settings --------------------------------------------------------------------


def test_apply_manager_writes_model_and_keeps_other_keys(cfg_home: Path, tmp_path: Path,
                                                        capsys: pytest.CaptureFixture) -> None:
    settings = tmp_path / "agent" / "settings.json"
    settings.parent.mkdir()
    settings.write_text(json.dumps({"model": "old", "hooks": {"Stop": []}, "theme": "dark"}), encoding="utf-8")
    with (cfg_home / "config.toml").open("a", encoding="utf-8") as handle:
        handle.write(f'\n[agents.anthropic]\nsettings = "{settings}"\n') if False else None
    text = (cfg_home / "config.toml").read_text(encoding="utf-8").replace(
        '[agents.anthropic]\ncommand = "agent-x"\n', f'[agents.anthropic]\ncommand = "agent-x"\nsettings = "{settings}"\n')
    (cfg_home / "config.toml").write_text(text, encoding="utf-8")
    config = load_config(cfg_home)
    assert models.state(config)["manager"]["in_sync"] is False
    assert main(["models", "apply-manager"]) == 0
    out = capsys.readouterr().out
    assert "m-big" in out and "effort high recorded in config.toml only" in out
    data = json.loads(settings.read_text(encoding="utf-8"))
    assert data == {"model": "m-big", "hooks": {"Stop": []}, "theme": "dark"}
    assert models.state(load_config(cfg_home))["manager"]["in_sync"] is True


def test_apply_manager_creates_file_and_refuses_without_setting(cfg_home: Path, tmp_path: Path) -> None:
    settings = tmp_path / "new" / "settings.json"
    config = load_config(cfg_home)
    config.agents["anthropic"]["settings"] = str(settings)
    models.apply_manager(config)
    assert json.loads(settings.read_text(encoding="utf-8")) == {"model": "m-big"}
    broken = tmp_path / "broken.json"
    broken.write_text("{nope", encoding="utf-8")
    config.agents["anthropic"]["settings"] = str(broken)
    with pytest.raises(models.ModelsError):
        models.apply_manager(config)
    assert broken.read_text(encoding="utf-8") == "{nope"
    with pytest.raises(models.ModelsError):
        models.apply_manager(Config())
    config.agents.pop("anthropic")
    with pytest.raises(models.ModelsError):
        models.apply_manager(config)


# -- the page's requests and state -------------------------------------------------------------


def test_setup_request_applies_models_set_and_apply_manager(cfg_home: Path, tmp_path: Path,
                                                            capsys: pytest.CaptureFixture) -> None:
    settings = tmp_path / "s.json"
    text = (cfg_home / "config.toml").read_text(encoding="utf-8").replace(
        'command = "agent-x"\n', f'command = "agent-x"\nsettings = "{settings}"\n')
    (cfg_home / "config.toml").write_text(text, encoding="utf-8")
    row = inbox_mod.add_request(cfg_home, "setup", "settings", "setup", json.dumps(
        {"action": "models-set", "role": "workers", "model": "m-q", "project": "demo"}))
    assert inbox_mod.setup_summary(inbox_mod.find_request(cfg_home, row["id"])["setup"]) == \
        "models workers demo agent=None".replace(" agent=None", "") + " model=m-q"
    assert main(["setup", "apply", row["id"]]) == 0
    assert tomllib.loads((cfg_home / "config.toml").read_text(encoding="utf-8"))[
        "models"]["workers"]["projects"]["demo"]["model"] == "m-q"
    assert inbox_mod.find_request(cfg_home, row["id"])["status"] == "done"
    assert not settings.exists()  # nothing touched the manager's file
    row = inbox_mod.add_request(cfg_home, "setup", "settings", "setup", json.dumps({"action": "apply-manager"}))
    assert main(["setup", "apply", row["id"]]) == 0
    assert json.loads(settings.read_text(encoding="utf-8"))["model"] == "m-big"
    bad = inbox_mod.add_request(cfg_home, "setup", "settings", "setup", json.dumps(
        {"action": "models-set", "role": "workers"}))
    assert main(["setup", "apply", bad["id"]]) == 3


def test_state_models_block(cfg_home: Path) -> None:
    st = models.state(load_config(cfg_home), ["models workers model=m"])
    assert st["manager"]["model"] == "m-big" and st["manager"]["effort_key"] is False
    assert st["workers"] == {"agent": "anthropic", "model": "m-small", "effort": "medium", "fallback": []}
    assert st["projects"]["demo"] == {"model": "m-mid"} and "demo/3" in st["steps"]
    assert st["choices"] == ["m-big", "m-small"] and "high" in st["efforts"]
    assert "anthropic" in st["agents"] and st["pending"] == ["models workers model=m"]
