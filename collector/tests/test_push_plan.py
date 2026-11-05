"""Tirith pushes main to the origin soon after every change of the plan (the dispatcher's push pass)."""

from __future__ import annotations

import subprocess
from datetime import datetime, timedelta, timezone
from pathlib import Path

import pytest

from tirith import canary as canary_mod
from tirith import dispatch as dispatch_mod
from tirith import plangate as plangate_mod
from tirith import pushplan as pushplan_mod
from tirith.home import load_config


@pytest.fixture(autouse=True)
def identity(monkeypatch: pytest.MonkeyPatch) -> None:
    pushplan_mod.wait()
    pushplan_mod._done.clear()
    for key in ("AUTHOR", "COMMITTER"):
        monkeypatch.setenv(f"GIT_{key}_NAME", "Tirith Test")
        monkeypatch.setenv(f"GIT_{key}_EMAIL", "t@example.invalid")


def tip(repo: Path, ref: str) -> str:
    return subprocess.run(["git", "-C", str(repo), "rev-parse", ref], capture_output=True, text=True).stdout.strip()


def edit_plan(home: Path, repo: Path, text: str) -> None:
    plangate_mod.commit_change(home, repo, canary_mod.PROJECT, lambda body: body + f"\n<!-- {text} -->\n", f"plan: {text}")


def fail(home: Path, config, now: datetime, state: dict) -> bool:
    """One pass that starts the push, then one that collects it."""
    pushplan_mod.run(home, config, now, state)
    pushplan_mod.wait()
    return pushplan_mod.run(home, config, now, state)


def test_push_plan_sends_a_plan_change_to_the_origin(home: Path) -> None:
    repo = canary_mod.ensure(home)
    origin = canary_mod.origin_of(home)
    config = load_config(home)
    now = datetime.now(timezone.utc)
    state: dict = {}
    assert not pushplan_mod.run(home, config, now, state)  # nothing ahead: nothing to push
    edit_plan(home, repo, "one")
    assert tip(origin, "main") != tip(repo, "main")
    pushplan_mod.run(home, config, now, state)
    pushplan_mod.wait()
    assert pushplan_mod.run(home, config, now, state)  # the finished push is collected
    assert tip(origin, "main") == tip(repo, "main")
    assert state["pushed"][canary_mod.PROJECT]["ok"] is True
    assert not pushplan_mod.run(home, config, now, state)  # already there


def test_push_plan_runs_in_the_dispatcher_pass_and_retries_a_failure(home: Path) -> None:
    repo = canary_mod.ensure(home)
    origin = canary_mod.origin_of(home)
    config = load_config(home)
    now = datetime.now(timezone.utc)
    edit_plan(home, repo, "two")
    subprocess.run(["git", "-C", str(repo), "remote", "set-url", "origin", str(origin) + "-gone"], check=True)
    state: dict = {}
    fail(home, config, now, state)
    assert state["pushed"][canary_mod.PROJECT]["ok"] is False and state["pushed"][canary_mod.PROJECT]["error"]
    assert not pushplan_mod.run(home, config, now + timedelta(seconds=5), state)  # not every pass
    subprocess.run(["git", "-C", str(repo), "remote", "set-url", "origin", str(origin)], check=True)
    later = now + timedelta(seconds=pushplan_mod.RETRY_S + 1)
    assert pushplan_mod.RETRY_S < pushplan_mod.MAX_DELAY_S == 300
    pushplan_mod.run(home, config, later, state)
    pushplan_mod.wait()
    assert pushplan_mod.run(home, config, later, state)
    assert state["pushed"][canary_mod.PROJECT]["ok"] is True
    assert tip(origin, "main") == tip(repo, "main")
    edit_plan(home, repo, "three")
    dispatch_mod.run(home, config, later, agents=[], lanes=[], status_of=lambda *a: "idle", spawn=None)
    pushplan_mod.wait()
    assert tip(origin, "main") == tip(repo, "main")


def test_push_plan_backoff_doubles_up_to_thirty_minutes_and_resets(home: Path) -> None:
    assert [pushplan_mod.retry_after(n) for n in (1, 2, 3, 4, 5, 6, 7, 20)] == [60, 120, 240, 480, 960, 1800, 1800, 1800]
    repo = canary_mod.ensure(home)
    origin = canary_mod.origin_of(home)
    config = load_config(home)
    now = datetime.now(timezone.utc)
    edit_plan(home, repo, "backoff")
    subprocess.run(["git", "-C", str(repo), "remote", "set-url", "origin", str(origin) + "-gone"], check=True)
    state: dict = {}
    book = state.setdefault("pushed", {})
    for fails, waited in ((1, 60), (2, 120), (3, 240)):
        assert fail(home, config, now, state)
        assert book[canary_mod.PROJECT]["fails"] == fails and book[canary_mod.PROJECT]["retry_s"] == waited
        too_soon = now + timedelta(seconds=waited - 1)
        assert not fail(home, config, too_soon, state)  # still waiting: no new push, nothing started
        assert book[canary_mod.PROJECT]["fails"] == fails
        now += timedelta(seconds=waited + 1)
    subprocess.run(["git", "-C", str(repo), "remote", "set-url", "origin", str(origin)], check=True)
    assert fail(home, config, now, state)
    assert book[canary_mod.PROJECT]["ok"] is True and book[canary_mod.PROJECT]["fails"] == 0
    assert tip(origin, "main") == tip(repo, "main")


def test_push_plan_backoff_does_not_stall_the_pass(home: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    import threading

    repo = canary_mod.ensure(home)
    config = load_config(home)
    edit_plan(home, repo, "slow")
    gate = threading.Event()
    monkeypatch.setattr(pushplan_mod, "push", lambda r: gate.wait(10) and None)
    state: dict = {}
    now = datetime.now(timezone.utc)
    assert not pushplan_mod.run(home, config, now, state)  # returns at once, the push is still running
    assert not pushplan_mod.run(home, config, now, state)  # and a second pass does not start a second push
    assert len(pushplan_mod._running) == 1
    gate.set()
    pushplan_mod.wait()
