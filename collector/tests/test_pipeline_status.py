"""Truthful pipeline status: `running` only while a started stage's process lives; the rest is skipped or failed."""

from __future__ import annotations

import os
from pathlib import Path

from tirith.cli import main
from tirith.collector import Collector
from tirith.home import now_iso
from tirith.store import append_jsonl


def _state(home: Path) -> dict:
    return Collector(home, client=None).cycle()


def _events(home: Path, rows: list[tuple]) -> None:
    for stage, event, at, extra in rows:
        extra = dict(extra)
        append_jsonl(home / "events.jsonl", {
            "at": at, "run": "ci#1", "project": "demo", "pipeline": "ci", "stage": stage, "event": event,
            "status": "running" if event == "start" else extra.pop("status"), **extra})


def _run(state: dict) -> dict:
    return next(p for p in state["pipelines"] if p["run"] == "ci#1")


def _declare(home: Path, stages: str = "build,test") -> None:
    main(["pipeline", "declare", "ci", "--project", "demo", "--stages", stages])


def test_live_stage_is_running(home: Path) -> None:
    _declare(home)
    _events(home, [("build", "start", now_iso(), {"pid": os.getpid()})])
    run = _run(_state(home))
    assert run["status"] == "running" and run["ended_at"] is None
    assert [s["status"] for s in run["stages"]] == ["running", "pending"]


def test_dead_pid_is_interrupted(home: Path) -> None:
    _declare(home)
    _events(home, [("build", "start", now_iso(), {"pid": 2 ** 22 + 12345})])
    run = _run(_state(home))
    assert run["status"] == "failed"
    assert [(s["status"], s.get("reason")) for s in run["stages"]] == [("failed", "interrupted"), ("skipped", None)]


def test_old_start_without_end_times_out(home: Path) -> None:
    _declare(home)
    _events(home, [("build", "start", "2026-01-01T10:00:00Z", {"pid": os.getpid()})])
    run = _run(_state(home))
    assert run["status"] == "failed" and run["stages"][0]["reason"] == "interrupted"


def test_unstarted_stages_of_a_finished_run_are_skipped(home: Path) -> None:
    _declare(home, "build,test,ship")
    _events(home, [("build", "start", "2026-10-28T10:00:00Z", {}),
                   ("build", "end", "2026-10-28T10:00:05Z", {"status": "passed"})])
    run = _run(_state(home))
    assert run["status"] == "passed"
    assert [s["status"] for s in run["stages"]] == ["passed", "skipped", "skipped"]


def test_failed_run_skips_the_rest(home: Path) -> None:
    _declare(home, "build,test,ship")
    _events(home, [("build", "start", "2026-10-28T10:00:00Z", {}),
                   ("build", "end", "2026-10-28T10:00:05Z", {"status": "passed"}),
                   ("test", "start", "2026-10-28T10:00:05Z", {}),
                   ("test", "end", "2026-10-28T10:00:09Z", {"status": "failed"})])
    run = _run(_state(home))
    assert run["status"] == "failed"
    assert [s["status"] for s in run["stages"]] == ["passed", "failed", "skipped"]


def test_never_started_run_stays_pending(home: Path) -> None:
    _declare(home)
    _events(home, [])
    append_jsonl(home / "events.jsonl", {"at": now_iso(), "run": "ci#1", "project": "demo", "pipeline": "ci"})
    assert _run(_state(home))["status"] == "pending"
