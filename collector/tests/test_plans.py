"""The plan buttons: several plans per project, ``plan-refresh`` / ``plan-new`` requests and their lanes."""

from __future__ import annotations

import json
from datetime import datetime, timedelta, timezone
from pathlib import Path

import jsonschema
import pytest

from conftest import make_hooks_mutable, make_repo, write_config, write_lane_manifest
from tirith import autoreview as autoreview_mod
from tirith import asks as asks_mod
from tirith import dispatch as dispatch_mod
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith import manifest as mf
from tirith import planlane
from tirith import roadmap as rm
from tirith.cli import launch_plan, main
from tirith.collector import Collector
from tirith.home import load_config
from tirith.store import read_json, read_jsonl

NOW = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)
TODO = "# TODO: main plan\n\nThe main goal.\n\n## 1. First\n\n- [x] Done.\n- [ ] Open.\n"
LAUNCH = 'agentcmd --brief {brief} --cwd {workspace}'
NEW_PLAN = {"title": "Ship the beta", "goal": "Beta to ten users.", "deadline": "2026-12-01", "steps": "auth\nonboarding"}


def _at(seconds_ago: float) -> str:
    return (NOW - timedelta(seconds=seconds_ago)).strftime("%Y-%m-%dT%H:%M:%SZ")


def _plan_text(title: str, items: str, extra: str = "") -> str:
    return f"# {title}\n\nGoal.\n{extra}\n## 1. Only step\n\n{items}\n"


@pytest.fixture
def project(home: Path, tmp_path: Path):
    repo = make_repo(tmp_path / "demo")
    write_lane_manifest(repo)
    (repo / "docs").mkdir()
    (repo / "docs" / "TODO.md").write_text(TODO, encoding="utf-8")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}],
                 extra=f'[launch]\ncommand = "{LAUNCH}"\n')
    return repo


@pytest.fixture
def plan_launches(monkeypatch):
    calls: list[tuple] = []

    def fake(home, project, kind, request_id=None):
        calls.append((project, kind, request_id))
        return f"plan-lane-{len(calls)}"
    monkeypatch.setattr(dispatch_mod, "_default_plan_launcher", fake)
    return calls


def _by_id(home: Path) -> dict:
    return {r["id"]: r for r in inbox_mod.requests(home)}


# -- several plans per project --------------------------------------------------------------------


def test_manifest_reads_plans(tmp_path: Path) -> None:
    (tmp_path / "tirith.toml").write_text('plans = ["docs/TODO.md", " docs/plans/a.md ", ""]\n', encoding="utf-8")
    assert mf.load(tmp_path).plans == ["docs/TODO.md", "docs/plans/a.md"]
    (tmp_path / "tirith.toml").write_text('plans = "nope"\n', encoding="utf-8")
    assert mf.load(tmp_path).plans == []
    assert mf.load(tmp_path / "none").plans == []


def test_plans_current_first_and_done_flag(tmp_path: Path) -> None:
    (tmp_path / "docs" / "plans").mkdir(parents=True)
    (tmp_path / "docs" / "TODO.md").write_text(TODO, encoding="utf-8")
    (tmp_path / "docs" / "plans" / "beta.md").write_text(
        _plan_text("Beta", "- [ ] one", "Deadline: 2026-12-01\n"), encoding="utf-8")
    (tmp_path / "docs" / "plans" / "old.md").write_text(_plan_text("Old", "- [x] a\n- [x] b"), encoding="utf-8")
    (tmp_path / "tirith.toml").write_text(
        'deadline = "2026-11-04T21:00:00-05:00"\n'
        'plans = ["docs/TODO.md", "docs/plans/beta.md", "docs/plans/old.md", "docs/plans/missing.md"]\n',
        encoding="utf-8")
    plans = rm.build_plans(tmp_path, "p", [{"project": "p", "status": "working", "step": "1", "id": "w"}])
    assert [p["id"] for p in plans] == ["TODO", "beta", "old"]  # the missing file is skipped
    cur, beta, old = plans
    assert [p["current"] for p in plans] == [True, False, False]
    assert cur["title"] == "main plan" and cur["deadline"] == "2026-11-04T21:00:00-05:00"
    assert cur["steps"][0]["agents"] == ["w"] and beta["steps"][0]["agents"] == []  # lanes belong to the current plan
    assert beta["deadline"] == "2026-12-01" and beta["finished"] is False
    assert old["finished"] is True and old["done"] == old["total"] == 2
    assert rm.build(tmp_path, "p", [])["file"] == cur["file"]  # roadmap is the current plan


def test_roadmap_key_sets_the_current_plan_and_is_listed_once(tmp_path: Path) -> None:
    (tmp_path / "docs" / "plans").mkdir(parents=True)
    (tmp_path / "docs" / "TODO.md").write_text(TODO, encoding="utf-8")
    (tmp_path / "docs" / "plans" / "beta.md").write_text(_plan_text("Beta", "- [ ] one"), encoding="utf-8")
    (tmp_path / "tirith.toml").write_text(
        'roadmap = "docs/plans/beta.md"\nplans = ["docs/TODO.md", "docs/plans/beta.md"]\n', encoding="utf-8")
    plans = rm.build_plans(tmp_path, "p", [])
    assert [(p["id"], p["current"]) for p in plans] == [("beta", True), ("TODO", False)]
    assert rm.build(tmp_path, "p", [])["file"].endswith("beta.md")


def test_plans_without_roadmap_file_uses_the_first_plan(tmp_path: Path) -> None:
    (tmp_path / "plans").mkdir()
    (tmp_path / "plans" / "x.md").write_text(_plan_text("X", "- [ ] one"), encoding="utf-8")
    (tmp_path / "tirith.toml").write_text('plans = ["plans/x.md"]\n', encoding="utf-8")
    assert rm.build(tmp_path, "p", [])["steps"][0]["title"] == "Only step"
    assert [p["id"] for p in rm.build_plans(tmp_path, "p", [])] == ["x"]
    assert rm.build_plans(None, "p", []) == [] and rm.build_plans(tmp_path / "nothing", "p", []) == []


def test_title_and_deadline_helpers() -> None:
    assert rm.title_of("# Plan: Beta\n") == "Beta" and rm.title_of("no heading") == ""
    assert rm.deadline_of("# T\nDeadline: 2026-12-01T10:00:00Z\n## 1. A\nDeadline: nope") == "2026-12-01T10:00:00Z"
    assert rm.deadline_of("# T\n## 1. A\nDeadline: 2026-12-01") is None


def test_collector_state_has_plans_and_validates(home: Path, project: Path) -> None:
    (project / "docs" / "plans").mkdir()
    (project / "docs" / "plans" / "beta.md").write_text(_plan_text("Beta", "- [ ] one"), encoding="utf-8")
    (project / "tirith.toml").write_text(
        'plans = ["docs/TODO.md", "docs/plans/beta.md"]\n' + (project / "tirith.toml").read_text(), encoding="utf-8")
    state = Collector(home, client=None).build_state(NOW)
    row = next(p for p in state["projects"] if p["id"] == "demo")
    assert [p["id"] for p in row["plans"]] == ["TODO", "beta"] and row["roadmap"]["file"] == row["plans"][0]["file"]
    state["inbox"] = [{"id": "in-1", "at": _at(1), "kind": "plan-new", "project": "demo", "step": "plan",
                       "text": "{}", "status": "open", "answers": []}]
    jsonschema.validate(state, json.loads((Path(__file__).resolve().parents[1] / "state.schema.json").read_text()))


# -- the requests -----------------------------------------------------------------------------------


def test_inbox_accepts_plan_requests(home: Path) -> None:
    refresh = inbox_mod.add_request(home, "plan-refresh", "demo", "", at=_at(3))
    assert refresh["step"] == "plan" and refresh["text"] == ""
    new = inbox_mod.add_request(home, "plan-new", "demo", "", json.dumps(NEW_PLAN | {"extra": 1}), at=_at(2))
    assert json.loads(new["text"]) == NEW_PLAN and new["step"] == "plan"
    rows = _by_id(home)
    assert rows[new["id"]]["plan"] == NEW_PLAN and rows[refresh["id"]]["manager_pending"] is True
    with pytest.raises(inbox_mod.InboxError):
        inbox_mod.add_request(home, "plan-new", "demo", "", "a title")
    with pytest.raises(inbox_mod.InboxError):
        inbox_mod.add_request(home, "plan-new", "demo", "", json.dumps({"goal": "no title"}))


def test_cli_inbox_add_and_link(home: Path, capsys) -> None:
    assert main(["inbox", "add", "--kind", "plan-refresh", "--project", "demo"]) == 0
    rid = capsys.readouterr().out.strip()
    assert _by_id(home)[rid]["step"] == "plan"
    url = inbox_mod.build_url("plan-new", "demo", "", json.dumps(NEW_PLAN), "tok")
    fields = inbox_mod.parse_url(url)
    assert fields["kind"] == "plan-new" and json.loads(fields["text"]) == NEW_PLAN


# -- the dispatcher ---------------------------------------------------------------------------------


def test_refresh_launches_one_lane_once_and_answers(project, home: Path, plan_launches) -> None:
    rid = inbox_mod.add_request(home, "plan-refresh", "demo", "", at=_at(5))["id"]
    collector = Collector(home, client=None)
    for _ in range(3):
        collector.build_state(NOW)
    assert plan_launches == [("demo", "plan-refresh", rid)]
    req = _by_id(home)[rid]
    assert req["status"] == "answered" and req["manager_pending"] is False
    assert req["answers"][0]["by"] == "tirith" and req["answers"][0]["lane"] == "plan-lane-1"
    assert read_json(home / "dispatch.json", {})["handled"] == {rid: "launched:plan-lane-1"}
    flows = [(f["from"], f["to"], f["kind"]) for f in read_jsonl(home / "flows.jsonl") if f["from"] != "guard"]
    assert flows == [("owner", "tirith", "request"), ("tirith", "agent:plan-lane-1", "brief")]


def test_second_refresh_while_one_runs_points_at_the_running_lane(project, home: Path) -> None:
    lane = {"id": "auditor", "project": "demo", "step": "plan-refresh"}
    rid = inbox_mod.add_request(home, "plan-refresh", "demo", "", at=_at(5))["id"]
    dispatch_mod.run(home, load_config(home), NOW, agents=[], lanes=[lane], status_of=lambda lane, now: "working",
                     plan_launcher=lambda *a: pytest.fail("launched"))
    (answer,) = _by_id(home)[rid]["answers"]
    assert "already being updated" in answer["text"] and "`auditor`" in answer["text"] and answer["lane"] == "auditor"


def test_a_working_refresh_in_another_project_does_not_block(project, home: Path) -> None:
    lane = {"id": "other", "project": "elsewhere", "step": "plan-refresh"}
    inbox_mod.add_request(home, "plan-refresh", "demo", "", at=_at(5))
    started = []
    dispatch_mod.run(home, load_config(home), NOW, agents=[], lanes=[lane], status_of=lambda lane, now: "working",
                     plan_launcher=lambda *a: started.append(a) or "x")
    assert len(started) == 1


def test_new_plan_launches_a_planner_lane(project, home: Path, plan_launches) -> None:
    rid = inbox_mod.add_request(home, "plan-new", "demo", "", json.dumps(NEW_PLAN), at=_at(5))["id"]
    Collector(home, client=None).build_state(NOW)
    assert plan_launches == [("demo", "plan-new", rid)]
    assert "writes the new plan" in _by_id(home)[rid]["answers"][0]["text"]


def test_refused_plan_request_tells_the_owner(project, home: Path, monkeypatch) -> None:
    def refuse(home, project, kind, request_id=None):
        raise launch_mod.LaunchError("tirith check fails for 'demo'")
    monkeypatch.setattr(dispatch_mod, "_default_plan_launcher", refuse)
    refresh = inbox_mod.add_request(home, "plan-refresh", "demo", "", at=_at(5))["id"]
    new = inbox_mod.add_request(home, "plan-new", "demo", "", json.dumps(NEW_PLAN), at=_at(4))["id"]
    collector = Collector(home, client=None)
    collector.build_state(NOW)
    state = collector.build_state(NOW)
    rows = _by_id(home)
    assert "tirith check fails" in rows[refresh]["answers"][0]["text"]
    assert not [w for w in state["waiting_on_you"] if w["id"].startswith("dispatch-")]  # the manager's to fix
    queued = {q["id"]: q for q in asks_mod.queue_items(home, only_open=True)}
    assert "Try again: tirith inbox add --kind plan-refresh --project demo" in queued[f"sq-dispatch-{refresh}"]["text"]
    assert f"sq-dispatch-{new}" in queued


def test_auto_launch_off_leaves_plan_requests_to_the_manager(project, home: Path, plan_launches) -> None:
    path = home / "config.toml"
    path.write_text(path.read_text() + "\n[dispatch]\nauto_launch = false\n")
    rid = inbox_mod.add_request(home, "plan-refresh", "demo", "", at=_at(5))["id"]
    Collector(home, client=None).build_state(NOW)
    assert plan_launches == [] and _by_id(home)[rid]["manager_pending"] is True


# -- the lanes --------------------------------------------------------------------------------------


@pytest.fixture
def real_project(home: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    repo = make_repo(tmp_path / "demo")
    write_lane_manifest(repo)
    (repo / "docs").mkdir()
    (repo / "docs" / "TODO.md").write_text(TODO, encoding="utf-8")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}],
                 extra=f'[launch]\ncommand = "{LAUNCH}"\n')
    assert main(["guard", "install", "demo"]) == 0
    started: list[list[str]] = []
    monkeypatch.setattr(launch_mod, "spawn_detached", lambda argv, home, log: started.append(argv) or 1)
    yield repo, started
    make_hooks_mutable(repo)
    make_hooks_mutable(tmp_path / "demo-lanes")


def test_launch_plan_refresh_writes_a_brief_with_the_plan_lanes_and_inbox(real_project, home: Path) -> None:
    repo, started = real_project
    ask = inbox_mod.add_request(home, "ask", "demo", "1", "why is it slow?", at=_at(9))["id"]
    rid = inbox_mod.add_request(home, "plan-refresh", "demo", "", at=_at(5))["id"]
    (home / "lanes").mkdir(exist_ok=True)
    (home / "lanes" / "old-lane.json").write_text(json.dumps({
        "id": "old-lane", "project": "demo", "step": "1", "feature": "Old work", "report": "Built the thing",
        "started_at": _at(100), "review": {"status": "merged"}}), encoding="utf-8")
    lane_id = launch_plan(home, "demo", "plan-refresh", rid, out=open("/dev/null", "w"))
    brief = (home / "briefs" / f"{lane_id}.md").read_text(encoding="utf-8")
    assert "- [ ] Open." in brief and "`old-lane` (step 1): Old work: Built the thing" in brief
    assert "ask [open] step 1: why is it slow?" in brief and ask  # the inbox
    assert "Edit only `docs/TODO.md`" in brief and "Done since last update" in brief and "Needs the owner" in brief
    (argv,) = started
    assert argv[argv.index("--step") + 1] == "plan-refresh" and argv[argv.index("--request") + 1] == rid
    assert (repo.parent / "demo-lanes" / lane_id).is_dir()


def test_launch_plan_new_names_the_file_and_the_registration(real_project, home: Path) -> None:
    repo, started = real_project
    (repo / "docs" / "plans").mkdir()
    (repo / "docs" / "plans" / "ship-the-beta.md").write_text("# taken\n", encoding="utf-8")
    (repo / "tirith.toml").write_text('plans = ["docs/TODO.md"]\n' + (repo / "tirith.toml").read_text())
    rid = inbox_mod.add_request(home, "plan-new", "demo", "", json.dumps(NEW_PLAN), at=_at(5))["id"]
    lane_id = launch_plan(home, "demo", "plan-new", rid, out=open("/dev/null", "w"))
    brief = (home / "briefs" / f"{lane_id}.md").read_text(encoding="utf-8")
    assert "`docs/plans/ship-the-beta-2.md`" in brief and "Beta to ten users." in brief
    assert "Deadline: 2026-12-01" in brief and "onboarding" in brief
    assert "`plans = [...]`" in brief and "docs/TODO.md" in brief and 'roadmap = "docs/plans/ship-the-beta-2.md"' in brief
    assert started[0][started[0].index("--step") + 1] == "plan-new"


def test_launch_plan_refuses_without_a_request_or_a_second_refresh(real_project, home: Path) -> None:
    with pytest.raises(launch_mod.LaunchError, match="needs its request"):
        launch_plan(home, "demo", "plan-new", None, out=open("/dev/null", "w"))
    (home / "lanes").mkdir(exist_ok=True)
    import os
    (home / "lanes" / "busy.json").write_text(json.dumps({
        "id": "busy", "project": "demo", "step": "plan-refresh", "pid": os.getpid(), "started_at": _at(5),
        "ended_at": None, "result": None}), encoding="utf-8")
    with pytest.raises(launch_mod.LaunchError, match="already has a plan update running"):
        launch_plan(home, "demo", "plan-refresh", None, out=open("/dev/null", "w"))


def test_slug_and_brief_helpers() -> None:
    assert planlane.slug_for("Ship the Beta!", set()) == "ship-the-beta"
    assert planlane.slug_for("Ship the Beta!", {"ship-the-beta"}) == "ship-the-beta-2"
    assert planlane.slug_for("???", set()) == "plan"
    brief = planlane.new_plan_brief("demo", {"title": "T", "goal": "", "deadline": "", "steps": ""}, "t", [])
    assert "Deadline" not in brief and "nothing yet" in brief
    for text in (brief, planlane.refresh_brief("demo", "docs/TODO.md", "## 1. A\n", [], [], Path("/h"))):
        assert "alias" in text and "No servers" in text


# -- the merge answers the request with the summary ----------------------------------------------------


def test_a_merged_refresh_answers_with_the_lane_report(project, home: Path) -> None:
    rid = inbox_mod.add_request(home, "plan-refresh", "demo", "", at=_at(60))["id"]
    lane = {"id": "auditor", "project": "demo", "step": "plan-refresh", "feature": "Update the plan",
            "request_id": rid, "report": "**Done since last update**\n- Hooks", "brief": None}
    review = {"status": "merged", "files": 1, "passed": 4, "at": _at(1)}
    autoreview_mod.settle(home, load_config(home), lane, review, {}, NOW, project)
    (answer,) = _by_id(home)[rid]["answers"]
    assert answer["text"].startswith("Merged auditor") and "**Done since last update**\n- Hooks" in answer["text"]
    assert not (home / "queue.jsonl").exists() or "update the plan for step" not in (home / "queue.jsonl").read_text()


def test_update_plan_ignores_plan_lanes(project, home: Path) -> None:
    lane = {"id": "auditor", "step": "plan-refresh", "project": "demo", "brief": str(home / "nope.md")}
    assert autoreview_mod.update_plan(home, project, lane) == ""
