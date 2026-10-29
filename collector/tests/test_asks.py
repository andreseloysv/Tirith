"""Asks: `wait add` with options, lane-ending rules, the owner's reply path and the steward queue."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

import pytest

from conftest import write_config
from tirith import asks as asks_mod
from tirith import inbox as inbox_mod
from tirith.cli import end_lane, main, register_lane
from tirith.collector import Collector
from tirith.store import read_jsonl

NOW = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)


def _state(home: Path) -> dict:
    return Collector(home, client=None).build_state(NOW)


def _manual(waits: list[dict]) -> list[dict]:
    return [w for w in waits if w["source"] == "manual"]


def test_wait_add_stores_options_command_kind(home: Path, capsys) -> None:
    write_config(home, [{"id": "demo", "name": "Demo"}])
    assert main(["wait", "add", "--project", "demo", "--text", "Ship **v2**?",
                 "--option", "Ship it|ship", "--option", "Hold", "--command", "make ship",
                 "--link", "https://example.com", "--kind", "approval"]) == 0
    wait_id = capsys.readouterr().out.strip().splitlines()[-1]
    (row,) = read_jsonl(home / "waits.jsonl")
    assert row["id"] == wait_id and row["kind"] == "approval" and row["command"] == "make ship"
    assert row["options"] == [{"label": "Ship it", "value": "ship"}, {"label": "Hold", "value": "Hold"}]
    (wait,) = _manual(_state(home)["waiting_on_you"])
    assert wait["options"] == row["options"] and wait["kind"] == "approval" and wait["status"] == "open"
    assert wait["link"] == "https://example.com"


def test_plain_wait_keeps_its_old_shape(home: Path, capsys) -> None:
    assert main(["wait", "add", "Tick the box", "--project", "demo"]) == 0
    (row,) = read_jsonl(home / "waits.jsonl")
    assert "options" not in row and "kind" not in row


def test_wait_add_rejects_bad_input(home: Path) -> None:
    with pytest.raises(SystemExit):
        main(["wait", "add", "--project", "demo"])
    with pytest.raises(SystemExit):
        main(["wait", "add", "q", "--option", "|x"])


def test_options_are_capped_and_labels_cut() -> None:
    opts = asks_mod.clean_options([f"L{i}|v{i}" for i in range(9)] + [{"label": "x" * 200, "value": "v"}])
    assert len(opts) == asks_mod.MAX_OPTIONS
    assert len(asks_mod.clean_options([{"label": "y" * 200}])[0]["label"]) == asks_mod.LABEL_MAX


def test_reply_marks_the_ask_answered_and_reaches_the_inbox(home: Path, capsys) -> None:
    write_config(home, [{"id": "demo", "name": "Demo"}])
    main(["wait", "add", "Which?", "--project", "demo", "--option", "A|a", "--option", "B|b", "--id", "w-7"])
    capsys.readouterr()
    assert main(["inbox", "reply", "--wait", "w-7", "--value", "b", "--text", "but later"]) == 0
    reply_id = capsys.readouterr().out.strip()
    (row,) = [r for r in read_jsonl(home / "inbox.jsonl") if r["kind"] == "reply"]
    assert row["id"] == reply_id and (row["wait"], row["value"], row["text"]) == ("w-7", "b", "but later")
    assert row["project"] == "demo"
    state = _state(home)
    assert _manual(state["waiting_on_you"]) == []
    (answered,) = state["answered_waits"]
    assert answered["id"] == "w-7" and answered["status"] == "answered"
    assert answered["answer"]["value"] == "b" and answered["answer"]["label"] == "B"
    assert state["inbox"] == []   # replies are not plan requests
    assert read_jsonl(home / "waits.jsonl")[0]["id"] == "w-7"   # the ask is kept, not deleted


def test_reply_needs_a_wait_and_something_to_say(home: Path, capsys) -> None:
    assert main(["inbox", "reply", "--wait", "w-1"]) == 3
    assert "value or a text" in capsys.readouterr().err
    with pytest.raises(inbox_mod.InboxError):
        inbox_mod.add_reply(home, "", value="x")


def test_watch_emits_open_replies_until_closed(home: Path) -> None:
    first = inbox_mod.add_reply(home, "w-1", "demo", "yes")
    inbox_mod.add_reply(home, "w-2", "demo", text="free words")
    inbox_mod.mark_done(home, first["id"])
    seen: list[dict] = []
    inbox_mod.watch(home, seen.append, sleep=lambda _s: None, should_stop=lambda: True)
    assert [(r["kind"], r["wait"], r["text"]) for r in seen] == [("reply", "w-2", "free words")]
    line = json.loads(inbox_mod.watch_line(seen[0]))
    assert line["kind"] == "reply" and line["wait"] == "w-2" and "step" not in line


def test_reply_through_the_url_handler(home: Path) -> None:
    from tirith import launch as launch_mod
    token = launch_mod.launch_token(home)
    url = inbox_mod.build_reply_url("w-3", "demo", "go", "now", token)
    assert inbox_mod.parse_url(url)["wait"] == "w-3"
    assert main(["url", "open", url]) == 0
    (row,) = read_jsonl(home / "inbox.jsonl")
    assert (row["kind"], row["wait"], row["value"], row["text"]) == ("reply", "w-3", "go", "now")
    bad = inbox_mod.build_reply_url("w-3", "demo", "go", "now", "wrong")
    assert main(["url", "open", bad]) == 3
    assert len(read_jsonl(home / "inbox.jsonl")) == 1


# -- lane endings ---------------------------------------------------------------------------------

REPORT = """done: The inbox is built end to end.

## What changed
- inbox.py

## Open
- Decide whether the brain may auto-answer.
- The token needs rotating.

## Tests
318 pass
"""


def _lane(home: Path, lane_id: str = "l1") -> None:
    write_config(home, [{"id": "demo", "name": "Demo"}])
    register_lane(home, lane_id=lane_id, project="demo", hero_class="warden", feature="Inbox",
                  workspace=None, log=None, pid=None, brief=None)


def test_open_section_extraction() -> None:
    text = asks_mod.open_section(REPORT)
    assert "auto-answer" in text and "rotating" in text and "318" not in text
    assert asks_mod.open_section("Open: ask legal first\n\nTests ok") == "ask legal first"
    assert asks_mod.open_section("**Left for you**\n- sign it\n\n**Tests**\nok") == "- sign it"
    assert asks_mod.open_section("## Open\nNone\n") == ""
    assert asks_mod.open_section("All good, nothing left.") == ""
    assert asks_mod.open_section(None) == ""


def test_open_section_makes_a_steward_item_and_no_owner_wait(home: Path) -> None:
    _lane(home)
    end_lane(home, "l1", "done", REPORT)
    assert not (home / "waits.jsonl").exists()
    (item,) = asks_mod.queue_items(home, only_open=True)
    assert item["kind"] == "review" and item["to"] == "steward" and item["lane"] == "l1"
    assert "auto-answer" in item["text"] and item["project"] == "demo"
    state = _state(home)
    assert _manual(state["waiting_on_you"]) == []
    assert [q["id"] for q in state["steward_queue"]] == [item["id"]]


def test_report_without_open_section_queues_nothing(home: Path) -> None:
    _lane(home)
    end_lane(home, "l1", "done", "done: all good\n\n## Tests\n318 pass")
    assert asks_mod.queue_items(home) == []


def test_queue_cli_lists_and_closes(home: Path, capsys) -> None:
    _lane(home)
    end_lane(home, "l1", "done", REPORT)
    assert main(["queue", "list", "--json"]) == 0
    (item,) = [json.loads(line) for line in capsys.readouterr().out.splitlines()]
    assert main(["queue", "done", item["id"]]) == 0
    assert main(["queue", "list"]) == 0 and capsys.readouterr().out.strip() == ""
    assert asks_mod.queue_items(home, only_open=True) == []
    assert main(["queue", "done", "sq-nope"]) == 3
    assert _state(home)["steward_queue"] == []


def test_failed_lane_end_by_the_steward_adds_no_owner_item(home: Path) -> None:
    _lane(home)
    end_lane(home, "l1", "failed", "boom")
    assert not (home / "waits.jsonl").exists()


# -- noise ----------------------------------------------------------------------------------------


def test_commit_guard_reminder_goes_away_when_the_guard_is_intact(
        home: Path, tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(tmp_path)}])
    main(["wait", "add", "Install the commit guard in the repo", "--project", "demo"])
    main(["wait", "add", "Pick a colour", "--project", "demo"])
    main(["wait", "add", "Reinstall the commit guard?", "--project", "demo", "--option", "Yes|yes"])
    monkeypatch.setattr("tirith.collector.verify_guard", lambda *_a, **_k: [])
    texts = [w["text"] for w in _state(home)["waiting_on_you"] if w["source"] == "manual"]
    assert texts == ["Pick a colour", "Reinstall the commit guard?"]   # a real question stays
    monkeypatch.setattr("tirith.collector.verify_guard", lambda *_a, **_k: ["hook missing"])
    texts = [w["text"] for w in _state(home)["waiting_on_you"] if w["source"] == "manual"]
    assert "Install the commit guard in the repo" in texts
