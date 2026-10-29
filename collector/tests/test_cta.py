"""Every owner item has a call to action; guard items and chaining; ``tirith lane stop``."""

from __future__ import annotations

import json
import os
import signal
import subprocess
import sys
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path
from types import SimpleNamespace

import pytest

import tirith_guard
from conftest import GIT, make_hooks_mutable, make_repo, write_config, write_lane_manifest
from tirith import tirithcmd
from tirith import actions as actions_mod
from tirith import asks as asks_mod
from tirith import dispatch as dispatch_mod
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith import lanestop as lanestop_mod
from tirith.cli import add_lane_ask, add_wait, main
from tirith.collector import Collector
from tirith.home import load_config
from tirith.store import append_jsonl, read_json, read_jsonl

WORDS = ["bannedword"]


def _state(home: Path) -> dict:
    return Collector(home, client=None).build_state()


def _has_cta(item: dict) -> bool:
    return asks_mod.has_call_to_action(item)


# -- the rule ------------------------------------------------------------------------------------


def test_add_wait_refuses_an_item_without_option_or_action(home: Path) -> None:
    with pytest.raises(SystemExit) as refused:
        add_wait(home, "demo", "Please look at this", why_owner="owner-decision")
    assert "call-to-action" in str(refused.value) and "--option" in str(refused.value)
    with pytest.raises(SystemExit):
        add_wait(home, "demo", "Copy this", command="ls")  # a command to copy is not a call to action
    assert not (home / "waits.jsonl").exists()


def test_add_wait_accepts_an_option_or_a_run_action(home: Path) -> None:
    add_wait(home, "demo", "Ship it?", options=["Ship|ship"], why_owner="owner-decision")
    run = actions_mod.make_actions(["make build"], ["Build"])
    add_wait(home, "demo", "Build it", actions=run, why_owner="owner-decision")
    assert len(read_jsonl(home / "waits.jsonl")) == 2


def test_wait_add_command_line_refuses_a_bare_question(home: Path) -> None:
    with pytest.raises(SystemExit):
        main(["wait", "add", "--why-owner", "owner-decision", "--project", "demo", "Is this fine?"])
    assert main(["wait", "add", "--why-owner", "owner-decision", "--project", "demo", "--option", "Yes|yes", "Is this fine?"]) == 0


def test_dispatcher_items_need_an_option(home: Path) -> None:
    with pytest.raises(ValueError):
        dispatch_mod.add_owner_item(home, "d-1", "demo", "x", [], "2026-10-29T00:00:00Z")
    dispatch_mod.add_owner_item(home, "d-1", "demo", "x", [{"label": "Wait", "command": None}],
                                "2026-10-29T00:00:00Z", why_owner="owner-decision")


def test_collector_sends_an_item_without_call_to_action_to_the_manager(home: Path) -> None:
    append_jsonl(home / "waits.jsonl", {"id": "w-old", "at": "2026-10-29T00:00:00Z", "project": "demo",
                                         "text": "Old row with no way to answer", "link": None, "done_at": None,
                                         "why_owner": "owner-decision"})
    for _ in range(2):  # twice: queued once
        state = _state(home)
        assert all(w["id"] != "w-old" for w in state["waiting_on_you"])
    queued = [r for r in asks_mod.queue_items(home) if r["kind"] == asks_mod.NO_CTA_KIND]
    assert len(queued) == 1 and "Old row" in queued[0]["text"]
    assert any(r["id"] == queued[0]["id"] for r in state["manager_queue"])


def test_every_generator_produces_items_with_a_call_to_action(home: Path, tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    try:
        write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo), "color": "#123456"}])
        add_wait(home, "demo", "Manual ask", options=["Yes|yes"])                      # cli
        lane = {"id": "l-1", "project": "demo", "feature": "Feat", "log": None}
        add_lane_ask(home, lane, "failed")                                              # lane failure
        dispatch_mod.add_owner_item(home, "d-1", "demo", "Dispatcher item",
                                    [{"label": "Wait", "command": None}], "2026-10-29T00:00:00Z",
                                    why_owner="owner-decision")
        with (home / "events.jsonl").open("a", encoding="utf-8") as handle:             # violation
            handle.write(json.dumps({"kind": "violation", "ts": datetime.now(timezone.utc).strftime(
                "%Y-%m-%dT%H:%M:%SZ"), "rule": "no-verify", "message": "tirith: no-verify: x. Do this instead: y",
                "lane": "l-1", "agent": "anthropic", "project": "demo"}) + "\n")
        collector = Collector(home, client=None)
        real_worker, real_cache = collector.worker, collector.cache
        collector.worker = SimpleNamespace(enabled=True, submit=lambda *a, **k: None, last_error=None)
        collector.cache = SimpleNamespace(get=lambda *a, **k: {"items": ["Agent waits for a decision"],
                                                               "created_at": "2026-10-29T00:00:00Z"})
        brain = collector._brain_waits({"id": "l-1", "project": "demo", "log": None,
                                        "report": "waiting", "started_at": "2026-10-29T00:00:00Z"})
        assert brain and not any(asks_mod.is_owner_item(w) for w in brain)              # brain-detected
        collector.worker, collector.cache = real_worker, real_cache
        state = collector.build_state()
        sources = {w["source"] for w in state["waiting_on_you"]}
        assert {"manual", "guard"} <= sources and "violation" not in sources  # violations: Rules only
        assert state["waiting_on_you"] and all(asks_mod.is_owner_item(w) for w in state["waiting_on_you"])
        assert sources <= set(asks_mod.OWNER_SOURCES)
        assert not [r for r in asks_mod.queue_items(home) if r["kind"] == asks_mod.NO_CTA_KIND]
    finally:
        make_hooks_mutable(repo)


def test_violations_never_reach_the_owners_turn(home: Path) -> None:
    """The blocked agent already got what to do instead; the owner sees violations in Rules only."""
    with (home / "events.jsonl").open("a", encoding="utf-8") as handle:
        handle.write(json.dumps({"kind": "violation", "ts": datetime.now(timezone.utc).strftime(
            "%Y-%m-%dT%H:%M:%SZ"), "rule": "no-verify", "message": "tirith: no-verify: x. Do this instead: y",
            "lane": "l-1", "agent": "anthropic", "project": "demo"}) + "\n")
    state = _state(home)
    assert state["violations"] and not [w for w in state["waiting_on_you"] if w["source"] == "violation"]


# -- guard items ---------------------------------------------------------------------------------


def _guard_wait(state: dict, project: str = "demo") -> dict | None:
    return next((w for w in state["waiting_on_you"] if w["id"] == f"guard-{project}"), None)


def test_missing_guard_item_runs_install(home: Path, tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo), "color": "#123456"}])
    wait = _guard_wait(_state(home))
    assert wait and [a["command"] for a in wait["actions"]] == [tirithcmd.resolved(home) + " guard install demo"]
    assert wait["actions"][0]["cwd"] == str(repo)
    assert [o["value"] for o in wait["options"]] == ["snooze", "never"]
    found = actions_mod.find_action(home, wait["actions"][0]["id"])  # the Run path finds it by id
    assert found and found["command"] == tirithcmd.resolved(home) + " guard install demo" and found["project"] == "demo"


def test_own_hook_guard_item_offers_the_chain(home: Path, tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    hook = repo / ".git" / "hooks" / "pre-commit"
    hook.write_text("#!/bin/sh\nexit 0\n")
    hook.chmod(0o755)
    write_config(home, [{"id": "demo", "name": "ChromaSol", "repo": str(repo), "color": "#123456"}])
    wait = _guard_wait(_state(home))
    assert "without the FORBIDDEN_WORDS_GUARD marker" in wait["text"]
    action = wait["actions"][0]
    assert action["command"] == tirithcmd.resolved(home) + " guard install demo --chain"
    assert action["label"] == "Add the word guard and keep ChromaSol's hook"
    assert [o["label"] for o in wait["options"]] == ["Not now", "Never for this project"]


def test_not_now_snoozes_for_seven_days_then_it_comes_back(home: Path, tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo), "color": "#123456"}])
    reply = inbox_mod.add_reply(home, "guard-demo", "demo", "snooze")
    collector = Collector(home, client=None)
    at = datetime.fromisoformat(reply["at"].replace("Z", "+00:00"))
    assert _guard_wait(collector.build_state(at + timedelta(days=6))) is None
    assert _guard_wait(collector.build_state(at + timedelta(days=8))) is not None


def test_never_for_this_project_writes_guard_false_and_stops_reporting(home: Path, tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo), "color": "#123456"},
                        {"id": "other", "name": "Other", "repo": str(tmp_path / "nope")}])
    inbox_mod.add_reply(home, "guard-demo", "demo", "never")
    state = _state(home)
    assert _guard_wait(state) is None
    assert load_config(home).project("demo").guard is False
    assert load_config(home).project("other").guard is True  # only that project
    proj = next(p for p in state["projects"] if p["id"] == "demo")
    assert proj["guard"]["ok"] is True and proj["guard"]["disabled"] is True
    assert _guard_wait(_state(home)) is None  # still quiet on the next cycle


# -- chaining in a temp repo ---------------------------------------------------------------------


def _git(repo: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run([GIT, "-C", str(repo), "-c", "user.email=t@example.invalid", "-c", "user.name=T",
                           *args], capture_output=True, text=True)


def test_chain_keeps_and_runs_the_existing_hook_and_the_guard_blocks_words(tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    try:
        hook = repo / ".git" / "hooks" / "pre-commit"
        hook.write_text('#!/bin/sh\necho ran >> "$(git rev-parse --git-dir)/own.log"\n[ -f block.flag ] && exit 1\nexit 0\n')
        hook.chmod(0o755)
        msg = repo / ".git" / "hooks" / "commit-msg"
        msg.write_text('#!/bin/sh\ngrep -q WIP "$1" && exit 1\nexit 0\n')
        msg.chmod(0o755)
        assert any("marker" in p for p in tirith_guard.verify(repo, WORDS))
        assert not any("marker" not in p and "pre-commit" in p and "missing" in p
                       for p in tirith_guard.install(repo, WORDS))  # plain install leaves the hook alone
        assert (repo / ".git" / "hooks" / "pre-commit.local").exists() is False
        tirith_guard.install(repo, WORDS, chain=True)
        hooks = repo / ".git" / "hooks"
        assert (hooks / "pre-commit.local").exists() and (hooks / "commit-msg.local").exists()
        assert "FORBIDDEN_WORDS_GUARD" in (hooks / "pre-commit").read_text()
        problems = tirith_guard.verify(repo, WORDS)
        assert not [p for p in problems if "immutable" not in p], problems  # chained installs verify
        (repo / "a.txt").write_text("fine\n")
        _git(repo, "add", "a.txt")
        done = _git(repo, "commit", "-m", "ok")
        assert done.returncode == 0, done.stderr
        assert (repo / ".git" / "own.log").read_text().count("ran") == 1  # the kept hook ran
        (repo / "b.txt").write_text("has bannedword in it\n")
        _git(repo, "add", "b.txt")
        blocked = _git(repo, "commit", "-m", "words")
        assert blocked.returncode != 0 and "bannedword" in blocked.stderr  # the guard blocks words
        _git(repo, "reset", "-q")
        (repo / "block.flag").write_text("x")
        _git(repo, "add", "block.flag")
        own = _git(repo, "commit", "-m", "own hook says no")
        assert own.returncode != 0  # and fails when the kept hook fails
        (repo / "block.flag").unlink()
        _git(repo, "reset", "-q")
        wip = _git(repo, "commit", "--allow-empty", "-m", "WIP thing")
        assert wip.returncode != 0  # the kept commit-msg hook runs too
    finally:
        make_hooks_mutable(repo)


def test_chain_never_overwrites_an_existing_local_hook(tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    try:
        hooks = repo / ".git" / "hooks"
        (hooks / "pre-commit").write_text("#!/bin/sh\nexit 0\n")
        (hooks / "pre-commit.local").write_text("#!/bin/sh\necho mine\n")
        tirith_guard.install(repo, WORDS, chain=True)
        assert (hooks / "pre-commit.local").read_text() == "#!/bin/sh\necho mine\n"
        assert "FORBIDDEN_WORDS_GUARD" not in (hooks / "pre-commit").read_text()  # left for the owner
    finally:
        make_hooks_mutable(repo)


def test_chain_command_line_and_fresh_guard_still_verify(home: Path, tmp_path: Path, capsys) -> None:
    repo = make_repo(tmp_path / "repo")
    try:
        (repo / ".git" / "hooks" / "pre-commit").write_text("#!/bin/sh\nexit 0\n")
        (repo / ".git" / "hooks" / "pre-commit").chmod(0o755)
        write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo), "guard_words": ["bannedword"]}])
        main(["guard", "install", "demo", "--chain"])
        main(["guard", "verify", "demo"])
        out = capsys.readouterr().out
        assert "not immutable" not in out or sys.platform != "darwin"
        assert "present but without" not in out and "missing" not in out
    finally:
        make_hooks_mutable(repo)


# -- lane stop -----------------------------------------------------------------------------------

SLEEPER = "import time\nwhile True: time.sleep(0.2)"
STUBBORN = ("import signal, time\nsignal.signal(signal.SIGTERM, signal.SIG_IGN)\n"
            "print('ignoring term', flush=True)\nwhile True: time.sleep(0.2)")


def _start_lane(home: Path, tmp_path: Path, lane_id: str, code: str):
    repo = make_repo(tmp_path / "repo")
    write_lane_manifest(repo)
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo), "color": "#123456",
                         "guard": "false"}])
    config = (home / "config.toml").read_text().replace('guard = "false"', "guard = false")
    (home / "config.toml").write_text(config)
    env = dict(os.environ, TIRITH_HOME=str(home), PYTHONPATH=str(Path(__file__).resolve().parent.parent))
    runner = subprocess.Popen(
        [sys.executable, "-m", "tirith", "lane", "run", "--project", "demo", "--class", "warden",
         "--feature", "Long job", "--id", lane_id, "--brief", str(repo / "brief.md"), "--",
         sys.executable, "-c", code], env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        lane = read_json(home / "lanes" / f"{lane_id}.json", {}) or {}
        if lane.get("pid") and lane.get("pgid"):
            time.sleep(0.5)  # the child is past its imports
            return runner, repo, lane
        time.sleep(0.1)
    runner.kill()
    raise AssertionError("the lane did not start: " + (runner.stdout.read() if runner.stdout else ""))


def _gone(pid: int) -> bool:
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return True
    return False


def test_lane_stop_ends_a_running_lane_as_failed_and_keeps_the_worktree(home: Path, tmp_path: Path, capsys) -> None:
    runner, repo, lane = _start_lane(home, tmp_path, "long-1", SLEEPER)
    try:
        assert main(["lane", "stop", "long-1", "--by", "owner", "--reason", "not needed any more"]) == 0
        assert runner.wait(timeout=20) != 0
        ended = read_json(home / "lanes" / "long-1.json", {})
        assert ended["result"] == "failed" and ended["ended_at"]
        assert ended["report"] == "Stopped by owner: not needed any more"
        assert "token" not in ended
        assert _gone(lane["pid"])
        assert Path(ended["workspace"]).is_dir()  # the worktree is kept
        assert not any(str(r.get("id")) == "lane-long-1" for r in read_jsonl(home / "waits.jsonl"))
        state = _state(home)
        agent = next(a for a in state["agents"] if a["id"] == "long-1")
        assert agent["status"] == "failed" and agent["report"].startswith("Stopped by owner")
    finally:
        if runner.poll() is None:
            runner.kill()
        make_hooks_mutable(repo)


def test_lane_stop_kills_a_child_that_ignores_sigterm(home: Path, tmp_path: Path) -> None:
    runner, repo, lane = _start_lane(home, tmp_path, "stubborn-1", STUBBORN)
    try:
        started = time.monotonic()
        ended = lanestop_mod.stop_lane(home, "stubborn-1", "manager", None, term_wait=1.0)
        assert time.monotonic() - started < 15
        runner.wait(timeout=20)
        assert ended["result"] == "failed" and ended["report"] == "Stopped by manager."
        assert _gone(lane["pid"])
    finally:
        if runner.poll() is None:
            runner.kill()
        make_hooks_mutable(repo)


def test_lane_stop_ends_the_lane_itself_when_lane_run_is_gone(home: Path) -> None:
    child = subprocess.Popen([sys.executable, "-c", SLEEPER], start_new_session=True)
    try:
        lane = {"id": "orphan", "project": "demo", "feature": "Orphan", "pid": child.pid, "pgid": child.pid,
                "started_at": "2026-10-29T00:00:00Z", "ended_at": None, "result": None, "report": None}
        (home / "lanes").mkdir(exist_ok=True)
        (home / "lanes" / "orphan.json").write_text(json.dumps(lane))
        ended = lanestop_mod.stop_lane(home, "orphan", "owner", "gone", term_wait=2.0, end_wait=0.3)
        child.wait(timeout=5)
        assert ended["result"] == "failed" and ended["report"] == "Stopped by owner: gone"
    finally:
        if child.poll() is None:
            child.kill()


def test_lane_stop_refuses_unknown_and_ended_lanes(home: Path, capsys) -> None:
    assert main(["lane", "stop", "nope"]) == 3
    (home / "lanes").mkdir(exist_ok=True)
    (home / "lanes" / "old.json").write_text(json.dumps({"id": "old", "ended_at": "2026-10-29T00:00:00Z",
                                                          "result": "done"}))
    assert main(["lane", "stop", "old"]) == 3
    assert "already ended" in capsys.readouterr().err


def test_stop_link_checks_token_and_lane(home: Path, capsys) -> None:
    token = launch_mod.launch_token(home)
    assert main(["url", "open", "tirith://stop?lane=x&token=wrong"]) == 3
    assert main(["url", "open", f"tirith://stop?lane=nope&token={token}"]) == 3
    assert "not a running lane" in capsys.readouterr().err
