"""state.js: the same JSON as a script for pages opened from file://."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from tirith import cli
from tirith.store import state_js

ROOT = Path(__file__).resolve().parents[2]
PREFIX, SUFFIX = "window.TIRITH_STATE = ", ";\n"


def _parse(text: str) -> dict:
    assert text.startswith(PREFIX) and text.endswith(SUFFIX)
    return json.loads(text[len(PREFIX):-len(SUFFIX)])


def test_collect_once_writes_both_files_identically(home: Path) -> None:
    cli.main(["manager", "Watching </script> tags"])
    assert cli.main(["collect", "--once"]) == 0
    state = json.loads((home / "state.json").read_text(encoding="utf-8"))
    script = (home / "state.js").read_text(encoding="utf-8")
    assert _parse(script) == state
    assert "</script>" not in script            # cannot close a <script> tag early
    assert not list(home.glob("state.*.tmp"))   # atomic: no temp files left behind


def test_state_js_roundtrip_with_unicode() -> None:
    payload = {"a": "café ✨ </b>", "n": [1, 2]}
    assert _parse(state_js(payload)) == payload


@pytest.mark.parametrize("name", ["state.example", "state.rooms.example", "state.demo"])
def test_fixture_scripts_match_their_json(name: str) -> None:
    fixtures = ROOT / "fixtures"
    expected = json.loads((fixtures / f"{name}.json").read_text(encoding="utf-8"))
    assert _parse((fixtures / f"{name}.js").read_text(encoding="utf-8")) == expected


def test_demo_fixture_validates() -> None:
    jsonschema = pytest.importorskip("jsonschema")
    schema = json.loads((ROOT / "collector" / "state.schema.json").read_text(encoding="utf-8"))
    demo = json.loads((ROOT / "fixtures" / "state.demo.json").read_text(encoding="utf-8"))
    jsonschema.validate(demo, schema)
    assert len(demo["rooms"]) == 1 and 6 <= len(demo["flows"]) <= 14


def test_tail_lines_reads_jsonl_transcripts_as_text(tmp_path):
    """Agent transcripts in JSONL become readable lines; records without text are dropped."""
    from tirith.store import tail_lines
    log = tmp_path / "agent.jsonl"
    log.write_text("\n".join([
        "plain line",
        '{"type": "assistant", "message": {"content": [{"type": "text", "text": "Fixing the map labels"}]}}',
        '{"type": "assistant", "message": {"content": [{"type": "tool_use", "input": {"command": "pytest -q"}}]}}',
        '{"type": "progress", "id": 3}',
    ]) + "\n")
    assert tail_lines(log, 10) == ["plain line", "Fixing the map labels", "$ pytest -q"]


def test_tail_lines_drops_json_tool_results(tmp_path):
    from tirith.store import tail_lines
    log = tmp_path / "agent.jsonl"
    log.write_text('{"type": "user", "message": {"content": [{"type": "tool_result", "content": "{\\"success\\": true}"}]}}\nDone with the report\n')
    assert tail_lines(log, 10) == ["Done with the report"]
