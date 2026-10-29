"""Editing the plan from the Plan card: ``plan-edit`` requests, applied by code, then one ``plan-refresh``."""

from __future__ import annotations

import json
import subprocess
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest

from conftest import make_repo, write_config, write_lane_manifest, write_plan
from tirith import dispatch as dispatch_mod
from tirith import inbox as inbox_mod
from tirith import roadmap as rm
from tirith.collector import Collector
from tirith.home import load_config

NOW = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)
TODO = """# TODO: main plan

## 1. First

- [x] Done one.
- [ ] Open one
      wrapped on a second line.
- [ ] Open two (dueño: do it)

## 2. Second

- [ ] Lonely item
"""
GIT = ["git", "-c", "user.email=t@example.invalid", "-c", "user.name=Tirith Test"]


def _at(seconds_ago: float) -> str:
    return (NOW - timedelta(seconds=seconds_ago)).strftime("%Y-%m-%dT%H:%M:%SZ")


def _payload(**kw) -> str:
    return json.dumps({"plan": "", **kw})


@pytest.fixture
def project(home: Path, tmp_path: Path, monkeypatch):
    for key, value in (("GIT_AUTHOR_NAME", "t"), ("GIT_AUTHOR_EMAIL", "t@example.invalid"),
                       ("GIT_COMMITTER_NAME", "t"), ("GIT_COMMITTER_EMAIL", "t@example.invalid")):
        monkeypatch.setenv(key, value)
    repo = make_repo(tmp_path / "demo")
    write_lane_manifest(repo)
    write_plan(repo, TODO)
    subprocess.run([*GIT, "-C", str(repo), "add", "."], check=True)
    subprocess.run([*GIT, "-C", str(repo), "commit", "-q", "-m", "plan"], check=True)
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo)}])
    return repo


def _by_id(home: Path) -> dict:
    return {r["id"]: r for r in inbox_mod.requests(home)}


def _run(home: Path) -> None:
    dispatch_mod.run(home, load_config(home), NOW, agents=[], lanes=[], status_of=lambda lane, now: "working",
                     plan_launcher=lambda *a: pytest.fail("a refresh was launched in the same pass"))


def _log(repo: Path) -> list[str]:
    return subprocess.run(["git", "-C", str(repo), "log", "--format=%s"], capture_output=True, text=True).stdout.split("\n")


def _refreshes(home: Path) -> list[dict]:
    return [r for r in inbox_mod.requests(home) if r["kind"] == "plan-refresh"]


# -- roadmap.edit -----------------------------------------------------------------------------------


def test_edit_replaces_one_item_and_keeps_the_rest() -> None:
    new = rm.edit(TODO, "1", "Open one wrapped on a second line.", new_text="Open one, rewritten")
    assert "- [ ] Open one, rewritten\n- [ ] Open two" in new
    assert "wrapped on" not in new and new.count("\n") == TODO.count("\n") - 1
    assert [i["text"] for i in rm.parse(new)[0]["items"]] == ["Done one.", "Open one, rewritten", "Open two (dueño: do it)"]


def test_delete_removes_the_item_with_its_wrapped_lines() -> None:
    new = rm.edit(TODO, "1", "Open one wrapped on a second line.", delete=True)
    assert [i["text"] for i in rm.parse(new)[0]["items"]] == ["Done one.", "Open two (dueño: do it)"]
    assert "wrapped" not in new and rm.parse(new)[1]["items"][0]["text"] == "Lonely item"


def test_add_goes_after_the_last_open_item_of_the_step() -> None:
    new = rm.edit(TODO, "1", add="Brand new")
    assert [i["text"] for i in rm.parse(new)[0]["items"]][-1] == "Brand new"
    assert "- [ ] Open two (dueño: do it)\n- [ ] Brand new\n\n## 2." in new
    assert len(rm.parse(new)[1]["items"]) == 1


def test_add_to_an_empty_step_and_a_step_at_the_end_of_the_file() -> None:
    assert [i["text"] for i in rm.parse(rm.edit("## 1. A\n\n## 2. B\n", "1", add="x"))[0]["items"]] == ["x"]
    assert [i["text"] for i in rm.parse(rm.edit("## 1. A\n- [ ] one", "1", add="two"))[0]["items"]] == ["one", "two"]


def test_edit_refuses_a_changed_or_ticked_item_or_a_missing_step() -> None:
    for item in ("Open one", "Done one.", "gone"):
        with pytest.raises(rm.EditError, match="changed in the plan meanwhile"):
            rm.edit(TODO, "1", item, new_text="x")
    with pytest.raises(rm.EditError, match="step 9"):
        rm.edit(TODO, "9", add="x")
    with pytest.raises(rm.EditError, match="empty"):
        rm.edit(TODO, "1", "Open two (dueño: do it)", new_text="  ")


def test_edit_only_looks_inside_its_step() -> None:
    with pytest.raises(rm.EditError):
        rm.edit(TODO, "1", "Lonely item", delete=True)


# -- the request ------------------------------------------------------------------------------------


def test_request_needs_one_valid_change(home: Path) -> None:
    for text in ("", "nope", "{}", _payload(item_text="a"), _payload(add="x", item_text="a"),
                 _payload(item_text="a", new_text="b", delete=True), _payload(add="  ")):
        with pytest.raises(inbox_mod.InboxError):
            inbox_mod.add_request(home, "plan-edit", "demo", "1", text)
    with pytest.raises(inbox_mod.InboxError):
        inbox_mod.add_request(home, "plan-edit", "demo", "", _payload(add="x"))
    rid = inbox_mod.add_request(home, "plan-edit", "demo", "1", _payload(item_text="a", delete=True))["id"]
    assert _by_id(home)[rid]["edit"] == {"plan": "", "item_text": "a", "delete": True}
    assert inbox_mod.edit_verb(_by_id(home)[rid]["edit"]) == "delete"


# -- the dispatcher ---------------------------------------------------------------------------------


@pytest.mark.parametrize("payload, verb, expect", [
    (dict(item_text="Open two (dueño: do it)", new_text="Open two, new"), "edit", "- [ ] Open two, new\n"),
    (dict(item_text="Open two (dueño: do it)", delete=True), "delete", None),
    (dict(add="Fresh item"), "add", "- [ ] Fresh item\n"),
])
def test_dispatcher_applies_and_commits_only_the_plan(project, home: Path, payload, verb, expect) -> None:
    (project / "README.md").write_text("# touched\n", encoding="utf-8")  # unrelated work stays uncommitted
    rid = inbox_mod.add_request(home, "plan-edit", "demo", "1", _payload(**payload), at=_at(5))["id"]
    _run(home)
    text = (project / "docs" / "TODO.md").read_text(encoding="utf-8")
    assert (expect in text) if expect else ("Open two" not in text)
    assert _log(project)[0] == f"Roadmap: {verb} in step 1 (owner)"
    changed = subprocess.run(["git", "-C", str(project), "show", "--name-only", "--format=", "HEAD"],
                             capture_output=True, text=True).stdout.split()
    assert changed == ["docs/TODO.md"]
    req = _by_id(home)[rid]
    assert req["status"] == "answered" and req["manager_pending"] is False
    assert req["answers"][0]["by"] == "tirith" and "checked again" in req["answers"][0]["text"]


def test_a_mismatch_is_refused_with_a_clear_answer_and_changes_nothing(project, home: Path) -> None:
    before = _log(project)
    rid = inbox_mod.add_request(home, "plan-edit", "demo", "1", _payload(item_text="Open two", new_text="x"),
                                at=_at(5))["id"]
    _run(home)
    req = _by_id(home)[rid]
    assert "changed in the plan meanwhile" in req["answers"][0]["text"] and req["answers"][0]["text"].startswith("Could not")
    assert (project / "docs" / "TODO.md").read_text(encoding="utf-8") == TODO
    assert _log(project) == before and _refreshes(home) == []


def test_uncommitted_plan_is_left_alone(project, home: Path) -> None:
    (project / "docs" / "TODO.md").write_text(TODO + "\n- [ ] mine\n", encoding="utf-8")
    rid = inbox_mod.add_request(home, "plan-edit", "demo", "2", _payload(add="x"), at=_at(5))["id"]
    _run(home)
    assert "uncommitted changes" in _by_id(home)[rid]["answers"][0]["text"]
    assert "- [ ] x" not in (project / "docs" / "TODO.md").read_text(encoding="utf-8")


def test_unknown_plan_and_project_are_refused(project, home: Path) -> None:
    a = inbox_mod.add_request(home, "plan-edit", "demo", "1", json.dumps({"plan": "nope", "add": "x"}), at=_at(5))["id"]
    b = inbox_mod.add_request(home, "plan-edit", "ghost", "1", _payload(add="x"), at=_at(4))["id"]
    _run(home)
    rows = _by_id(home)
    assert "was not found" in rows[a]["answers"][0]["text"] and "no repo folder" in rows[b]["answers"][0]["text"]


def test_a_refresh_is_queued_once_per_project(project, home: Path) -> None:
    ids = [inbox_mod.add_request(home, "plan-edit", "demo", "2", _payload(add=f"item {n}"), at=_at(5 - n))["id"]
           for n in range(3)]
    _run(home)
    (refresh,) = _refreshes(home)
    assert refresh["project"] == "demo" and refresh["step"] == "plan" and refresh["answers"] == []
    texts = [_by_id(home)[i]["answers"][0]["text"] for i in ids]
    assert "will be checked again" in texts[0] and all("already queued" in t for t in texts[1:])
    started = []  # the next pass hands the one refresh to the auditor launcher
    dispatch_mod.run(home, load_config(home), NOW, agents=[], lanes=[], status_of=lambda lane, now: "working",
                     plan_launcher=lambda *a: started.append(a) or "auditor-1")
    assert len(started) == 1 and len(_refreshes(home)) == 1 and _log(project)[:3] == ["Roadmap: add in step 2 (owner)"] * 3


def test_the_queued_refresh_then_starts_the_auditor_lane(project, home: Path) -> None:
    inbox_mod.add_request(home, "plan-edit", "demo", "1", _payload(add="x"), at=_at(5))
    _run(home)
    started = []
    dispatch_mod.run(home, load_config(home), NOW, agents=[], lanes=[], status_of=lambda lane, now: "working",
                     plan_launcher=lambda *a: started.append(a) or "auditor-1")
    assert [a[1:3] for a in started] == [("demo", "plan-refresh")]
    # a new edit after that queues a new refresh again (the old one was answered)
    inbox_mod.add_request(home, "plan-edit", "demo", "1", _payload(add="y"), at=_at(2))
    _run(home)
    assert len(_refreshes(home)) == 2


def test_collector_state_shows_the_edit_request(project, home: Path) -> None:
    inbox_mod.add_request(home, "plan-edit", "demo", "1", _payload(add="Shown"), at=_at(5))
    collector = Collector(home, client=None)
    collector.build_state(NOW)
    state = collector.build_state(NOW)
    row = next(r for r in state["inbox"] if r["kind"] == "plan-edit")
    assert row["edit"]["add"] == "Shown" and row["manager_pending"] is False
    plan = next(p for p in state["projects"] if p["id"] == "demo")["roadmap"]
    assert any(i["text"] == "Shown" for s in plan["steps"] if s["id"] == "1" for i in s["items"])
