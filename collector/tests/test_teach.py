"""Every block teaches the alternative; repeated blocks go to the manager queue; no rule was removed."""

from __future__ import annotations

import io
import json
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest

from conftest import write_config
from tirith import asks, gates, launch, managerd, manifest
from tirith.cli import main
from tirith.home import load_config

AGENTS = '[agents.anthropic]\ncommand = "alpha-cli"\n'
MANIFEST = '[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "test"\nrun = "uv run -q pytest -q"\n'
NO_VERIFY = "--no" + "-verify"
HOOKS_KEY = "core." + "hooksPath"

# one call per gate that fires in a hook (the ids that existed before this change)
CALLS = {
    "subagent-tool": ("Task", {"description": "x"}),
    "agent-cli": ("Bash", {"command": "alpha-cli -p hi"}),
    "no-verify": ("Bash", {"command": f"git commit {NO_VERIFY} -m x"}),
    "hooks-path": ("Bash", {"command": f"git config {HOOKS_KEY} /x"}),
    "git-hooks": ("Bash", {"command": "rm .git/hooks/pre-commit"}),
    "guard-reinstall": ("Bash", {"command": "tirith guard reinstall demo"}),
    "raw-stage": ("Bash", {"command": "uv run -q pytest -q"}),
}


@pytest.fixture
def proj(home: Path, tmp_path: Path) -> Path:
    folder = tmp_path / "proj"
    folder.mkdir()
    (folder / "tirith.toml").write_text(MANIFEST, encoding="utf-8")
    write_config(home, [{"id": "demo", "name": "Demo", "repo": str(folder)}], AGENTS)
    return folder


def _rules(home: Path, proj: Path, tool: str, data: dict) -> dict[str, gates.Violation]:
    found = gates.check_call(load_config(home), tool, data, str(proj), "demo", home)
    return {v.rule: v for v in found}


def test_every_gate_still_fires_and_has_an_alternative(home: Path, proj: Path) -> None:
    for rule, (tool, data) in CALLS.items():
        found = _rules(home, proj, tool, data)
        assert rule in found, rule
        assert found[rule].action == "block"
        instead = found[rule].instead.strip()
        assert instead and (len(instead) > 15 or instead.startswith(gates.NO_ALTERNATIVE)), rule


def test_protected_and_plan_gates_teach(home: Path, proj: Path) -> None:
    config = load_config(home)
    protected = str(gates.protected_paths(config, home)[0])
    plan_path = str(proj / "docs" / "TODO.md")
    for tool, data, rule in [("Edit", {"file_path": protected}, "settings-edit"),
                             ("Write", {"file_path": plan_path}, "roadmap-edit"),
                             ("Edit", {"file_path": str(proj / ".git" / "hooks" / "pre-commit")}, "git-hooks")]:
        found = _rules(home, proj, tool, data)
        assert rule in found and found[rule].instead.strip(), rule
    plan = _rules(home, proj, "Write", {"file_path": plan_path})["roadmap-edit"]
    assert "tirith plan add" in plan.instead and "tirith plan report" in plan.instead
    raw = _rules(home, proj, "Bash", CALLS["raw-stage"][1])["raw-stage"]
    assert "tirith run --project demo --stage test" in raw.instead


def test_built_in_rule_ids_are_all_still_present() -> None:
    expected = {"subagent-tool", "agent-cli", "no-verify", "hooks-path", "git-hooks", "settings-edit",
                "roadmap-edit", "raw-stage", "lane-brief", "project-check", "commit-guard"}
    assert expected <= set(manifest.BUILT_IN_RULES)
    assert expected <= set(gates.DESCRIPTIONS)


def test_block_message_says_a_block_is_not_a_stop(home: Path, proj: Path, monkeypatch, capsys) -> None:
    payload = {"hook_event_name": "PreToolUse", "session_id": "s", "cwd": str(proj), "tool_name": "Bash",
               "tool_input": {"command": "uv run -q pytest -q"}}
    monkeypatch.setattr("sys.stdin", io.StringIO(json.dumps(payload)))
    assert main(["hook", "--agent", "anthropic"]) == 2
    err = capsys.readouterr().err
    assert "Do this instead: tirith run --project" in err and gates.NOT_A_STOP in err


def test_three_denies_in_ten_minutes_queue_one_manager_item(home: Path) -> None:
    violation = gates.Violation("raw-stage", "pytest run raw", "tirith run --project demo --stage test")
    start = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)
    assert gates.note_deny(home, violation, "lane-a", "demo", start) is None
    assert gates.note_deny(home, violation, "lane-a", "demo", start + timedelta(minutes=2)) is None
    item = gates.note_deny(home, violation, "lane-a", "demo", start + timedelta(minutes=4))
    assert item and item["to"] == "manager" and "tirith run --project demo --stage test" in item["text"]
    assert len(asks.queue_items(home)) == 1
    assert gates.note_deny(home, violation, "lane-a", "demo", start + timedelta(minutes=5)) is None


def test_denies_outside_the_window_or_of_other_rules_do_not_add_up(home: Path) -> None:
    one = gates.Violation("raw-stage", "x", "tirith run")
    other = gates.Violation("no-verify", "y", "git commit")
    start = datetime(2026, 10, 29, 12, 0, tzinfo=timezone.utc)
    for minutes in (0, 6, 12):
        assert gates.note_deny(home, one, "l", "demo", start + timedelta(minutes=minutes)) is None
    assert gates.note_deny(home, other, "l", "demo", start + timedelta(minutes=12)) is None
    assert gates.note_deny(home, one, None, "demo", start) is None
    assert asks.queue_items(home) == []


def test_templates_say_a_block_is_not_a_stop() -> None:
    assert any("is not a stop" in rule for rule in launch.HARD_RULES)
    assert any("is not a stop" in rule for rule in managerd.RULES)
    assert "is not a stop" in launch.build_brief("t", "", ["a"], "7")
