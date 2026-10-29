"""Your turn lists only what the manager cannot do, with a reason, and its words match its buttons."""

from __future__ import annotations

from pathlib import Path

import pytest

from tirith import asks as asks_mod
from tirith import autopilot as autopilot_mod
from tirith import dispatch as dispatch_mod
from tirith import roadmap as roadmap_mod
from tirith.store import read_jsonl

AT = "2026-10-29T12:00:00Z"
STEP7 = "Reinstalar la guarda nueva en RelaGame (dueño: physical-device botón Run)."
REINSTALL = "Reinstalar la guarda nueva en RelaGame (dueño: physical-device run `uv run tirith guard reinstall relagame`)."


def _raise(home: Path, text: str, folder: Path | None = None) -> dict:
    project = {"id": "tirith", "name": "Tirith", "folder": str(folder or home)}
    step = {"id": "7", "title": 'Reglas de código ("errores de compilación" para agentes)'}
    item = {"text": text, "done": False, "who": "owner", "owner_item": roadmap_mod.owner_item_id("tirith", "7", text)}
    autopilot_mod._owner_item(home, project, step, item, AT)
    return {r["id"]: r for r in read_jsonl(home / "waits.jsonl")}[item["owner_item"]]


def test_nothing_offers_to_hand_the_item_back() -> None:
    assert [o["label"] for o in asks_mod.GOT_IT_OPTIONS] == ["Got it"]
    for path in (Path(asks_mod.__file__).parent, Path(__file__).parents[2] / "web" / "js"):
        for source in path.glob("*.*s"):
            assert "manager handle it" not in source.read_text(encoding="utf-8").lower(), source


def test_reasons_are_an_explicit_list() -> None:
    assert set(asks_mod.WHY_OWNER) == {"credentials", "payment", "personal-account", "physical-device",
                                       "owner-decision"}
    assert asks_mod.reason_tag("credentials") == "Needs your credentials"
    assert asks_mod.reason_tag("personal-account", "your Apple account") == "Needs your Apple account"
    assert asks_mod.reason_tag("owner-decision", "pick a date") == "Needs your decision: pick a date"
    assert [asks_mod.why_key(k) for k in ("account", "device", "legal", "security", None)] == [
        "personal-account", "physical-device", "owner-decision", None, None]


def test_an_owner_item_without_a_known_reason_goes_to_the_manager(home: Path) -> None:
    assert dispatch_mod.add_owner_item(home, "x", "tirith", "Do it.", [{"label": "Done", "value": "done"}], AT,
                                       why_owner="because") == "manager"
    assert not (home / "waits.jsonl").exists()


def test_command_in_the_tag_becomes_a_run_action_in_the_collector_folder(home: Path, tmp_path: Path) -> None:
    wait = _raise(home, REINSTALL, tmp_path)
    assert wait["text"] == "Reinstalar la guarda nueva en RelaGame."
    assert "Tirith step" not in wait["text"] and "dueño" not in wait["text"]
    (action,) = wait["actions"]
    assert action["command"] == "uv run tirith guard reinstall relagame" and action["cwd"] == str(tmp_path)
    assert action["label"] == "Reinstalar la guarda nueva en RelaGame"
    assert wait["options"] == [] and wait["why_owner"] == "physical-device"
    assert wait["reason_tag"] == "Needs your physical device"
    assert asks_mod.has_call_to_action(wait)


def test_url_in_the_tag_is_an_open_link(home: Path) -> None:
    wait = _raise(home, "Sign in to the portal (dueño: credentials https://portal.example.invalid/login).")
    assert wait["link"] == "https://portal.example.invalid/login" and not wait.get("actions")
    assert [o["label"] for o in wait["options"]] == ["Done"]
    assert wait["text"] == "Sign in to the portal." and wait["why_owner"] == "credentials"


def test_without_command_or_url_it_is_done_or_not_now(home: Path) -> None:
    wait = _raise(home, "Decide the colour (dueño: owner-decision (the palette)).")
    assert [o["label"] for o in wait["options"]] == ["Done", "Not now"] and wait["why_owner"] == "owner-decision"
    assert wait["reason_tag"] == "Needs your decision: the palette"
    assert wait["text"] == "Decide the colour."


def test_text_that_promises_a_missing_button_is_rewritten(home: Path) -> None:
    wait = _raise(home, STEP7)
    assert wait["text"] == "Reinstalar la guarda nueva en RelaGame."
    assert not asks_mod.missing_buttons(wait["text"], wait["options"], wait.get("actions"), wait["link"])


@pytest.mark.parametrize("text", ["Press the Run button.", "Haz clic en Run para seguir", "use el botón Run",
                                  "click Run", "Open button below"])
def test_button_claims_are_detected(text: str) -> None:
    assert asks_mod.button_claims(text)
    assert asks_mod.missing_buttons(text, [{"label": "Done"}], [], None)
    assert not asks_mod.missing_buttons("click Run", [], [{"command": "x"}], None)


def test_the_generator_never_stores_a_text_with_a_missing_button(home: Path) -> None:
    dispatch_mod.add_owner_item(home, "w1", "tirith", "Reinstall it (click Run).",
                                [{"label": "Done", "value": "done"}], AT, why_owner="physical-device")
    (row,) = read_jsonl(home / "waits.jsonl")
    assert not asks_mod.button_claims(row["text"]) and row["text"].startswith("Reinstall it")
