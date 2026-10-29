"""Tirith checks itself: each check and its remedy, with fake clocks, processes and spawns."""

from __future__ import annotations

import json
import os
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest

from conftest import make_repo, write_config, write_lane_manifest, write_plan
from tirith import asks as asks_mod
from tirith import dispatch as dispatch_mod
from tirith import inbox as inbox_mod
from tirith import managerd as managerd_mod
from tirith import selfcheck as selfcheck_mod
from tirith.home import load_config
from tirith.store import write_lane

NOW = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)
AGENTS = '[agents.anthropic]\ncommand = "agent-bin"\n\n[manager]\nsession = false\n'
Z = "%Y-%m-%dT%H:%M:%SZ"
DEAD_PID = 2 ** 22 + 7


def at(seconds: float = 0) -> datetime:
    return NOW + timedelta(seconds=seconds)


@pytest.fixture
def project(home: Path, tmp_path: Path) -> Path:
    repo = make_repo(tmp_path / "demo")
    write_lane_manifest(repo)
    write_plan(repo)
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}], AGENTS)
    return repo


class Fake:
    def __init__(self) -> None:
        self.spawned: list[list[str]] = []
        self.killed: list[dict] = []

    def spawn(self, argv, home, log) -> int:
        self.spawned.append(argv)
        return 1

    def kill(self, lane) -> None:
        self.killed.append(lane)


def check(home: Path, state: dict, when: datetime, fake: Fake, **kw) -> bool:
    return selfcheck_mod.run(home, load_config(home), when, state, lanes=kw.pop("lanes", []),
                             spawn=fake.spawn, kill=fake.kill, **kw)


def level(state: dict, name: str) -> str:
    return state["health"]["checks"][name]["level"]


def chat(home: Path, when: datetime, text: str = "hello") -> str:
    return inbox_mod.add_request(home, "chat", text=text, at=when.strftime(Z))["id"]


def says(home: Path) -> list[str]:
    return [s["text"] for s in inbox_mod.says(home)]


def queue_ids(home: Path) -> list[str]:
    return [q["id"] for q in asks_mod.queue_items(home, only_open=True) if q["kind"] == "selfcheck"]


# -- all well -----------------------------------------------------------------------------------


def test_a_quiet_home_is_green(home: Path, project: Path) -> None:
    state: dict = {}
    assert check(home, state, NOW, Fake())
    assert state["health"]["level"] == "green"
    public = selfcheck_mod.public(state["health"])
    assert public["level"] == "green"
    assert [c["id"] for c in public["checks"]] == ["collector", "manager", "autopilot", "disk", "reviews", "timeouts"]
    assert all(c["level"] == "ok" and c["text"] for c in public["checks"])


def test_public_is_none_before_the_first_pass() -> None:
    assert selfcheck_mod.public(None) is None
    assert selfcheck_mod.public({}) is None


def test_an_unchanged_pass_does_not_rewrite_the_state(home: Path, project: Path) -> None:
    state: dict = {}
    fake = Fake()
    check(home, state, NOW, fake)
    assert not check(home, state, at(5), fake)
    assert check(home, state, at(40), fake)  # the time stamp moves at most every SAVE_EVERY_S


# -- collector ----------------------------------------------------------------------------------


def test_collector_gap_is_kept_for_a_while(home: Path, project: Path) -> None:
    state: dict = {}
    fake = Fake()
    check(home, state, NOW, fake)
    check(home, state, at(300), fake)
    text = state["health"]["checks"]["collector"]["text"]
    assert "5 min" in text and level(state, "collector") == "ok"
    for seconds in range(330, 1000, 30):
        check(home, state, at(seconds), fake)
    assert "paused" not in state["health"]["checks"]["collector"]["text"]


# -- manager ------------------------------------------------------------------------------------


def test_a_chat_message_nobody_works_on_restarts_the_manager_and_is_told(home: Path, project: Path) -> None:
    fake, state = Fake(), {}
    first, second = chat(home, NOW), chat(home, at(10))
    state["manager"] = {"tries": 2, "key": "old", "at": NOW.strftime(Z), "error": "no command"}
    check(home, state, at(100), fake)
    assert level(state, "manager") == "ok"  # under 120 s
    check(home, state, at(200), fake)
    assert level(state, "manager") == "healing" and state["health"]["level"] == "amber"
    assert state["manager"]["tries"] == 0 and "key" not in state["manager"] and "error" not in state["manager"]
    assert first in state["manager"]["evidence"] and queue_ids(home) == []
    assert says(home) == []  # told once the messages are answered, not before
    for rid in (first, second):
        inbox_mod.add_answer(home, rid, "ok", by="manager")
    check(home, state, at(260), fake)
    assert says(home) == ["The manager got stuck for 3 min; I restarted it and answered your 2 message(s)."]
    check(home, state, at(300), fake)
    assert len(says(home)) == 1 and state["health"]["level"] == "green"


def test_the_restart_reaches_the_next_manager_brief(home: Path, project: Path) -> None:
    chat(home, NOW)
    spawned: list[list[str]] = []
    state = dispatch_mod.load_state(home)
    state["manager"] = {"tries": 2, "key": "old", "at": NOW.strftime(Z)}
    config = load_config(home)
    selfcheck_mod.run(home, config, at(200), state, lanes=[], spawn=lambda *a: 1, kill=lambda l: None)
    assert managerd_mod.run(home, config, at(200), state, [], spawn=lambda argv, h, log: spawned.append(argv) or 1)
    brief = Path(spawned[0][spawned[0].index("--brief") + 1]).read_text(encoding="utf-8")
    assert "## Self-check" in brief and "no manager turn working" in brief
    assert "evidence" not in state["manager"]


def test_a_manager_that_is_working_or_attached_is_not_stuck(home: Path, project: Path) -> None:
    chat(home, NOW)
    state = {"manager": {"lane": "manager-1", "at": at(100).strftime(Z)}}
    check(home, state, at(200), Fake())  # a spawned lane not registered yet still counts
    assert level(state, "manager") == "ok"
    (home / "manager.json").write_text(json.dumps({"pid": os.getpid(), "at": at(190).strftime(Z)}))
    state = {}
    check(home, state, at(200), Fake())
    assert level(state, "manager") == "ok"


def test_restarts_are_limited_then_one_queue_item_and_red(home: Path, project: Path) -> None:
    rid = chat(home, NOW)
    state: dict = {}
    fake = Fake()
    for n in range(selfcheck_mod.MAX_TRIES):
        check(home, state, at(200 + 100 * n), fake)
        assert level(state, "manager") == "healing"
    check(home, state, at(200 + 100 * selfcheck_mod.MAX_TRIES), fake)
    assert level(state, "manager") == "failed" and state["health"]["level"] == "red"
    ids = queue_ids(home)
    assert len(ids) == 1
    check(home, state, at(900), fake)
    assert queue_ids(home) == ids  # once
    inbox_mod.add_answer(home, rid, "ok", by="manager")
    check(home, state, at(950), fake)
    assert queue_ids(home) == [] and state["health"]["level"] == "green"  # the item closes itself


def test_a_pause_with_its_own_item_is_not_restarted_over(home: Path, project: Path) -> None:
    chat(home, NOW)
    item = asks_mod.queue_add(home, "demo", "manager-x", "paused", kind="manager-paused",
                              item_id="sq-manager-paused-x")
    state = {"manager": {"paused": item["id"], "tries": 2}}
    check(home, state, at(200), Fake())
    assert level(state, "manager") == "failed" and state["manager"]["tries"] == 2


def test_auto_launch_off_leaves_the_manager_alone(home: Path, project: Path) -> None:
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(project)}],
                 AGENTS + "[dispatch]\nauto_launch = false\n")
    chat(home, NOW)
    state: dict = {}
    check(home, state, at(500), Fake())
    assert level(state, "manager") == "ok"


def lingering_lane(tmp_path: Path, age: float) -> dict:
    log = tmp_path / "m.log"
    log.write_text(json.dumps({"type": "result"}) + "\n", encoding="utf-8")
    stamp = (NOW - timedelta(seconds=age)).timestamp()
    os.utime(log, (stamp, stamp))
    return {"id": "manager-1", "pid": os.getpid(), "pgid": os.getpid(), "log": str(log), "project": "demo"}


def test_a_manager_process_alive_after_its_result_is_ended_and_the_owner_told(home: Path, project: Path,
                                                                            tmp_path: Path) -> None:
    lane = lingering_lane(tmp_path, 30)
    state, fake = {"manager": {"ended": "manager-1"}}, Fake()
    check(home, state, NOW, fake, lanes=[lane])
    assert fake.killed == [] and level(state, "manager") == "ok"  # the app's own reaper has until 60 s
    lane = lingering_lane(tmp_path, 90)
    check(home, state, NOW, fake, lanes=[lane])
    assert fake.killed == [lane] and level(state, "manager") == "healing"
    assert says(home) == ["A manager process stayed open after answering; I closed it."]
    check(home, state, at(10), fake, lanes=[lane])  # inside the grace: not killed twice, not said twice
    assert len(fake.killed) == 1 and len(says(home)) == 1


def test_a_process_that_survives_every_kill_is_a_queue_item(home: Path, project: Path, tmp_path: Path) -> None:
    lane = lingering_lane(tmp_path, 90)
    state, fake = {"manager": {"lane": "manager-1"}}, Fake()
    for n in range(selfcheck_mod.MAX_TRIES + 1):
        check(home, state, at(100 * n), fake, lanes=[lane])
    assert len(fake.killed) == selfcheck_mod.MAX_TRIES
    assert level(state, "manager") == "failed" and len(queue_ids(home)) == 1


# -- autopilot ----------------------------------------------------------------------------------


def plan(status: str = "ready") -> list[dict]:
    return [{"id": "demo", "roadmap": {"steps": [
        {"id": "1", "status": status, "items": [{"text": "Do it", "done": False, "who": "agent"}]}]}}]


def test_ready_steps_nothing_working_and_no_reason_for_15_min_is_a_queue_item(home: Path, project: Path) -> None:
    state, fake = {}, Fake()
    check(home, state, NOW, fake, projects=plan())
    check(home, state, at(600), fake, projects=plan())
    assert queue_ids(home) == [] and level(state, "autopilot") == "ok"
    check(home, state, at(960), fake, projects=plan())
    assert len(queue_ids(home)) == 1 and level(state, "autopilot") == "healing"
    item = asks_mod.queue_items(home, only_open=True)[0]
    assert "demo step 1" in item["text"] and "16 min" in item["text"]
    check(home, state, at(1000), fake, projects=plan())
    assert len(queue_ids(home)) == 1
    check(home, state, at(1100), fake, projects=plan("blocked"))
    assert queue_ids(home) == [] and level(state, "autopilot") == "ok"


def test_a_step_with_a_reason_is_not_a_problem(home: Path, project: Path) -> None:
    state = {"autopilot": {"paused": {"demo|1": {"at": NOW.strftime(Z), "items": ["Do it"], "why": "stuck"}}}}
    check(home, state, NOW, Fake(), projects=plan())
    check(home, state, at(2000), Fake(), projects=plan())
    assert queue_ids(home) == [] and level(state, "autopilot") == "ok"


def test_a_working_agent_or_autopilot_off_is_not_idle(home: Path, project: Path) -> None:
    working = [{"id": "lane-1", "status": "working"}]
    state = {"autopilot": {"lanes": {"lane-1": {"project": "demo", "step": "2", "at": NOW.strftime(Z),
                                                "items": [], "settled": False}}}}
    check(home, state, NOW, Fake(), projects=plan(), agents=working)
    check(home, state, at(2000), Fake(), projects=plan(), agents=working)
    assert queue_ids(home) == []
    state = {"autopilot": {"on": False}}
    check(home, state, NOW, Fake(), projects=plan())
    check(home, state, at(2000), Fake(), projects=plan())
    assert queue_ids(home) == [] and "off" in state["health"]["checks"]["autopilot"]["text"]


# -- disk ---------------------------------------------------------------------------------------


def test_low_disk_is_red_in_plain_words_and_recovers(home: Path, project: Path) -> None:
    state = {"disk": {"free_bytes": 2 * 1024 ** 3, "low": True}}
    check(home, state, NOW, Fake())
    assert level(state, "disk") == "failed" and state["health"]["level"] == "red"
    assert "2.0 GB" in state["health"]["checks"]["disk"]["text"]
    state["disk"]["low"] = False
    check(home, state, at(5), Fake())
    assert level(state, "disk") == "ok"


# -- reviews ------------------------------------------------------------------------------------


def review_lane(home: Path, project: Path, status: str, when: datetime, message: str = "", pid: int = 0) -> dict:
    lane = {"id": "lane-a", "project": "demo", "workspace": str(project / "w"), "ended_at": NOW.strftime(Z),
            "result": "done", "review": {"status": status, "at": when.strftime(Z), "pid": pid, "message": message}}
    write_lane(home, lane)
    return lane


def test_a_review_running_over_20_min_with_a_dead_process_is_run_again(home: Path, project: Path) -> None:
    lane = review_lane(home, project, "running", NOW, pid=DEAD_PID)
    state, fake = {}, Fake()
    check(home, state, at(1100), fake, lanes=[lane])
    assert fake.spawned == [] and level(state, "reviews") == "ok"
    check(home, state, at(1300), fake, lanes=[lane])
    assert len(fake.spawned) == 1 and fake.spawned[0][-3:] == ["lane", "review", "lane-a"]
    assert level(state, "reviews") == "healing"
    assert says(home) == ["The review of lane lane-a was stuck for 22 min; I started it again."]
    check(home, state, at(1310), fake, lanes=[lane])
    assert len(fake.spawned) == 1  # inside the grace


def test_a_stuck_review_that_is_alive_is_ended_first(home: Path, project: Path) -> None:
    lane = review_lane(home, project, "running", NOW, pid=os.getpid())
    fake = Fake()
    check(home, {}, at(1300), fake, lanes=[lane])
    assert fake.killed == [{"pid": os.getpid()}] and fake.spawned == []


def test_a_review_that_never_finishes_is_a_queue_item_after_the_tries(home: Path, project: Path) -> None:
    lane = review_lane(home, project, "running", NOW, pid=DEAD_PID)
    state, fake = {}, Fake()
    for n in range(selfcheck_mod.MAX_TRIES + 1):
        check(home, state, at(1300 + 100 * n), fake, lanes=[lane])
    assert len(fake.spawned) == selfcheck_mod.MAX_TRIES
    assert level(state, "reviews") == "failed" and len(queue_ids(home)) == 1


FAIL = "FAILED tests/test_billing.py::test_total - assert 1 == 2\n1 failed, 40 passed"


def test_unrelated_test_failure_is_run_again_once_then_queued(home: Path, project: Path) -> None:
    lane = review_lane(home, project, "tests-failed", NOW, FAIL)
    state, fake = {}, Fake()

    def diff(repo, lane_id):
        return ["collector/tirith/selfcheck.py", "web/js/v-health.js"]

    check(home, state, at(10), fake, lanes=[lane], diff=diff)
    assert len(fake.spawned) == 1 and level(state, "reviews") == "healing"
    assert "did not touch" in says(home)[0]
    check(home, state, at(200), fake, lanes=[lane], diff=diff)
    assert len(fake.spawned) == 1 and level(state, "reviews") == "failed"
    assert len(queue_ids(home)) == 1 and "test_billing.py" in asks_mod.queue_items(home, only_open=True)[0]["text"]


def test_a_rerun_in_progress_keeps_the_count(home: Path, project: Path) -> None:
    lane = review_lane(home, project, "tests-failed", NOW, FAIL)
    state, fake = {}, Fake()

    def diff(repo, lane_id):
        return ["README.md"]

    check(home, state, at(10), fake, lanes=[lane], diff=diff)
    lane = review_lane(home, project, "running", at(20), pid=os.getpid())
    check(home, state, at(30), fake, lanes=[lane], diff=diff)
    lane = review_lane(home, project, "tests-failed", at(40), FAIL)
    check(home, state, at(300), fake, lanes=[lane], diff=diff)
    assert len(fake.spawned) == 1 and level(state, "reviews") == "failed"


@pytest.mark.parametrize("files", [["collector/tests/test_billing.py"], ["collector/tirith/billing.py"],
                                   ["web/js/billing.js"]])
def test_failures_in_what_the_lane_changed_are_left_alone(home: Path, project: Path, files: list[str]) -> None:
    lane = review_lane(home, project, "tests-failed", NOW, FAIL)
    fake, state = Fake(), {}
    check(home, state, at(10), fake, lanes=[lane], diff=lambda repo, lane_id: files)
    assert fake.spawned == [] and level(state, "reviews") == "ok"


def test_unrelated_needs_a_named_test() -> None:
    assert selfcheck_mod.unrelated(FAIL, ["a.py"])
    assert not selfcheck_mod.unrelated("tests failed", ["a.py"])
    assert selfcheck_mod.unrelated("not ok web/js/chat.test.mjs", ["collector/x.py"])
    assert not selfcheck_mod.unrelated("not ok web/js/chat.test.mjs", ["web/js/chat.js"])


# -- the dispatcher and the state ---------------------------------------------------------------


def test_dispatch_runs_it_and_the_state_has_plain_words(home: Path, project: Path) -> None:
    from tirith.collector import Collector
    dispatch_mod.run(home, load_config(home), NOW, agents=[], lanes=[], status_of=lambda l, n: "working",
                     manager_spawn=lambda *a: 1, free=lambda folder: 500 * 1024 ** 3)
    assert dispatch_mod.load_state(home)["health"]["level"] == "green"
    health = Collector(home, load_config(home)).build_state(NOW)["health"]
    assert health["level"] in ("green", "red")  # the real disk of the machine running the test decides
    assert [c["id"] for c in health["checks"]][0] == "collector" and health["checks"][0]["label"] == "Tirith itself"
