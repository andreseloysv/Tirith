"""The CEO (tirith.ceo): typed result, delivery to the manager and the owner, records, schedule, read-only lanes."""

from __future__ import annotations

import json
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest

from tirith import ceo
from tirith import gates, lanegates
from tirith import inbox as inbox_mod
from tirith.home import Config, ProjectConfig, load_config
from tirith.store import read_jsonl, write_lane

RESULT = {"result": "directives",
          "findings": [{"id": "parked-steps", "change": "new", "evidence": "9 steps answered wait", "check": "waits = 0"}],
          "directives": [{"id": "d-1", "text": "Give every wait a deadline", "why": "steps parked for days",
                          "expect": "waits older than 2 h = 0", "check": "count", "runs": 2}],
          "proposals": [{"id": "p-1", "text": "Count only Tirith's CPU in the load limit", "why": "other apps"}],
          "report": ["3 items reached main today (target 10)."],
          "bottleneck": {"what": "steps wait for the owner", "evidence": "9 waits", "action": "d-1"}}


def local(hour: int, minute: int = 0, day: int = 4) -> datetime:
    return datetime(2026, 11, day, hour, minute).astimezone().astimezone(timezone.utc)


def enable(home: Path, extra: str = "") -> None:
    path = home / "config.toml"
    path.write_text((path.read_text() if path.exists() else "") + f"\n[ceo]\nenabled = true\n{extra}\n")


class Waits:
    def __init__(self) -> None:
        self.calls = []

    def __call__(self, home, project, text, **kw):
        self.calls.append((project, text, kw))
        return f"w-{len(self.calls)}"


# -- the result ------------------------------------------------------------------------------------


def test_the_result_is_one_checked_json_object() -> None:
    data = ceo.parse_result("Here it is:\n" + json.dumps(RESULT) + "\n")
    assert data["result"] == "directives" and data["report"] == ["3 items reached main today (target 10)."]
    for bad, why in [("no json", "no JSON"), ('{"result": "great"}', "result must be"),
                     ('{"result": "healthy", "findings": {}}', "must be a list"),
                     ('{"result": "healthy", "directives": [{"text": "x"}]}', "needs text, why")]:
        with pytest.raises(ceo.ResultError, match=why):
            ceo.parse_result(bad)


def test_directives_go_to_the_manager_and_proposals_to_the_owner(home: Path) -> None:
    waits = Waits()
    sent = ceo.deliver(home, ceo.parse_result(json.dumps(RESULT)), "ceo-1", "tirith", local(14), add_wait=waits)
    assert sent == {"directives": ["d-1"], "proposals": ["p-1"]}
    (req,) = [r for r in inbox_mod.requests(home) if r["kind"] == "directive"]
    assert req["from"] == "ceo" and req["manager_pending"] and "Give every wait a deadline" in req["text"]
    assert "done (what you changed), cannot (why) or disagree (why)" in req["text"]
    (call,) = waits.calls
    assert call[2]["why_owner"] == "owner-decision" and call[2]["options"] == ceo.OPTIONS
    assert ceo.findings(home)["parked-steps"]["status"] == "open"
    assert ceo.directives(home)["d-1"]["status"] == "open" and ceo.decisions(home)["p-1"]["status"] == "proposed"


def test_at_most_three_directives_are_open_and_a_rejected_proposal_is_not_asked_again(home: Path) -> None:
    waits = Waits()
    many = {**RESULT, "directives": [{"id": f"d-{n}", "text": f"t{n}", "why": "w"} for n in range(5)]}
    assert len(ceo.deliver(home, many, "ceo-1", "tirith", local(9), add_wait=waits)["directives"]) == 3
    assert ceo.deliver(home, many, "ceo-2", "tirith", local(14), add_wait=waits) == {"directives": [], "proposals": []}
    inbox_mod.add_reply(home, "w-1", "tirith", "decline", "not now, Godot is needed")
    assert ceo.settle_decisions(home)
    assert ceo.decisions(home)["p-1"]["status"] == "rejected"
    assert ceo.decisions(home)["p-1"]["owner_text"] == "not now, Godot is needed"
    ceo.deliver(home, RESULT, "ceo-3", "tirith", local(20), add_wait=waits)
    assert len(waits.calls) == 1  # the owner said no: not asked again


def test_the_managers_answer_settles_a_directive(home: Path) -> None:
    ceo.deliver(home, RESULT, "ceo-1", "tirith", local(9), add_wait=Waits())
    req = next(r for r in inbox_mod.requests(home) if r["kind"] == "directive")
    assert not ceo.settle_directives(home)
    inbox_mod.add_answer(home, req["id"], "done: every wait now says until when")
    assert ceo.settle_directives(home)
    assert ceo.directives(home)["d-1"]["status"] == "answered"
    assert ceo.directives(home)["d-1"]["answer"] == "done: every wait now says until when"


# -- the schedule ------------------------------------------------------------------------------------


@pytest.fixture
def started(home: Path, monkeypatch) -> list:
    write_lane_config(home)
    runs: list = []

    def fake_start(home, config, now, snap, cfg, catch_up, spawn=None):
        runs.append({"catch_up": catch_up, "snap": snap})
        return f"ceo-{len(runs)}"
    monkeypatch.setattr(ceo, "start", fake_start)
    monkeypatch.setattr(ceo, "snapshot", lambda home, config, now: {"at": now.isoformat(), "merged_today": 0})
    return runs


def write_lane_config(home: Path) -> None:
    (home / "config.toml").write_text("", encoding="utf-8")


def test_off_by_default_then_one_review_per_scheduled_hour(home: Path, started: list) -> None:
    config = load_config(home)
    assert not ceo.run(home, config, local(9, 5)) and started == []  # [ceo] enabled is not set
    enable(home, "hours = [9, 14, 20]")
    assert not ceo.run(home, config, local(8, 59))  # before the first hour
    assert ceo.run(home, config, local(9, 5)) and len(started) == 1
    assert not ceo.run(home, config, local(9, 40))  # same slot
    write_lane(home, {"id": "ceo-1", "ended_at": "x", "report": "no json", "project": "tirith"})
    ceo.run(home, config, local(10))
    assert [r["status"] for r in read_jsonl(home / "ceo" / "runs.jsonl")] == ["started", "failed"]
    assert ceo.run(home, config, local(14, 1))
    assert read_jsonl(home / "ceo" / "runs.jsonl")[-1]["status"] == "skipped"  # nothing changed: no cost
    assert len(started) == 1


def test_an_open_directive_or_a_changed_snapshot_is_worth_a_review(home: Path, started: list, monkeypatch) -> None:
    enable(home)
    config = load_config(home)
    ceo.run(home, config, local(9, 1))
    ceo.deliver(home, RESULT, "ceo-1", "tirith", local(9, 30), add_wait=Waits())
    ceo.run(home, config, local(14, 1))
    assert len(started) == 2  # same numbers, but a directive is open
    monkeypatch.setattr(ceo, "snapshot", lambda home, config, now: {"merged_today": 4})
    ceo.run(home, config, local(20, 1))
    assert len(started) == 3


def test_the_daily_budget_and_the_catch_up_after_downtime(home: Path, started: list, monkeypatch) -> None:
    enable(home, "hours = [9, 10, 11]\nruns_per_day = 2")
    config = load_config(home)
    n = iter(range(100))
    monkeypatch.setattr(ceo, "snapshot", lambda home, config, now: {"tick": next(n)})
    for hour in (9, 10, 11):
        write_lane(home, {"id": f"ceo-{len(started)}", "ended_at": "x", "report": json.dumps(RESULT)}) if started else None
        ceo.run(home, config, local(hour, 1))
    assert len(started) == 2 and "budget of 2" in read_jsonl(home / "ceo" / "runs.jsonl")[-1]["why"]
    ceo.run(home, config, local(9, 1, day=6))
    assert started[-1]["catch_up"]  # more than a day since the last review


def test_a_finished_review_is_settled_once_and_shown(home: Path, started: list, monkeypatch) -> None:
    enable(home)
    config = load_config(home)
    monkeypatch.setattr(ceo, "deliver", lambda home, data, lane, project, now, add_wait=None: {"directives": ["d-1"], "proposals": []})
    write_lane(home, {"id": "ceo-aa", "ended_at": "x", "report": "```json\n" + json.dumps(RESULT) + "\n```",
                      "project": "tirith"})
    ceo.run(home, config, local(8))
    ceo.run(home, config, local(8, 30))
    done = [r for r in read_jsonl(home / "ceo" / "runs.jsonl") if r["status"] == "done"]
    assert len(done) == 1 and done[0]["result"] == "directives" and done[0]["directives"] == ["d-1"]
    shown = ceo.public(home, local(8, 31))
    assert shown["enabled"] and shown["last_result"] == "directives" and shown["report"] == RESULT["report"]
    assert shown["next_at"] and not shown["running"]


# -- read-only lanes -----------------------------------------------------------------------------------


def _ceo_rules(tmp_path: Path, tool: str, tmpdir: str = "/tmp", **data) -> list[str]:
    repo = tmp_path / "proj"
    (repo / ".git").mkdir(parents=True, exist_ok=True)
    cfg = Config(projects=[ProjectConfig(id="proj", name="proj", repo=repo)])
    return [v.rule for v in gates.check_call(cfg, tool, data, str(repo), "proj", lane="ceo-1234",
                                             env={"TMPDIR": tmpdir, "HOME": "/home/x"})]


@pytest.mark.parametrize("tool, data", [
    ("Edit", {"file_path": "README.md", "old_string": "a", "new_string": "b"}),
    ("Bash", {"command": "git commit -am x"}), ("Bash", {"command": "git -C /x/proj merge lane/a"}),
    ("Bash", {"command": "git push origin main"}), ("Bash", {"command": "tirith lane stop a --reason x"}),
    ("Bash", {"command": "tirith inbox answer in-1 ok"}), ("Bash", {"command": "cd /x && tirith plan edit"}),
])
def test_the_ceo_changes_nothing(tmp_path: Path, tool: str, data: dict) -> None:
    assert "ceo-read-only" in _ceo_rules(tmp_path, tool, **data)


@pytest.mark.parametrize("command", ["git log --oneline -5", "git -C /x status", "git diff HEAD~1",
                                     "tirith lane stats --days 7", "tirith status", "tirith inbox list --open",
                                     "cat ~/.tirith/dispatch.json", "grep -r wait ~/.tirith/inbox.jsonl"])
def test_the_ceo_reads(tmp_path: Path, command: str) -> None:
    assert _ceo_rules(tmp_path, "Bash", command=command) == []


def test_the_ceo_cannot_write_into_the_project(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setattr(lanegates, "TEMP_ROOTS", ())  # the test folders live in the temp area
    monkeypatch.setattr(lanegates.tempfile, "gettempdir", lambda: "/nonexistent-tmp")
    target = tmp_path / "proj" / "a.txt"
    assert "workspace-write" in _ceo_rules(tmp_path, "Bash", tmpdir="/nonexistent-tmp", command=f"echo x > {target}")
    assert lanegates.ceo_read_only("Read", {"file_path": "x"}, None, {}) == []


def test_a_review_started_moments_ago_blocks_a_second_one(home: Path, started: list) -> None:
    """`tirith ceo run` and the collector's pass once started two reviews in the same second."""
    enable(home)
    config = load_config(home)
    assert ceo.run(home, config, local(14, 49), force=True) and len(started) == 1
    assert not ceo.run(home, config, local(14, 49))  # its lane file is not there yet: it counts as running
    assert not ceo.run(home, config, local(14, 50), force=True)
    assert len(started) == 1
    assert ceo.run(home, config, local(15, 5), force=True) and len(started) == 2  # after STARTING_S


def test_one_ceo_pass_at_a_time(home: Path, started: list) -> None:
    enable(home)
    with ceo._pass_lock(home) as mine:
        assert mine
        assert not ceo.run(home, load_config(home), local(14, 1), force=True)
    assert len(started) == 0


def test_a_directive_id_used_before_does_not_overwrite_the_earlier_one(home: Path) -> None:
    ceo.deliver(home, RESULT, "ceo-aaaa", "tirith", local(9), add_wait=Waits())
    ceo.deliver(home, {**RESULT, "directives": [{"id": "d-1", "text": "Another", "why": "w"}]}, "ceo-bbbb", "tirith",
                local(14), add_wait=Waits())
    assert ceo.directives(home)["d-1"]["text"] == "Give every wait a deadline"
    assert ceo.directives(home)["d-1-bbbb"]["text"] == "Another"


def red(home: Path, text: str | None) -> None:
    checks = {"reviews": {"level": "failed", "text": text}} if text else {"reviews": {"level": "ok", "text": "fine"}}
    (home / "dispatch.json").write_text(json.dumps({"health": {"checks": checks}}), encoding="utf-8")


def test_a_red_self_check_wakes_the_ceo_once(home: Path, started: list) -> None:
    enable(home, "hours = [9, 14, 20]")
    config = load_config(home)
    red(home, "Tests fail on code lane l-1 did not touch.")
    assert ceo.run(home, config, local(11)) and len(started) == 1  # out of schedule
    assert started[0]["snap"]["alarm"] == {"reviews": "Tests fail on code lane l-1 did not touch."}
    assert "## Alarm" in ceo.brief(home, started[0]["snap"], ceo.settings(home), False)
    assert "## Alarm" not in ceo.brief(home, {"merged_today": 0}, ceo.settings(home), False)
    write_lane(home, {"id": "ceo-1", "ended_at": "x", "report": json.dumps(RESULT), "project": "tirith"})
    ceo.run(home, config, local(11, 30))
    assert len(started) == 1  # the same alarm is not raised twice
    red(home, None)
    ceo.run(home, config, local(12))
    red(home, "Tests fail on code lane l-1 did not touch.")
    ceo.run(home, config, local(12, 30))
    assert len(started) == 2  # it went green and red again


def test_ceo_alarm_storm_same_text_is_throttled(home: Path, started: list) -> None:
    enable(home, "hours = [9, 14, 20]")
    config = load_config(home)
    text = "Tests fail on code lane l-1 did not touch."
    red(home, text)
    assert ceo.run(home, config, local(11)) and len(started) == 1
    write_lane(home, {"id": "ceo-1", "ended_at": "x", "project": "tirith",
                      "report": json.dumps({**RESULT, "next_review_min": 60})})
    for minute in (3, 6, 9, 12, 15):  # flapping green/red inside 20 min and before next_review_min
        red(home, None)
        ceo.run(home, config, local(11, minute))
        red(home, text)
        ceo.run(home, config, local(11, minute + 1))
    assert len(started) == 1
    red(home, None)
    ceo.run(home, config, local(11, 40))
    red(home, text)
    ceo.run(home, config, local(11, 45))  # 45 min since the alarm, but the review chose 60
    assert len(started) == 1
    red(home, "A different failure.")  # new text goes through
    ceo.run(home, config, local(11, 50))
    assert len(started) == 2


def test_ceo_same_alarm_no_rerun_after_a_run_that_sent_a_directive(home: Path, started: list) -> None:
    enable(home, "hours = [9, 14, 20]")
    config = load_config(home)
    text = "Disk is almost full."
    red(home, text)
    assert ceo.run(home, config, local(11)) and len(started) == 1
    # the review runs 40 min (longer than the 20 min limit), flaps green during it, and ends with a directive
    red(home, None)
    ceo.run(home, config, local(11, 20))
    red(home, text)
    write_lane(home, {"id": "ceo-1", "ended_at": "x", "project": "tirith",
                      "report": json.dumps({**RESULT, "next_review_min": 10})})
    assert ceo.run(home, config, local(11, 40)) is True  # settles the run
    assert len(started) == 1  # the run just ended: no new one at once
    red(home, None)
    ceo.run(home, config, local(11, 41))
    red(home, text)
    ceo.run(home, config, local(11, 42))
    ceo.run(home, config, local(11, 59))
    assert len(started) == 1  # nor within 20 min of its end
    ceo.run(home, config, local(12, 1))
    assert len(started) == 2  # after 20 min the same alarm may wake it again
    write_lane(home, {"id": "ceo-2", "ended_at": "x", "project": "tirith",
                      "report": json.dumps({**RESULT, "next_review_min": 10})})
    ceo.run(home, config, local(12, 30))
    red(home, "A different failure.")
    ceo.run(home, config, local(12, 31))
    assert len(started) == 3  # new text starts one


def test_the_ceo_picks_when_it_looks_again(home: Path, started: list, monkeypatch) -> None:
    enable(home)
    assert not ceo.settings(home)["fixed"]  # no [ceo] hours: its own interval, an hour until it picks one
    config = load_config(home)
    n = iter(range(100))
    monkeypatch.setattr(ceo, "snapshot", lambda home, config, now: {"tick": next(n)})
    assert ceo.run(home, config, local(1, 0)) and len(started) == 1
    assert not ceo.run(home, config, local(1, 30))  # running
    write_lane(home, {"id": "ceo-1", "ended_at": "x", "report": json.dumps({**RESULT, "next_review_min": 15})})
    ceo.run(home, config, local(1, 40))  # settled: it asked to look again in 15 min
    assert len(started) == 1 and ceo.public(home, local(1, 40))["every_min"] == 15
    ceo.run(home, config, local(1, 50))
    assert len(started) == 1
    ceo.run(home, config, local(1, 56))
    assert len(started) == 2
    assert [ceo.interval(v) for v in (5, 9999, None, "x", 120)] == [10, 480, 60, 60, 120]  # 10 min to 8 h
    assert "next_review_min" in ceo.brief(home, {"merged_today": 0}, ceo.settings(home), False)


def _pipeline(project: str, workflow: str, status: str, at: str) -> dict:
    return {"run": f"github:o/r:{workflow}#1", "project": project, "pipeline": workflow, "status": status,
            "source": "github", "started_at": at}


def test_the_snapshot_holds_what_the_hourly_check_looks_at(home: Path) -> None:
    now = local(5)
    iso = lambda dt: dt.strftime("%Y-%m-%dT%H:%M:%SZ")  # noqa: E731
    (home / "state.json").write_text(json.dumps({
        "autopilot": {"open_items": 7},
        "waiting_on_you": [{"id": "w-1", "project": "other"}],
        "pipelines": [_pipeline("tirith", "tirith", "failed", "2026-11-04T03:00:00Z"),
                      _pipeline("tirith", "tirith", "passed", "2026-11-04T02:00:00Z"),
                      _pipeline("tirith", "pages", "running", "2026-11-04T04:00:00Z")]}))
    write_lane(home, {"id": "l-1", "project": "tirith", "ended_at": iso(now - timedelta(minutes=90)),
                      "review": {"status": "merged", "at": iso(now - timedelta(minutes=80))}})
    (home / "testmon" / "tirith").mkdir(parents=True)
    (home / "testmon" / "agentsmessenger").mkdir(parents=True)
    (home / "testmon" / "tirith" / ".testmondata").write_bytes(b"map")
    proposal = inbox_mod.add_request(home, inbox_mod.PROPOSAL, "tirith", "17", "Autopilot: …", at=iso(now - timedelta(hours=1)),
                                     sender="tirith")
    inbox_mod.add_answer(home, proposal["id"], "Autopilot: step 17 → wait: the cleanup waits on the owner's decision.",
                         at=iso(now - timedelta(minutes=50)))
    plan = inbox_mod.add_request(home, "plan-refresh", "tirith", "plan", "", at=iso(now - timedelta(hours=1)))
    inbox_mod.add_answer(home, plan["id"], "Could not start an agent for \"Update the plan\": permanent task IDs must "
                         "be unique", by="tirith", at=iso(now - timedelta(minutes=40)))
    snap = ceo.snapshot(home, load_config(home), now)
    assert snap["open_items"] == 7 and snap["idle_min"] == 90 and snap["last_merge_min"] == 80
    assert snap["ci_main"] == {"tirith/tirith": "failed"}  # the newest finished run of each workflow
    (home / "ci-main.json").write_text(json.dumps({"workflows": {"tirith/tirith": {"status": "passed"},
                                                                 "tirith/pages": {"status": "passed"}}}))
    assert ceo.snapshot(home, load_config(home), now)["ci_main"] == {"tirith/pages": "passed", "tirith/tirith": "passed"}
    (home / "ci-main.json").unlink()  # the collector's file wins: it saw every run, state.json keeps the newest
    assert snap["test_maps"]["tirith"]["present"] and snap["test_maps"]["agentsmessenger"] == {"present": False, "age_h": None}
    assert snap["launch_failures"]["count"] == 1 and "must be unique" in snap["launch_failures"]["last"][0]
    assert [w["step"] for w in snap["owner_waits_unasked"]] == ["17"]  # nothing of tirith is on the owner's list
    later = {**snap, "idle_min": 110, "last_merge_min": 100, "at": "x",
             "test_maps": {**snap["test_maps"], "tirith": {"present": True, "age_h": 9.9}}}
    assert ceo.fingerprint(later) == ceo.fingerprint(snap)  # the clock alone is no news
    assert ceo.fingerprint({**snap, "idle_min": 150}) != ceo.fingerprint(snap)  # a new idle hour is
    assert "## First: is Tirith working?" in ceo.brief(home, snap, ceo.settings(home), False)
    assert "needs no owner approval" in ceo.brief(home, snap, ceo.settings(home), False)


def test_a_check_that_stays_amber_wakes_the_ceo_after_code_had_its_chance(home: Path, started: list) -> None:
    # "The autopilot: some plan items have nobody on them" stayed healing for hours and the CEO never looked
    enable(home, "hours = [23]")  # no scheduled review in the way
    config = load_config(home)
    amber = {"autopilot": {"level": "healing", "text": "Some plan items have nobody on them."}}
    (home / "dispatch.json").write_text(json.dumps({"health": {"checks": amber}}), encoding="utf-8")
    ceo.run(home, config, local(10))
    ceo.run(home, config, local(10, ceo.UNWELL_ALARM_MIN - 1))
    assert started == []  # code may still heal it
    ceo.run(home, config, local(10, ceo.UNWELL_ALARM_MIN + 1))
    assert len(started) == 1 and started[0]["snap"]["alarm"] == {"autopilot": "Some plan items have nobody on them."}
    (home / "dispatch.json").write_text(json.dumps({"health": {"checks": {"autopilot": {"level": "ok"}}}}))
    ceo.run(home, config, local(11))
    assert "autopilot" in json.loads((home / "ceo" / "book.json").read_text())["unwell"]  # ok a moment: still counted
    ceo.run(home, config, local(11, ceo.UNWELL_FORGET_MIN + 1))
    assert "autopilot" not in json.loads((home / "ceo" / "book.json").read_text()).get("unwell", {})  # healed: forgotten


def test_a_check_that_comes_and_goes_still_wakes_the_ceo(home: Path, started: list) -> None:
    # "Some plan items have nobody on them" turned ok for a moment now and then, and each time the clock restarted
    enable(home, "hours = [23]")
    config = load_config(home)
    path = home / "dispatch.json"
    amber = json.dumps({"health": {"checks": {"autopilot": {"level": "healing", "text": "Nobody on them."}}}})
    green = json.dumps({"health": {"checks": {"autopilot": {"level": "ok"}}}})
    for minute, text in ((0, amber), (8, green), (9, amber), (15, green), (16, amber), (21, amber)):
        path.write_text(text)
        ceo.run(home, config, local(10, minute))
    assert len(started) == 1 and started[0]["snap"]["alarm"] == {"autopilot": "Nobody on them."}


def test_the_ceo_sets_the_capacity_within_the_owners_range(home: Path) -> None:
    (home / "config.toml").write_text("[ceo]\nenabled = true\nmax_parallel_range = [2, 6]\n\n[autopilot]\nmax_parallel = 4\n")
    sent = ceo.deliver(home, {**RESULT, "proposals": [], "capacity": {"max_parallel": 9, "why": "CPU idle"}},
                       "ceo-1", "tirith", local(10), add_wait=Waits())
    assert sent["capacity"] == 6 and load_config(home).autopilot_max_parallel == 6  # clamped to the owner's range
    assert ceo.decisions(home)["capacity-ceo-1"]["text"] == "max_parallel = 6"
    assert "capacity" not in ceo.deliver(home, {**RESULT, "proposals": [], "directives": [], "capacity": {"max_parallel": 6}},
                                         "ceo-2", "tirith", local(11), add_wait=Waits())  # already so: nothing to do


def test_the_ceo_sees_how_fast_the_work_goes(home: Path) -> None:
    now = local(12)
    iso = lambda dt: dt.strftime("%Y-%m-%dT%H:%M:%SZ")  # noqa: E731
    log = home / "logs" / "ci-1-test.log"
    log.parent.mkdir(parents=True)
    log.write_text("FAILED tests/test_x.py::test_y - boom\n1 failed, 1700 passed in 1140.0s (0:19:00)\n")
    rows = []
    for n in range(3):
        write_lane(home, {"id": f"l-{n}", "project": "tirith", "step": "17", "feature": "Upkeep",
                          "started_at": iso(now - timedelta(minutes=60)), "ended_at": iso(now - timedelta(minutes=50)),
                          "review": {"status": "merged", "at": iso(now - timedelta(minutes=40))}})
        rows.append(json.dumps({"at": iso(now - timedelta(minutes=55)), "lane": f"l-{n}", "stage": "test", "event": "end",
                                "log": str(log)}))
    (home / "events.jsonl").write_text("\n".join(rows) + "\n")
    t = ceo.throughput(home, load_config(home), now, cpu=lambda: 60.0)
    assert t["merged_today"] == 3 and t["merged_by_step"] == {"tirith step 17: Upkeep": 3}
    assert (t["lane_minutes_median"], t["start_to_main_minutes_median"]) == (10, 20)
    assert (t["whole_suites_in_lanes"], t["whole_suite_minutes_median"]) == (3, 19.0)
    assert t["red_in_many_lanes"] == ["tests/test_x.py::test_y"] and t["cpu_idle_pct"] == 60.0
    assert "## Throughput: is enough getting done?" in ceo.brief(home, {"throughput": t}, ceo.settings(home), False)
    assert "upkeep_share_today" not in t  # no upkeep steps configured: no share


def test_the_ceo_sees_the_share_of_upkeep(home: Path) -> None:
    now = local(12)
    iso = lambda dt: dt.strftime("%Y-%m-%dT%H:%M:%SZ")  # noqa: E731
    with (home / "config.toml").open("a", encoding="utf-8") as handle:
        handle.write('\n[ceo]\nupkeep_steps = ["tirith:17"]\nupkeep_max_share = 0.25\n')
    for n, step in enumerate(("17", "17", "14", "22")):
        write_lane(home, {"id": f"l-{n}", "project": "tirith", "step": step, "feature": "x",
                          "started_at": iso(now - timedelta(minutes=60))})
    t = ceo.throughput(home, load_config(home), now, cpu=lambda: 60.0)
    assert (t["upkeep_lanes_today"], t["upkeep_share_today"], t["upkeep_max_share"]) == (2, 0.5, 0.25)
    assert "crowded out" in ceo.brief(home, {"throughput": t}, ceo.settings(home), False)


def test_every_review_names_the_biggest_bottleneck() -> None:
    neck = {"what": "lanes re-run the whole suite", "evidence": "148 suites, 325 min", "action": "d-9"}
    assert ceo.parse_result(json.dumps({**RESULT, "bottleneck": neck}))["bottleneck"] == neck
    data = ceo.parse_result(json.dumps({k: v for k, v in RESULT.items() if k != "bottleneck"}))  # it says so
    assert data["bottleneck"] is None and "No bottleneck named" in data["report"][-1]


def test_the_owner_can_change_a_decision_on_a_closed_proposal(home: Path, monkeypatch) -> None:
    monkeypatch.delenv("TIRITH_LANE", raising=False)
    ceo.deliver(home, RESULT, "ceo-1", "tirith", local(9), add_wait=Waits())
    assert ceo.owner_decides(home, "p-1", "decline")["status"] == "rejected"
    assert ceo.owner_decides(home, "p-1", "approve", "CPU is idle")["status"] == "approved"
    assert ceo.decisions(home)["p-1"]["status"] == "approved"  # the last row of an id wins
    monkeypatch.setenv("TIRITH_LANE", "lane-a")
    with pytest.raises(ValueError, match="only the owner"):
        ceo.owner_decides(home, "p-1", "decline")


PROCESS = {"now": "one item per lane, whole suite per merge", "alternative": "trains of 3 lanes per merge",
           "evidence": "40 merges, 4 min suite each", "gain": "about 100 min of suite a day", "change": "d-7"}


def test_once_a_day_the_ceo_must_question_the_way_of_working(home: Path, monkeypatch) -> None:
    assert ceo.process_due(home, local(9))  # never asked yet
    assert "## Is this the best way to work?" in ceo.brief(home, {"process_review_due": True}, ceo.settings(home), False)
    assert "best way to work" not in ceo.brief(home, {"process_review_due": False}, ceo.settings(home), False)
    assert ceo.parse_result(json.dumps(RESULT))["process"] is None
    assert ceo.parse_result(json.dumps({**RESULT, "process": {"now": "x"}}))["process"] is None  # half an answer
    assert ceo.parse_result(json.dumps({**RESULT, "process": PROCESS}))["process"] == PROCESS
    # a settled review that answered it: not due again for a day, and the owner sees the answer
    book: dict = {}
    write_lane(home, {"id": "ceo-1", "ended_at": "x", "report": json.dumps({**RESULT, "process": PROCESS}),
                      "project": "tirith"})
    monkeypatch.setattr(ceo, "deliver", lambda *a, **k: {})
    assert ceo._settle_lanes(home, book, local(9))
    ceo._save_book(home, book)
    assert not ceo.process_due(home, local(20)) and ceo.process_due(home, local(9, 1, day=5))
    assert ceo.public(home, local(10))["process"] == PROCESS
