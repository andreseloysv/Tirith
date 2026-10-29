"""Agent presets, usage parsing, lane diffs and ``tirith doctor --agent``."""

from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

from conftest import GIT, make_hooks_mutable, make_repo, write_config, write_lane_manifest
from tirith import agentrun
from tirith.cli import end_lane, main, register_lane
from tirith.collector import Collector
from tirith.home import Config

DATA = Path(__file__).parent / "data"


def _config(**agents) -> Config:
    return Config(agents=agents)


def _exe(path: Path) -> Path:
    path.write_text("#!/bin/sh\n", encoding="utf-8")
    path.chmod(0o755)
    return path


# -- presets -----------------------------------------------------------------------------------


def test_anthropic_preset_defaults() -> None:
    cmd = agentrun.build_command(_config(anthropic={"command": "agent-x"}), "anthropic", "p", "do it")
    assert cmd[:3] == ["agent-x", "-p", "do it"]
    assert cmd[3:8] == ["--output-format", "stream-json", "--verbose",
                        "--permission-mode", "acceptEdits"]
    allowed = cmd[cmd.index("--allowedTools") + 1]
    assert "Edit" in allowed.split(",") and "Bash(git status:*)" in allowed


def test_config_and_project_override_win() -> None:
    cfg = _config(anthropic={
        "command": "agent-x", "model": "m-global", "permissions": ["Read"],
        "projects": {"p": {"model": "m-proj", "permissions": ["Read", "Write"], "add_dirs": ["/x/y"]}}})
    cmd = agentrun.build_command(cfg, "anthropic", "p", "go")
    assert cmd[cmd.index("--model") + 1] == "m-proj"
    assert cmd[cmd.index("--allowedTools") + 1] == "Read,Write"
    assert cmd[cmd.index("--add-dir") + 1] == "/x/y"
    other = agentrun.build_command(cfg, "anthropic", "q", "go", model="m-cli", effort="high")
    assert other[other.index("--model") + 1] == "m-cli"
    assert other[other.index("--effort") + 1] == "high"
    assert other[other.index("--allowedTools") + 1] == "Read"


def test_codex_and_copilot_presets() -> None:
    cfg = _config(codex={"command": "agent-c"}, copilot={"command": "agent-p", "model": "m1"})
    assert agentrun.build_command(cfg, "codex", None, "hi") == [
        "agent-c", "exec", "--json", "--sandbox", "workspace-write", "hi"]
    cmd = agentrun.build_command(cfg, "copilot", None, "hi")
    assert cmd[:4] == ["agent-p", "--output-format", "json", "--no-ask-user"]
    assert cmd[cmd.index("--model") + 1] == "m1"
    assert "shell(git status)" in cmd and cmd[-2:] == ["-p", "hi"]
    with pytest.raises(SystemExit):
        agentrun.build_command(cfg, "codex", None, "hi", effort="high")
    cmd = agentrun.build_command(cfg, "copilot", None, "hi", effort="high")
    assert cmd[cmd.index("--reasoning-effort") + 1] == "high"


VARIADIC = {"--allowedTools", "--allow-tool", "--add-dir"}


@pytest.mark.parametrize("alias", ["anthropic", "muse", "codex", "copilot"])
def test_prompt_is_never_swallowed_by_a_variadic_option(alias: str) -> None:
    cfg = _config(**{alias: {"command": "agent-x", "model": "m1", "add_dirs": ["/x/y"]}})
    cmd = agentrun_mod_build(cfg, alias, "PROMPT-TEXT")
    at = cmd.index("PROMPT-TEXT")
    assert cmd.count("PROMPT-TEXT") == 1
    assert cmd[at - 1] not in VARIADIC  # not the value slot of a list option
    after = cmd[at + 1:]
    assert not any(c in VARIADIC for c in cmd[at - 1:at])
    if alias == "anthropic":
        assert cmd[at - 1] == "-p"  # right behind the print flag
        assert after[0].startswith("--")
    if alias == "copilot":
        assert cmd[at - 1] == "-p" and after == []
    if alias in ("muse", "codex"):
        assert after == []
    if alias != "anthropic":  # list options sit before the prompt flag, never after the prompt
        flag = at - 1 if cmd[at - 1] == "-p" else at
        assert all(cmd.index(v) < flag for v in VARIADIC if v in cmd)


def agentrun_mod_build(cfg, alias, prompt, **kw):
    return agentrun.build_command(cfg, alias, "p", prompt, **kw)


def test_dash_prompt_and_stdin_for_long_prompts() -> None:
    cfg = _config(anthropic={"command": "agent-x"}, muse={"command": "agent-m"})
    long = "# Lane\n" + "x" * agentrun.STDIN_MIN
    assert agentrun.wants_stdin(cfg, "anthropic", "p", long)
    assert agentrun.wants_stdin(cfg, "anthropic", "p", "-oops")
    assert not agentrun.wants_stdin(cfg, "anthropic", "p", "short")
    assert not agentrun.wants_stdin(cfg, "muse", "p", long)  # that alias has no stdin mode
    cmd = agentrun.build_command(cfg, "anthropic", "p", long, via_stdin=True)
    assert cmd[:2] == ["agent-x", "-p"] and long not in cmd and cmd[2] == "--output-format"


def test_codex_turn_completed_usage(tmp_path: Path) -> None:
    log = tmp_path / "l.jsonl"
    log.write_text(
        '{"type":"thread.started","thread_id":"t"}\n'
        '{"type":"turn.completed","usage":{"input_tokens":100,"cached_input_tokens":40,"output_tokens":7}}\n'
        '{"type":"turn.completed","usage":{"input_tokens":50,"cached_input_tokens":0,"output_tokens":3}}\n',
        encoding="utf-8")
    usage = agentrun.parse_usage(log)
    assert usage["input_tokens"] == 150 and usage["output_tokens"] == 10
    assert usage["cache_tokens"] == 40 and usage["turns"] == 2 and usage["cost_usd"] is None


def test_muse_preset_and_missing_support() -> None:
    cfg = _config(muse={"command": "agent-m"})
    assert agentrun.build_command(cfg, "muse", None, "hi") == ["agent-m", "exec", "--json", "hi"]
    with pytest.raises(SystemExit):
        agentrun.build_command(cfg, "muse", None, "hi", effort="high")


def test_unknown_alias_and_missing_command() -> None:
    with pytest.raises(SystemExit):
        agentrun.build_command(_config(), "nope", None, "x")
    with pytest.raises(SystemExit):
        agentrun.build_command(_config(anthropic={}), "anthropic", None, "x")


def test_lane_run_with_agent(home: Path, tmp_path: Path, capsys,
                             monkeypatch: pytest.MonkeyPatch) -> None:
    repo = make_repo(tmp_path / "repo")
    stub = tmp_path / "stub.sh"
    stub.write_text('#!/bin/sh\necho "{\\"type\\":\\"result\\",\\"result\\":\\"args: $*\\"}"\n',
                    encoding="utf-8")
    stub.chmod(0o755)
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}],
                 f'[agents.anthropic]\ncommand = "{stub}"\nargs = ["-p"]\n'
                 'permissions = ["Read"]\nmodel = "m1"\n')
    monkeypatch.delenv("TIRITH_LANE", raising=False)
    monkeypatch.setattr("tirith.cli.verify_guard", lambda *a, **k: [])
    write_lane_manifest(repo)
    try:
        code = main(["lane", "run", "--project", "demo", "--class", "warden", "--feature", "f",
                     "--id", "l1", "--brief", str(repo / "brief.md"),
                     "--agent", "anthropic", "--prompt", "hello"])
    finally:
        make_hooks_mutable(repo)
    assert code == 0
    assert "args: -p hello --model m1 --allowedTools Read" in capsys.readouterr().out


# -- usage -------------------------------------------------------------------------------------


def test_usage_from_result_record() -> None:
    usage = agentrun.parse_usage(DATA / "usage-result.jsonl")
    assert usage == {"input_tokens": 100, "output_tokens": 2000, "cache_tokens": 4300,
                     "cost_usd": 0.5, "model": "alpha-1", "turns": 12, "duration_s": 93}


def test_usage_from_run_usage_records() -> None:
    usage = agentrun.parse_usage(DATA / "usage-run.jsonl")
    assert usage["input_tokens"] == 1500 and usage["output_tokens"] == 300
    assert usage["cache_tokens"] == 50 and usage["cost_usd"] is None
    assert usage["model"] == "beta-2" and usage["turns"] == 2 and usage["duration_s"] == 4


def test_usage_missing_or_empty(tmp_path: Path) -> None:
    assert agentrun.parse_usage(None) is None
    assert agentrun.parse_usage(tmp_path / "none.log") is None
    plain = tmp_path / "plain.log"
    plain.write_text("just text\n", encoding="utf-8")
    assert agentrun.parse_usage(plain) is None


def test_sum_usage() -> None:
    a = agentrun.parse_usage(DATA / "usage-result.jsonl")
    b = agentrun.parse_usage(DATA / "usage-run.jsonl")
    total = agentrun.sum_usage([a, b, None])
    assert total["input_tokens"] == 1600 and total["cost_usd"] == 0.5 and total["model"] == "mixed"
    assert agentrun.sum_usage([None]) is None


def test_state_has_usage_per_agent_session_and_project(home: Path, tmp_path: Path) -> None:
    log = tmp_path / "a.log"
    log.write_text((DATA / "usage-result.jsonl").read_text(encoding="utf-8"), encoding="utf-8")
    register_lane(home, lane_id="l1", project="demo", hero_class="warden", feature="f",
                  workspace=None, log=str(log), pid=None, brief=None)
    register_lane(home, lane_id="l2", project="demo", hero_class="warden", feature="g",
                  workspace=None, log=None, pid=None, brief=None)
    state = Collector(home, client=None).cycle()
    agents = {a["id"]: a for a in state["agents"]}
    assert agents["l1"]["usage"]["cost_usd"] == 0.5 and agents["l2"]["usage"] is None
    assert agents["l1"]["diff"] is None
    proj = next(p for p in state["projects"] if p["id"] == "demo")
    assert proj["usage"]["output_tokens"] == 2000
    assert all("usage" in s for s in state["sessions"])


# -- diffs -------------------------------------------------------------------------------------


def test_diff_stored_at_lane_end(home: Path, tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    lane = register_lane(home, lane_id="d1", project="demo", hero_class="warden", feature="f",
                         workspace=str(repo), log=None, pid=None, brief=None)
    assert lane["start_commit"]
    (repo / "README.md").write_text("# demo repo\nmore\n", encoding="utf-8")
    (repo / "new.txt").write_text("hello\nworld\n", encoding="utf-8")
    (repo / "blob.bin").write_bytes(b"\0\1\2\3")
    done = end_lane(home, "d1", "done")
    patch_file = Path(done["diff"]["path"])
    assert patch_file == home / "diffs" / "d1.patch"
    text = patch_file.read_text(encoding="utf-8")
    assert "+more" in text and "+++ b/new.txt" in text and "+hello" in text
    assert "Binary file blob.bin not shown" in text and "\0" not in text
    assert done["diff"]["size"] == len(text.encode())
    assert "TIRITH_DIFFS" in (home / "diffs" / "d1.js").read_text(encoding="utf-8")
    state = Collector(home, client=None).cycle()
    agent = next(a for a in state["agents"] if a["id"] == "d1")
    assert agent["diff"] == {"path": str(patch_file), "size": done["diff"]["size"],
                             "script": "diffs/d1.js"}


def test_diff_is_bounded(tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    start = subprocess.run([GIT, "-C", str(repo), "rev-parse", "HEAD"], capture_output=True,
                           text=True).stdout.strip()
    for n in range(8):
        (repo / f"big{n}.txt").write_text("x" * 200_000 + "\n", encoding="utf-8")
    patch = agentrun.build_patch(str(repo), start)
    assert len(patch.encode()) <= agentrun.DIFF_MAX + 100


def test_no_diff_without_start_commit(home: Path, tmp_path: Path) -> None:
    lane = {"id": "x", "workspace": str(tmp_path), "start_commit": None}
    assert agentrun.store_diff(home, lane) is None


# -- doctor ------------------------------------------------------------------------------------


def test_doctor_reports_each_check(tmp_path: Path) -> None:
    wrapper = tmp_path / "bin"
    wrapper.mkdir()
    exe = _exe(tmp_path / "agent-x")
    cfg = _config(anthropic={"command": str(exe)})
    path = f"{wrapper}:{tmp_path}:/usr/bin:/bin"
    checks = {n: (ok, d) for n, ok, d in agentrun.doctor_agent(cfg, "anthropic", path, wrapper)}
    assert checks["executable"][0] and checks["path-order"][0]
    assert not checks["hooks-skip"][0] and "no git wrapper" in checks["hooks-skip"][1]
    bad = {n: ok for n, ok, _ in agentrun.doctor_agent(_config(), "anthropic", "/usr/bin", wrapper)}
    assert not bad["executable"] and not bad["path-order"]


def test_doctor_wrapper_refusing_skipped_hooks(tmp_path: Path) -> None:
    wrapper = tmp_path / "bin"
    wrapper.mkdir()
    flag = "--no-" + "verify"
    (wrapper / "git").write_text(
        f'#!/bin/sh\ncase " $* " in *" {flag} "*) echo refused >&2; exit 1;; esac\n'
        f'exec {GIT} "$@"\n', encoding="utf-8")
    (wrapper / "git").chmod(0o755)
    cfg = _config(anthropic={"command": str(_exe(tmp_path / "agent-x"))})
    path = f"{wrapper}:{tmp_path}:/usr/bin:/bin"
    checks = {n: ok for n, ok, _ in agentrun.doctor_agent(cfg, "anthropic", path, wrapper)}
    assert checks == {"executable": True, "path-order": True, "hooks-skip": True}


def test_doctor_wrapper_letting_it_through(tmp_path: Path) -> None:
    wrapper = tmp_path / "bin"
    wrapper.mkdir()
    (wrapper / "git").write_text(f'#!/bin/sh\nexec {GIT} "$@"\n', encoding="utf-8")
    (wrapper / "git").chmod(0o755)
    cfg = _config(anthropic={"command": str(_exe(tmp_path / "agent-x"))})
    path = f"{wrapper}:{tmp_path}:/usr/bin:/bin"
    checks = {n: ok for n, ok, _ in agentrun.doctor_agent(cfg, "anthropic", path, wrapper)}
    assert checks["hooks-skip"] is False


def test_doctor_cli(home: Path, capsys) -> None:
    code = main(["--home", str(home), "doctor", "--agent", "anthropic"])
    assert code == 1 and "FAIL executable" in capsys.readouterr().out
