"""One timeout mechanism: the runner, the learned and clamped values, agent changes and the Bash hook."""

from __future__ import annotations

import json
import os
import time
from datetime import datetime, timezone
from pathlib import Path

import pytest

from conftest import make_repo, write_config
from tirith import asks as asks_mod
from tirith import hooks as hooks_mod
from tirith import runstage as runstage_mod
from tirith import selfcheck as selfcheck_mod
from tirith import timeouts as timeouts_mod
from tirith.cli import main
from tirith.home import Config, load_config
from tirith.store import read_json, read_jsonl, write_lane


def alive(pid: int) -> bool:
    try:
        os.kill(pid, 0)
    except OSError:
        return False
    return True


def start_lane(home: Path, lane_id: str = "w1", ended: bool = False) -> None:
    write_lane(home, {"id": lane_id, "project": "demo", "ended_at": "2026-10-29T12:00:00Z" if ended else None})


# -- the runner ---------------------------------------------------------------------------------


def test_run_returns_output_and_records_the_duration(home: Path) -> None:
    done = timeouts_mod.run(["echo", "hi"], "git", text=True)
    assert done.returncode == 0 and done.stdout.strip() == "hi" and not done.timed_out
    assert len(read_json(home / "timeouts.json", {})["durations"]["git"]) == 1


def test_run_stops_the_whole_group_and_says_how_to_raise_it(home: Path, tmp_path: Path) -> None:
    pidfile = tmp_path / "child.pid"
    started = time.monotonic()
    done = timeouts_mod.run(["/bin/sh", "-c", f"sleep 60 & echo $! > {pidfile}; wait"], "npm", timeout=1)
    assert time.monotonic() - started < 15
    assert done.timed_out and done.returncode == timeouts_mod.EXIT_TIMED_OUT
    assert "timed out after 1 s (class npm" in done.error and "tirith timeout set --class npm" in done.error
    child = int(pidfile.read_text())
    time.sleep(0.3)
    assert not alive(child)  # the child of the shell went with it
    book = read_json(home / "timeouts.json", {})
    assert book["timeouts"]["npm"][0]["seconds"] == 1 and "sleep 60" in book["timeouts"]["npm"][0]["command"]
    assert "npm" not in book["durations"]  # a timeout says nothing about how long the class takes


def test_run_kills_a_child_that_ignores_sigterm(home: Path, tmp_path: Path, monkeypatch) -> None:
    pidfile = tmp_path / "stubborn.pid"
    script = f"trap '' TERM; sh -c 'trap \"\" TERM; echo $$ > {pidfile}; sleep 60' & wait"
    monkeypatch.setattr(timeouts_mod, "KILL_GRACE_S", 0.5)
    done = timeouts_mod.run(["/bin/sh", "-c", script], "generic", timeout=1)
    assert done.timed_out
    time.sleep(0.3)
    assert not alive(int(pidfile.read_text()))


def test_a_missing_program_is_an_error_result_not_an_exception(home: Path) -> None:
    done = timeouts_mod.run(["definitely-not-a-program-xyz"], "generic")
    assert done.returncode == 127 and done.error


def test_a_timed_out_git_commit_removes_the_lock_it_left(home: Path, tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setitem(timeouts_mod.POLICY, "git-commit", (1, 1, 900, "a git commit"))
    repo = make_repo(tmp_path / "repo")
    done = timeouts_mod.run(["/bin/sh", "-c", "touch .git/index.lock; sleep 60"], "git-commit", cwd=repo)
    assert done.timed_out and done.cleaned == (str(repo / ".git" / "index.lock"),)
    assert not (repo / ".git" / "index.lock").exists()
    assert "removed" in done.error


def test_lock_cleanup_only_when_nobody_holds_it(tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    lock = repo / ".git" / "index.lock"
    lock.write_text("")
    argv = ["git", "-C", str(repo), "commit"]
    started = time.time() - 5
    assert timeouts_mod.cleanup_git(argv, None, started, holders=lambda path: True) == []
    assert lock.exists()  # another process holds it
    old = started - 3600
    os.utime(lock, (old, old))
    assert timeouts_mod.cleanup_git(argv, None, started, holders=lambda path: False) == []
    assert lock.exists()  # not created by this command
    os.utime(lock, None)
    assert timeouts_mod.cleanup_git(argv, None, started, holders=lambda path: False) == [str(lock)]
    assert not lock.exists()


def test_a_lock_in_a_linked_worktree_is_found(tmp_path: Path) -> None:
    from conftest import GIT
    import subprocess
    repo = make_repo(tmp_path / "repo")
    tree = tmp_path / "tree"
    subprocess.run([GIT, "-C", str(repo), "worktree", "add", "-q", "-b", "lane/x", str(tree)], check=True)
    git_dir = timeouts_mod._git_dir(["git"], tree)
    assert git_dir is not None and git_dir.parent.name == "worktrees"


# -- learning and clamping ---------------------------------------------------------------------


def test_nothing_learned_until_enough_samples(home: Path) -> None:
    for _ in range(timeouts_mod.MIN_SAMPLES - 1):
        timeouts_mod.record(home, "git", 2.0, "ok")
    assert timeouts_mod.describe(home, Config(), "git")["source"] == "default"
    timeouts_mod.record(home, "git", 2.0, "ok")
    row = timeouts_mod.describe(home, Config(), "git")
    assert row["source"] == "learned" and row["seconds"] == pytest.approx(6.0)  # p95 2 s x 3


def test_the_learned_value_is_clamped_to_min_and_max(home: Path) -> None:
    for _ in range(6):
        timeouts_mod.record(home, "git", 0.01, "ok")
        timeouts_mod.record(home, "cargo", 5000, "ok")
    assert timeouts_mod.effective(home, Config(), "git") == 5  # min
    assert timeouts_mod.effective(home, Config(), "cargo") == 7200  # max


def test_a_stage_learns_from_its_own_history_and_falls_back_to_the_class(home: Path) -> None:
    assert timeouts_mod.effective(home, Config(), "stage:test") == 1800
    for _ in range(6):
        timeouts_mod.record(home, "stage:test", 100, "ok")
    assert timeouts_mod.effective(home, Config(), "stage:test") == 300
    assert timeouts_mod.effective(home, Config(), "stage:build") == 1800


def test_the_owner_config_wins(home: Path) -> None:
    write_config(home, [], extra="[timeouts]\ngit = 77\n")
    config = load_config(home)
    for _ in range(6):
        timeouts_mod.record(home, "git", 2.0, "ok")
    assert timeouts_mod.describe(home, config, "git")["source"] == "config"
    assert timeouts_mod.effective(home, config, "git") == 77


def test_the_policy_uses_the_value_for_the_deadline(home: Path) -> None:
    write_config(home, [], extra="[timeouts]\ngeneric = 1\n")
    done = timeouts_mod.run(["sleep", "30"], "generic", home=home)
    assert done.timed_out and "1 s" in done.error


# -- agents set a timeout -----------------------------------------------------------------------


def test_an_agent_sets_a_value_within_bounds_with_a_reason(home: Path, monkeypatch, capsys) -> None:
    start_lane(home)
    monkeypatch.setenv("TIRITH_LANE", "w1")
    assert main(["timeout", "set", "--class", "git-commit", "--seconds", "400", "--reason", "slow hooks"]) == 0
    row = timeouts_mod.describe(home, Config(), "git-commit")
    assert row["seconds"] == 400 and row["source"] == "agent" and row["reason"] == "slow hooks"
    flow = read_jsonl(home / "flows.jsonl")[-1]
    assert flow["from"] == "agent:w1" and "400" in flow["text"] and flow["why"] == "slow hooks"
    capsys.readouterr()
    assert main(["timeout", "get", "--class", "git-commit"]) == 0
    assert "git-commit: 400 s (agent" in capsys.readouterr().out


def test_an_agent_is_refused_beyond_the_limits_or_without_a_reason(home: Path, monkeypatch) -> None:
    start_lane(home)
    monkeypatch.setenv("TIRITH_LANE", "w1")
    with pytest.raises(SystemExit, match="takes 30 to 900"):
        main(["timeout", "set", "--class", "git-commit", "--seconds", "5000", "--reason", "x"])
    with pytest.raises(SystemExit, match="takes 30 to 900"):
        main(["timeout", "set", "--class", "git-commit", "--seconds", "1", "--reason", "x"])
    with pytest.raises(SystemExit, match="--reason"):
        main(["timeout", "set", "--class", "git-commit", "--seconds", "100"])
    with pytest.raises(SystemExit, match="unknown class"):
        main(["timeout", "set", "--class", "bogus", "--seconds", "100", "--reason", "x"])
    assert timeouts_mod.describe(home, Config(), "git-commit")["source"] == "default"


def test_an_agent_cannot_change_what_the_owner_fixed(home: Path, monkeypatch) -> None:
    write_config(home, [], extra="[timeouts]\ngit-commit = 100\n")
    start_lane(home)
    monkeypatch.setenv("TIRITH_LANE", "w1")
    with pytest.raises(SystemExit, match="config.toml"):
        main(["timeout", "set", "--class", "git-commit", "--seconds", "200", "--reason", "x"])


def test_an_agents_value_ends_with_its_lane_but_the_owners_stays(home: Path, monkeypatch) -> None:
    start_lane(home)
    monkeypatch.setenv("TIRITH_LANE", "w1")
    main(["timeout", "set", "--class", "npm", "--seconds", "1200", "--reason", "big install"])
    assert timeouts_mod.effective(home, Config(), "npm") == 1200
    start_lane(home, ended=True)
    assert timeouts_mod.effective(home, Config(), "npm") == 900
    monkeypatch.delenv("TIRITH_LANE")
    main(["timeout", "set", "--class", "npm", "--seconds", "1500"])
    assert timeouts_mod.describe(home, Config(), "npm")["source"] == "owner"
    assert read_jsonl(home / "flows.jsonl")[-1]["from"] == "owner"


# -- the Bash hook ------------------------------------------------------------------------------


def test_bash_gets_the_timeout_of_the_policy(home: Path) -> None:
    out = timeouts_mod.bash_update(home, Config(), {"command": "ls"})
    assert out == {"command": "ls", "timeout": 120_000}


def test_bash_keeps_a_larger_value_within_the_maximum(home: Path) -> None:
    config = Config()
    assert timeouts_mod.bash_update(home, config, {"command": "ls", "timeout": 300_000}) is None  # already fine
    assert timeouts_mod.bash_update(home, config, {"command": "ls", "timeout": 9_000_000})["timeout"] == 600_000
    assert timeouts_mod.bash_update(home, config, {"command": "ls", "timeout": 5_000})["timeout"] == 120_000
    assert timeouts_mod.bash_update(home, config, {"command": "ls", "timeout": 120_000}) is None
    assert timeouts_mod.bash_update(home, config, {"command": "ls", "run_in_background": True}) is None


def test_bash_commands_are_classified() -> None:
    assert timeouts_mod.classify_command("cargo test -q") == "cargo"
    assert timeouts_mod.classify_command("FOO=1 npm run build") == "npm"
    assert timeouts_mod.classify_command("git commit -m x") == "git-commit"
    assert timeouts_mod.classify_command("git status") == "agent-bash"


def test_the_anthropic_hook_answers_a_lane_bash_call(home: Path, capsys) -> None:
    event = {"event": "PreToolUse", "tool": "Bash", "lane": "w1", "ts": "2026-10-29T12:00:00Z"}
    payload = {"tool_name": "Bash", "tool_input": {"command": "ls"}, "cwd": str(home)}
    assert hooks_mod.decide("anthropic", event, payload, home) == 0
    out = json.loads(capsys.readouterr().out)["hookSpecificOutput"]
    assert out["hookEventName"] == "PreToolUse" and out["updatedInput"] == {"command": "ls", "timeout": 120_000}
    payload["tool_input"] = {"command": "ls", "timeout": 9_000_000}
    hooks_mod.decide("anthropic", event, payload, home)
    assert json.loads(capsys.readouterr().out)["hookSpecificOutput"]["updatedInput"]["timeout"] == 600_000


def test_the_hook_stays_silent_outside_a_lane_and_for_other_agents(home: Path, capsys) -> None:
    payload = {"tool_name": "Bash", "tool_input": {"command": "ls"}, "cwd": str(home)}
    hooks_mod.decide("anthropic", {"event": "PreToolUse", "tool": "Bash"}, payload, home)
    hooks_mod.decide("muse", {"event": "PreToolUse", "tool": "Bash", "lane": "w1"}, payload, home)
    assert capsys.readouterr().out == ""


# -- stages and the self-check ------------------------------------------------------------------


def test_a_stage_past_its_timeout_is_stopped_and_fails(home: Path, tmp_path: Path) -> None:
    started = time.monotonic()
    code, output = runstage_mod.run_stage(home, "demo", "ci", "slow", ["sleep", "60"], tmp_path, echo=False, timeout=1)
    assert time.monotonic() - started < 15
    assert code == timeouts_mod.EXIT_TIMED_OUT and "timed out after 1 s (class stage:slow" in output
    end = [e for e in read_jsonl(home / "events.jsonl") if e["event"] == "end"][-1]
    assert end["status"] == "failed" and end["timed_out"] is True
    assert read_json(home / "timeouts.json", {})["timeouts"]["stage:slow"]


def test_a_stage_without_a_timeout_gets_the_policys(home: Path, tmp_path: Path) -> None:
    runstage_mod.run_stage(home, "demo", "ci", "quick", ["true"], tmp_path, echo=False)
    start = [e for e in read_jsonl(home / "events.jsonl") if e["event"] == "start"][-1]
    assert start["timeout"] == 1800
    assert read_json(home / "timeouts.json", {})["durations"]["stage:quick"]


def test_repeated_timeouts_of_a_class_reach_the_manager_queue(home: Path) -> None:
    now = datetime.now(timezone.utc)
    state: dict = {}
    run = lambda: selfcheck_mod.run(home, load_config(home), now, state, lanes=[])  # noqa: E731
    run()
    assert state["health"]["checks"]["timeouts"]["level"] == "ok"
    for _ in range(selfcheck_mod.TIMEOUTS_REPEATED):
        timeouts_mod.record(home, "git-commit", 1, "timeout", command="git commit", timeout=180)
    run()
    assert state["health"]["checks"]["timeouts"]["level"] == "failed"
    items = [q for q in asks_mod.queue_items(home, only_open=True) if q["kind"] == "selfcheck"]
    assert len(items) == 1 and "git-commit" in items[0]["text"] and "git commit" in items[0]["text"]
    run()
    assert len([q for q in asks_mod.queue_items(home, only_open=True) if q["kind"] == "selfcheck"]) == 1


def test_the_state_lists_every_class_for_settings(home: Path) -> None:
    rows = timeouts_mod.table(home, Config())
    assert [r["class"] for r in rows] == list(timeouts_mod.POLICY)
    assert all(r["label"] and r["words"] and r["seconds"] > 0 for r in rows)


# -- git calls follow the policy ----------------------------------------------------------------


def test_no_fixed_git_timeout() -> None:
    import re
    source = Path(timeouts_mod.__file__).parent
    found = []
    for path in sorted(source.glob("*.py")):
        text = path.read_text(encoding="utf-8")
        for match in re.finditer(r"(?:timeouts_mod\.run|subprocess\.(?:run|check_output))\(\s*\[\s*(?:\"git\"|git\b)", text):
            depth, end = 0, match.start()
            for end, char in enumerate(text[match.start():], match.start()):
                depth += char == "("
                depth -= char == ")"
                if depth == 0 and char == ")":
                    break
            if re.search(r"\btimeout\s*=", text[match.start():end]):
                found.append(f"{path.name}:{text.count(chr(10), 0, match.start()) + 1}")
        if re.search(r"^GIT_\w*(TIMEOUT|SECONDS|_S)\s*=", text, re.M):
            found.append(f"{path.name}: fixed git limit constant")
    assert not found, f"git calls must use the timeouts policy, not their own limit: {found}"


def test_git_deadline_follows_policy_and_hints_only_raise(home: Path) -> None:
    config = load_config(home)
    policy = timeouts_mod.effective(home, config, "git")
    quiet = dict(load=0.0, cpus=4)
    assert timeouts_mod.deadline(home, config, "git", 1, **quiet) == policy
    assert timeouts_mod.deadline(home, config, "git", policy + 50, **quiet) == policy + 50
    assert timeouts_mod.deadline(home, config, "generic", 7, **quiet) == 7


def test_git_deadline_scales_with_load_within_the_class_maximum(home: Path) -> None:
    config = load_config(home)
    policy = timeouts_mod.effective(home, config, "git")
    assert timeouts_mod.deadline(home, config, "git", load=4.0, cpus=4) == policy
    assert timeouts_mod.deadline(home, config, "git", load=8.0, cpus=4) == pytest.approx(policy * 2)
    assert timeouts_mod.deadline(home, config, "git", load=4000.0, cpus=4) == timeouts_mod.bounds("git")[2]
