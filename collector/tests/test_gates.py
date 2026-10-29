"""Gates: blocking hooks, shims, lane run refusals and violations in state.json."""

from __future__ import annotations

import io
import json
import os
import pty
import subprocess
import sys
from pathlib import Path

import pytest

from conftest import make_hooks_mutable, make_repo, write_config, write_lane_manifest
from tirith import gates
from tirith.cli import main
from tirith.collector import Collector
from tirith.home import load_config
from tirith.store import read_json, read_jsonl

AGENTS = '[agents.anthropic]\ncommand = "alpha-cli"\n\n[agents.muse]\ncommand = "beta-cli"\n'
MANIFEST = """
[[pipelines]]
id = "ci"
[[pipelines.stages]]
name = "build"
run = "make build"
[[pipelines.stages]]
name = "test"
run = "cd src && uv run -q pytest -q"
"""
SKIP_FLAG = "--no" + "-verify"  # keeps the literal out of shell-level guards that scan commands


@pytest.fixture
def proj(home: Path, tmp_path: Path) -> Path:
    folder = tmp_path / "proj"
    folder.mkdir()
    (folder / "tirith.toml").write_text(MANIFEST, encoding="utf-8")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(folder)}], AGENTS)
    return folder


def _call(monkeypatch, cwd: Path, tool: str, tool_input: dict, agent: str = "anthropic") -> int:
    payload = {"hook_event_name": "PreToolUse", "session_id": "s1", "cwd": str(cwd),
               "tool_name": tool, "tool_input": tool_input}
    monkeypatch.setattr("sys.stdin", io.StringIO(json.dumps(payload)))
    return main(["hook", "--agent", agent])


def _bash(monkeypatch, cwd: Path, command: str) -> int:
    return _call(monkeypatch, cwd, "Bash", {"command": command})


@pytest.mark.parametrize("command,rule", [
    ("alpha-cli -p 'do it'", "agent-cli"),
    ("cd x && /usr/local/bin/alpha-cli --resume", "agent-cli"),
    ("FOO=1 npx -y alpha-cli -p hi", "agent-cli"),
    ("bunx alpha-cli@latest", "agent-cli"),
    ("echo hi | beta-cli", "agent-cli"),
    ("bash -c 'beta-cli -p x'", "agent-cli"),
    (f"git commit {SKIP_FLAG} -m x", "no-verify"),
    (f"git push {SKIP_FLAG}", "no-verify"),
    ("git commit -n -m x", "no-verify"),
    ("git commit -an", "no-verify"),
    ("git -c core.hooksPath=/dev/null commit -m x", "hooks-path"),
    ("git config core.hooksPath /tmp/h", "hooks-path"),
    ("rm .git/hooks/pre-commit", "git-hooks"),
    ("echo x > .git/hooks/pre-commit", "git-hooks"),
    ("chmod -x /r/.git/hooks/commit-msg", "git-hooks"),
    ("make build", "raw-stage"),
    ("cd src && uv run -q pytest -q", "raw-stage"),
    ("uv run -q pytest -q", "raw-stage"),
])
def test_bash_commands_are_blocked(proj, home, monkeypatch, capsys, command, rule) -> None:
    assert _bash(monkeypatch, proj, command) == 2
    err = capsys.readouterr().err
    assert f"tirith: {rule}: " in err and ". Do this instead: " in err
    assert err.count("tirith: ") == 1
    rows = [r for r in read_jsonl(home / "events.jsonl") if r["kind"] == "violation"]
    assert [r["rule"] for r in rows] == [rule] and rows[0]["agent"] == "anthropic"


@pytest.mark.parametrize("command", [
    "ls -la", "git commit -m 'fix n'", "git commit -mfixn", "git status && git diff --stat",
    "echo alpha-cli", "cat notes-about-beta-cli.txt", "uv run which pytest",
    "tirith run --stage test", "tirith run --stage build -- make build",
    "tirith lane run --project demo --class warden --feature x --brief b.md -- alpha-cli -p x",
    "cat .git/hooks/pre-commit", "make buildall", "git config user.name x",
])
def test_unrelated_commands_pass(proj, home, monkeypatch, capsys, command) -> None:
    assert _bash(monkeypatch, proj, command) == 0
    assert capsys.readouterr().err == ""
    assert not [r for r in read_jsonl(home / "events.jsonl") if r["kind"] == "violation"]


def test_message_names_the_exact_replacements(proj, monkeypatch, capsys) -> None:
    _bash(monkeypatch, proj, "cd src && uv run -q pytest -q")
    assert capsys.readouterr().err.strip().endswith("tirith run --project demo --stage test")
    _bash(monkeypatch, proj, "alpha-cli -p x")
    err = capsys.readouterr().err
    assert ('tirith lane run --project demo --class <class> --feature "<what the agent does>" '
            "--brief <brief-file> -- alpha-cli …") in err


def test_subagent_tool_blocked_with_inferred_feature(proj, monkeypatch, capsys) -> None:
    for tool in ("Task", "Agent"):
        code = _call(monkeypatch, proj, tool, {"description": "Audit the schema", "prompt": "x"})
        assert code == 2
        err = capsys.readouterr().err
        assert "tirith: subagent-tool: " in err and '--feature "Audit the schema"' in err
    assert _call(monkeypatch, proj, "Read", {"file_path": "a.py"}) == 0


@pytest.fixture
def settings_file(home: Path, proj: Path, tmp_path: Path) -> Path:
    path = tmp_path / "agent" / "settings.json"
    path.parent.mkdir()
    path.write_text("{}", encoding="utf-8")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(proj)}],
                 AGENTS.replace('command = "alpha-cli"', f'command = "alpha-cli"\nsettings = "{path}"'))
    return path


@pytest.mark.parametrize("template", [
    "rm {p}", "echo '{{}}' > {p}", "sed -i '' s/a/b/ {p}", "mv {p} {p}.off", "tee {p}",
    "cd /tmp && rm ../{rel}", "tirith hook uninstall --agent anthropic",
    "uv run tirith hook uninstall --agent muse --settings /x",
])
def test_agent_hooks_cannot_be_removed(proj, home, settings_file, monkeypatch, capsys, template) -> None:
    command = template.format(p=settings_file, rel=settings_file.relative_to(settings_file.parents[1]))
    if command.startswith("cd /tmp"):
        command = f"rm {settings_file}"
    assert _bash(monkeypatch, proj, command) == 2
    assert "tirith: settings-edit: " in capsys.readouterr().err
    assert [r["rule"] for r in read_jsonl(home / "events.jsonl") if r["kind"] == "violation"] == ["settings-edit"]


def test_agent_settings_are_protected_from_edit_tools_but_readable(
        proj, settings_file, monkeypatch, capsys) -> None:
    for tool in ("Edit", "Write", "MultiEdit"):
        assert _call(monkeypatch, proj, tool, {"file_path": str(settings_file)}) == 2
        assert "tirith: settings-edit: " in capsys.readouterr().err
    assert _call(monkeypatch, proj, "Read", {"file_path": str(settings_file)}) == 0
    assert _bash(monkeypatch, proj, f"cat {settings_file}") == 0
    assert _bash(monkeypatch, proj, "tirith hook install --agent anthropic") == 0


def test_edit_tools_cannot_touch_git_hooks(proj, monkeypatch, capsys) -> None:
    assert _call(monkeypatch, proj, "Write", {"file_path": str(proj / ".git/hooks/pre-commit")}) == 2
    assert "tirith: git-hooks: " in capsys.readouterr().err
    assert _call(monkeypatch, proj, "Edit", {"file_path": str(proj / "a.py")}) == 0


def test_only_pre_tool_use_blocks(proj, monkeypatch) -> None:
    payload = {"hook_event_name": "PostToolUse", "cwd": str(proj), "tool_name": "Bash",
               "tool_input": {"command": "alpha-cli"}}
    monkeypatch.setattr("sys.stdin", io.StringIO(json.dumps(payload)))
    assert main(["hook", "--agent", "anthropic"]) == 0


def test_muse_blocks_with_the_same_exit_code(proj, monkeypatch, capsys) -> None:
    payload = {"event": "PreToolUse", "tool": "Bash", "input": {"command": "beta-cli"},
               "cwd": str(proj)}
    monkeypatch.setattr("sys.stdin", io.StringIO(json.dumps(payload)))
    assert main(["hook", "--agent", "muse"]) == 2
    assert "tirith: agent-cli: " in capsys.readouterr().err


def test_env_cannot_switch_gates_off_but_config_can(proj, home, monkeypatch) -> None:
    monkeypatch.setenv("TIRITH_GATES", "off")
    assert _bash(monkeypatch, proj, "alpha-cli") == 2
    config = home / "config.toml"
    config.write_text(config.read_text(encoding="utf-8") + "\n[gates]\nenabled = false\n",
                      encoding="utf-8")
    assert _bash(monkeypatch, proj, "alpha-cli") == 0


def test_violations_reach_state_and_waiting(proj, home, monkeypatch) -> None:
    monkeypatch.setenv("TIRITH_LANE", "w1")
    _bash(monkeypatch, proj, f"git commit {SKIP_FLAG} -m x")
    old = {"kind": "violation", "ts": "2020-01-01T00:00:00Z", "rule": "agent-cli", "agent": "muse",
           "lane": None, "project": None, "message": "tirith: agent-cli: old"}
    with (home / "events.jsonl").open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(old) + "\n")
    state = Collector(home, client=None).cycle()
    assert sorted(v["rule"] for v in state["violations"]) == ["agent-cli", "no-verify"]
    mine = next(v for v in state["violations"] if v["rule"] == "no-verify")
    assert mine["lane"] == "w1" and mine["agent"] == "anthropic"
    assert mine["message"].startswith("tirith: no-verify")
    # violations are not the owner's turn (the agent was already told what to do instead)
    assert not [w for w in state["waiting_on_you"] if w["source"] == "violation"]


def test_violations_capped_at_fifty(proj, home) -> None:
    with (home / "events.jsonl").open("a", encoding="utf-8") as handle:
        for n in range(60):
            handle.write(json.dumps({"kind": "violation", "ts": f"2020-01-01T00:00:{n:02d}Z",
                                     "rule": "r", "message": f"m{n}"}) + "\n")
    state = Collector(home, client=None).cycle()
    assert len(state["violations"]) == 50 and state["violations"][-1]["message"] == "m59"


# -- shims ---------------------------------------------------------------------------------------


@pytest.fixture
def fake_bin(tmp_path: Path, monkeypatch) -> Path:
    folder = tmp_path / "realbin"
    folder.mkdir()
    for name in ("alpha-cli", "beta-cli"):
        exe = folder / name
        exe.write_text(f'#!/bin/sh\necho real {name} "$@"\n', encoding="utf-8")
        exe.chmod(0o755)
    monkeypatch.setenv("PATH", f"{folder}{os.pathsep}{os.environ['PATH']}")
    return folder


def test_shim_install_verify_uninstall(proj, home, fake_bin, tmp_path, capsys, monkeypatch) -> None:
    shims = tmp_path / "shims"
    assert main(["shim", "verify", "--dir", str(shims)]) == 1
    assert "no shim at" in capsys.readouterr().out
    assert main(["shim", "install", "--dir", str(shims)]) == 0
    assert (shims / "alpha-cli").read_text(encoding="utf-8").splitlines()[1].startswith(gates.SHIM_MARKER)
    assert os.access(shims / "beta-cli", os.X_OK)
    assert main(["shim", "install", "--dir", str(shims)]) == 0  # idempotent
    assert main(["shim", "verify", "--dir", str(shims)]) == 1  # shim dir is not in PATH yet
    assert "is not first" in capsys.readouterr().out
    monkeypatch.setenv("PATH", f"{shims}{os.pathsep}{os.environ['PATH']}")
    assert main(["shim", "verify", "--dir", str(shims)]) == 0
    assert main(["shim", "uninstall", "--dir", str(shims)]) == 0
    assert not list(shims.iterdir())


def test_shim_never_overwrites_a_foreign_file(proj, fake_bin, tmp_path, capsys) -> None:
    shims = tmp_path / "shims"
    shims.mkdir()
    (shims / "alpha-cli").write_text("#!/bin/sh\necho mine\n", encoding="utf-8")
    assert main(["shim", "install", "--dir", str(shims)]) == 1
    assert "is not a Tirith shim" in capsys.readouterr().out
    assert (shims / "alpha-cli").read_text(encoding="utf-8").endswith("echo mine\n")
    assert main(["shim", "uninstall", "--dir", str(shims)]) == 1
    assert (shims / "alpha-cli").exists() and not (shims / "beta-cli").exists()


def _run_shim(shims: Path, env: dict, tty: bool = False) -> subprocess.CompletedProcess:
    env = {**os.environ, **env}
    cmd = [str(shims / "alpha-cli"), "-p", "hi"]
    if not tty:
        return subprocess.run(cmd, env=env, text=True, stdin=subprocess.DEVNULL,
                              capture_output=True)
    master, slave = pty.openpty()
    try:
        proc = subprocess.run(cmd, env=env, stdin=slave, stdout=slave,
                              stderr=subprocess.PIPE, text=True)
        out = os.read(master, 4096).decode()
    finally:
        os.close(master)
        os.close(slave)
    return subprocess.CompletedProcess(proc.args, proc.returncode, out, proc.stderr)


def test_shim_runs_interactively_or_with_the_lane_token(proj, home, fake_bin, tmp_path) -> None:
    shims = tmp_path / "shims"
    main(["shim", "install", "--dir", str(shims)])
    base = {"TIRITH_HOME": str(home), "PATH": f"{shims}{os.pathsep}{os.environ['PATH']}"}
    blocked = _run_shim(shims, base)
    assert blocked.returncode == 2 and blocked.stdout == ""
    assert "tirith: agent-cli: " in blocked.stderr and "tirith lane run --project" in blocked.stderr
    (home / "lanes").mkdir(exist_ok=True)
    (home / "lanes" / "l1.json").write_text(json.dumps({"id": "l1", "token": "tok-123"}),
                                            encoding="utf-8")
    ok = _run_shim(shims, {**base, "TIRITH_LANE": "l1", "TIRITH_LANE_TOKEN": "tok-123"})
    assert ok.returncode == 0 and ok.stdout.strip() == "real alpha-cli -p hi"
    for env in ({"TIRITH_LANE": "l1", "TIRITH_LANE_TOKEN": "wrong"},
                {"TIRITH_LANE": "l1", "TIRITH_LANE_TOKEN": ""},
                {"TIRITH_LANE": "nope", "TIRITH_LANE_TOKEN": "tok-123"},
                {"TIRITH_LANE_TOKEN": "tok-123"}):
        assert _run_shim(shims, {**base, **env}).returncode == 2
    interactive = _run_shim(shims, base, tty=True)
    assert interactive.returncode == 0 and "real alpha-cli -p hi" in interactive.stdout


# -- lane run ------------------------------------------------------------------------------------


def _lane_repo(home: Path, tmp_path: Path, monkeypatch) -> Path:
    repo = make_repo(tmp_path / "repo")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}], AGENTS)
    monkeypatch.setattr("tirith.cli.verify_guard", lambda *a, **k: [])
    write_lane_manifest(repo)
    return repo


def _lane(brief, *tail) -> list[str]:
    args = ["lane", "run", "--project", "demo", "--class", "warden", "--feature", "F", "--id", "g1"]
    return args + (["--brief", str(brief)] if brief else []) + ["--", *tail]


def test_lane_run_refuses_without_a_brief(home, tmp_path, monkeypatch, capsys) -> None:
    repo = _lane_repo(home, tmp_path, monkeypatch)
    try:
        for brief in (None, repo / "missing.md"):
            assert main(_lane(brief, sys.executable, "-c", "print(1)")) == 3
            err = capsys.readouterr().err
            assert "tirith: lane-brief: " in err and "--brief <file>" in err
        (repo / "empty.md").write_text("  \n", encoding="utf-8")
        assert main(_lane(repo / "empty.md", sys.executable, "-c", "print(1)")) == 3
        assert not (home / "lanes" / "g1.json").exists()
    finally:
        make_hooks_mutable(repo)


def test_lane_run_refuses_when_tirith_check_fails(home, tmp_path, monkeypatch, capsys) -> None:
    repo = _lane_repo(home, tmp_path, monkeypatch)
    (repo / "tirith.toml").unlink()
    try:
        assert main(_lane(repo / "brief.md", sys.executable, "-c", "print(1)")) == 3
        err = capsys.readouterr().err
        assert "tirith: project-check: " in err and "manifest-missing" in err
        assert "tirith check --project demo" in err
        assert not (home / "lanes" / "g1.json").exists()
    finally:
        make_hooks_mutable(repo)


def test_lane_run_gives_the_child_a_token_and_clears_it(home, tmp_path, monkeypatch) -> None:
    repo = _lane_repo(home, tmp_path, monkeypatch)
    child = ("import json,os,sys;"
             "lane=json.load(open(os.path.join(os.environ['TIRITH_HOME'],'lanes','g1.json')));"
             "sys.exit(0 if lane['token']==os.environ['TIRITH_LANE_TOKEN'] and lane['token'] else 9)")
    try:
        assert main(_lane(repo / "brief.md", sys.executable, "-c", child)) == 0
    finally:
        make_hooks_mutable(repo)
    assert "token" not in read_json(home / "lanes" / "g1.json", {})


def test_lane_env_points_the_shims_at_the_given_home(tmp_path, monkeypatch) -> None:
    monkeypatch.setenv("TIRITH_HOME", str(tmp_path / "other"))
    from tirith.cli import _wrapper_path_env
    env = _wrapper_path_env(tmp_path / "given", "l1", "tok")
    assert env["TIRITH_HOME"] == str(tmp_path / "given")
    assert env["TIRITH_LANE"] == "l1" and env["TIRITH_LANE_TOKEN"] == "tok"


def test_config_gates_flag_defaults_on(home) -> None:
    assert load_config(home).gates_enabled is True


# -- Windows .cmd shims ---------------------------------------------------------------------------


def test_cmd_shim_text_and_target(tmp_path) -> None:
    text = gates.cmd_shim_script("alpha", "alpha-cli", r"C:\bin\alpha-cli.exe")
    lines = text.split("\r\n")
    assert lines[0] == "@echo off" and lines[1].startswith(gates.CMD_SHIM_MARKER)
    assert 'set "REAL=C:\\bin\\alpha-cli.exe"' in lines
    assert "findstr" in text and "exit /b 2" in text and "agent-cli" in text
    assert gates.shim_target(tmp_path, "alpha-cli.exe", windows=True) == tmp_path / "alpha-cli.cmd"
    assert gates.shim_target(tmp_path, "alpha-cli", windows=False) == tmp_path / "alpha-cli"
    target = tmp_path / "alpha-cli.cmd"
    target.write_bytes(text.encode())
    assert gates.is_shim(target)
