"""``steward`` is the old name of ``manager``: old rows, flags and keys still work."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

from tirith import asks, dispatch, inbox as inbox_mod
from tirith.cli import main
from tirith.collector import Collector
from tirith.home import load_config
from tirith.store import read_jsonl

NOW = datetime(2026, 10, 28, 20, 0, tzinfo=timezone.utc)


def _write(path: Path, rows: list[dict]) -> None:
    path.write_text("\n".join(json.dumps(r) for r in rows) + "\n")


def test_old_answer_row_counts_as_the_manager(home: Path) -> None:
    _write(home / "inbox.jsonl", [
        {"id": "r1", "at": "2026-10-28T19:00:00Z", "kind": "ask", "text": "q", "from": "owner"},
        {"id": "r1", "at": "2026-10-28T19:01:00Z", "kind": "answer", "by": "steward", "text": "a"}])
    (req,) = inbox_mod.requests(home)
    assert req["answers"][0]["by"] == "manager"
    assert req["manager_pending"] is False and req["steward_pending"] is False


def test_by_steward_is_accepted_and_stored_as_manager(home: Path, capsys) -> None:
    main(["inbox", "add", "--kind", "ask", "--project", "demo", "--step", "2", "--text", "q"])
    rid = capsys.readouterr().out.strip()
    assert main(["inbox", "answer", rid, "ok", "--by", "steward"]) == 0
    assert [r["by"] for r in read_jsonl(home / "inbox.jsonl") if r["kind"] == "answer"] == ["manager"]


def test_old_flow_rows_read_as_manager(home: Path) -> None:
    _write(home / "flows.jsonl", [{"id": "f-1", "at": "2026-10-28T19:00:00Z", "from": "steward",
                                   "to": "owner", "kind": "message", "text": "hi"}])
    (flow,) = Collector(home, client=None).build_state(NOW)["flows"]
    assert (flow["from"], flow["from_name"]) == ("manager", "The Manager")


def test_flow_command_accepts_both_spellings(home: Path) -> None:
    assert main(["flow", "--from", "steward", "--to", "owner", "--kind", "message", "--text", "x"]) == 0
    assert main(["flow", "--from", "manager", "--to", "owner", "--kind", "message", "--text", "y"]) == 0


def test_state_has_new_and_old_keys_with_the_same_values(home: Path) -> None:
    main(["manager", "Watching"])
    state = Collector(home, client=None).build_state(NOW)
    assert state["manager"] == state["steward"] and state["manager"]["activity"] == "Watching"
    assert state["manager"]["title"] == "The Manager"
    assert state["manager_queue"] == state["steward_queue"]


def test_steward_command_is_a_hidden_alias(home: Path, capsys) -> None:
    assert main(["steward", "Old habit"]) == 0
    assert json.loads((home / "orchestrator.json").read_text())["activity"] == "Old habit"
    assert "manager updated" in capsys.readouterr().out


def test_old_config_table_and_role_still_work(home: Path, tmp_path: Path) -> None:
    (home / "config.toml").write_text(f'[steward]\nname = "Old"\nsession_cwd = "{tmp_path}"\n')
    config = load_config(home)
    assert config.manager_name == "Old" and config.manager_title == "The Manager"
    assert dispatch.is_manager(config, {"TIRITH_ROLE": "steward"}, None)
    assert dispatch.is_manager(config, {"TIRITH_ROLE": "manager"}, None)
    assert dispatch.is_manager(config, {}, str(tmp_path))


def test_manager_config_table_wins(home: Path) -> None:
    (home / "config.toml").write_text('[steward]\nname = "Old"\n\n[manager]\nname = "New"\n')
    assert load_config(home).manager_name == "New"


def test_hook_install_role_steward_writes_manager(tmp_path: Path, home: Path) -> None:
    settings = tmp_path / "settings.json"
    assert main(["hook", "install", "--agent", "anthropic", "--role", "steward", "--settings", str(settings)]) == 0
    command = json.loads(settings.read_text())["hooks"]["Stop"][0]["hooks"][0]["command"]
    assert command.startswith("TIRITH_ROLE=manager ")


def test_old_queue_file_is_read(home: Path) -> None:
    _write(home / "steward_queue.jsonl", [{"id": "sq-1", "at": "x", "kind": "review", "text": "t", "to": "steward"}])
    (item,) = asks.queue_items(home)
    assert item["to"] == "manager"
    asks.queue_done(home, "sq-1")
    assert asks.queue_items(home, only_open=True) == []
