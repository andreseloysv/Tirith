"""The plan gate: no agent starts or finishes without the plan (``tirith.plangate``, ``docs/CONTRACT-roadmap.md``)."""

from __future__ import annotations

import io
import json
import subprocess
import sys
import threading
import time
from datetime import date, datetime, timezone
from pathlib import Path

import pytest

from conftest import GIT, make_hooks_mutable, make_repo, write_config, write_lane_manifest, write_plan
from tirith import asks as asks_mod
from tirith import gates, launch as launch_mod
from tirith import plangate, roadmap
from tirith import dispatch as dispatch_mod
from tirith.cli import launch_plan, launch_step, launch_verify, main, register_lane
from tirith.home import load_config
from tirith.store import read_json, write_json_atomic

PLAN = """# TODO

## 1. Work

- [ ] Build the button.
- [ ] Write the docs.
- [x] Already built.

## 2. More

- [ ] Second step item.
"""


@pytest.fixture
def proj(home: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    monkeypatch.delenv("TIRITH_LANE", raising=False)
    monkeypatch.setattr("tirith.cli.verify_guard", lambda *a, **k: [])
    repo = make_repo(tmp_path / "demo")
    write_lane_manifest(repo)
    write_plan(repo, PLAN)
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}],
                 extra='[launch]\ncommand = "agentcmd --brief {brief} --cwd {workspace}"\n'
                       '[agents.anthropic]\ncommand = "agent-bin"\n')
    assert main(["guard", "install", "demo"]) == 0
    started: list[list[str]] = []
    monkeypatch.setattr(launch_mod, "spawn_detached", lambda argv, home, log: started.append(argv) or 4242)
    yield repo, started
    make_hooks_mutable(repo)
    make_hooks_mutable(tmp_path / "demo-lanes")


def _plan(repo: Path) -> str:
    return (repo / "docs" / "TODO.md").read_text(encoding="utf-8")


def _commits(repo: Path) -> list[str]:
    return subprocess.run([GIT, "-C", str(repo), "log", "--format=%s"], capture_output=True, text=True).stdout.splitlines()


def _lane(home: Path, repo: Path, lane_id: str = "l1", item: str = "Build the button.", **extra) -> dict:
    lane = register_lane(home, lane_id=lane_id, project="demo", hero_class="warden", feature="Work",
                         workspace=str(repo), log=None, pid=None, brief=None, step="1", item=item)
    lane.update(extra)
    write_json_atomic(home / "lanes" / f"{lane_id}.json", lane)
    return lane


def _run(repo: Path, *extra: str, code: str = "pass") -> int:
    return main(["lane", "run", "--project", "demo", "--class", "warden", "--feature", "Work", "--brief",
                 str(repo / "brief.md"), "--workspace", str(repo), *extra, "--", sys.executable, "-c", code])


# -- the item notes -----------------------------------------------------------------------------


def test_status_notes_round_trip() -> None:
    assert roadmap.split_status("Build it.") == ("Build it.", "open", None)
    note = roadmap.with_status("Build it.", "progress", "web-panel")
    assert note == "Build it. (en curso: web-panel)"
    assert roadmap.split_status(note) == ("Build it.", "progress", "web-panel")
    assert roadmap.split_status("Build it. (en revisión)") == ("Build it.", "review", None)
    assert roadmap.split_status(roadmap.with_status("X", "blocked", "needs a key (sic)")) == ("X", "blocked", "needs a key sic")
    assert roadmap.split_status(roadmap.with_status("X", "reopened", "tests failed (3)")) == ("X", "reopened", "tests failed 3")
    assert roadmap.item_id("A (en curso: l1)") == roadmap.item_id("A")


def test_items_in_progress_are_not_agent_items() -> None:
    steps = roadmap.parse("## 1. S\n\n- [ ] Free.\n- [ ] Taken. (en curso: l1)\n- [ ] Mine. (reabierto: failed)\n"
                          "- [ ] In review. (en revisión)\n- [ ] Stuck. (bloqueado: why)\n")
    assert roadmap.agent_items(steps[0], date(2026, 1, 1)) == ["Free.", "Mine. (reabierto: failed)"]


def test_set_item_add_sub_and_add_step() -> None:
    text = "## 1. S\n\n- [ ] One.\n  more of one\n- [ ] Two.\n"
    new = roadmap.set_item(text, "1", "One. more of one", "One. (en curso: l1)")
    assert new == "## 1. S\n\n- [ ] One. (en curso: l1)\n- [ ] Two.\n"
    ticked = roadmap.set_item(new, "1", "One.", tick=True)
    assert "- [x] One. (en curso: l1)" in ticked
    sub = roadmap.add_sub(new, "1", "One.", "Revisar: One.")
    assert sub == "## 1. S\n\n- [ ] One. (en curso: l1)\n  - [ ] Revisar: One.\n- [ ] Two.\n"
    assert roadmap.add_sub(sub, "1", "One.", "Second sub").splitlines()[4] == "  - [ ] Second sub"
    with pytest.raises(roadmap.EditError):
        roadmap.set_item(text, "1", "Nope.", "x")
    assert roadmap.add_step("## 1. S\n", "0", "Ops").endswith("## 0. Ops\n\n")
    assert roadmap.add_step("## 0. Ops\n", "0", "Ops") == "## 0. Ops\n"


def test_build_shows_item_states(home: Path, tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "r")
    write_plan(repo, "## 1. S\n\n- [ ] A. (en curso: l1)\n- [ ] B. (en revisión)\n- [ ] C.\n- [x] D.\n")
    row = roadmap.build(repo, "r", [])["steps"][0]
    assert [(i["state"], i.get("detail")) for i in row["items"]] == [
        ("progress", "l1"), ("review", None), ("open", None), ("done", None)]
    assert row["status"] == "ready"


# -- start: a linked item, no exceptions ----------------------------------------------------------


def test_lane_run_refuses_without_an_item(proj, home: Path, capsys) -> None:
    repo, _ = proj
    assert _run(repo, "--id", "no-item") == 3
    err = capsys.readouterr().err
    assert "Add it to the plan first: tirith plan add --project demo" in err
    assert not (home / "lanes" / "no-item.json").exists()


@pytest.mark.parametrize("step,item,why", [
    ("1", "Not in the plan.", "not an open item of step 1"),
    ("1", "Already built.", "already done"),
    ("7", "Build the button.", "step 7 is not in"),
    ("2", "Build the button.", "not an open item of step 2"),
])
def test_lane_run_refuses_an_item_that_is_not_open_in_the_plan(proj, home: Path, capsys, step, item, why) -> None:
    repo, _ = proj
    assert _run(repo, "--id", "x", "--step", step, "--item", item) == 3
    err = capsys.readouterr().err
    assert why in err and f"tirith plan add --project demo --step {step}" in err
    assert not (home / "lanes" / "x.json").exists()


def test_lane_run_with_an_item_marks_it_in_progress(proj, home: Path, capsys) -> None:
    repo, _ = proj
    before = len(_commits(repo))
    assert _run(repo, "--id", "go", "--step", "1", "--item", "Build the button.") == 0
    assert "- [ ] Build the button. (en curso: go)" in _plan(repo)
    assert _commits(repo)[0] == "Roadmap: start in step 1 (go)" and len(_commits(repo)) == before + 1
    lane = read_json(home / "lanes" / "go.json", {})
    assert lane["item"] == "Build the button." and lane["item_step"] == "1"


def test_item_can_be_given_by_its_id(proj, home: Path) -> None:
    repo, _ = proj
    assert _run(repo, "--id", "byid", "--step", "1", "--item", roadmap.item_id("Write the docs.")) == 0
    assert read_json(home / "lanes" / "byid.json", {})["item"] == "Write the docs."


def test_an_item_a_live_lane_has_cannot_start_twice(proj, home: Path, capsys) -> None:
    repo, _ = proj
    import os
    write_plan(repo, PLAN.replace("Build the button.", "Build the button. (en curso: busy)"))
    register_lane(home, lane_id="busy", project="demo", hero_class="warden", feature="x", workspace=None, log=None,
                  pid=os.getpid(), brief=None, step="1")
    assert _run(repo, "--id", "second", "--step", "1", "--item", "Build the button.") == 3
    assert "busy" in capsys.readouterr().err  # the duplicate-lane rule or the item check
    done = read_json(home / "lanes" / "busy.json", {})
    done["ended_at"] = "2026-01-01T00:00:00Z"
    write_json_atomic(home / "lanes" / "busy.json", done)
    assert _run(repo, "--id", "third", "--step", "1", "--item", "Build the button.") == 0


@pytest.mark.parametrize("note,why", [("(en revisión)", "in review"), ("(bloqueado: no key)", "blocked (no key)")])
def test_review_and_blocked_items_do_not_start(proj, home: Path, capsys, note, why) -> None:
    repo, _ = proj
    write_plan(repo, PLAN.replace("Build the button.", f"Build the button. {note}"))
    assert _run(repo, "--id", "x", "--step", "1", "--item", "Build the button.") == 3
    assert why in capsys.readouterr().err


def test_a_project_without_a_plan_cannot_start_agents(home: Path, tmp_path: Path, capsys, monkeypatch) -> None:
    monkeypatch.setattr("tirith.cli.verify_guard", lambda *a, **k: [])
    repo = make_repo(tmp_path / "bare")
    write_lane_manifest(repo)
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}])
    assert _run(repo, "--id", "x", "--step", "1", "--item", "Anything.") == 3
    assert "no plan file" in capsys.readouterr().err
    make_hooks_mutable(repo)


def test_tirith_launch_needs_an_item(proj, home: Path, capsys) -> None:
    repo, started = proj
    assert main(["launch", "--project", "demo", "--step", "1"]) == 3
    assert "Add it to the plan first" in capsys.readouterr().err and started == []
    assert main(["launch", "--project", "demo", "--step", "1", "--item", "Write the docs."]) == 0
    argv = started[0]
    assert argv[argv.index("--item") + 1] == "Write the docs."
    brief = (home / "briefs" / (argv[argv.index("--id") + 1] + ".md")).read_text(encoding="utf-8")
    assert "## Your item\n- Write the docs." in brief
    assert "tirith plan report --lane $TIRITH_LANE" in brief and "tirith plan add" in brief


def test_tirith_picks_the_item_for_its_own_launches(proj, home: Path) -> None:
    repo, started = proj
    launch_step(home, "demo", "1", out=io.StringIO(), by_tirith=True)
    assert started[0][started[0].index("--item") + 1] == "Build the button."
    launch_step(home, "demo", "2", out=io.StringIO(), auto_item=True)
    assert started[1][started[1].index("--item") + 1] == "Second step item."


def test_tirith_creates_the_items_of_its_own_work(proj, home: Path) -> None:
    repo, started = proj
    lane_id = launch_plan(home, "demo", "plan-refresh", out=io.StringIO(), by_tirith=True)
    text = f"Revisar el plan ({date.today().isoformat()})"
    assert f"- [ ] {text} (en curso: {lane_id})" in _plan(repo)
    assert "## 0. Trabajo automático de Tirith" in _plan(repo)
    argv = started[-1]
    assert argv[argv.index("--item") + 1] == text and argv[argv.index("--item-step") + 1] == "0"
    assert argv[argv.index("--step") + 1] == "plan-refresh"
    verify_id = launch_verify(home, "demo", "agent can do: open the link", out=io.StringIO(), by_tirith=True)
    assert f"- [ ] Verificar: Verify open the link (en curso: {verify_id})" in _plan(repo)
    # the steps of Tirith's own work are never the autopilot's
    built = roadmap.build(repo, "demo", [])
    assert next(s for s in built["steps"] if s["id"] == "0")["status"] != "ready"


def test_the_manager_lane_has_its_standing_item(proj, home: Path) -> None:
    from tirith import managerd
    repo, started = proj
    cfg = load_config(home)
    now = datetime.now(timezone.utc)
    lane_id = managerd.launch(home, cfg, managerd.pending_items(home, now), now,
                              spawn=lambda argv, h, log: started.append(argv))
    argv = started[-1]
    assert argv[argv.index("--item") + 1] == plangate.MANAGER_ITEM
    assert f"{plangate.MANAGER_ITEM} (en curso: {lane_id})" in _plan(repo)
    assert "tirith plan add" in managerd.RULES[1] + managerd.RULES[2]


# -- tirith plan add (the manager's way in) --------------------------------------------------------


def test_plan_add_writes_and_commits_only_the_plan(proj, home: Path, capsys) -> None:
    repo, _ = proj
    (repo / "scratch.txt").write_text("dirty\n", encoding="utf-8")  # not the plan: stays out of the commit
    assert main(["plan", "add", "--project", "demo", "--step", "1", "--text", "Review the web panel"]) == 0
    assert "- [ ] Review the web panel\n" in _plan(repo)
    assert _commits(repo)[0] == "Roadmap: add in step 1 (owner)"
    status = subprocess.run([GIT, "-C", str(repo), "status", "--porcelain"], capture_output=True, text=True).stdout
    assert "scratch.txt" in status and "TODO.md" not in status
    # an item that was added can start an agent
    assert _run(repo, "--id", "rv", "--step", "1", "--item", "Review the web panel") == 0


def test_plan_add_names_the_lane_and_refuses_bad_input(proj, home: Path, capsys, monkeypatch) -> None:
    repo, _ = proj
    monkeypatch.setenv("TIRITH_LANE", "manager-ab12")
    assert main(["plan", "add", "--project", "demo", "--step", "1", "--text", "X"]) == 0
    assert _commits(repo)[0] == "Roadmap: add in step 1 (manager-ab12)"
    assert main(["plan", "add", "--project", "demo", "--step", "9", "--text", "X"]) == 3
    assert "step 9 is not in the plan" in capsys.readouterr().err
    assert main(["plan", "add", "--project", "demo", "--step", "9", "--title", "New step", "--text", "Y"]) == 0
    assert "## 9. New step\n\n- [ ] Y\n" in _plan(repo)
    assert main(["plan", "add", "--project", "nope", "--step", "1", "--text", "X"]) == 3


def test_plan_add_refuses_a_plan_with_uncommitted_changes(proj, home: Path, capsys) -> None:
    repo, _ = proj
    (repo / "docs" / "TODO.md").write_text(_plan(repo) + "\n- [ ] by hand\n", encoding="utf-8")
    assert main(["plan", "add", "--project", "demo", "--step", "1", "--text", "X"]) == 3
    assert "uncommitted changes" in capsys.readouterr().err


def test_the_writer_lock_serializes_changes(proj, home: Path) -> None:
    repo, _ = proj
    before = len(_commits(repo))
    errors: list[Exception] = []

    def add(n: int) -> None:
        try:
            plangate.add_item(home, repo, "demo", "1", f"Parallel item {n}", "owner")
        except Exception as err:  # noqa: BLE001
            errors.append(err)

    threads = [threading.Thread(target=add, args=(n,)) for n in range(6)]
    [t.start() for t in threads]
    [t.join() for t in threads]
    assert not errors
    assert all(f"- [ ] Parallel item {n}\n" in _plan(repo) for n in range(6))
    assert len(_commits(repo)) == before + 6


# -- finish: an outcome, no exceptions --------------------------------------------------------------


def test_report_done_marks_the_item_in_review(proj, home: Path, capsys) -> None:
    repo, _ = proj
    _lane(home, repo)
    assert main(["plan", "report", "--lane", "l1", "--status", "done"]) == 0
    assert "- [ ] Build the button. (en revisión)" in _plan(repo)
    assert _commits(repo)[0] == "Roadmap: report done in step 1 (l1)"
    assert read_json(home / "lanes" / "l1.json", {})["outcome"]["status"] == "done"


def test_report_done_can_add_more_items(proj, home: Path) -> None:
    repo, _ = proj
    _lane(home, repo)
    assert main(["plan", "report", "--lane", "l1", "--status", "done", "--add", "Polish the button", "--add",
                 "Polish the button", "--add", "Test the button"]) == 0
    text = _plan(repo)
    assert text.count("Polish the button") == 1 and "- [ ] Test the button" in text


def test_report_partial_needs_the_rest_as_items(proj, home: Path, capsys) -> None:
    repo, _ = proj
    _lane(home, repo)
    assert main(["plan", "report", "--lane", "l1", "--status", "partial"]) == 3
    assert "--add" in capsys.readouterr().err
    assert main(["plan", "report", "--lane", "l1", "--status", "partial", "--add", "The other half"]) == 0
    text = _plan(repo)
    assert "Build the button. (en revisión)" in text and "- [ ] The other half" in text


def test_report_blocked_annotates_with_the_reason(proj, home: Path, capsys) -> None:
    repo, _ = proj
    _lane(home, repo)
    assert main(["plan", "report", "--lane", "l1", "--status", "blocked"]) == 3
    assert "--reason" in capsys.readouterr().err
    assert main(["plan", "report", "--lane", "l1", "--status", "blocked", "--reason", "the API is down (503)"]) == 0
    assert "- [ ] Build the button. (bloqueado: the API is down 503)" in _plan(repo)
    # a blocked lane is not reviewed or merged
    assert roadmap.agent_items(roadmap.parse(_plan(repo))[0], date.today()) == ["Write the docs."]


def test_report_blocked_on_the_owner_uses_the_owner_tag(proj, home: Path) -> None:
    repo, _ = proj
    _lane(home, repo)
    assert main(["plan", "report", "--lane", "l1", "--status", "blocked", "--reason",
                 "credentials: the store login"]) == 0
    item = next(i for i in roadmap.parse(_plan(repo))[0]["items"] if i["text"].startswith("Build"))
    assert "(dueño: credentials: the store login)" in item["text"]
    assert roadmap.tag(item["text"])[0] == "owner"


def test_report_needs_a_known_lane_with_an_item(proj, home: Path, capsys) -> None:
    repo, _ = proj
    assert main(["plan", "report", "--lane", "ghost", "--status", "done"]) == 3
    register_lane(home, lane_id="bare", project="demo", hero_class="warden", feature="x", workspace=None, log=None,
                  pid=None, brief=None)
    assert main(["plan", "report", "--lane", "bare", "--status", "done"]) == 3
    assert "not linked to a plan item" in capsys.readouterr().err
    assert main(["plan", "report", "--status", "done"]) == 3  # no lane and no $TIRITH_LANE


def test_report_is_kept_when_the_plan_is_dirty_and_written_later(proj, home: Path) -> None:
    repo, _ = proj
    _lane(home, repo)
    clean = _plan(repo)
    (repo / "docs" / "TODO.md").write_text(clean + "\n", encoding="utf-8")
    assert main(["plan", "report", "--lane", "l1", "--status", "done"]) == 0
    assert read_json(home / "lanes" / "l1.json", {})["outcome"]["applied"] is False
    subprocess.run([GIT, "-C", str(repo), "checkout", "--", "docs/TODO.md"], check=True)
    assert plangate.sync(home, load_config(home), [{"id": "l1"}])
    assert "Build the button. (en revisión)" in _plan(repo)
    assert read_json(home / "lanes" / "l1.json", {})["outcome"]["applied"] is True


def test_a_lane_that_ends_without_a_report_has_no_outcome(proj, home: Path, capsys) -> None:
    repo, _ = proj
    assert _run(repo, "--id", "mute", "--step", "1", "--item", "Build the button.",
                code="import json;print(json.dumps({'type':'result','result':'did things'}))") == 0
    lane = read_json(home / "lanes" / "mute.json", {})
    assert lane["result"] == "no-outcome" and lane["report"] == "did things"


def test_a_lane_that_reports_ends_done(proj, home: Path, capsys) -> None:
    import tirith
    repo, _ = proj
    collector = Path(tirith.__file__).resolve().parent.parent
    code = (f"import sys; sys.path.insert(0, {str(collector)!r}); from tirith.cli import main; "
            "sys.exit(main(['plan', 'report', '--status', 'done']))")  # $TIRITH_LANE names the lane
    assert _run(repo, "--id", "ok", "--step", "1", "--item", "Build the button.", code=code) == 0
    lane = read_json(home / "lanes" / "ok.json", {})
    assert lane["result"] == "done" and lane["outcome"]["status"] == "done"
    assert "Build the button. (en revisión)" in _plan(repo)


def test_no_outcome_is_started_again_once_then_goes_to_the_manager(proj, home: Path) -> None:
    repo, _ = proj
    _lane(home, repo, "first", result="no-outcome", ended_at="2026-01-01T00:00:00Z", agent="anthropic")
    calls: list[tuple] = []

    def relauncher(home_, lane, alias, note=None):
        calls.append((lane["id"], alias, note))
        return "second"

    cfg = load_config(home)
    assert plangate.sync(home, cfg, [{"id": "first"}], relauncher)
    assert len(calls) == 1 and calls[0][1] == "anthropic" and "without recording its outcome" in calls[0][2]
    assert read_json(home / "lanes" / "first.json", {})["no_outcome_handled"] == "second"
    assert "(reabierto: first terminó sin informe)" in _plan(repo)
    assert not asks_mod.queue_items(home, only_open=True)
    assert not plangate.sync(home, cfg, [{"id": "first"}], relauncher)  # once
    _lane(home, repo, "second", item="Build the button.", result="no-outcome", ended_at="2026-01-01T00:00:00Z",
          retry_of="first")
    assert plangate.sync(home, cfg, [{"id": "second"}], relauncher)
    assert len(calls) == 1  # not started a third time
    queue = asks_mod.queue_items(home, only_open=True)
    assert [q["id"] for q in queue] == ["sq-no-outcome-second"] and "twice" in queue[0]["text"]


def test_relaunch_keeps_the_item_and_adds_the_note(proj, home: Path) -> None:
    from tirith.cli import relaunch_lane
    repo, started = proj
    brief = home / "briefs" / "old.md"
    brief.parent.mkdir(parents=True, exist_ok=True)
    brief.write_text("# Lane: old\n", encoding="utf-8")
    lane = _lane(home, repo, "old", brief=str(brief), feature="Work")
    subprocess.run([GIT, "-C", str(repo), "branch", "lane/old", "HEAD"], check=True)
    new_id = relaunch_lane(home, lane, None, by_tirith=True, note="NOTE: report your outcome")
    argv = started[-1]
    assert argv[argv.index("--item") + 1] == "Build the button." and argv[argv.index("--retry-of") + 1] == "old"
    assert "NOTE: report your outcome" in Path(argv[argv.index("--brief") + 1]).read_text(encoding="utf-8")


def test_standing_lanes_give_their_item_back(proj, home: Path) -> None:
    repo, _ = proj
    plangate.ops_item(home, repo, "demo", plangate.MANAGER_ITEM, "manager-1")
    lane = {"id": "manager-1", "project": "demo", "class": "steward", "result": "done", "step": None,
            "item": plangate.MANAGER_ITEM, "item_step": "0"}
    assert plangate.lane_ended(home, lane) == "done"
    assert f"- [ ] {plangate.MANAGER_ITEM}\n" in _plan(repo)
    plangate.ops_item(home, repo, "demo", "Chat con el dueño: chat-1", "chat-1")
    chat = {"id": "chat-1", "project": "demo", "class": "chat", "result": "done", "item": "Chat con el dueño: chat-1",
            "item_step": "0"}
    assert plangate.lane_ended(home, chat) == "done"
    assert "- [x] Chat con el dueño: chat-1" in _plan(repo)


# -- the reviews ------------------------------------------------------------------------------------


def test_review_started_adds_a_sub_item_and_a_pass_ticks_both(proj, home: Path) -> None:
    repo, _ = proj
    lane = _lane(home, repo)
    assert main(["plan", "report", "--lane", "l1", "--status", "done"]) == 0
    plangate.review_started(home, repo, lane)
    text = _plan(repo)
    assert "- [ ] Build the button. (en revisión)\n  - [ ] Revisar: Build the button. (en curso: revisión de l1)" in text
    assert plangate.review_passed(home, repo, lane).startswith("ticked")
    text = _plan(repo)
    assert "- [x] Build the button.\n" in text and "  - [x] Revisar: Build the button.\n" in text
    assert _commits(repo)[0] == "Roadmap: done in step 1 (l1)"


def test_a_failed_review_reopens_the_item_with_the_reason(proj, home: Path) -> None:
    repo, _ = proj
    lane = _lane(home, repo)
    assert main(["plan", "report", "--lane", "l1", "--status", "done"]) == 0
    plangate.review_started(home, repo, lane)
    plangate.review_failed(home, repo, lane, "the tests failed: 2 failed")
    text = _plan(repo)
    assert "- [ ] Build the button. (reabierto: the tests failed: 2 failed)" in text
    assert "  - [x] Revisar: Build the button. — falló: the tests failed: 2 failed" in text
    assert roadmap.agent_items(roadmap.parse(text)[0], date.today())[0].startswith("Build the button.")


def test_the_claim_check_needs_the_work_on_main(proj, home: Path) -> None:
    repo, _ = proj
    subprocess.run([GIT, "-C", str(repo), "branch", "lane/l1", "HEAD"], check=True)
    (repo / "x.txt").write_text("x", encoding="utf-8")
    subprocess.run([GIT, "-C", str(repo), "add", "x.txt"], check=True)
    subprocess.run([GIT, "-C", str(repo), "commit", "-q", "-m", "more"], check=True)
    subprocess.run([GIT, "-C", str(repo), "checkout", "-q", "-b", "side", "lane/l1"], check=True)
    (repo / "y.txt").write_text("y", encoding="utf-8")
    subprocess.run([GIT, "-C", str(repo), "add", "y.txt"], check=True)
    subprocess.run([GIT, "-C", str(repo), "commit", "-q", "-m", "only on the lane"], check=True)
    subprocess.run([GIT, "-C", str(repo), "branch", "-f", "lane/l1", "side"], check=True)
    subprocess.run([GIT, "-C", str(repo), "checkout", "-q", "-"], check=True)
    lane = _lane(home, repo)
    assert not plangate.on_main(repo, lane)
    assert "not on main" in plangate.review_passed(home, repo, lane)
    assert "- [ ] Build the button. (reabierto: the lane's work is not on main)" in _plan(repo)


def test_autoreview_ticks_on_merge_and_reopens_on_failure(proj, home: Path) -> None:
    from tirith import autoreview, review as review_mod
    repo, _ = proj
    cfg = load_config(home)
    lane = _lane(home, repo)
    assert main(["plan", "report", "--lane", "l1", "--status", "done"]) == 0
    now = datetime.now(timezone.utc)
    rec: dict = {}
    autoreview.settle(home, cfg, lane, {"status": review_mod.MERGED, "files": 1, "passed": 3, "at": "t1"}, rec, now,
                      repo)
    assert "- [x] Build the button.\n" in _plan(repo)
    lane2 = _lane(home, repo, "l2", item="Write the docs.")
    assert main(["plan", "report", "--lane", "l2", "--status", "done"]) == 0
    autoreview.settle(home, cfg, lane2, {"status": review_mod.TESTS_FAILED, "message": "2 failed", "at": "t2"}, {},
                      now, repo)
    assert "- [ ] Write the docs. (reabierto: the tests failed: 2 failed)" in _plan(repo)


# -- the gate on the plan file -----------------------------------------------------------------------


@pytest.fixture
def gate(proj, home: Path):
    repo, _ = proj
    lane_dir = repo.parent / "demo-lanes" / "w"
    lane_dir.mkdir(parents=True)
    (lane_dir / "tirith.toml").write_text((repo / "tirith.toml").read_text(encoding="utf-8"), encoding="utf-8")
    return repo, lane_dir, load_config(home)


@pytest.mark.parametrize("tool,key", [("Edit", "file_path"), ("Write", "file_path"), ("MultiEdit", "file_path")])
def test_agents_cannot_edit_the_plan_with_the_edit_tools(gate, tool, key) -> None:
    repo, lane_dir, cfg = gate
    for path in (lane_dir / "docs" / "TODO.md", repo / "docs" / "TODO.md"):
        found = gates.check_call(cfg, tool, {key: str(path)}, lane_dir)
        assert [v.rule for v in found] == ["roadmap-edit"], path
        assert "tirith plan add" in found[0].message() and "tirith plan report" in found[0].message()
    assert gates.check_call(cfg, "Edit", {"file_path": str(lane_dir / "README.md")}, lane_dir) == []
    assert gates.check_call(cfg, "Edit", {"file_path": "docs/TODO.md"}, lane_dir)[0].rule == "roadmap-edit"


@pytest.mark.parametrize("command", [
    "sed -i '' 's/a/b/' docs/TODO.md",
    "echo '- [ ] x' >> docs/TODO.md",
    "cat > docs/TODO.md <<'EOF'\nnew\nEOF",
    "rm docs/TODO.md",
    "git checkout -- docs/TODO.md",
    "python3 - docs/TODO.md",
])
def test_agents_cannot_write_the_plan_from_the_shell(gate, command) -> None:
    _, lane_dir, cfg = gate
    assert "roadmap-edit" in [v.rule for v in gates.check_call(cfg, "Bash", {"command": command}, lane_dir)]


@pytest.mark.parametrize("command", [
    "cat docs/TODO.md", "grep -n step docs/TODO.md", "git diff docs/TODO.md", "git log -p -- docs/TODO.md",
    "tirith plan add --project demo --step 1 --text 'x'",
    "tirith plan report --lane l1 --status done --add 'docs/TODO.md follow-up'",
])
def test_readers_and_the_plan_commands_are_fine(gate, command) -> None:
    _, lane_dir, cfg = gate
    assert gates.check_call(cfg, "Bash", {"command": command}, lane_dir) == []


def test_the_gate_follows_a_configured_roadmap_file(proj, home: Path) -> None:
    repo, _ = proj
    (repo / "tirith.toml").write_text('roadmap = "plans/main.md"\nplans = ["plans/beta.md"]\n'
                                      + (repo / "tirith.toml").read_text(encoding="utf-8"), encoding="utf-8")
    cfg = load_config(home)
    for name in ("plans/main.md", "plans/beta.md"):
        assert gates.check_call(cfg, "Write", {"file_path": str(repo / name)}, repo)[0].rule == "roadmap-edit"
    assert gates.check_call(cfg, "Write", {"file_path": str(repo / "docs" / "TODO.md")}, repo) == []


def test_the_rule_has_a_plain_description() -> None:
    from tirith import manifest
    assert "roadmap-edit" in manifest.BUILT_IN_RULES and gates.DESCRIPTIONS["roadmap-edit"]


# -- the dispatcher -----------------------------------------------------------------------------------


def test_the_owner_plan_edit_goes_through_the_single_writer(proj, home: Path) -> None:
    repo, _ = proj
    req = {"project": "demo", "step": "1", "edit": {"plan": "", "item_text": "Write the docs.",
                                                    "new_text": "Write all the docs."}}
    assert dispatch_mod._apply_edit(home, repo, req) == "Roadmap: edit in step 1 (owner)"
    assert "- [ ] Write all the docs." in _plan(repo)
    assert (home / "locks" / "roadmap-demo.lock").exists()


def test_the_autopilot_skips_tirith_own_step(proj, home: Path) -> None:
    from tirith import autopilot
    repo, _ = proj
    plangate.ops_item(home, repo, "demo", "Reopened chore", None)
    built = roadmap.build(repo, "demo", [])
    ops = next(s for s in built["steps"] if s["id"] == "0")
    assert ops["status"] == "todo"
    assert autopilot._agent_open(next(s for s in built["steps"] if s["id"] == "1")) == [
        "Build the button.", "Write the docs."]


def test_an_abandoned_git_lock_in_the_main_checkout_is_recovered(proj, home: Path) -> None:
    import os
    repo, _ = proj
    lock = repo / ".git" / "index.lock"
    lock.write_text("", encoding="utf-8")
    assert plangate.recover_git_lock(repo) is False and lock.exists()  # fresh: a git command may hold it
    old = time.time() - 3600
    os.utime(lock, (old, old))
    assert plangate.commit_change(home, repo, "demo", lambda t: t + "\n- [ ] More.\n", "Roadmap: test") is True
    assert not lock.exists()
