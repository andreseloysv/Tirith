"""Concrete owner how-to: validation, the gate, the done check (``tirith.howto``)."""

from __future__ import annotations

import json
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest

from tirith import asks as asks_mod
from tirith import dispatch as dispatch_mod
from tirith import howto as howto_mod
from tirith import managerd, planlane, verifylane
from tirith.cli import main
from tirith.collector import Collector
from tirith.store import read_jsonl

AT = "2026-10-29T12:00:00Z"
OPTION = [{"label": "Done", "value": "done"}]
NOTARIZE = {
    "steps": [
        {"text": "Open the Apple page", "url": "https://appleid.apple.com/account/manage"},
        {"text": "Type the app password in the prompt",
         "terminal": "xcrun notarytool store-credentials tirith-notary --apple-id me@example.com --team-id AB12CD34EF"}],
    "done_check": "xcrun notarytool history --keychain-profile tirith-notary",
    "already_done": ["Developer ID certificate exists (Team AB12CD34EF)"]}


def _waits(home: Path) -> list[dict]:
    return read_jsonl(home / "waits.jsonl")


def _concrete(home: Path) -> list[dict]:
    return [q for q in asks_mod.queue_items(home, only_open=True) if q["kind"] == "make-concrete"]


def test_a_concrete_howto_is_kept_and_normalised() -> None:
    clean = howto_mod.clean(NOTARIZE)
    assert clean["steps"][1]["terminal"].startswith("xcrun notarytool store-credentials")
    assert clean["done_check"] == NOTARIZE["done_check"] and len(clean["already_done"]) == 1
    assert howto_mod.clean(None) is None and howto_mod.clean({}) is None
    assert howto_mod.parse(json.dumps(NOTARIZE)) == clean


@pytest.mark.parametrize("text", [
    "<TEAMID>", "YOUR_APPLE_ID", "xxx", "{{team}}", "[your team id]", "TODO", "fill in ..."])
def test_placeholders_are_found(text: str) -> None:
    assert howto_mod.has_placeholder(f"--team-id {text}")


@pytest.mark.parametrize("text", ["--team-id AB12CD34EF", "tirith run --stage test", "echo 'a b' | wc -c"])
def test_real_commands_have_no_placeholder(text: str) -> None:
    assert howto_mod.has_placeholder(text) is None


@pytest.mark.parametrize("bad", [
    {"steps": [{"text": "run", "terminal": "xcrun notarytool --team-id <TEAMID>"}]},
    {"steps": [{"text": "Use YOUR_PASSWORD here"}]},
    {"steps": [{"text": "run", "terminal": "echo 'unclosed"}]},
    {"steps": [{"text": "go", "url": "appleid.apple.com"}]},
    {"steps": [{"text": ""}]},
    {"steps": [{"text": "ok"}], "done_check": "check --id xxx"}])
def test_a_non_concrete_howto_is_refused(bad: dict) -> None:
    with pytest.raises(howto_mod.HowtoError, match="make it concrete: look up"):
        howto_mod.clean(bad)


def test_the_gate_sends_a_placeholder_back_to_the_manager(home: Path) -> None:
    bad = {"steps": [{"text": "run", "terminal": "xcrun notarytool --team-id <TEAMID>"}]}
    assert dispatch_mod.add_owner_item(home, "i-1", "demo", "Notarize.", OPTION, AT,
                                       why_owner="credentials", howto=bad) == "manager"
    assert not (home / "waits.jsonl").exists()
    (queued,) = _concrete(home)
    assert queued["text"].startswith("make it concrete: look up ") and "<TEAMID>" in queued["text"]


def test_the_gate_checks_the_text_of_an_item_with_steps(home: Path) -> None:
    assert dispatch_mod.add_owner_item(home, "i-1", "demo", "Use team <TEAMID>.", OPTION, AT,
                                       why_owner="credentials", howto={"steps": [{"text": "go"}]}) == "manager"
    assert len(_concrete(home)) == 1


def test_a_concrete_howto_reaches_the_owner_and_the_state(home: Path) -> None:
    shown = {**NOTARIZE, "done_check": "false"}  # a real machine may pass the notarize check: never run it in a test
    assert dispatch_mod.add_owner_item(home, "i-1", "demo", "Notarize.", OPTION, AT,
                                       why_owner="credentials", howto=shown) == "owner"
    (row,) = _waits(home)
    assert row["howto"]["done_check"] == "false"
    (wait,) = Collector(home, client=None).build_state()["waiting_on_you"]
    assert wait["howto"]["steps"][1]["terminal"].endswith("AB12CD34EF")


def test_wait_add_takes_howto_json(home: Path, capsys) -> None:
    assert main(["wait", "add", "Notarize", "--project", "demo", "--option", "Done|done",
                 "--why-owner", "credentials", "--howto", json.dumps(NOTARIZE)]) == 0
    (row,) = _waits(home)
    assert row["howto"]["already_done"] == NOTARIZE["already_done"]
    bad = json.dumps({"steps": [{"text": "run", "terminal": "x --id YOUR_ID"}]})
    assert main(["wait", "add", "Again", "--project", "demo", "--option", "Done|done",
                 "--why-owner", "credentials", "--howto", bad]) == 0
    assert len(_waits(home)) == 1 and len(_concrete(home)) == 1


def test_the_done_check_closes_the_item_and_tells_the_manager(home: Path) -> None:
    dispatch_mod.add_owner_item(home, "i-1", "demo", "Notarize.", OPTION, AT, why_owner="credentials", howto=NOTARIZE)
    now = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)
    ran: list[str] = []

    def runner(command: str) -> bool:
        ran.append(command)
        return len(ran) > 1

    assert howto_mod.check_open(home, now, runner) == [] and ran == [NOTARIZE["done_check"]]
    assert howto_mod.check_open(home, now + timedelta(seconds=60), runner) == []  # too soon: not run again
    assert len(ran) == 1
    assert howto_mod.check_open(home, now + timedelta(seconds=300), runner) == ["i-1"]
    assert _waits(home)[-1]["done_reason"] == howto_mod.DONE_REASON and _waits(home)[-1]["done_at"]
    flows = read_jsonl(home / "flows.jsonl")
    assert any(f["to"] == "manager" and "i-1" in f["text"] for f in flows)
    assert howto_mod.check_open(home, now + timedelta(seconds=900), runner) == [] and len(ran) == 2
    state = Collector(home, client=None).build_state()
    assert not state["waiting_on_you"]


def test_the_real_check_command_runs(home: Path) -> None:
    assert howto_mod._run_check("true") and not howto_mod._run_check("false")
    assert not howto_mod._run_check("exit 3")


def test_the_terminal_command_is_found_by_step_number() -> None:
    assert howto_mod.terminal_command(NOTARIZE, 1).startswith("xcrun notarytool store-credentials")
    assert howto_mod.terminal_command(NOTARIZE, 0) is None and howto_mod.terminal_command(NOTARIZE, 9) is None
    script = howto_mod.applescript('echo "a" \\ b')
    assert script == 'tell application "Terminal"\nactivate\ndo script "echo \\"a\\" \\\\ b"\nend tell'


def test_every_agent_that_writes_owner_items_gets_the_rules() -> None:
    for text in ("\n".join(managerd.RULES), "\n".join(planlane.OWNER_GATE_RULES), verifylane.brief("demo", "agent can do: x")):
        assert "look up every fact the machine can tell" in text and "Never ask for a secret in chat" in text
