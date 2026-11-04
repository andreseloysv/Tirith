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
          "report": ["3 items reached main today (target 10)."]}


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
    enable(home)
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
