"""The autopilot: ready steps get a lane by themselves, within limits; owner items; the on/off switch."""

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

    def cycle(self, seconds: float = 30) -> None:
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


def _waits(home: Path) -> dict:
    return {r["id"]: r for r in read_jsonl(home / "waits.jsonl")}


# -- launching ----------------------------------------------------------------------------------


def test_launches_one_lane_per_ready_step_oldest_first(home, repo) -> None:
    pilot = Pilot(home, repo)
    pilot.cycle()
    assert pilot.launched == [("demo", "1"), ("demo", "2"), ("demo", "3")]  # not 4 (owner) nor 5 (date)
    pilot.cycle()
    assert len(pilot.launched) == 3  # a step with a working lane is never started twice


def test_max_parallel_limits_running_lanes(home, repo) -> None:
    pilot = Pilot(home, repo, "[autopilot]\nmax_parallel = 2")
    pilot.cycle()
    assert pilot.launched == [("demo", "1"), ("demo", "2")]
    pilot.end("lane-1", {"status": "merged"})
    pilot.tick("Build the first thing.")
    pilot.cycle()
    assert pilot.launched[2:] == [("demo", "3")]  # a slot freed up: the next oldest ready step


def test_every_launch_is_a_flow_and_in_the_state(home, repo) -> None:
    pilot = Pilot(home, repo)
    pilot.cycle()
    flows = [f for f in read_jsonl(home / "flows.jsonl") if f["kind"] == "brief"]
    assert [(f["from"], f["to"], f["text"]) for f in flows][0] == (
        "tirith", "agent:lane-1", "Autopilot started demo step 1")
    public = autopilot_mod.public(home, load_config(home))
    assert public["enabled"] is True and public["max_parallel"] == 4
    assert [(s["lane"], s["step"]) for s in public["started"]] == [("lane-1", "1"), ("lane-2", "2"), ("lane-3", "3")]


def test_disabled_in_config_launches_nothing(home, repo) -> None:
    pilot = Pilot(home, repo, "[autopilot]\nenabled = false")
    pilot.cycle()
    assert pilot.launched == []
    assert autopilot_mod.public(home, load_config(home))["enabled"] is False


def test_owner_switch_off_and_on(home, repo) -> None:
    pilot = Pilot(home, repo)
    off = inbox_mod.add_request(home, "autopilot", "all", "autopilot", "off", at="2026-10-29T11:59:00Z")
    pilot.cycle()
    assert pilot.launched == []
    assert autopilot_mod.public(home, load_config(home))["enabled"] is False
    assert {r["id"]: r for r in inbox_mod.requests(home)}[off["id"]]["status"] == "done"
    assert not inbox_mod.requests(home)[0]["manager_pending"]
    inbox_mod.add_request(home, "autopilot", "all", "autopilot", "on")
    pilot.cycle()
    assert len(pilot.launched) == 3
    with pytest.raises(inbox_mod.InboxError):
        inbox_mod.add_request(home, "autopilot", "all", "autopilot", "maybe")


def test_refused_launch_is_one_manager_item_and_retried_later(home, repo) -> None:
    pilot = Pilot(home, repo)

    def refuse(home, project, step, request_id=None):
        raise dispatch_mod.launch_mod.LaunchError("tirith check fails for 'demo'")
    pilot.launcher = refuse
    pilot.cycle()
    pilot.cycle()
    assert not [i for i in _waits(home) if i.startswith("autopilot-demo-")]  # never the owner's
    items = {q["id"]: q for q in asks_mod.queue_items(home, only_open=True) if q["kind"] == "launch-refused"}
    assert sorted(items) == ["sq-autopilot-demo-1", "sq-autopilot-demo-2", "sq-autopilot-demo-3"]
    assert "Fix it: tirith check --project demo" in items["sq-autopilot-demo-1"]["text"]


# -- not converging -----------------------------------------------------------------------------


def test_one_lane_without_progress_pauses_the_step(home, repo) -> None:
    pilot = Pilot(home, repo, "[autopilot]\nmax_parallel = 1")
    pilot.cycle()
    pilot.end("lane-1", report="I tried two ways and neither compiled.\n\nDetails follow.")
    pilot.cycle()
    assert pilot.launched == [("demo", "1"), ("demo", "2")]  # step 1 is paused at once, the autopilot moves on
    queue = asks_mod.queue_items(home, only_open=True)
    assert [q["id"] for q in queue] == ["sq-autopilot-demo-1"]
    assert "step 1" in queue[0]["text"].lower() and "neither compiled" in queue[0]["text"]
    assert "Details follow" not in queue[0]["text"]
    assert pilot.state()["paused"]["demo|1"]["items"] == ["Build the first thing."]
    pilot.cycle()
    assert pilot.launched.count(("demo", "1")) == 1  # not relaunched while the text is the same


def test_progress_does_not_pause(home, repo) -> None:
    pilot = Pilot(home, repo, "[autopilot]\nmax_parallel = 1")
    pilot.cycle()
    pilot.tick("Build the first thing.")
    pilot.end("lane-1", {"status": "merged"})
    pilot.cycle()
    assert "demo|1" not in pilot.state()["paused"]


def test_lane_waiting_for_its_review_is_not_counted(home, repo) -> None:
    pilot = Pilot(home, repo, "[autopilot]\nmax_parallel = 1")
    pilot.cycle()
    pilot.end("lane-1", {"status": "pending"})
    pilot.cycle()
    assert pilot.state()["paused"] == {} and pilot.launched.count(("demo", "1")) == 1  # review pending: not again


def test_paused_step_resumes_when_its_items_change(home, repo) -> None:
    pilot = Pilot(home, repo, "[autopilot]\nmax_parallel = 1")
    pilot.cycle()
    pilot.end("lane-1")
    pilot.cycle()
    assert "demo|1" in pilot.state()["paused"]
    path = repo / "docs" / "TODO.md"
    path.write_text(path.read_text().replace("Build the first thing.", "Build the first thing, smaller."))
    pilot.cycle()
    assert "demo|1" not in pilot.state()["paused"]
    assert [q for q in asks_mod.queue_items(home, only_open=True) if q["kind"] != "launch-refused"] == []


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


def test_state_json_carries_the_autopilot_and_item_ids(home, repo) -> None:
    from tirith.collector import Collector
    path = home / "config.toml"
    path.write_text(path.read_text() + "\n[autopilot]\nenabled = false\n")  # the test must not start lanes
    state = Collector(home, client=None).build_state(NOW)
    assert state["autopilot"]["enabled"] is False
    road = state["projects"][0]["roadmap"]
    assert [s["status"] for s in road["steps"]] == ["ready", "ready", "ready", "blocked-owner", "blocked-date"]
    owner = [i for i in road["steps"][3]["items"] if i["who"] == "owner"]
    assert len(owner) == 2 and all(i["owner_item"] in _waits(home) for i in owner)
    assert len(read_json(home / "dispatch.json", {})["autopilot"]["owner_items"]) == 3


def test_a_fallback_relaunch_is_the_same_steps_lane(home, repo) -> None:
    pilot = Pilot(home, repo, "[autopilot]\nmax_parallel = 2")
    pilot.cycle()
    pilot.end("lane-1")  # ran out of credits; the fallback continued it as lane-1b
    pilot.agents.append({"id": "lane-1b", "project": "demo", "step": "1", "status": "working"})
    state = dispatch_mod.load_state(home)
    state["fallbacks"] = {"lane-1": "lane-1b"}
    dispatch_mod.save_state(home, state)
    pilot.cycle()
    assert pilot.launched == [("demo", "1"), ("demo", "2")]  # no second lane on step 1
    assert pilot.state()["paused"] == {} and pilot.state()["lanes"]["lane-1b"]["settled"] is False
    pilot.end("lane-1b", {"status": "merged"})
    pilot.cycle()
    assert pilot.state()["lanes"]["lane-1b"]["settled"] is True


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


@pytest.mark.parametrize("report", ALREADY_BUILT)
def test_report_saying_already_built_asks_the_plan_updater_to_tick(home, repo, report) -> None:
    pilot = Pilot(home, repo, "[autopilot]\nmax_parallel = 1")
    pilot.cycle()
    pilot.end("lane-1", report=report)
    pilot.cycle()
    assert autopilot_mod.report_kind(report) == "done"
    refresh = [r for r in inbox_mod.requests(home) if r["kind"] == "plan-refresh"]
    assert [(r["project"], r["step"]) for r in refresh] == [("demo", "1")]  # limited to that step
    assert pilot.state()["paused"]["demo|1"]["why"] == "done"
    assert pilot.launched.count(("demo", "1")) == 1
    pilot.cycle()
    assert len([r for r in inbox_mod.requests(home) if r["kind"] == "plan-refresh"]) == 1  # asked once
    assert [q for q in asks_mod.queue_items(home, only_open=True) if q["kind"] != "launch-refused"] == []


@pytest.mark.parametrize("report", CANNOT_DO)
def test_report_saying_it_cannot_proposes_owner_only(home, repo, report) -> None:
    pilot = Pilot(home, repo, "[autopilot]\nmax_parallel = 1")
    pilot.cycle()
    pilot.end("lane-1", report=report)
    pilot.cycle()
    assert autopilot_mod.report_kind(report) == "cannot"
    queue = asks_mod.queue_items(home, only_open=True)
    assert [q["id"] for q in queue] == ["sq-autopilot-demo-1"] and "owner-only" in queue[0]["text"]
    assert pilot.state()["paused"]["demo|1"]["why"] == "cannot"
    assert [r for r in inbox_mod.requests(home) if r["kind"] == "plan-refresh"] == []
    assert pilot.launched.count(("demo", "1")) == 1


def test_other_reports_are_stuck_with_their_summary() -> None:
    for report in NOT_FINISHED:
        assert autopilot_mod.report_kind(report) == "stuck"
        assert autopilot_mod.report_summary(report).startswith("I added `codex`")
    assert autopilot_mod.report_summary(None) == "The lane ended without a report."


def test_refresh_brief_is_limited_to_the_step() -> None:
    from tirith.planlane import refresh_brief
    assert "Limited to step 3" in refresh_brief("demo", "docs/TODO.md", "# TODO", [], [], Path("."), step="3")
    assert "Limited to step" not in refresh_brief("demo", "docs/TODO.md", "# TODO", [], [], Path("."))


# -- the header counts ------------------------------------------------------------------------------


def test_header_counts_only_live_lanes_and_unpaused_ready_steps(home, repo) -> None:
    pilot = Pilot(home, repo, "[autopilot]\nmax_parallel = 1")
    pilot.cycle()
    count = lambda: autopilot_mod.public(home, load_config(home), pilot.agents, pilot.projects(), pilot.now)
    assert (count()["working"], count()["queued"]) == (1, 2)  # step 1 working, steps 2 and 3 queued
    for lane in ("lane-1", "lane-2", "lane-3"):
        pilot.end(lane, report="I couldn't do it.")
        pilot.cycle()
    got = count()
    assert (got["working"], got["queued"]) == (0, 0)  # three ended lanes in the started log: none counted
    assert len(got["started"]) == 3


def test_header_working_never_exceeds_the_agents_list(home, repo) -> None:
    pilot = Pilot(home, repo, "[autopilot]\nmax_parallel = 1")
    pilot.cycle()
    got = autopilot_mod.public(home, load_config(home), [], pilot.projects(), pilot.now)
    assert got["working"] == 0  # a lane inside the grace period with no agent row is not "working" on screen
    assert len(got["started"]) == 1
