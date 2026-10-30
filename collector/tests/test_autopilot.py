"""The autopilot: open plan items nobody works on become proposals to the manager; owner items; the switch."""

from __future__ import annotations

import os
from datetime import date, datetime, timedelta, timezone
from pathlib import Path

import pytest

from conftest import write_config
from tirith import tirithcmd
from tirith import asks as asks_mod
from tirith import autopilot as autopilot_mod
from tirith import dispatch as dispatch_mod
from tirith import inbox as inbox_mod
from tirith import roadmap as roadmap_mod
from tirith.home import load_config
from tirith.store import read_json, read_jsonl

NOW = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)
TODAY = date(2026, 10, 29)
PLAN = """# TODO

## 1. First

- [ ] Build the first thing.

## 2. Second

- [ ] Build the second thing.
- [ ] Log in to the portal (dueño: credentials https://portal.example.invalid/login).

## 3. Third

- [ ] Build the third thing.

## 4. Owner only

- [ ] Install it (dueño: physical-device) `tirith url install`.
- [ ] Decide the colour (dueño: owner-decision).

## 5. Later

- [ ] Check the quota (desde 2026-11-02).
"""


@pytest.fixture
def repo(home: Path, tmp_path: Path) -> Path:
    folder = tmp_path / "demo"
    (folder / "docs").mkdir(parents=True)
    (folder / "docs" / "TODO.md").write_text(PLAN, encoding="utf-8")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(folder)}])
    return folder


class Pilot:
    """Drives ``dispatch.run`` like the collector does: the plan is re-read every cycle."""

    def __init__(self, home: Path, repo: Path, extra: str = "") -> None:
        self.home, self.repo, self.launched = home, repo, []
        self.agents: list[dict] = []
        self.now = NOW
        if extra:
            path = home / "config.toml"
            path.write_text(path.read_text() + "\n" + extra + "\n")

    def launcher(self, home, project, step, request_id=None) -> str:
        self.launched.append((project, step))
        lane = f"lane-{len(self.launched)}"
        self.agents.append({"id": lane, "project": project, "step": step, "status": "working"})
        return lane

    def projects(self) -> list[dict]:
        road = roadmap_mod.build(self.repo, "demo", self.agents, TODAY)
        return [{"id": "demo", "name": "Demo", "folder": str(self.repo), "roadmap": road}]

    def cycle(self, seconds: float = 31) -> None:
        self.now += timedelta(seconds=seconds)
        dispatch_mod.run(self.home, load_config(self.home), self.now, agents=self.agents, lanes=[],
                         status_of=lambda lane, now: "working", launcher=self.launcher,
                         projects=self.projects())

    def end(self, lane: str, review: dict | None = None, report: str | None = None) -> None:
        row = next(a for a in self.agents if a["id"] == lane)
        row["status"] = "done"
        if report:
            row["report"] = report
        if review:
            row["review"] = review

    def tick(self, text: str) -> None:
        path = self.repo / "docs" / "TODO.md"
        path.write_text(path.read_text().replace(f"- [ ] {text}", f"- [x] {text}"), encoding="utf-8")
        stat = path.stat()  # the plan is cached by mtime and size, and a tick keeps the size
        os.utime(path, ns=(stat.st_atime_ns, stat.st_mtime_ns + 10 ** 9))

    def state(self) -> dict:
        return dispatch_mod.load_state(self.home)["autopilot"]

    def state_all(self) -> dict:
        return dispatch_mod.load_state(self.home)


def _waits(home: Path) -> dict:
    return {r["id"]: r for r in read_jsonl(home / "waits.jsonl")}


# -- proposals instead of launches --------------------------------------------------------------


def _proposals(home: Path) -> list[dict]:
    return [r for r in inbox_mod.requests(home) if r["kind"] == "autopilot-proposal"]


def test_it_never_launches_it_proposes_once_per_item(home, repo) -> None:
    pilot = Pilot(home, repo)
    pilot.cycle()   # first sight: the debounce starts
    assert _proposals(home) == []
    pilot.cycle()
    got = _proposals(home)
    assert pilot.launched == []   # the launcher is never called
    assert sorted((r["project"], r["step"]) for r in got) == [("demo", "1"), ("demo", "2"), ("demo", "3")]
    assert all(r["from"] == "tirith" and r["manager_pending"] for r in got)
    first = next(r for r in got if r["step"] == "1")
    assert first["text"].startswith("Autopilot: demo step 1 (First): 1 item needs a decision.")
    assert "Build the first thing. [open, nobody on it]" in first["text"] and "TODO.md" in first["text"]
    assert len(first["text"]) <= autopilot_mod.MAX_TEXT
    for _ in range(3):
        pilot.cycle()
    assert len(_proposals(home)) == 3   # deduplicated: one per item


def test_a_proposal_is_flowed_and_counted(home, repo) -> None:
    pilot = Pilot(home, repo)
    pilot.cycle()
    pilot.cycle()
    flows = [f for f in read_jsonl(home / "flows.jsonl") if f["text"].startswith("Autopilot proposes")]
    assert len(flows) == 3 and flows[0]["from"] == "tirith" and flows[0]["to"] == "manager"
    got = autopilot_mod.public(home, load_config(home), pilot.agents, pilot.projects(), pilot.now)
    assert got["proposed"] == 3 and got["queued"] == 0 and got["started"] == []
    assert got["open_items"] == 3 and got["enabled"] is True


def test_the_debounce_ignores_a_normal_handoff(home, repo) -> None:
    pilot = Pilot(home, repo)
    pilot.cycle(0)
    for step in ("1", "2", "3"):
        pilot.agents.append({"id": f"lane-{step}", "project": "demo", "step": step, "status": "working"})
    pilot.cycle(10)
    for agent in pilot.agents:   # the lanes end and their reviews start at once
        agent["status"], agent["review"] = "done", {"status": "running"}
    pilot.cycle(10)
    pilot.cycle(10)
    assert _proposals(home) == []   # nobody sat unattended for 30 s


def test_a_changed_item_gets_a_new_proposal_and_the_old_one_is_closed(home, repo) -> None:
    pilot = Pilot(home, repo)
    pilot.cycle()
    pilot.cycle()
    old = next(r for r in _proposals(home) if r["step"] == "1")
    path = repo / "docs" / "TODO.md"
    path.write_text(path.read_text().replace("Build the first thing.", "Build the first thing. (reabierto: it broke)"))
    pilot.cycle()
    pilot.cycle()
    step1 = [r for r in _proposals(home) if r["step"] == "1"]
    assert len(step1) == 2 and step1[0]["id"] == old["id"] and step1[0]["status"] == "done"
    assert step1[0]["answers"] == []   # closing never speaks in the chat
    assert step1[1]["status"] == "open" and "reopened" in step1[1]["text"]
    assert "its facts changed" in next(f for f in read_jsonl(home / "flows.jsonl") if "closed its proposal" in f["text"])["text"]


def test_a_ticked_item_closes_its_unanswered_proposal(home, repo) -> None:
    pilot = Pilot(home, repo)
    pilot.cycle()
    pilot.cycle()
    pilot.tick("Build the first thing.")
    pilot.cycle()
    step1 = next(r for r in _proposals(home) if r["step"] == "1")
    assert step1["status"] == "done" and not step1["manager_pending"]
    assert step1["answers"] == []
    assert any("done or gone" in f["text"] for f in read_jsonl(home / "flows.jsonl"))
    assert len(pilot.state()["proposals"]) == 2
    assert [r["step"] for r in _proposals(home) if r["status"] != "done"] == ["2", "3"]


def test_the_managers_decision_answers_the_proposal_and_it_is_not_asked_again(home, repo) -> None:
    pilot = Pilot(home, repo)
    pilot.cycle()
    pilot.cycle()
    req = next(r for r in _proposals(home) if r["step"] == "1")
    inbox_mod.add_answer(home, req["id"], "Autopilot: step 1 item Build the first thing. → started lane lane-9")
    inbox_mod.mark_done(home, req["id"])
    done = next(r for r in _proposals(home) if r["step"] == "1")
    assert done["status"] == "done" and not done["manager_pending"] and done["answers"][0]["by"] == "manager"
    for _ in range(3):
        pilot.cycle()
    assert len([r for r in _proposals(home) if r["step"] == "1"]) == 1   # "wait" or "launched": not again


def test_the_facts_of_earlier_lanes_are_in_the_proposal(home, repo) -> None:
    pilot = Pilot(home, repo)
    path = repo / "docs" / "TODO.md"
    path.write_text(path.read_text().replace("Build the first thing.", "Build the first thing. (en revisión: lane-old)"))
    pilot.agents.append({"id": "lane-old", "project": "demo", "step": "1", "status": "done", "outcome": "done",
                         "ended_at": "2026-10-29T10:00:00Z", "report": "I built it and merged.\n\nMore.",
                         "review": {"status": "merged", "claims": {"ok": True}}})
    pilot.cycle()
    pilot.cycle()
    text = next(r for r in _proposals(home) if r["step"] == "1")["text"]
    assert "marked in review, no check running" in text
    assert "lane lane-old" in text and "branch merged" in text and "I built it and merged." in text
    assert "claims:" in text and "More." not in text


def test_every_kind_of_note_is_reported_once_nobody_is_on_it(home, repo) -> None:
    path = repo / "docs" / "TODO.md"
    text = path.read_text()
    for word, note in (("first", "(en curso: lane-q)"), ("second", "(reabierto: it broke)"), ("third", "(bloqueado: waiting)")):
        text = text.replace(f"- [ ] Build the {word} thing.", f"- [ ] Build the {word} thing. {note}")
    path.write_text(text)
    pilot = Pilot(home, repo)
    pilot.cycle()
    pilot.cycle()
    why = {r["step"]: r["text"] for r in _proposals(home)}
    assert "marked in progress" in why["1"] and "reopened" in why["2"] and "marked blocked" in why["3"]


def test_an_item_with_a_lane_working_or_a_review_running_is_not_reported(home, repo) -> None:
    pilot = Pilot(home, repo)
    pilot.agents.append({"id": "w", "project": "demo", "step": "1", "status": "working"})
    pilot.agents.append({"id": "r", "project": "demo", "step": "2", "status": "done", "review": {"status": "running"}})
    pilot.agents.append({"id": "i", "project": "demo", "step": "3", "status": "done",
                         "review": {"status": "merge-failed", "message": "the review was interrupted; run it again"}})
    for _ in range(3):
        pilot.cycle()
    assert _proposals(home) == []
    pilot.agents[1]["review"] = {"status": "merged"}   # merged, the item still open: now it sits
    pilot.cycle()
    pilot.cycle()
    assert [r["step"] for r in _proposals(home)] == ["2"]


def test_step_zero_and_owner_items_get_no_proposal(home, repo) -> None:
    path = repo / "docs" / "TODO.md"
    path.write_text("# TODO\n\n## 0. Tirith own work\n\n- [ ] Review something.\n\n## 1. Owner\n\n"
                    "- [ ] Decide (dueño: owner-decision).\n")
    pilot = Pilot(home, repo)
    for _ in range(3):
        pilot.cycle()
    assert _proposals(home) == []


# -- the switch ---------------------------------------------------------------------------------


def test_disabled_in_config_proposes_nothing(home, repo) -> None:
    pilot = Pilot(home, repo, "[autopilot]\nenabled = false")
    for _ in range(3):
        pilot.cycle()
    assert _proposals(home) == []
    assert autopilot_mod.public(home, load_config(home))["enabled"] is False


def test_owner_switch_off_and_on(home, repo) -> None:
    pilot = Pilot(home, repo)
    off = inbox_mod.add_request(home, "autopilot", "all", "autopilot", "off", at="2026-10-29T11:59:00Z")
    for _ in range(3):
        pilot.cycle()
    assert _proposals(home) == []
    assert autopilot_mod.public(home, load_config(home))["enabled"] is False
    assert {r["id"]: r for r in inbox_mod.requests(home)}[off["id"]]["status"] == "done"
    assert not inbox_mod.requests(home)[0]["manager_pending"]
    inbox_mod.add_request(home, "autopilot", "all", "autopilot", "on")
    pilot.cycle()
    pilot.cycle()
    assert len(_proposals(home)) == 3
    with pytest.raises(inbox_mod.InboxError):
        inbox_mod.add_request(home, "autopilot", "all", "autopilot", "maybe")


def test_the_switch_sticks_across_restarts_and_a_lost_state(home, repo) -> None:
    pilot = Pilot(home, repo)
    inbox_mod.add_request(home, "autopilot", "all", "autopilot", "off")
    pilot.cycle()
    assert read_json(home / "autopilot.json", {})["on"] is False
    (home / "dispatch.json").unlink()   # the state file is lost
    for _ in range(3):
        pilot.cycle()
    assert _proposals(home) == []
    assert autopilot_mod.public(home, load_config(home))["enabled"] is False
    stale = dispatch_mod.load_state(home)
    stale["autopilot"]["on"] = True   # a stale writer saved the old switch
    dispatch_mod.save_state(home, stale)
    pilot.cycle()
    assert autopilot_mod.public(home, load_config(home))["enabled"] is False


def test_an_old_switch_in_the_state_is_written_to_the_file(home) -> None:
    assert autopilot_mod._sync_switch(home, {"autopilot": {"on": False}}) is False
    assert read_json(home / "autopilot.json", {})["on"] is False


# -- selfcheck ----------------------------------------------------------------------------------


def test_items_sitting_unreported_are_listed(home, repo) -> None:
    pilot = Pilot(home, repo)
    pilot.cycle(0)
    state = dispatch_mod.load_state(home)
    assert len(autopilot_mod.sitting(home, state, pilot.projects(), pilot.agents, pilot.now)) == 3
    pilot.agents.append({"id": "w", "project": "demo", "step": "1", "status": "working"})
    assert len(autopilot_mod.sitting(home, state, pilot.projects(), pilot.agents, pilot.now)) == 2


def test_a_proposal_unanswered_for_ten_minutes_is_a_manager_health_problem(home, repo) -> None:
    from tirith import selfcheck as selfcheck_mod
    pilot = Pilot(home, repo)
    pilot.cycle()
    pilot.cycle()
    state = dispatch_mod.load_state(home)
    config = load_config(home)
    book = {"queued": {}, "tries": {}, "notices": {}}

    def check(at):
        return selfcheck_mod._autopilot(config, at, state, book, home, pilot.agents, pilot.projects(), set())
    assert check(pilot.now)["level"] == "ok"
    later = pilot.now + timedelta(seconds=selfcheck_mod.PROPOSAL_STUCK_S + 5)
    got = check(later)
    assert got["level"] == "healing" and "3 proposals" in got["text"]
    assert {r["kind"] for r in selfcheck_mod._chat_waiting(home, later)} == {"autopilot-proposal"}
    assert selfcheck_mod._chat_waiting(home, pilot.now) == []   # not yet overdue
    inbox_mod.add_answer(home, _proposals(home)[0]["id"], "Autopilot: waiting")
    assert len(selfcheck_mod._chat_waiting(home, later)) == 2


def test_an_item_nobody_reported_makes_the_autopilot_check_amber_and_queues_the_manager(home, repo) -> None:
    from tirith import selfcheck as selfcheck_mod
    pilot = Pilot(home, repo)
    pilot.cycle(0)
    state = dispatch_mod.load_state(home)
    config = load_config(home)
    book = {"queued": {}, "tries": {}, "notices": {}}
    state["autopilot"]["seen"] = {k: {**v, "at": "2026-10-29T11:50:00Z"} for k, v in state["autopilot"]["seen"].items()}
    got = selfcheck_mod._autopilot(config, pilot.now, state, book, home, pilot.agents, pilot.projects(), set())
    assert got["level"] == "healing"
    assert [q["kind"] for q in asks_mod.queue_items(home, only_open=True)] == ["selfcheck"]


def test_a_proposal_is_never_escalated_to_the_owner(home, repo) -> None:
    pilot = Pilot(home, repo)
    pilot.cycle()
    pilot.cycle()
    pilot.cycle(seconds=dispatch_mod.ESCALATE_AFTER_S + 60)
    assert dispatch_mod.load_state(home)["escalated"] == {}


# -- owner items --------------------------------------------------------------------------------


def test_owner_items_with_command_link_and_plain(home, repo) -> None:
    pilot = Pilot(home, repo)
    pilot.cycle()
    waits = _waits(home)
    owner = {k: v for k, v in waits.items() if k.startswith("owner-demo-")}
    assert len(owner) == 3
    by_text = {v["text"]: v for v in owner.values()}
    command = next(v for t, v in by_text.items() if "Install it" in t)
    assert command["actions"][0]["command"] == tirithcmd.resolved(home) + " url install" and command["actions"][0]["cwd"] == str(repo)
    link = next(v for t, v in by_text.items() if "Log in" in t)
    assert link["link"] == "https://portal.example.invalid/login" and link["options"][0]["label"] == "Done"
    plain = next(v for t, v in by_text.items() if "colour" in t)
    assert [o["label"] for o in plain["options"]] == ["Done", "Not now"]
    for wait in owner.values():
        assert asks_mod.has_call_to_action(wait)


def test_owner_items_are_deduplicated_and_closed_when_ticked(home, repo) -> None:
    pilot = Pilot(home, repo)
    pilot.cycle()
    pilot.cycle()
    ids = [r["id"] for r in read_jsonl(home / "waits.jsonl") if r["id"].startswith("owner-demo-")]
    assert len(ids) == 3
    pilot.tick("Decide the colour (dueño: owner-decision).")
    pilot.cycle()
    rows = [r for r in read_jsonl(home / "waits.jsonl") if r["id"].startswith("owner-demo-") and r.get("done_at")]
    assert len(rows) == 1 and "colour" in rows[0]["text"]
    pilot.cycle()
    assert len([r for r in read_jsonl(home / "waits.jsonl") if r["id"].startswith("owner-demo-")]) == 4
    assert len(pilot.state()["owner_items"]) == 2


# -- what the lane's report says (real reports of ~/.tirith/lanes, shortened) ---------------------------

ALREADY_BUILT = [
    "I changed no files, because the brief's one item, protecting the hooks so an agent can't remove them, "
    "was already built and merged by the earlier `protect` lane.\n\nThe tests pass: `tirith run --project "
    "tirith --stage test` gave 678 passed.",
    "I changed no files. The protection this brief asks for is already in the committed code, so there was "
    "nothing left to build.\n\n- **Settings are guarded.** `PreToolUse` blocks edits to an agent's settings file.",
]
CANNOT_DO = [
    "The brief's one task was \"test a real launch from the browser with the `tirith://` link\". I couldn't do "
    "that, so I added an automated test of the same path. The tests pass: 680 passed.\n\n**Not done: the real "
    "browser click.** The owner still has to run `tirith url install`.",
    "The brief's only item, a real launch from the browser with the `tirith://` link, is not done. The lane "
    "can't do it, so the owner has to. The tests pass: 727 passed.",
    "I can't read the help for either CLI, because both commands need approval that I don't have. I'll stop "
    "here and won't edit the contract or the presets on a guess.\n\n**What I did**\n- Made no file changes.",
]
NOT_FINISHED = [
    "I added `codex` and `copilot` presets to `lane run`, but I haven't run either agent, so the lane isn't "
    "done. The unit tests pass.\n\n**What changed**\n- `collector/tirith/agentrun.py`: new entries.",
]


def test_report_kinds() -> None:
    for report in ALREADY_BUILT:
        assert autopilot_mod.report_kind(report) == "done"
    for report in CANNOT_DO:
        assert autopilot_mod.report_kind(report) == "cannot"


def test_other_reports_are_stuck_with_their_summary() -> None:
    for report in NOT_FINISHED:
        assert autopilot_mod.report_kind(report) == "stuck"
        assert autopilot_mod.report_summary(report).startswith("I added `codex`")
    assert autopilot_mod.report_summary(None) == "The lane ended without a report."


def test_refresh_brief_is_limited_to_the_step() -> None:
    from tirith.planlane import refresh_brief
    assert "Limited to step 3" in refresh_brief("demo", "docs/TODO.md", "# TODO", [], [], Path("."), step="3")
    assert "Limited to step" not in refresh_brief("demo", "docs/TODO.md", "# TODO", [], [], Path("."))


# -- the header counts and why an item is not reported ----------------------------------------------


def _public(pilot) -> dict:
    return autopilot_mod.public(pilot.home, load_config(pilot.home), pilot.agents, pilot.projects(), pilot.now)


def test_header_says_what_is_true(home, repo) -> None:
    pilot = Pilot(home, repo)
    pilot.agents.append({"id": "w", "project": "demo", "step": "1", "status": "working"})
    pilot.agents.append({"id": "r", "project": "demo", "step": "2", "status": "done", "review": {"status": "running"}})
    pilot.cycle()
    pilot.cycle()
    got = _public(pilot)
    assert (got["working"], got["steps_working"], got["steps_in_review"]) == (1, 1, 1)
    assert got["proposed"] == 1 and got["open_items"] == 3   # only step 3 had nobody on it
    why = {s["step"]: s["why"] for s in got["skipped"]}
    assert why == {"1": "an agent is already on it",
                   "2": "the work of an earlier agent on it is being checked by Tirith"}


def test_a_review_never_waits_for_the_owner(home, repo) -> None:
    pilot = Pilot(home, repo)
    for status in ("pending", "running", "ready", "merge-failed"):
        pilot.agents[:] = [{"id": "old", "project": "demo", "step": "1", "status": "done", "review": {"status": status}}]
        why = {s["step"]: s["why"] for s in _public(pilot)["skipped"]}.get("1", "")
        assert "you" not in why, (status, why)
    pilot.agents[:] = [{"id": "old", "project": "demo", "step": "1", "status": "done",
                        "review": {"status": "merge-failed", "message": "the review was interrupted; run it again"}}]
    assert "being checked" in {s["step"]: s["why"] for s in _public(pilot)["skipped"]}["1"]
    pilot.agents[:] = [{"id": "old", "project": "demo", "step": "1", "status": "done", "review": {"status": "tests-failed"}}]
    assert _public(pilot)["skipped"] == []   # a failed review is the manager's: it is reported as a proposal


def test_a_step_with_only_owner_items_left_is_skipped() -> None:
    step = {"id": "9", "status": "ready", "items": [{"text": "Decide", "done": False, "who": "owner"}]}
    assert autopilot_mod._why_not(autopilot_mod._own({}), [], "demo", step, NOW) == "only the owner's items are left"


def test_state_json_carries_the_autopilot_and_item_ids(home, repo) -> None:
    from tirith.collector import Collector
    path = home / "config.toml"
    path.write_text(path.read_text() + "\n[autopilot]\nenabled = false\n")  # the test must not post proposals
    state = Collector(home, client=None).build_state(NOW)
    assert state["autopilot"]["enabled"] is False and state["autopilot"]["started"] == []
    road = state["projects"][0]["roadmap"]
    assert [s["status"] for s in road["steps"]] == ["ready", "ready", "ready", "blocked-owner", "blocked-date"]
    owner = [i for i in road["steps"][3]["items"] if i["who"] == "owner"]
    assert len(owner) == 2 and all(i["owner_item"] in _waits(home) for i in owner)
    assert len(read_json(home / "dispatch.json", {})["autopilot"]["owner_items"]) == 3


# -- one open proposal per step -------------------------------------------------------------------


def _open(home: Path) -> list[dict]:
    return [r for r in _proposals(home) if r["status"] != "done"]


def _two_items(repo: Path) -> None:
    path = repo / "docs" / "TODO.md"
    path.write_text(path.read_text().replace("- [ ] Build the first thing.",
                                             "- [ ] Build the first thing.\n- [ ] Build the other thing."))


def test_one_proposal_per_step_lists_every_item_in_under_800_characters(home, repo) -> None:
    _two_items(repo)
    pilot = Pilot(home, repo)
    pilot.cycle()
    pilot.cycle()
    step1 = [r for r in _open(home) if r["step"] == "1"]
    assert len(step1) == 1 and len(_open(home)) == 3
    assert "2 items need a decision" in step1[0]["text"]
    assert "Build the first thing." in step1[0]["text"] and "Build the other thing." in step1[0]["text"]
    assert len(step1[0]["text"]) <= 800
    for _ in range(3):
        pilot.cycle()
    assert len(_open(home)) == 3


def test_a_long_step_stays_under_the_limit(home, repo) -> None:
    path = repo / "docs" / "TODO.md"
    items = "\n".join(f"- [ ] Item number {n} " + "very long words " * 12 for n in range(15))
    path.write_text(f"# TODO\n\n## 1. First\n\n{items}\n")
    pilot = Pilot(home, repo)
    pilot.cycle()
    pilot.cycle()
    text = _open(home)[0]["text"]
    assert len(text) <= 800 and "more" in text and text.rstrip().endswith("close it.")


def test_changed_facts_replace_the_proposal_never_add_a_second_open_one(home, repo) -> None:
    pilot = Pilot(home, repo)
    pilot.cycle()
    pilot.cycle()
    old = next(r for r in _open(home) if r["step"] == "1")
    pilot.agents.append({"id": "lane-x", "project": "demo", "step": "1", "status": "done", "outcome": "done",
                         "report": "Tried it.", "review": {"status": "tests-failed", "message": "red"}})
    pilot.cycle()
    assert [r["id"] for r in _open(home) if r["step"] == "1"] == [old["id"]]   # debouncing: still the old one
    pilot.cycle()
    now = [r for r in _open(home) if r["step"] == "1"]
    assert len(now) == 1 and now[0]["id"] != old["id"] and "lane lane-x" in now[0]["text"]


def test_items_whose_lane_is_in_the_merge_queue_get_no_proposal(home, repo) -> None:
    path = repo / "docs" / "TODO.md"
    path.write_text(path.read_text().replace("Build the first thing.", "Build the first thing. (en revisión: lane-q)"))
    pilot = Pilot(home, repo)
    pilot.agents.append({"id": "lane-q", "project": "demo", "step": "1", "status": "done", "outcome": "done"})
    (home / "mergeq.json").write_text('{"queue": [{"lane": "lane-q", "project": "demo", "status": "running"}]}')
    for _ in range(3):
        pilot.cycle()
    assert sorted(r["step"] for r in _open(home)) == ["2", "3"]
    assert autopilot_mod.sitting(home, pilot.state_all(), pilot.projects(), pilot.agents, pilot.now) == []


def test_an_item_sent_back_by_the_queue_is_proposed_with_the_reason(home, repo) -> None:
    path = repo / "docs" / "TODO.md"
    path.write_text(path.read_text().replace("Build the first thing.", "Build the first thing. (en revisión: lane-q)"))
    pilot = Pilot(home, repo)
    pilot.agents.append({"id": "lane-q", "project": "demo", "step": "1", "status": "done", "outcome": "done",
                         "review": {"status": "sent-back", "message": "conflict in app.py"}})
    pilot.cycle()
    pilot.cycle()
    text = next(r for r in _open(home) if r["step"] == "1")["text"]
    assert "sent back by the merge queue: conflict in app.py" in text


def test_a_proposal_closes_quietly_when_the_queue_takes_the_item(home, repo) -> None:
    path = repo / "docs" / "TODO.md"
    path.write_text(path.read_text().replace("Build the first thing.", "Build the first thing. (en revisión: lane-q)"))
    pilot = Pilot(home, repo)
    pilot.agents.append({"id": "lane-q", "project": "demo", "step": "1", "status": "done", "outcome": "done"})
    pilot.cycle()
    pilot.cycle()
    assert any(r["step"] == "1" for r in _open(home))
    (home / "mergeq.json").write_text('{"queue": [{"lane": "lane-q", "project": "demo", "status": "queued"}]}')
    pilot.cycle()
    step1 = next(r for r in _proposals(home) if r["step"] == "1")
    assert step1["status"] == "done" and step1["answers"] == []


def test_old_per_item_proposals_are_collapsed_into_one_per_step(home, repo) -> None:
    _two_items(repo)
    for text in ("old one", "old two", "old three"):
        inbox_mod.add_request(home, inbox_mod.PROPOSAL, "demo", "1", text, at="2026-10-29T10:00:00Z", sender="tirith")
    inbox_mod.add_request(home, inbox_mod.PROPOSAL, "demo", "9", "gone step", at="2026-10-29T10:00:00Z", sender="tirith")
    pilot = Pilot(home, repo)
    pilot.cycle()
    assert len([r for r in _open(home) if r["step"] == "1"]) == 1   # at once, not after the debounce
    assert [r for r in _open(home) if r["step"] == "9"] == []
    assert all(r["answers"] == [] for r in _proposals(home))
    pilot.cycle()
    assert len([r for r in _open(home) if r["step"] == "1"]) == 1
    assert "2 items need a decision" in next(r for r in _open(home) if r["step"] == "1")["text"]
    flows = [f for f in read_jsonl(home / "flows.jsonl") if "closed its proposal" in f["text"]]
    assert len(flows) >= 4


def test_the_selfcheck_counts_open_proposals_per_step(home, repo) -> None:
    for text in ("a", "b"):
        inbox_mod.add_request(home, inbox_mod.PROPOSAL, "demo", "1", text, at="2026-10-29T10:00:00Z", sender="tirith")
    assert autopilot_mod.open_per_step(home) == {"demo|1": 2}
