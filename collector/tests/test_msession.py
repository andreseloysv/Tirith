"""The persistent manager session (docs/CONTRACT-inbox.md, "One manager"), against a fake stream-json agent."""

from __future__ import annotations

import json
import subprocess
import sys
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest

from conftest import make_hooks_mutable, make_repo, write_config, write_lane_manifest
from tirith import dispatch as dispatch_mod
from tirith import inbox as inbox_mod
from tirith import managerd as managerd_mod
from tirith import msession as msession_mod
from tirith.cli import main
from tirith.collector import Collector
from tirith.home import load_config
from tirith.store import read_json, read_jsonl

FAKE = str(Path(__file__).with_name("fake_manager_agent.py"))
AGENTS = ('[agents.anthropic]\ncommand = "agent-bin"\npermissions = ["Bash(tirith:*)", "Bash(ls:*)"]\n'
          '\n[manager]\nmax_turns = 5\ncontext_tokens = 5000\n')


class Clock:
    def __init__(self):
        self.now = datetime.now(timezone.utc)

    def tick(self, seconds: float) -> datetime:
        self.now += timedelta(seconds=seconds)
        return self.now


def fake_popen(argv, **kwargs):
    """Run the fake agent instead of ``tirith lane run --interactive ... -- <agent> <chat args>``."""
    i = argv.index("--")
    return subprocess.Popen([sys.executable, FAKE, *argv[i + 2:]], **kwargs)


class Rig:
    def __init__(self, home: Path, repo: Path, monkeypatch, popen=fake_popen):
        self.home, self.repo, self.clock, self.popen = home, repo, Clock(), popen
        self.log = home.parent / "fake.log"
        monkeypatch.setenv("FAKE_LOG", str(self.log))

    def cycle(self) -> None:
        dispatch_mod.run(self.home, load_config(self.home), self.clock.now, agents=[], lanes=[],
                         status_of=lambda lane, n: "working", manager_popen=self.popen,
                         manager_spawn=lambda argv, h, log: 1)  # a one-shot turn never really starts here

    def until(self, cond, timeout: float = 25.0, step: float = 0.0):
        end = time.time() + timeout
        while time.time() < end:
            self.cycle()
            got = cond()
            if got:
                return got
            time.sleep(0.03)
            self.clock.tick(step)
        raise AssertionError("condition not reached")

    def chat(self, text: str) -> str:
        return inbox_mod.add_request(self.home, "chat", text=text, at=self.clock.now.strftime("%Y-%m-%dT%H:%M:%SZ"))["id"]

    def request(self, rid: str) -> dict:
        return inbox_mod.find_request(self.home, rid)

    def fake_log(self) -> list[dict]:
        return [json.loads(l) for l in self.log.read_text().splitlines()] if self.log.exists() else []

    def book(self) -> dict:
        return (dispatch_mod.load_state(self.home).get("manager") or {}).get("session") or {}

    def session(self):
        return msession_mod.current(self.home)


@pytest.fixture
def rig(home: Path, tmp_path: Path, monkeypatch):
    repo = make_repo(tmp_path / "demo")
    write_lane_manifest(repo)
    (repo / "docs").mkdir()
    (repo / "docs" / "TODO.md").write_text("# TODO\n\n## 1. First\n\n- [ ] Open thing.\n", encoding="utf-8")
    subprocess.run(["git", "-C", str(repo), "add", "."], check=True)  # the plan gate edits TODO.md: it must be clean
    subprocess.run(["git", "-C", str(repo), "-c", "user.email=t@example.invalid", "-c", "user.name=T",
                    "commit", "-q", "-m", "plan"], check=True)
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}], AGENTS)
    r = Rig(home, repo, monkeypatch)
    yield r
    sess = r.session()
    if sess is not None:
        sess.terminate()
        msession_mod.register(home, None)


def test_message_goes_to_stdin_and_the_stream_becomes_transcript_steps_and_an_answer(rig: Rig) -> None:
    rid = rig.chat("Why is step 1 open? PERM-FREE")
    rig.until(lambda: rig.book().get("done_turns"))
    messages = [r["message"] for r in rig.fake_log() if "message" in r]
    assert len(messages) == 1 and rid in messages[0] and "## Rules" in messages[0]  # the first message has the brief
    # the stream is in the transcript, line by line
    lines = msession_mod.transcript_path(rig.home).read_text().splitlines()
    assert any('"stream_event"' in l for l in lines) and any('"type": "result"' in l for l in lines)
    # the manager did not answer by itself: the result text lands as the inbox answer
    answers = rig.request(rid)["answers"]
    assert [a["text"] for a in answers] == ["Answer from the fake manager."] and answers[0]["by"] == "manager"
    # the turn: CLI-style steps, a diff for the Edit, one task list, no tool result for the task list
    view = msession_mod.read_turn(rig.home, rig.book()["turns"][0]["id"])
    kinds = [(s["kind"], s["name"]) for s in view["steps"]]
    assert ("tool", "Read") in kinds and ("todo", "TodoWrite") in kinds and ("tool", "Edit") in kinds
    edit = next(s for s in view["steps"] if s["name"] == "Edit")
    assert edit["diff"]["rows"] == [["-", "a = 1"], ["+", "a = 2"], ["+", "b = 3"]]
    assert view["summary"]["done"] and view["summary"]["cost_usd"] == 0.0123 and view["requests"] == [rid]
    assert view["steps"][0]["at"]


def test_second_message_reaches_the_same_process_and_waits_for_the_turn_to_end(rig: Rig) -> None:
    rig.chat("[[SLOW]] first")
    rig.until(lambda: rig.session() is not None and rig.session().busy)
    rig.until(lambda: any("message" in r for r in rig.fake_log()))  # busy is set a moment before the agent logs it
    second = rig.chat("second, typed while it works")
    for _ in range(5):  # busy: not written to the agent
        rig.cycle()
        time.sleep(0.02)
    assert [r["message"] for r in rig.fake_log() if "message" in r][-1].count("second") == 0
    assert rig.session().busy
    msession_mod.send_control(rig.home, "interrupt")
    rig.until(lambda: any(second in r.get("message", "") for r in rig.fake_log()))
    starts = [r for r in rig.fake_log() if "start" in r]
    assert len(starts) == 1  # one process the whole time
    assert "New for you" in [r["message"] for r in rig.fake_log() if "message" in r][-1]


def test_interrupt_ends_the_turn_and_the_message_is_not_offered_again(rig: Rig) -> None:
    rid = rig.chat("[[SLOW]] please")
    rig.until(lambda: rig.session() is not None and rig.session().busy)
    msession_mod.send_control(rig.home, "interrupt")
    rig.until(lambda: rig.book().get("done_turns"))
    view = msession_mod.read_turn(rig.home, rig.book()["turns"][0]["id"])
    assert view["interrupted"] and view["error"]
    for _ in range(5):
        rig.cycle()
    assert len([r for r in rig.fake_log() if "message" in r]) == 1
    assert rig.request(rid)["answers"] == []  # an interrupted turn writes no answer
    assert rig.session().alive


def test_permissions_allowed_denied_by_code_or_left_to_the_owner(rig: Rig) -> None:
    rig.chat('PERM:Bash:{"command": "tirith inbox list"}')
    rig.until(lambda: any(r.get("permission") for r in rig.fake_log()))
    assert [r["permission"] for r in rig.fake_log() if "permission" in r] == ["allow"]
    rig.until(lambda: rig.book().get("done_turns"))
    rig.chat('PERM:Edit:{"file_path": "/etc/hosts", "old_string": "a", "new_string": "b"}')
    rig.until(lambda: len([r for r in rig.fake_log() if "permission" in r]) == 2)
    assert [r["permission"] for r in rig.fake_log() if "permission" in r][-1] == "deny"  # never edits files itself
    assert not read_jsonl(rig.home / "waits.jsonl")  # neither reached the owner
    rig.until(lambda: len(rig.book()["done_turns"]) == 2)
    rig.chat('PERM:Bash:{"command": "curl https://example.com"}')
    waits = rig.until(lambda: read_jsonl(rig.home / "waits.jsonl"))
    wait = waits[0]
    assert wait["id"].startswith("perm-") and [o["value"] for o in wait["options"]] == ["allow", "deny"]
    assert "curl https://example.com" in wait["text"]
    inbox_mod.add_reply(rig.home, wait["id"], value="allow")
    rig.until(lambda: len([r for r in rig.fake_log() if "permission" in r]) == 3)
    assert [r["permission"] for r in rig.fake_log() if "permission" in r][-1] == "allow"
    rig.until(lambda: read_jsonl(rig.home / "waits.jsonl")[-1].get("done_at"))  # the item closes


def test_denied_by_the_owner_is_written_back_as_deny(rig: Rig) -> None:
    rig.chat('PERM:Bash:{"command": "rm -rf build"}')
    wait = rig.until(lambda: read_jsonl(rig.home / "waits.jsonl"))[0]
    inbox_mod.add_reply(rig.home, wait["id"], value="deny")
    rig.until(lambda: any(r.get("permission") for r in rig.fake_log()))
    assert [r["permission"] for r in rig.fake_log() if "permission" in r] == ["deny"]


def test_a_dead_session_is_restarted_with_resume_and_a_fast_resume_failure_starts_fresh(rig: Rig) -> None:
    rig.chat("hello")
    rig.until(lambda: rig.book().get("done_turns"))
    sid = rig.book()["session_id"]
    assert sid
    rig.session().terminate()
    rig.until(lambda: rig.session() is None or not rig.session().alive)
    rig.cycle()
    rig.chat("again")
    rig.clock.tick(60)
    rig.until(lambda: len([r for r in rig.fake_log() if "start" in r]) == 2)
    second = [r for r in rig.fake_log() if "start" in r][1]
    assert second["resume"] == sid  # same conversation
    assert "--include-partial-messages" in second["start"]
    # the resumed session dies at once: the next start is fresh, with the whole handoff again
    rig.session().terminate()
    rig.until(lambda: rig.book().get("session_id") is None)
    rig.chat("third")
    rig.clock.tick(700)  # two fast deaths in a row: the one-shot manager answers for 10 minutes first
    rig.until(lambda: len([r for r in rig.fake_log() if "start" in r]) == 3)
    assert [r for r in rig.fake_log() if "start" in r][2]["resume"] is None
    rig.until(lambda: "## Rules" in [r.get("message", "") for r in rig.fake_log()][-1])


def test_control_stop_and_restart(rig: Rig) -> None:
    rig.chat("hello")
    rig.until(lambda: rig.book().get("done_turns"))
    first = rig.session()
    msession_mod.send_control(rig.home, "stop")
    rig.until(lambda: rig.session() is None)
    assert rig.book()["stopped"]
    lane = read_json(rig.home / "lanes" / f"{first.lane_id}.json", None)
    assert lane is None or lane["stop"]["by"] == "owner"  # (the fake runs without `lane run`)
    rig.clock.tick(60)
    rig.chat("typed while stopped")  # a message written after the stop wants it back
    rig.until(lambda: rig.session() is not None and rig.session().alive)
    msession_mod.send_control(rig.home, "restart", "fresh")
    rig.until(lambda: rig.session() is None or rig.session() is not first and not first.alive)
    assert rig.book().get("stopped") is None


def test_context_over_the_limit_compacts_then_restarts_with_a_handoff(rig: Rig, monkeypatch) -> None:
    monkeypatch.setenv("FAKE_CTX", "9000")  # over [manager] context_tokens
    rig.chat("hello")
    rig.until(lambda: rig.book().get("done_turns"))
    rig.until(lambda: any(r.get("message") == "/compact" for r in rig.fake_log()))  # asked to compact first
    # the fake reports a small context for /compact, so the conversation goes on in the same process
    rig.until(lambda: rig.book().get("compact_turn") and len(rig.book()["done_turns"]) >= 2)
    assert len([r for r in rig.fake_log() if "start" in r]) == 1
    # the next real turn is big again: compacting did not help twice, so it restarts fresh
    rig.chat("more")
    rig.until(lambda: len(rig.book()["done_turns"]) >= 3)
    rig.until(lambda: rig.session() is None)
    assert rig.book()["turns_total"] == 0 and rig.book()["primed"] is False
    rig.chat("after the fresh start")
    rig.clock.tick(60)
    rig.until(lambda: len([r for r in rig.fake_log() if "start" in r]) == 2, timeout=10)
    assert [r for r in rig.fake_log() if "start" in r][1]["resume"] is None
    # the fresh session got the whole handoff (it may be asked to compact right after: the fake context stays big)
    rig.until(lambda: len([r for r in rig.fake_log() if "## Rules" in r.get("message", "")]) >= 2)


def test_long_conversation_compacts_after_max_turns(rig: Rig) -> None:
    for i in range(5):  # [manager] max_turns = 5
        rig.chat(f"message {i}")
        rig.until(lambda n=i: len(rig.book().get("done_turns", [])) >= n + 1)
    rig.until(lambda: any(r.get("message") == "/compact" for r in rig.fake_log()))


def test_a_session_that_cannot_start_falls_back_to_the_one_shot_manager(rig: Rig) -> None:
    def broken(argv, **kwargs):
        raise OSError("no such file")
    rig.popen = broken
    spawned = []
    rig.chat("hello")
    dispatch_mod.run(rig.home, load_config(rig.home), rig.clock.now, agents=[], lanes=[],
                     status_of=lambda lane, n: "working", manager_popen=broken,
                     manager_spawn=lambda argv, h, log: spawned.append(argv) or 1)
    assert "could not start" in rig.book()["error"] and len(spawned) == 1 and "--brief" in spawned[0]


def test_cli_manager_say_and_control(home: Path, rig: Rig, capsys) -> None:
    assert main(["manager", "say", "are", "you", "there"]) == 0
    rid = capsys.readouterr().out.strip()
    assert inbox_mod.find_request(home, rid)["text"] == "are you there"
    for verb in ("interrupt", "stop", "restart"):
        assert main(["manager", verb]) == 0
    assert [r["op"] for r in read_jsonl(msession_mod.control_path(home))] == ["interrupt", "stop", "restart"]
    assert main(["manager", "reviewing", "the", "plan"]) == 0  # not a verb: the old activity line
    assert read_json(home / "orchestrator.json", {})["activity"] == "reviewing the plan"


def test_state_has_one_row_per_turn_with_the_lanes_started_in_it(rig: Rig) -> None:
    rid = rig.chat("hello")
    rig.until(lambda: rig.book().get("done_turns"))
    state = Collector(rig.home, load_config(rig.home)).build_state(datetime.now(timezone.utc))
    turn = state["manager"]["turns"][-1]
    assert turn["turn_requests"] == [rid] and turn["status"] == "done" and turn["live"]["steps"] >= 4
    assert turn["live"]["cost_usd"] == 0.0123 and turn["live"]["detail"]["path"].endswith(".steps.js")
    assert state["manager"]["presence"]["session"]["lane"] == rig.session().lane_id


def test_rules_decide_by_code(home: Path, rig: Rig) -> None:
    config = load_config(home)

    def verdict(tool, data):
        return msession_mod.decide(config, home, "anthropic", tool, data, rig.repo, "demo")[0]

    assert verdict("Read", {"file_path": "x"}) == "allow"
    assert verdict("Bash", {"command": "tirith inbox answer a b && ls -la"}) == "allow"
    assert verdict("Bash", {"command": "tirith inbox list; rm -rf x"}) == "ask"
    assert verdict("Bash", {"command": "ls $(whoami)"}) == "ask"
    assert verdict("Bash", {"command": "git commit --no-verify -m x"}) == "deny"  # a gate
    assert verdict("Write", {"file_path": str(rig.repo / "a.py"), "content": "x"}) == "deny"
    assert verdict("Write", {"file_path": str(home / "note.md"), "content": "x"}) == "allow"


def test_end_to_end_through_lane_run_interactive(home: Path, tmp_path: Path, monkeypatch) -> None:
    """The real wrapper: `tirith lane run --interactive` registers the lane, the fake agent talks on its pipes."""
    repo = make_repo(tmp_path / "e2e")
    write_lane_manifest(repo)
    (repo / "docs").mkdir()
    (repo / "docs" / "TODO.md").write_text("# TODO\n\n## 1. First\n\n- [ ] Open thing.\n", encoding="utf-8")
    subprocess.run(["git", "-C", str(repo), "add", "."], check=True)
    subprocess.run(["git", "-C", str(repo), "-c", "user.email=t@example.invalid", "-c", "user.name=T",
                    "commit", "-q", "-m", "plan"], check=True)
    wrapper = tmp_path / "agent-bin"
    wrapper.write_text("#!/bin/sh\nexec " + sys.executable + " " + FAKE + ' "$@"\n')
    wrapper.chmod(0o755)
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}],
                 '[agents.anthropic]\ncommand = "' + str(wrapper) + '"\n')
    cfg = home / "config.toml"
    cfg.write_text(cfg.read_text().replace('name = "Demo"', 'name = "Demo"\nguard = false'))
    monkeypatch.delenv("TIRITH_LANE", raising=False)
    rig = Rig(home, repo, monkeypatch, popen=None)
    rid = rig.chat("hello through the wrapper")
    try:
        rig.until(lambda: rig.book().get("done_turns"), timeout=30)
        assert rig.request(rid)["answers"][0]["text"] == "Answer from the fake manager."
        lane = read_json(home / "lanes" / (rig.session().lane_id + ".json"), {})
        assert lane["id"].startswith("manager-") and lane["by_tirith"] and lane["result"] is None
        rig.session().proc.stdin.close()  # the agent sees EOF and leaves: `lane run` ends the lane
        rig.until(lambda: read_json(home / "lanes" / (lane["id"] + ".json"), {}).get("ended_at"), timeout=90)
        ended = read_json(home / "lanes" / (lane["id"] + ".json"), {})
        assert ended["result"] == "done"
        assert not read_jsonl(home / "manager_queue.jsonl")  # a session that ends cleanly raises nothing
    finally:
        sess = rig.session()
        if sess is not None:
            sess.terminate()
        make_hooks_mutable(repo)


def test_a_session_that_keeps_dying_hands_over_to_the_one_shot_manager(rig: Rig, capsys) -> None:
    def dying(argv, **kwargs):
        return subprocess.Popen([sys.executable, "-c", "import sys; sys.exit(2)"], **kwargs)

    spawned: list = []
    rig.chat("hello")
    for _ in range(4):
        end = time.time() + 3
        while time.time() < end:
            dispatch_mod.run(rig.home, load_config(rig.home), rig.clock.now, agents=[], lanes=[],
                             status_of=lambda lane, n: "working", manager_popen=dying,
                             manager_spawn=lambda argv, h, log: spawned.append(argv) or 1)
            if spawned or (rig.session() is not None and not rig.session().alive):
                break
            time.sleep(0.05)
        rig.clock.tick(60)
    assert spawned, rig.book()
    assert "--brief" in spawned[0]  # the one-shot manager answers
    assert "exited at once" in capsys.readouterr().err


def test_a_session_refused_at_start_records_why_and_the_one_shot_starts_in_the_same_pass(rig: Rig, capsys) -> None:
    def refused(argv, **kwargs):  # `tirith lane run` printing its refusal, as the real one does
        log = rig.home / "logs" / f"{argv[argv.index('--id') + 1]}.launch.log"
        log.parent.mkdir(parents=True, exist_ok=True)
        log.write_text("tirith: plan-item: no agent starts without a plan item: that item is already in progress "
                       "by lane manager-old\n")
        return subprocess.Popen([sys.executable, "-c", "import sys; sys.exit(3)"], **kwargs)

    spawned: list = []
    rig.chat("hello")
    started = time.time()
    while time.time() - started < 20 and not spawned:
        dispatch_mod.run(rig.home, load_config(rig.home), rig.clock.now, agents=[], lanes=[],
                         status_of=lambda lane, n: "working", manager_popen=refused,
                         manager_spawn=lambda argv, h, log: spawned.append(argv) or 1)
        time.sleep(0.05)
    assert spawned and "--brief" in spawned[0]  # the first death hands over at once, no waiting
    assert "already in progress by lane manager-old" in rig.book()["error"]
    assert "already in progress by lane manager-old" in capsys.readouterr().err  # the collector log says it too
