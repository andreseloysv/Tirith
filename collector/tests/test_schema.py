"""The fixture and freshly collected state must satisfy the JSON schema."""

from __future__ import annotations

import json
import os
from pathlib import Path

import pytest

jsonschema = pytest.importorskip("jsonschema")

from tirith.cli import main  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
SCHEMA = json.loads((ROOT / "collector" / "state.schema.json").read_text(encoding="utf-8"))


def test_fixture_validates() -> None:
    fixture = json.loads((ROOT / "fixtures" / "state.example.json").read_text(encoding="utf-8"))
    jsonschema.validate(fixture, SCHEMA)


def test_collected_state_validates(home: Path) -> None:
    main(["lane", "start", "--project", "demo", "--class", "herald",
          "--feature", "Ship it", "--id", "h-1", "--pid", str(os.getpid())])
    main(["wait", "add", "Press the release button", "--project", "demo", "--option", "Pressed|done"])
    main(["note", "Release train left", "--project", "demo"])
    main(["steward", "Watching the release"])
    main(["collect", "--once"])
    state = json.loads((home / "state.json").read_text(encoding="utf-8"))
    jsonschema.validate(state, SCHEMA)
    # The contract addition: every collected project carries guard status.
    assert all("guard" in project for project in state["projects"])
