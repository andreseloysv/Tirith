"""The absolute ``tirith`` command: resolver, run-time rewrite of stored actions, the link handler script."""

from __future__ import annotations

import shlex
import sys
from pathlib import Path

from conftest import write_config
from tirith import actions as actions_mod
from tirith import launch as launch_mod
from tirith import tirithcmd
from tirith.cli import main
from tirith.store import append_jsonl

ROOT = Path(__file__).resolve().parents[2]


def test_resolver_prefers_config_then_repo_bin_then_uv(tmp_path: Path) -> None:
    assert tirithcmd.tirith_command("/opt/my tirith") == "'/opt/my tirith'"
    assert tirithcmd.tirith_command() == shlex.quote(str(ROOT / "bin" / "tirith"))
    bare = tmp_path / "repo"
    (bare / "collector").mkdir(parents=True)
    assert tirithcmd.tirith_command(repo=bare) == f"uv run --project {bare / 'collector'} tirith"


def test_resolver_reads_app_tirith_from_the_config(home: Path) -> None:
    assert tirithcmd.resolved(home) == shlex.quote(str(ROOT / "bin" / "tirith"))
    write_config(home, [], '[app]\ntirith = "/x/tirith"\n')
    assert tirithcmd.resolved(home) == "/x/tirith"


def test_rewrite_only_touches_a_bare_command_word() -> None:
    cmd = "/r/bin/tirith"
    assert tirithcmd.rewrite("tirith shim install", cmd=cmd) == "/r/bin/tirith shim install"
    assert tirithcmd.rewrite("cd x && tirith url install; tirith check", cmd=cmd) == \
        "cd x && /r/bin/tirith url install; /r/bin/tirith check"
    assert tirithcmd.rewrite("echo tirith\ntirith a", cmd=cmd) == "echo tirith\n/r/bin/tirith a"
    for same in ("echo tirith", "/r/bin/tirith x", "tirithx y", "git commit -m 'tirith: x'"):
        assert tirithcmd.rewrite(same, cmd=cmd) == same


def test_a_stored_bare_action_runs_with_the_resolved_command(home: Path, tmp_path: Path) -> None:
    script = tmp_path / "faketirith"
    script.write_text('#!/bin/sh\necho "fake:$*"\n')
    script.chmod(0o755)
    write_config(home, [], f'[app]\ntirith = "{script}"\n')
    action = actions_mod.make_actions(["tirith shim install"])[0]
    append_jsonl(home / "inbox.jsonl", {"kind": "answer", "id": "q1", "actions": [action]})
    result = actions_mod.run_action(home, action["id"], tmp_path)
    assert result["status"] == "passed"
    assert "fake:shim install" in actions_mod.log_path(home, action["id"]).read_text()


def test_generated_actions_store_the_resolved_command(home: Path) -> None:
    action = actions_mod.register_generated(home, "demo", "g1", "Guard", "tirith guard install /r", None)
    assert action["command"] == tirithcmd.resolved(home) + " guard install /r"


def test_handler_script_calls_the_absolute_command_and_logs(home: Path) -> None:
    script = launch_mod.applescript(str(home))
    assert str(ROOT / "bin" / "tirith") in script
    assert "url open" in script and str(home / "logs" / "url-handler.log") in script
    assert "$HOME/.local/bin" in script and "/dev/null" not in script


def test_verify_checks_that_the_handler_command_exists(tmp_path: Path) -> None:
    assert launch_mod.command_problems("") != []
    assert launch_mod.command_problems(shlex.quote(str(tmp_path / "gone"))) != []
    assert launch_mod.command_problems(shlex.quote(sys.executable) + " -m tirith") == []
    assert launch_mod.command_problems(shlex.quote(str(ROOT / "bin" / "tirith"))) == []


def test_url_open_logs_each_link_and_the_result(home: Path) -> None:
    url = launch_mod.build_url("demo", "2", "wrong-token")
    assert main(["url", "open", url]) == 3
    log = (home / "logs" / "url-handler.log").read_text(encoding="utf-8")
    assert "received" in log and "token=***" in log and "wrong-token" not in log
    assert "result exit 3" in log
