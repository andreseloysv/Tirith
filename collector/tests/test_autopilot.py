"""The autopilot: ready steps get a lane by themselves, within limits; owner items; the on/off switch."""

from __future__ import annotations

import os
from datetime import date, datetime, timedelta, timezone
from pathlib import Path

import pytest

from conftest import write_config
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
- [ ] Log in to the portal (dueño: https://portal.example.invalid/login).

## 3. Third

- [ ] Build the third thing.

## 4. Owner only

- [ ] Install it (dueño) `tirith url install`.
- [ ] Decide the colour (dueño).

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

    def end(self, lane: str, review: dict | None = None) -> None:
        row = next(a for a in self.agents if a["id"] == lane)
        row["status"] = "done"
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


def test_refused_launch_is_one_owner_item_and_retried_later(home, repo) -> None:
    pilot = Pilot(home, repo)

    def refuse(home, project, step, request_id=None):
        raise dispatch_mod.launch_mod.LaunchError("tirith check fails for 'demo'")
    pilot.launcher = refuse
    pilot.cycle()
    pilot.cycle()
    items = [i for i in _waits(home) if i.startswith("autopilot-demo-")]
    assert sorted(items) == ["autopilot-demo-1", "autopilot-demo-2", "autopilot-demo-3"]
    assert _waits(home)["autopilot-demo-1"]["options"][0]["command"] == "tirith check --project demo"


# -- not converging -----------------------------------------------------------------------------


def test_two_lanes_without_progress_pause_the_step(home, repo) -> None:
    pilot = Pilot(home, repo, "[autopilot]\nmax_parallel = 1")
    pilot.cycle()
    pilot.end("lane-1")  # ended, nothing ticked
    pilot.cycle()
    assert pilot.launched == [("demo", "1"), ("demo", "1")]
    pilot.end("lane-2")
    pilot.cycle()
    assert pilot.launched[2:] == [("demo", "2")]  # step 1 is paused, the autopilot moves on
    queue = asks_mod.queue_items(home, only_open=True)
    assert [q["id"] for q in queue] == ["sq-autopilot-demo-1"]
    assert "step 1 is not converging" in queue[0]["text"].lower()
    assert pilot.state()["paused"]["demo|1"]["items"] == ["Build the first thing."]


def test_progress_resets_the_counter(home, repo) -> None:
    pilot = Pilot(home, repo, "[autopilot]\nmax_parallel = 1")
    pilot.cycle()
    pilot.end("lane-1")
    pilot.cycle()
    assert pilot.state()["stuck"] == {"demo|1": 1}
    pilot.tick("Build the first thing.")
    pilot.end("lane-2", {"status": "merged"})
    pilot.cycle()
    assert "demo|1" not in pilot.state()["stuck"] and "demo|1" not in pilot.state()["paused"]


def test_lane_waiting_for_its_review_is_not_counted(home, repo) -> None:
    pilot = Pilot(home, repo, "[autopilot]\nmax_parallel = 1")
    pilot.cycle()
    pilot.end("lane-1", {"status": "pending"})
    pilot.cycle()
    assert pilot.state()["stuck"] == {} and pilot.launched.count(("demo", "1")) == 1  # review pending: not again


def test_paused_step_resumes_when_its_items_change(home, repo) -> None:
    pilot = Pilot(home, repo, "[autopilot]\nmax_parallel = 1")
    for lane in ("lane-1", "lane-2"):
        pilot.cycle()
        pilot.end(lane)
    pilot.cycle()
    assert "demo|1" in pilot.state()["paused"]
    path = repo / "docs" / "TODO.md"
    path.write_text(path.read_text().replace("Build the first thing.", "Build the first thing, smaller."))
    pilot.cycle()
    assert "demo|1" not in pilot.state()["paused"]
    assert asks_mod.queue_items(home, only_open=True) == []


# -- owner items --------------------------------------------------------------------------------


def test_owner_items_with_command_link_and_plain(home, repo) -> None:
    pilot = Pilot(home, repo)
    pilot.cycle()
    waits = _waits(home)
    owner = {k: v for k, v in waits.items() if k.startswith("owner-demo-")}
    assert len(owner) == 3
    by_text = {v["text"]: v for v in owner.values()}
    command = next(v for t, v in by_text.items() if "Install it" in t)
    assert command["actions"][0]["command"] == "tirith url install" and command["actions"][0]["cwd"] == str(repo)
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
    pilot.tick("Decide the colour (dueño).")
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
