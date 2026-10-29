"""No item and no action is shown twice in "Your turn"."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

from conftest import write_config
from tirith.collector import Collector

NOW = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)

ACTION = {"id": "ac-c5967d8e", "label": "Update ChromaSol's old guard",
          "command": "uv run tirith guard reinstall chromasol --chain",
          "cwd": "/work/collector", "created_by": "steward", "at": "2026-10-29T07:44:39Z"}


def _row(wait_id: str, actions: list[dict]) -> dict:
    return {"id": wait_id, "at": "2026-10-29T07:44:39Z", "project": "demo", "text": "Replace the old guard.",
            "link": None, "done_at": None, "session": "s", "kind": "decision",
            "options": [{"label": "Not now", "value": "later"}], "command": None, "actions": actions}


def _manual(home: Path, rows: list[dict]) -> list[dict]:
    write_config(home, [{"id": "demo", "name": "Demo"}])
    (home / "waits.jsonl").write_text("".join(json.dumps(r) + "\n" for r in rows))
    state = Collector(home, client=None).build_state(NOW)
    return [w for w in state["waiting_on_you"] if w["source"] == "manual"]


def test_repeated_action_is_listed_once(home: Path) -> None:
    other_id = {**ACTION, "id": "ac-other"}  # same command and label under another id
    (wait,) = _manual(home, [_row("guard-demo-3", [ACTION, dict(ACTION), other_id])])
    assert [a["id"] for a in wait["actions"]] == ["ac-c5967d8e"]


def test_same_text_and_project_is_one_item(home: Path) -> None:
    rows = [_row("guard-demo-3", [ACTION]), _row("guard-demo-4", [dict(ACTION)])]
    (wait,) = _manual(home, rows)
    assert len(wait["actions"]) == 1
