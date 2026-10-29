"""The owner gate: nothing reaches the owner unless only the owner can do it (``tirith.ownergate``)."""

from __future__ import annotations

from pathlib import Path

import pytest

from tirith import asks as asks_mod
from tirith import dispatch as dispatch_mod
from tirith import launch as launch_mod
from tirith import ownergate
from tirith import roadmap as roadmap_mod
from tirith import verifylane
from tirith.cli import add_wait, main
from tirith.collector import Collector
from tirith.home import load_config
from tirith.store import read_jsonl

AT = "2026-10-29T12:00:00Z"
OPTION = [{"label": "Done", "value": "done"}]
CHECK = 'Click "Finish this step" to test the tirith:// link'


def _waits(home: Path) -> list[dict]:
    return read_jsonl(home / "waits.jsonl")


def _acd(home: Path) -> list[dict]:
    return [q for q in asks_mod.queue_items(home, only_open=True) if q["kind"] == asks_mod.AGENT_CAN_DO_KIND]


@pytest.mark.parametrize("why, tag", [
    ("credentials", "Needs your credentials"), ("payment", "Needs your payment"),
    ("personal-account", "Needs your personal account"), ("physical-device", "Needs your physical device"),
    ("owner-decision", "Needs your decision")])
def test_each_category_reaches_the_owner(home: Path, why: str, tag: str) -> None:
    assert dispatch_mod.add_owner_item(home, "i-1", "demo", "Do it.", OPTION, AT, why_owner=why) == "owner"
    (row,) = _waits(home)
    assert row["why_owner"] == why and row["reason_tag"] == tag and not _acd(home)


def test_the_reason_is_shown_in_plain_words(home: Path) -> None:
    dispatch_mod.add_owner_item(home, "i-1", "demo", "Sign in.", OPTION, AT, why_owner="personal-account",
                                why_detail="your Apple account")
    (row,) = _waits(home)
    assert row["reason_tag"] == "Needs your Apple account"
    state = Collector(home, client=None).build_state()
    (wait,) = state["waiting_on_you"]
    assert wait["reason_tag"] == "Needs your Apple account" and wait["why_owner"] == "personal-account"


@pytest.mark.parametrize("why", [None, "", "because", "security", "agent"])
def test_a_missing_or_unknown_reason_is_not_an_owner_item(home: Path, why) -> None:
    assert dispatch_mod.add_owner_item(home, "i-1", "demo", CHECK, OPTION, AT, why_owner=why) == "manager"
    assert not (home / "waits.jsonl").exists()
    (queued,) = _acd(home)
    assert queued["text"] == f"agent can do: {CHECK}" and queued["project"] == "demo"


def test_the_same_check_is_queued_once(home: Path) -> None:
    for _ in range(3):
        ownergate.admit(home, "i-1", "demo", CHECK, None)
    assert len(_acd(home)) == 1


def test_wait_add_without_a_reason_goes_to_the_manager(home: Path, capsys) -> None:
    assert main(["wait", "add", CHECK, "--project", "demo", "--option", "Done|done"]) == 0
    err = capsys.readouterr().err
    assert "owner-gate" in err and "credentials" in err
    assert not (home / "waits.jsonl").exists() and len(_acd(home)) == 1
    assert main(["wait", "add", "Pay the fee", "--project", "demo", "--option", "Done|done",
                 "--why-owner", "payment"]) == 0
    (row,) = _waits(home)
    assert row["why_owner"] == "payment"
    assert add_wait(home, "demo", "Pick", options=["A|a"], why_owner="nope") == ""


def test_chat_say_with_options_passes_the_gate_too(home: Path) -> None:
    assert main(["chat", "say", "Look at the Overview", "--project", "demo", "--option", "Ok|ok"]) == 0
    assert not (home / "waits.jsonl").exists() and len(_acd(home)) == 1


def test_a_text_about_something_out_of_plan_is_dropped_and_logged(home: Path) -> None:
    text = "Buy the Windows certificate (fuera de este plan: no Windows build)"
    assert dispatch_mod.add_owner_item(home, "i-1", "demo", text, OPTION, AT, why_owner="payment") == "dropped"
    assert not (home / "waits.jsonl").exists() and not _acd(home)
    (logged,) = read_jsonl(home / ownergate.DROPPED_FILE)
    assert logged["id"] == "i-1" and "Windows certificate" in logged["text"] and logged["via"] == "owner-item"


def test_a_text_that_repeats_an_out_of_plan_item_is_dropped() -> None:
    out = ["Sign the Windows installer with a certificate (fuera de este plan)"]
    assert ownergate.is_out_of_plan("Windows certificate to sign the installer", out)
    assert not ownergate.is_out_of_plan("Open the Overview", out)
    assert not ownergate.is_out_of_plan("anything", [])


def test_a_roadmap_tag_needs_its_reason() -> None:
    plain = roadmap_mod.owner_parts("Click Finish this step (dueño: botón Run).")
    assert plain["why_owner"] is None
    tagged = roadmap_mod.owner_parts("Sign in to the store (dueño: personal-account (Apple ID) https://x.example.invalid).")
    assert tagged["why_owner"] == "personal-account" and tagged["why_detail"] == "Apple ID"
    assert tagged["url"] == "https://x.example.invalid"
    legacy = roadmap_mod.owner_parts("Buy it (dueño: account).")
    assert legacy["why_owner"] == "personal-account"
    assert roadmap_mod.owner_parts("Do it (dueño: security)")["why_owner"] is None  # no longer a reason


REPORT = """Merged.

**Still missing**
- Visibility per agent

**Needs the owner**
- Click "Finish this step" and check the tirith:// link opens
- [personal-account: Apple ID] Sign in to App Store Connect
- [payment] Windows certificate (fuera de este plan: no Windows build)
- [because] Look at the Overview

**Notes**
- [credentials] not in the owner section
"""


def test_a_report_s_owner_list_passes_the_gate(home: Path) -> None:
    out = ownergate.filter_report(home, "demo", "refresh", REPORT)
    owner = out.split("**Needs the owner**")[1].split("**Notes**")[0]
    assert "Sign in to App Store Connect (Needs your Apple ID)" in owner
    assert "tirith://" not in owner and "Windows" not in owner and "Overview" not in owner
    assert "- [credentials] not in the owner section" in out  # other sections are untouched
    assert "3 check(s) an agent can do were handed to the manager" in out
    texts = sorted(q["text"] for q in _acd(home))
    assert len(texts) == 2 and texts[0].startswith("agent can do: Click") and "Overview" in texts[1]
    (dropped,) = read_jsonl(home / ownergate.DROPPED_FILE)
    assert "Windows certificate" in dropped["text"]


def test_a_report_with_only_agent_work_says_nothing_is_the_owners(home: Path) -> None:
    out = ownergate.filter_report(home, "demo", "refresh", "**Needs the owner**\n- Look at the Overview\n")
    assert ownergate.NOTHING in out and "Overview" not in out.split("handed")[0]
    clean = "**Needs the owner**\n- nothing\n"
    assert ownergate.filter_report(home, "demo", "refresh", clean) == clean.rstrip("\n")


def test_a_report_check_about_an_out_of_plan_item_is_dropped(home: Path) -> None:
    out = ownergate.filter_report(
        home, "demo", "r", "**Needs the owner**\n- [payment] Windows signing certificate\n",
        out_items=["Sign the Windows installer with a certificate (fuera de este plan)"])
    assert ownergate.NOTHING in out and not _acd(home)
    assert len(read_jsonl(home / ownergate.DROPPED_FILE)) == 1


def _legacy(home: Path) -> None:
    rows = [
        {"id": "w-1", "at": AT, "project": "demo", "text": "Look at the Overview", "options": OPTION, "done_at": None},
        {"id": "w-2", "at": AT, "project": "demo", "text": "Pay the fee", "options": OPTION, "done_at": None,
         "reason": "payment", "reason_tag": "Needs you: involves a payment"},
        {"id": "w-3", "at": AT, "project": "demo", "text": "Plug in the phone", "options": OPTION, "done_at": None,
         "reason": "device"},
        {"id": "w-4", "at": AT, "project": "demo", "text": "Install it", "options": OPTION, "done_at": None,
         "reason": "security"},
        {"id": "w-5", "at": AT, "project": "demo", "text": "Old", "options": OPTION, "done_at": AT},
        {"id": "w-6", "at": AT, "project": "demo", "text": "Fine", "options": OPTION, "done_at": None,
         "why_owner": "credentials"},
    ]
    (home / "waits.jsonl").write_text("".join(__import__("json").dumps(r) + "\n" for r in rows))


def test_open_items_are_rechecked_once_by_the_gate(home: Path) -> None:
    _legacy(home)
    assert ownergate.migrate(home, AT) == {"kept": 2, "moved": 2, "dropped": 0}
    latest = {r["id"]: r for r in _waits(home)}
    assert latest["w-2"]["why_owner"] == "payment" and latest["w-2"]["reason_tag"] == "Needs your payment"
    assert latest["w-3"]["why_owner"] == "physical-device" and not latest["w-3"]["done_at"]
    for moved in ("w-1", "w-4"):
        assert latest[moved]["done_at"] == AT and latest[moved]["done_reason"] == ownergate.MOVED_REASON
    assert latest["w-6"]["done_at"] is None and latest["w-5"]["done_at"] == AT
    assert sorted(q["text"] for q in _acd(home)) == ["agent can do: Install it", "agent can do: Look at the Overview"]
    assert ownergate.migrate(home, AT) == {"kept": 0, "moved": 0, "dropped": 0}  # nothing left to do


def test_an_out_of_plan_open_item_is_dropped_by_the_migration(home: Path) -> None:
    (home / "waits.jsonl").write_text(
        '{"id": "w-1", "at": "%s", "project": "demo", "text": "Windows certificate (fuera de este plan)", '
        '"options": [], "done_at": null}\n' % AT)
    assert ownergate.migrate(home, AT)["dropped"] == 1 and not _acd(home)
    assert _waits(home)[-1]["done_at"] == AT


def test_the_collector_runs_the_migration_at_startup(home: Path) -> None:
    _legacy(home)
    state = Collector(home, client=None).build_state()
    assert sorted(w["id"] for w in state["waiting_on_you"] if w["source"] == "manual") == ["w-2", "w-3", "w-6"]
    assert len(_acd(home)) == 2


def test_the_collector_never_shows_an_item_without_a_reason(home: Path) -> None:
    collector = Collector(home, client=None)
    shown = collector._enforce_cta([
        {"id": "x-1", "project": "demo", "text": "Look", "source": "manual", "options": OPTION},
        {"id": "x-2", "project": "demo", "text": "Real", "source": "manual", "options": OPTION,
         "why_owner": "owner-decision"}])
    assert [w["id"] for w in shown] == ["x-2"]


# -- the manager queue becomes a lane -----------------------------------------------------------


class Launcher:
    def __init__(self) -> None:
        self.calls: list[tuple[str, str]] = []

    def __call__(self, home: Path, project: str, text: str) -> str:
        self.calls.append((project, text))
        return f"verify-{len(self.calls)}"


def test_an_agent_can_do_entry_becomes_a_verification_lane(home: Path) -> None:
    ownergate.admit(home, "i-1", "demo", CHECK, None)
    launcher = Launcher()
    config = load_config(home)
    assert verifylane.run(home, config, launcher) == 1
    assert launcher.calls == [("demo", f"agent can do: {CHECK}")]
    assert not _acd(home)  # the queue item is closed with the lane id
    (done,) = [q for q in asks_mod.queue_items(home) if q["status"] == "done"]
    assert "verify-1" in done["result"]
    assert verifylane.run(home, config, launcher) == 0 and len(launcher.calls) == 1  # once


def test_no_lane_while_the_disk_is_full_or_for_a_check_without_a_project(home: Path) -> None:
    ownergate.admit(home, "i-1", "demo", CHECK, None)
    ownergate.admit(home, "i-2", None, "Read the agents count", None)
    launcher = Launcher()
    assert verifylane.run(home, load_config(home), launcher, room=False) == 0
    assert verifylane.run(home, load_config(home), launcher) == 1
    assert [p for p, _ in launcher.calls] == ["demo"]
    assert [q["text"] for q in _acd(home)] == ["agent can do: Read the agents count"]


def test_a_refused_verification_lane_stays_with_the_manager(home: Path) -> None:
    ownergate.admit(home, "i-1", "demo", CHECK, None)

    def refuse(home, project, text):
        raise launch_mod.LaunchError("tirith check fails")
    assert verifylane.run(home, load_config(home), refuse) == 0
    assert len(_acd(home)) == 1
    assert verifylane.run(home, load_config(home), Launcher()) == 0  # not tried again every cycle


def test_the_verification_brief_tells_the_lane_how() -> None:
    brief = verifylane.brief("demo", f"agent can do: {CHECK}")
    assert "open \"tirith://" in brief and "playwright" in brief and CHECK in brief
    assert "[owner-decision]" in brief and "No servers" in brief


def test_the_plan_prompt_explains_the_gate() -> None:
    from tirith import planlane
    text = planlane.refresh_brief("demo", "docs/TODO.md", "## 1. A\n", [], [], Path("/h"))
    for key in asks_mod.WHY_OWNER:
        assert f"`{key}`" in text
    assert "fuera de este plan" in text and "agent can do" in text


@pytest.mark.parametrize("text, detail, kind", [
    ("Tests passed for web-panel. Merge it into the project branch?", "choice: merge it or drop it", None),
    ("What now?", "choice: retry, show the log or drop it", None),
    ("Lane web-panel is done.", None, "review"),
    ("Start it again?", None, "relaunch")])
def test_a_merge_drop_review_or_relaunch_question_never_reaches_the_owner(
        home: Path, text: str, detail: str | None, kind: str | None) -> None:
    verdict = dispatch_mod.add_owner_item(home, "i-m", "demo", text, OPTION, AT, why_owner="owner-decision",
                                          why_detail=detail, kind=kind)
    assert verdict == "manager" and _waits(home) == []
    (queued,) = asks_mod.queue_items(home, only_open=True)
    assert queued["kind"] == ownergate.NOT_OWNER_KIND and "not for the owner" in queued["text"]


def test_a_real_decision_still_reaches_the_owner(home: Path) -> None:
    assert dispatch_mod.add_owner_item(home, "i-d", "demo", "Which name should the app have?", OPTION, AT,
                                       why_owner="owner-decision", why_detail="choice: the name") == "owner"
