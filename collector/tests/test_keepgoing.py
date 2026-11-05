"""Keep going until done: a lane that ends incomplete is resumed with what is missing (tirith.keepgoing)."""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

import pytest

from conftest import make_hooks_mutable, write_config
from test_lane_run import _lane_run_setup
from tirith import asks as asks_mod
from tirith import gates
from tirith import keepgoing as keepgoing_mod
from tirith.cli import main
from tirith.home import load_config
from tirith.store import read_json

FAKE = '''
import json, os, subprocess, sys
here = os.path.dirname(os.path.abspath(__file__))
with open(os.path.join(here, "calls.jsonl"), "a") as f:
    f.write(json.dumps(sys.argv[1:]) + "\\n")
calls = sum(1 for _ in open(os.path.join(here, "calls.jsonl")))
mode = open(os.path.join(here, "mode")).read().strip()
resumed = "--resume" in sys.argv
open("work" + str(calls) + ".txt", "w").write("x")
if mode.startswith("stage") and not resumed:
    import datetime
    events = os.path.join(os.environ["TIRITH_HOME"], "events.jsonl")
    now = lambda: datetime.datetime.now(datetime.timezone.utc).isoformat()
    base = {"run": "r1", "stage": "test", "project": "demo", "lane": "loop", "log": os.path.join(here, "test.log")}
    open(base["log"], "w").write("FAILED tests/test_a.py::test_x - boom\\n")
    with open(events, "a") as f:
        f.write(json.dumps({**base, "at": now(), "event": "start", "status": "running", "pid": os.getppid(),
                            "timeout": 60}) + "\\n")
    ender = ("import json,time,datetime; time.sleep(1.5); open(%r,'a').write(json.dumps({**%r, 'at': "
             "datetime.datetime.now(datetime.timezone.utc).isoformat(), 'event': 'end', 'status': 'failed'}) + chr(10))"
             % (events, base))
    subprocess.Popen([sys.executable, "-c", ender], start_new_session=True)
    print(json.dumps({"type": "system", "session_id": "sess-1"}))
    print(json.dumps({"type": "result", "result": "Waiting for the test stage.", "session_id": "sess-1"}))
    sys.exit(0)
if mode == "slow":  # round 1 fixes one gap (the plan report), round 2 fixes none
    if calls == 2:
        sys.path.insert(0, @ROOT@)
        from tirith.cli import main
        main(["plan", "report", "--status", "done"])
    print(json.dumps({"type": "system", "session_id": "sess-1"}))
    print(json.dumps({"type": "result", "result": "Finished.", "session_id": "sess-1"}))
    sys.exit(0)
if mode == "never" or (mode == "fix-on-resume" and not resumed):
    pass  # leaves the file uncommitted and gives no plan report
else:
    subprocess.run(["git", "add", "-A"], check=True)
    subprocess.run(["git", "-c", "user.email=a@b.c", "-c", "user.name=a", "commit", "-q", "--no-verify",
                    "-m", "work"], check=True)
    sys.path.insert(0, @ROOT@)
    from tirith.cli import main
    main(["plan", "report", "--status", "done"])
print(json.dumps({"type": "system", "session_id": "sess-1"}))
print(json.dumps({"type": "result", "result": "Finished.", "session_id": "sess-1"}))
'''.replace("@ROOT@", repr(str(Path(__file__).resolve().parent.parent)))


def _run(home: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch, mode: str, tests_green: bool = True,
         command: str | None = None):
    repo = _lane_run_setup(home, tmp_path, monkeypatch)
    agent = tmp_path / "agent"
    agent.mkdir()
    (agent / "fake.py").write_text(FAKE, encoding="utf-8")
    (agent / "mode").write_text(mode, encoding="utf-8")
    monkeypatch.setattr(keepgoing_mod.claimcheck_mod, "test_runs",
                        lambda *a: [{"at": 9e12, "counts": [], "run": "r"}] if tests_green else [])
    monkeypatch.setattr(keepgoing_mod, "POLL_S", 0.1)
    monkeypatch.setenv("TIRITH_HOME", str(home))
    as_agent = [] if command == "inferred" else ["--as-agent", "anthropic"]
    exe = agent / "agentcli" if command == "inferred" else None
    if exe:
        with (home / "config.toml").open("a", encoding="utf-8") as cfg:
            cfg.write(f'\n[agents.anthropic]\ncommand = "{exe}"\n')
        exe.write_text(f"#!/bin/sh\nexec {sys.executable} {agent / 'fake.py'} \"$@\"\n", encoding="utf-8")
        exe.chmod(0o755)
    try:
        code = main(["lane", "run", "--project", "demo", "--class", "warden", "--feature", "Loop",
                     "--brief", str(repo / "brief.md"), "--step", "1", "--item", "Do the thing.",
                     "--id", "loop", "--workspace", str(repo), *as_agent, "--",
                     *([str(exe)] if exe else [sys.executable, str(agent / "fake.py")]), "-p", "the brief", "--output-format",
                     "stream-json"])
    finally:
        make_hooks_mutable(repo)
    calls = [json.loads(l) for l in (agent / "calls.jsonl").read_text().splitlines()]
    return code, calls, read_json(home / "lanes" / "loop.json", {})


def test_missing_report_and_uncommitted_work_resume_the_same_session(home, tmp_path, monkeypatch) -> None:
    code, calls, lane = _run(home, tmp_path, monkeypatch, "fix-on-resume")
    assert code == 0 and len(calls) == 2
    first, second = calls
    assert first[:2] == ["-p", "the brief"] and "--resume" not in first
    assert second[0] == "-p" and "Missing:" in second[1] and "no plan report" in second[1]
    assert "uncommitted changes" not in second[1] and "round 1 of 3" in second[1]
    assert second[second.index("--resume") + 1] == "sess-1"
    assert lane["outcome"]["status"] == "done" and lane["result"] == "done"
    assert keepgoing_mod.round_label(lane) == "round 1/3"
    assert not asks_mod.queue_items(home, only_open=True)


def test_brief_and_finish_gate_agree_on_commit(home, tmp_path, monkeypatch) -> None:
    """The brief says "Do NOT commit"; a lane that obeys it is never sent back for it: Tirith commits."""
    from tirith import launch as launch_mod
    assert any("Do NOT commit" in r for r in launch_mod.HARD_RULES)
    code, calls, lane = _run(home, tmp_path, monkeypatch, "fix-on-resume")
    assert all("uncommitted" not in c[1] for c in calls[1:])
    ws = Path(lane["workspace"])
    assert not keepgoing_mod._git(ws, "status", "--porcelain").stdout.strip()
    assert "work1.txt" in keepgoing_mod._git(ws, "show", "--stat", "--format=%s", "HEAD~0").stdout \
        or "work1.txt" in keepgoing_mod._git(ws, "log", "--stat", "--format=%s").stdout
    assert keepgoing_mod.has_work(ws, lane.get("start_commit"))


def test_a_round_that_fixes_nothing_goes_to_the_manager_queue_at_once(home, tmp_path, monkeypatch) -> None:
    code, calls, lane = _run(home, tmp_path, monkeypatch, "never")
    assert len(calls) == 2 and "--resume" in calls[1]
    (item,) = [i for i in asks_mod.queue_items(home, only_open=True) if i["kind"] == keepgoing_mod.QUEUE_KIND]
    assert item["id"] == "sq-incomplete-loop" and "no plan report" in item["text"]
    assert "same gaps after round 1 of 3" in item["text"]
    assert keepgoing_mod.round_label(lane) == "round 1/3, stalled"


def test_a_round_that_fixes_something_earns_another(home, tmp_path, monkeypatch) -> None:
    code, calls, lane = _run(home, tmp_path, monkeypatch, "slow", tests_green=False)
    assert len(calls) == 3 and all("--resume" in c for c in calls[1:])
    assert "no plan report" in calls[1][1] and "no plan report" not in calls[2][1]
    (item,) = [i for i in asks_mod.queue_items(home, only_open=True) if i["kind"] == keepgoing_mod.QUEUE_KIND]
    assert "same gaps after round 2 of 3" in item["text"]
    assert keepgoing_mod.round_label(lane) == "round 2/3, stalled"


def test_stalled_only_when_every_gap_sent_back_is_still_missing() -> None:
    assert not keepgoing_mod.stalled([], ["a"])
    assert keepgoing_mod.stalled(["a", "b"], ["a", "b"])
    assert keepgoing_mod.stalled(["a"], ["a", "b"])
    assert not keepgoing_mod.stalled(["a", "b"], ["a"])
    assert not keepgoing_mod.stalled(["claim check: x"], ["claim check: y"])


def test_three_rounds_then_one_manager_queue_item(home, tmp_path) -> None:
    lane = {"id": "loop", "project": "demo", "feature": "Loop"}
    keepgoing_mod.queue_gaps(home, lane, ["no plan report"], keepgoing_mod.MAX_ROUNDS)
    (item,) = asks_mod.queue_items(home, only_open=True)
    assert "still has gaps after 3 rounds" in item["text"]


def test_keepalive_waits_for_the_running_stage_then_resumes_with_its_result(home, tmp_path, monkeypatch) -> None:
    code, calls, lane = _run(home, tmp_path, monkeypatch, "stage")
    assert code == 0 and len(calls) == 2
    second = calls[1]
    assert second[second.index("--resume") + 1] == "sess-1"
    assert "stage `test`" in second[1] and "failed" in second[1] and "test_x" in second[1] and "test.log" in second[1]
    assert "Missing:" not in second[1]
    assert not lane.get("keep_going")  # a stage wait is not a keep-going round
    assert lane["outcome"]["status"] == "done"


def test_keepalive_infers_the_alias_of_a_known_agent_cli(home, tmp_path, monkeypatch) -> None:
    _, calls, _ = _run(home, tmp_path, monkeypatch, "fix-on-resume", command="inferred")
    assert len(calls) == 2 and "--resume" in calls[1]
    config = load_config(home)
    assert keepgoing_mod.infer_alias(config, [str(tmp_path / "agent" / "agentcli"), "-p"]) == "anthropic"
    assert keepgoing_mod.infer_alias(config, [sys.executable]) is None


def test_keepgoing_resume_not_blocked(home, tmp_path, monkeypatch) -> None:
    """The collector closes a lane whose pid is gone; the follow-up round still starts through the agent shim."""
    from datetime import datetime, timedelta, timezone

    from tirith import collector as collector_mod
    from tirith.store import read_lanes

    monkeypatch.setattr(keepgoing_mod, "POLL_S", 0.1)
    repo = _lane_run_setup(home, tmp_path, monkeypatch)
    agent = tmp_path / "agent"
    agent.mkdir()
    (agent / "fake.py").write_text(FAKE, encoding="utf-8")
    (agent / "mode").write_text("fix-on-resume", encoding="utf-8")
    exe = agent / "agentcli"
    exe.write_text(f"#!/bin/sh\nexec {sys.executable} {agent / 'fake.py'} \"$@\"\n", encoding="utf-8")
    exe.chmod(0o755)
    with (home / "config.toml").open("a", encoding="utf-8") as cfg:
        cfg.write('\n[agents.anthropic]\ncommand = "agentcli"\n')
    monkeypatch.setenv("PATH", f"{agent}{os.pathsep}{os.environ['PATH']}")
    assert [r[1] for r in gates.install_shims(load_config(home), home / "shims")] == ["installed"]
    monkeypatch.setattr(keepgoing_mod.claimcheck_mod, "test_runs", lambda *a: [{"at": 9e12, "counts": [], "run": "r"}])
    real_gaps = keepgoing_mod.gaps

    def gaps_after_collector_tick(home_, lane, repo_, report):
        later = datetime.now(timezone.utc) + timedelta(hours=1)
        collector_mod.close_dead_lanes(home_, read_lanes(home_), later)
        return real_gaps(home_, lane, repo_, report)

    monkeypatch.setattr(keepgoing_mod, "gaps", gaps_after_collector_tick)
    monkeypatch.setenv("TIRITH_HOME", str(home))
    try:
        code = main(["lane", "run", "--project", "demo", "--class", "warden", "--feature", "Loop",
                     "--brief", str(repo / "brief.md"), "--step", "1", "--item", "Do the thing.",
                     "--id", "loop", "--workspace", str(repo), "--as-agent", "anthropic", "--",
                     "agentcli", "-p", "the brief", "--output-format", "stream-json"])
    finally:
        make_hooks_mutable(repo)
    calls = [json.loads(l) for l in (agent / "calls.jsonl").read_text().splitlines()]
    lane = read_json(home / "lanes" / "loop.json", {})
    assert code == 0 and len(calls) == 2 and "--resume" in calls[1]
    assert lane["outcome"]["status"] == "done" and lane["result"] == "done"


def test_keepalive_relaunch_without_session_says_what_was_left(home, tmp_path) -> None:
    write_config(home, [], extra='[agents.plain]\ncommand = "plainagent"\n')
    built = keepgoing_mod.resume_command(load_config(home), "plain", None, ["plainagent", "b"], None,
                                         "Missing: x", "I did y", "BRIEF", "The previous run left a.py")
    argv, stdin = built
    assert "The previous run left a.py" in (stdin or argv[-1])


def test_failing_tests_are_listed(home, tmp_path, monkeypatch) -> None:
    _, calls, _ = _run(home, tmp_path, monkeypatch, "fix-on-resume", tests_green=False)
    assert "no passing test run recorded" in calls[1][1]


def test_alias_without_resume_flag_starts_again_with_the_previous_report(home) -> None:
    write_config(home, [], extra='[agents.plain]\ncommand = "plainagent"\n')
    config = load_config(home)
    built = keepgoing_mod.resume_command(config, "plain", None, ["plainagent", "brief"], None,
                                         "Missing: x", "I did y", "THE BRIEF")
    assert built is not None
    argv, stdin = built
    assert argv[0] == "plainagent"
    text = stdin or argv[-1]
    assert "THE BRIEF" in text and "I did y" in text and "Missing: x" in text


def test_resume_drops_session_id(home) -> None:
    write_config(home, [], extra='[agents.anthropic]\ncommand = "agentcli"\nprint_flag = "-p"\n')
    config = load_config(home)
    command = ["agentcli", "-p", "brief", "--session-id", "abc", "--model", "m"]
    argv, stdin = keepgoing_mod.resume_command(config, "anthropic", None, command, "abc", "More", "r", "b")
    assert "--session-id" not in argv and "abc" in argv
    assert argv[argv.index("--resume") + 1] == "abc" and argv[:2] == ["agentcli", "-p"]
    assert argv[-2:] == ["--model", "m"]


def test_session_id_is_the_last_one_in_the_log(tmp_path) -> None:
    log = tmp_path / "a.log"
    log.write_text('{"session_id": "a"}\nnoise\n{"type":"result","session_id":"b"}\n', encoding="utf-8")
    assert keepgoing_mod.session_id(log) == "b" and keepgoing_mod.session_id(tmp_path / "none") is None


EXISTING_GATE_IDS = ("subagent-tool", "agent-cli", "guard-reinstall", "hooks-path", "no-verify", "git-hooks",
                     "raw-stage", "settings-edit", "roadmap-edit")


def test_existing_gate_ids_are_still_present_and_active(home) -> None:
    source = Path(gates.__file__).read_text(encoding="utf-8")
    for gate_id in EXISTING_GATE_IDS:
        assert f'"{gate_id}"' in source, gate_id
    config = load_config(home)
    got = {v.rule for v in gates.check_call(config, "Bash", {"command": "git commit --no-verify -m x"}, "/tmp")}
    assert "no-verify" in got
