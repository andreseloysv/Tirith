"""Agent runs and stages are stopped for inactivity, not for lasting long (tirith.timeouts idle limit)."""

from __future__ import annotations

import json
import subprocess
import sys
import time
from pathlib import Path

import pytest

from conftest import make_hooks_mutable
from test_lane_run import _lane_run_setup
from tirith import keepgoing as keepgoing_mod
from tirith import runstage as runstage_mod
from tirith import timeouts as timeouts_mod
from tirith.cli import main
from tirith.home import Config
from tirith.store import read_json, read_jsonl

CHATTY = "import time\nfor i in range(12):\n    print(i, flush=True)\n    time.sleep(0.25)\n"


def _spawn(code: str, cwd: Path) -> subprocess.Popen:
    return subprocess.Popen([sys.executable, "-c", code], cwd=str(cwd), stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL, start_new_session=True)


def test_steady_output_never_times_out_though_it_outlasts_the_idle_limit(home: Path, tmp_path: Path) -> None:
    code, output = None, ""
    beat = {"at": 0.0}

    def probe() -> float:  # the fake process "writes" every 0.25 s: its log is always fresh
        return time.time()

    proc = _spawn(CHATTY, tmp_path)
    watch = timeouts_mod.supervise(proc, "agent-turn", home=home, timeout=60, idle=1.0, probe=probe)
    code = proc.wait()
    ended = watch()
    assert code == 0 and not ended["timed_out"] and ended["seconds"] > 2.5  # ran 3x longer than the idle limit
    assert not read_json(home / "timeouts.json", {}).get("timeouts")


def test_touch_counts_as_progress(home: Path, tmp_path: Path) -> None:
    proc = _spawn("import time; time.sleep(3)", tmp_path)
    watch = timeouts_mod.supervise(proc, "agent-turn", home=home, timeout=60, idle=1.0)
    stop = time.monotonic() + 3.2
    while proc.poll() is None and time.monotonic() < stop:
        watch.touch()
        time.sleep(0.1)
    proc.wait()
    assert not watch()["timed_out"]


def test_silence_times_out_and_says_so(home: Path, tmp_path: Path) -> None:
    proc = _spawn("import time; time.sleep(60)", tmp_path)
    began = time.monotonic()
    watch = timeouts_mod.supervise(proc, "agent-turn", home=home, timeout=600, idle=1.0, what="lane x")
    proc.wait()
    ended = watch()
    assert time.monotonic() - began < 15
    assert ended["timed_out"] and ended["idle"]
    assert "no progress for 1 s (class agent-turn; lane x" in ended["message"] and "--idle" in ended["message"]
    (row,) = read_json(home / "timeouts.json", {})["timeouts"]["agent-turn"]
    assert row["seconds"] == 1


def test_the_hard_cap_still_stops_a_run_that_keeps_making_progress(home: Path, tmp_path: Path) -> None:
    proc = _spawn("import time; time.sleep(60)", tmp_path)
    watch = timeouts_mod.supervise(proc, "agent-turn", home=home, timeout=1, idle=30, probe=time.time)
    proc.wait()
    ended = watch()
    assert ended["timed_out"] and not ended["idle"] and "timed out after 1 s" in ended["message"]


def test_a_changing_worktree_is_progress(tmp_path: Path) -> None:
    probe = timeouts_mod.Probe(workspace=tmp_path)
    first = probe()
    time.sleep(0.05)
    (tmp_path / "a.txt").write_text("x")
    assert probe() > first
    (tmp_path / ".git").mkdir()
    before = probe()
    time.sleep(0.05)
    (tmp_path / ".git" / "noise").write_text("x")
    assert probe() == before  # .git is not a sign of work


def test_child_cpu_is_progress(tmp_path: Path) -> None:
    proc = _spawn("while True: pass", tmp_path)
    try:
        probe = timeouts_mod.Probe(pgid=proc.pid)
        probe()
        seen, stop = None, time.monotonic() + 20
        while seen is None and time.monotonic() < stop:  # a loaded machine gives the loop less CPU
            time.sleep(0.5)
            seen = probe()
        assert seen is not None
    finally:
        proc.kill()
        proc.wait()


def test_a_silent_stage_is_stopped_but_a_chatty_one_is_not(home: Path, tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setitem(timeouts_mod.IDLE_POLICY, "stage", (1, 1, 3600))
    code, output = runstage_mod.run_stage(home, "demo", "ci", "mute", [sys.executable, "-c", "import time; time.sleep(60)"],
                                          tmp_path, echo=False)
    assert code == timeouts_mod.EXIT_TIMED_OUT and "no progress for 1 s (class stage:mute" in output
    end = [e for e in read_jsonl(home / "events.jsonl") if e["event"] == "end"][-1]
    assert end["status"] == "failed" and end["timed_out"] is True
    code, output = runstage_mod.run_stage(home, "demo", "ci", "chatty", [sys.executable, "-c", CHATTY],
                                          tmp_path, echo=False)
    assert code == 0 and "11" in output


def test_idle_limits_have_a_policy_a_setter_and_a_row_in_settings(home: Path) -> None:
    assert timeouts_mod.effective_idle(home, Config(), "agent-turn") == 1200
    assert timeouts_mod.effective_idle(home, Config(), "git") is None
    timeouts_mod.set_timeout(home, Config(), "agent-turn", 1800, "slow model", idle=True)
    assert timeouts_mod.describe_idle(home, Config(), "agent-turn")["source"] == "owner"
    assert timeouts_mod.effective(home, Config(), "agent-turn") == 28800  # the cap is its own value
    with pytest.raises(timeouts_mod.TimeoutRefused):
        timeouts_mod.set_timeout(home, Config(), "agent-turn", 10, "too small", idle=True)
    with pytest.raises(timeouts_mod.TimeoutRefused):
        timeouts_mod.set_timeout(home, Config(), "git", 60, "no idle for git", idle=True)
    rows = {r["class"]: r for r in timeouts_mod.table(home, Config())}
    assert rows["agent-turn"]["idle"]["seconds"] == 1800 and rows["stage"]["idle"]["seconds"] == 900
    assert rows["git"]["idle"] is None


def test_the_owner_fixes_the_idle_limit_in_config(home: Path) -> None:
    config = Config(timeouts={"agent-turn-idle": 600})
    assert timeouts_mod.describe_idle(home, config, "agent-turn")["source"] == "config"
    with pytest.raises(timeouts_mod.TimeoutRefused):
        timeouts_mod.set_timeout(home, config, "agent-turn", 900, "", idle=True)


def test_the_idle_limit_is_learned_from_the_silences_but_never_below_the_default(home: Path) -> None:
    for _ in range(5):
        timeouts_mod.record_silence(home, "agent-turn", 30)
    assert timeouts_mod.describe_idle(home, Config(), "agent-turn")["seconds"] == 1200
    for _ in range(10):
        timeouts_mod.record_silence(home, "agent-turn", 900)
    row = timeouts_mod.describe_idle(home, Config(), "agent-turn")
    assert row["source"] == "learned" and row["seconds"] == 2700


def test_cli_sets_idle_and_cap(home: Path, capsys, monkeypatch) -> None:
    monkeypatch.delenv("TIRITH_LANE", raising=False)
    assert main(["--home", str(home), "timeout", "set", "--class", "agent-turn", "--idle", "1500", "--cap", "36000",
                 "--reason", "big lanes"]) == 0
    assert main(["--home", str(home), "timeout", "get", "--class", "agent-turn"]) == 0
    out = capsys.readouterr().out
    assert "cap 36000 s" in out and "idle 1500 s" in out
    with pytest.raises(SystemExit):
        main(["--home", str(home), "timeout", "set", "--class", "agent-turn"])


FAKE = '''
import json, os, sys, time
here = os.path.dirname(os.path.abspath(__file__))
with open(os.path.join(here, "calls.jsonl"), "a") as f:
    f.write(json.dumps(sys.argv[1:]) + "\\n")
calls = sum(1 for _ in open(os.path.join(here, "calls.jsonl")))
open("work" + str(calls) + ".txt", "w").write("x")
print(json.dumps({"type": "system", "session_id": "sess-1"}), flush=True)
if calls == 1 or os.path.exists(os.path.join(here, "always")):
    time.sleep(120)  # goes silent
print(json.dumps({"type": "result", "result": "Finished.", "session_id": "sess-1"}))
'''


def _idle_lane(home: Path, tmp_path: Path, monkeypatch, always: bool):
    monkeypatch.setitem(timeouts_mod.IDLE_POLICY, "agent-turn", (3, 1, 7200))
    monkeypatch.setattr(keepgoing_mod, "gaps", lambda *a, **k: [])  # only the idle path is under test
    repo = _lane_run_setup(home, tmp_path, monkeypatch)
    agent = tmp_path / "agent"
    agent.mkdir()
    (agent / "fake.py").write_text(FAKE, encoding="utf-8")
    if always:
        (agent / "always").write_text("1")
    try:
        code = main(["lane", "run", "--project", "demo", "--class", "warden", "--feature", "Idle",
                     "--brief", str(repo / "brief.md"), "--step", "1", "--item", "Do the thing.",
                     "--id", "idle", "--workspace", str(repo), "--as-agent", "anthropic", "--",
                     sys.executable, str(agent / "fake.py"), "-p", "the brief", "--output-format", "stream-json"])
    finally:
        make_hooks_mutable(repo)
    calls = [json.loads(l) for l in (agent / "calls.jsonl").read_text().splitlines()]
    return code, calls, read_json(home / "lanes" / "idle.json", {})


def test_a_lane_gone_silent_with_work_is_resumed_in_the_same_session(home, tmp_path, monkeypatch) -> None:
    code, calls, lane = _idle_lane(home, tmp_path, monkeypatch, always=False)
    assert code == 0 and len(calls) == 2 and lane["ended_at"]
    assert calls[1][calls[1].index("--resume") + 1] == "sess-1"
    assert "You were idle for 1 min" in calls[1][1] and "continue" in calls[1][1].lower()
    assert lane["idle_resumes"] == 1


def test_a_lane_that_stays_silent_goes_to_the_manager_after_its_attempts(home, tmp_path, monkeypatch) -> None:
    from tirith import asks as asks_mod
    code, calls, lane = _idle_lane(home, tmp_path, monkeypatch, always=True)
    assert code == timeouts_mod.EXIT_TIMED_OUT and len(calls) == 1 + keepgoing_mod.MAX_ROUNDS
    assert lane["result"] == "failed" and "no progress" in lane["report"]
    assert any(i["id"] == "sq-lane-idle" for i in asks_mod.queue_items(home, only_open=True))
