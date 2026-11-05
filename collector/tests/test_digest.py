"""Digest: what reached main as cards, the static page, and the opinion that becomes plan items."""

from __future__ import annotations

from pathlib import Path

from test_plangate import _plan, proj  # noqa: F401  (the fixture and plan reader)
from tirith import digest as digest_mod
from tirith import roadmap
from tirith.cli import main
from tirith.store import write_json_atomic


def _history(home: Path) -> None:
    rows = [{"lane": "a1", "project": "demo", "result": "merged", "at": "2026-11-01T10:00:00Z",
             "message": "tests passed; merged", "commit": "abcdef1234567"},
            {"lane": "a2", "project": "demo", "result": "red-suite", "at": "2026-11-01T11:00:00Z", "message": "x"},
            {"lane": "b1", "project": "other", "result": "merged", "at": "2026-11-01T12:00:00Z", "message": "y"}]
    write_json_atomic(home / "mergeq.json", {"queue": [], "history": rows, "last": {}})


def test_digest_lists_only_merged_lanes_of_the_project(home: Path) -> None:
    _history(home)
    data = digest_mod.build(home, "demo")
    assert [c["lane"] for c in data["cards"]] == ["a1"] and data["cards"][0]["commit"] == "abcdef1234"


def test_digest_page_has_cards_and_the_opinion_button(home: Path, capsys) -> None:
    _history(home)
    assert main(["digest", "--project", "demo"]) == 0
    page = Path(capsys.readouterr().out.strip()).read_text(encoding="utf-8")
    assert "a1" in page and "b1" not in page and "Give your opinion" in page
    assert "tirith digest feedback --project demo" in page


def test_digest_feedback_becomes_plan_items(proj, home: Path, capsys) -> None:  # noqa: F811
    repo, _ = proj
    assert main(["digest", "feedback", "--project", "demo", "--step", "1",
                 "--text", "- Make the cards bigger\n\n2. Show the date"]) == 0
    plan = roadmap.without_metadata(_plan(repo))
    assert "- [ ] Make the cards bigger\n" in plan and "- [ ] Show the date\n" in plan


def test_digest_feedback_defaults_to_a_step_and_refuses_empty(proj, home: Path, capsys) -> None:  # noqa: F811
    repo, _ = proj
    assert main(["digest", "feedback", "--project", "demo", "--text", "Only one"]) == 0
    assert "Only one" in _plan(repo)
    assert main(["digest", "feedback", "--project", "demo", "--text", "  \n "]) == 3


def test_while_away_summarizes_entered_failed_and_next(proj, home: Path, capsys) -> None:  # noqa: F811
    repo, _ = proj
    _history(home)
    data = digest_mod.while_away(home, "demo", "2026-11-01T10:30:00Z", repo)
    assert data["entered"] == [] and [r["lane"] for r in data["failed"]] == ["a2"]
    assert data["next"]
    data = digest_mod.while_away(home, "demo", "2026-11-01T00:00:00Z", repo)
    assert [r["lane"] for r in data["entered"]] == ["a1"]
    assert main(["digest", "away", "--project", "demo", "--since", "2026-11-01T00:00:00Z"]) == 0
    out = capsys.readouterr().out
    assert "Came in (1)" in out and "Failed (1)" in out and "red-suite" in out and "Next (" in out
