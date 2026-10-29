"""The parity harness comparison (``scripts/parity/compare.py``): what counts as ported, equal or different."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts" / "parity"))

import compare  # noqa: E402
import fixtures  # noqa: E402


def _state(**over) -> dict:
    row = {"id": "p", "name": "P", "roadmap": {"done": 1, "steps": [{"id": "1"}]}, "guard": {"ok": True}}
    return {"version": 1, "generated_at": "2026-01-01T00:00:00Z", "projects": [row], "agents": [], **over}


def test_equal_ported_sections_and_unported_ones_are_not_failures() -> None:
    rust = {"version": 1, "generated_at": "later", "projects": [{"id": "p", "name": "P",
                                                                  "roadmap": {"done": 1, "steps": [{"id": "1"}]}}]}
    ok, not_ported, diffs = compare.compare_states(_state(), rust)
    assert diffs == []
    assert "version" in ok and "projects[].roadmap" in ok
    assert "agents" in not_ported and "projects[].guard" in not_ported


def test_a_different_value_is_a_readable_diff() -> None:
    rust = {"version": 1, "projects": [{"id": "p", "name": "P", "roadmap": {"done": 2, "steps": [{"id": "1"}]}}]}
    ok, _, diffs = compare.compare_states(_state(), rust)
    assert diffs == ["projects[p].roadmap.done: python 1 != rust 2"]
    assert "projects[].roadmap" not in ok and "projects[].name" in ok


def test_missing_deeper_keys_rows_and_extra_keys_are_differences() -> None:
    rust = {"version": 1, "projects": [{"id": "p", "name": "P", "roadmap": {"done": 1}, "extra": 1}]}
    _, _, diffs = compare.compare_states(_state(), rust)
    assert any("roadmap.steps: missing in rust" in d for d in diffs)
    assert "projects[p].extra: only in rust" in diffs
    _, _, diffs = compare.compare_states(_state(), {"version": 1, "projects": []})
    assert any(d.startswith("projects: ids/order") for d in diffs)


def test_volatile_fields_are_ignored_everywhere() -> None:
    a = {"version": 1, "x": {"generated_at": "1"}, "projects": []}
    b = {"version": 1, "x": {"generated_at": "2"}, "projects": []}
    assert compare.compare_states(a, b)[2] == []


def test_every_fixture_builds(tmp_path: Path) -> None:
    for name, build in fixtures.FIXTURES.items():
        home = build(tmp_path / name)
        assert (home / "config.toml").is_file(), name
