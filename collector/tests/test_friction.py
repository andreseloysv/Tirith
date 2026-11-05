"""Friction: people needed and waits are logged; the biggest goes on top of the plan (``tirith.friction``)."""

from __future__ import annotations

from pathlib import Path

from test_plangate import PLAN, _plan, proj  # noqa: F401  (fixture)
from tirith import friction, plangate, roadmap
from tirith.cli import main


def test_friction_record_and_rank(home: Path):
    friction.record(home, "demo", friction.WAIT, "merge queue waited 3", 120)
    friction.record(home, "demo", friction.WAIT, "merge queue waited 7", 200)
    friction.record(home, "demo", friction.PERSON, "credentials: needs the Apple key")
    friction.record(home, "other", friction.PERSON, "elsewhere")
    top = friction.ranked(home, "demo")
    assert [g["kind"] for g in top] == [friction.PERSON, friction.WAIT]  # 600 beats 320
    assert top[1]["times"] == 2 and top[1]["seconds"] == 320
    assert len(friction.ranked(home)) == 3


def test_friction_cli_record_and_list(home: Path, capsys):
    assert main(["friction", "record", "--project", "demo", "--detail", "tests waited", "--seconds", "30"]) == 0
    assert main(["friction", "list", "--project", "demo"]) == 0
    assert "tests waited" in capsys.readouterr().out


def test_friction_surface_puts_biggest_first_once(home: Path, proj):  # noqa: F811
    repo, _ = proj
    friction.record(home, "demo", friction.WAIT, "small wait", 5)
    assert friction.surface(home, "demo") is None  # happened once: not enough
    for _ in range(3):
        friction.record(home, "demo", friction.PERSON, "credentials: Apple key")
    text = friction.surface(home, "demo")
    assert text and "Apple key" in text
    steps = roadmap.parse(_plan(repo))
    ops = next(s for s in steps if s["id"] == plangate.OPS_STEP)
    assert "Apple key" in ops["items"][0]["text"]
    assert friction.surface(home, "demo") is None  # not added twice
    assert _plan(repo).count("Apple key") == 1


def test_friction_logged_when_owner_asked(home: Path):
    from tirith.cli import add_wait
    add_wait(home, "demo", "Give the Apple key", options=["Done|done"], why_owner="credentials")
    rows = friction.ranked(home, "demo")
    assert rows and rows[0]["kind"] == friction.PERSON
