"""The hourly end-to-end test lane: the real run reaches the bare origin; a broken step turns the check red."""

from __future__ import annotations

import subprocess
from datetime import datetime, timezone
from pathlib import Path

import pytest

from tirith import canary as canary_mod
from tirith import mergeq as mergeq_mod
from tirith import selfcheck as selfcheck_mod
from tirith.home import load_config
from tirith.store import read_lanes


@pytest.fixture(autouse=True)
def identity(monkeypatch: pytest.MonkeyPatch) -> None:
    for key in ("AUTHOR", "COMMITTER"):
        monkeypatch.setenv(f"GIT_{key}_NAME", "Tirith Test")
        monkeypatch.setenv(f"GIT_{key}_EMAIL", "t@example.invalid")


def dispatcher(home: Path, spawn):
    """What the collector's own dispatch pass does for the merge queue."""
    def pump() -> None:
        mergeq_mod.run(home, load_config(home), read_lanes(home), spawn=spawn)
    return pump


def drain_spawn(home: Path):
    def spawn(argv, where, log) -> None:  # stands in for the detached `tirith merge run`
        mergeq_mod.drain(home, load_config(home))
    return spawn


def test_canary_run_reaches_the_bare_origin_and_ticks_the_plan(home: Path) -> None:
    row = canary_mod.run(home, wait_s=60, pump=dispatcher(home, drain_spawn(home)), sleep=lambda s: None)
    assert row["state"] == "ok", row
    origin = canary_mod.origin_of(home)
    assert subprocess.run(["git", "-C", str(origin), "cat-file", "-e", "main:canary.txt"]).returncode == 0
    assert "- [x] Canary run" in (canary_mod.repo_of(home) / "docs" / "TODO.md").read_text(encoding="utf-8")
    lanes = canary_mod.repo_of(home).parent / "repo-lanes"
    assert not lanes.exists() or not list(lanes.iterdir())
    level, _, _ = canary_mod.verdict(home, datetime.now(timezone.utc))
    assert level == "ok"


def test_missing_spawner_turns_the_check_red_naming_the_step(home: Path) -> None:
    row = canary_mod.run(home, wait_s=0.2, pump=dispatcher(home, None), sleep=lambda s: None)
    assert row["state"] == "failed" and row["step"] == "merge-start"
    now = datetime.now(timezone.utc)
    book: dict = {"tries": {}, "queued": {}, "notices": {}}
    check = selfcheck_mod._canary(home, now, book, set())
    assert check["level"] == "failed"
    assert "merge-start" in check["text"] and row["log"] in check["text"]
    assert "canary" in book["queued"]


def test_canary_project_is_hidden_and_due_hourly(home: Path) -> None:
    canary_mod.ensure(home)
    assert load_config(home).project("canary") is not None and canary_mod.hidden("canary")
    now = datetime.now(timezone.utc)
    started = []
    assert canary_mod.tick(home, now, lambda *a: started.append(a))
    assert not canary_mod.tick(home, now, lambda *a: started.append(a))  # one at a time
    assert len(started) == 1
