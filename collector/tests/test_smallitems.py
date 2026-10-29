"""Small plan items with a done check, the size guard and the lane budget (``tirith.plangate``)."""

from __future__ import annotations

import re
from pathlib import Path

import pytest

from tirith import gates, managerd, plangate, roadmap


def test_check_note_round_trip() -> None:
    text = roadmap.with_check("Build the button.", "pytest -q tests/test_x.py")
    assert roadmap.check_of(text) == "pytest -q tests/test_x.py"
    assert roadmap.without_check(text) == "Build the button."
    assert roadmap.check_of(roadmap.with_status(text, roadmap.STATE_PROGRESS, "lane-1")) == "pytest -q tests/test_x.py"
    assert roadmap.split_status(roadmap.with_status(text, roadmap.STATE_REVIEW))[0] == text
    assert roadmap.check_of("Plain item.") is None


def test_build_shows_the_check() -> None:
    steps = roadmap.parse("## 1. A\n\n- [ ] Do it. (check: `true`)\n- [ ] Plain.\n")
    assert roadmap.check_of(steps[0]["items"][0]["text"]) == "true"
    assert roadmap.check_of(steps[0]["items"][1]["text"]) is None


@pytest.mark.parametrize("text", [
    "x" * 301,
    "Add the login page. Then wire the API. Finally write the docs.",
    "Add the login page and also rewrite the router",
    "Add the login page; rewrite the router",
])
def test_size_guard_refuses_big_items(text: str) -> None:
    assert plangate.size_problem(text)


def test_size_guard_accepts_small_items() -> None:
    assert plangate.size_problem("Add the login page (check: `pytest -q`).") is None
    assert plangate.size_problem("Add the login page. Test it.") is None


def test_add_item_says_split_it(tmp_path: Path) -> None:
    with pytest.raises(plangate.PlanGateError, match="split it"):
        plangate.add_item(tmp_path, tmp_path, "p", "1", "a. Then b. Then c.", "owner")
    with pytest.raises(plangate.PlanGateError, match="backticks"):
        plangate.add_item(tmp_path, tmp_path, "p", "1", "Do it", "owner", check="echo `x`")


def test_run_check(tmp_path: Path) -> None:
    assert plangate.run_check("true", tmp_path)[0] is True
    passed, out = plangate.run_check("echo no >&2; false", tmp_path)
    assert not passed and "no" in out


def test_lane_budget() -> None:
    assert plangate.budget_note(3, 100, 20) is None
    assert "split" in plangate.budget_note(41, 10, 0)
    assert "split" in plangate.budget_note(5, 1000, 600)


def test_manager_prompt_asks_for_small_items() -> None:
    assert any("30 minutes" in rule and "--check" in rule for rule in managerd.RULES)


EXPECTED_GATE_IDS = {"git-hooks", "raw-stage", "roadmap-edit", "settings-edit"}


def test_existing_gate_ids_still_present_and_active() -> None:
    source = Path(gates.__file__).read_text(encoding="utf-8")
    found = set(re.findall(r'Violation\(\s*"([a-z-]+)"', source))
    assert EXPECTED_GATE_IDS <= found
    assert hasattr(gates, "_subagent") and hasattr(gates, "_agent_cli") and hasattr(gates, "_git_rules")
    assert hasattr(gates, "_roadmap_bash") and hasattr(gates, "_settings_bash") and hasattr(gates, "_raw_stage")
    assert 'tool in SUBAGENT_TOOLS' in source and "_commit_no_verify" in source
