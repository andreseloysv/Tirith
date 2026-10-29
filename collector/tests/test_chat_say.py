"""`tirith chat say`: the manager writes to the owner in the chat."""

from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path

import pytest

from conftest import write_config
from tirith import inbox as inbox_mod
from tirith.cli import main
from tirith.collector import Collector
from tirith.store import read_jsonl

NOW = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)


def _manual(state: dict) -> list[dict]:
    return [w for w in state["waiting_on_you"] if w["source"] == "manual"]


def test_plain_message_is_a_say_row_in_the_state(home: Path, capsys) -> None:
    write_config(home, [{"id": "demo", "name": "Demo"}])
    assert main(["chat", "say", "The **release** is merged.", "--project", "demo"]) == 0
    say_id = capsys.readouterr().out.strip().splitlines()[-1]
    (row,) = read_jsonl(home / "inbox.jsonl")
    assert row["id"] == say_id and row["kind"] == "say" and row["from"] == "manager"
    state = Collector(home, client=None).build_state(NOW)
    assert [(s["id"], s["project"], s["text"]) for s in state["says"]] == [(say_id, "demo", "The **release** is merged.")]
    assert state["inbox"] == [] and _manual(state) == []
    assert inbox_mod.requests(home) == []   # not a request: nothing for the manager to answer


def test_message_with_options_or_actions_is_an_owner_item(home: Path, capsys) -> None:
    write_config(home, [{"id": "demo", "name": "Demo"}])
    assert main(["chat", "say", "--why-owner", "owner-decision", "Ship it?", "--project", "demo", "--option", "Ship|ship"]) == 0
    assert main(["chat", "say", "--why-owner", "owner-decision", "Run the check.", "--project", "demo", "--run", "make check", "--label", "Check"]) == 0
    waits = _manual(Collector(home, client=None).build_state(NOW))
    assert sorted(w["text"] for w in waits) == ["Run the check.", "Ship it?"]
    assert next(w for w in waits if w["text"] == "Ship it?")["options"] == [{"label": "Ship", "value": "ship"}]
    assert next(w for w in waits if w["text"] == "Run the check.")["actions"][0]["command"] == "make check"
    assert not [r for r in read_jsonl(home / "inbox.jsonl") if r.get("kind") == "say"]


def test_same_rules_as_wait_add(home: Path) -> None:
    write_config(home, [{"id": "demo", "name": "Demo"}])
    assert main(["chat", "say", "--why-owner", "owner-decision", "Please run `rm -rf build` now", "--option", "Ok|ok"]) == 2
    assert not (home / "waits.jsonl").exists() or read_jsonl(home / "waits.jsonl") == []
    with pytest.raises(SystemExit):
        main(["chat", "say", "   "])
    assert not (home / "inbox.jsonl").exists() or read_jsonl(home / "inbox.jsonl") == []
