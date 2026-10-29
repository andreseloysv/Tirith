"""``tirith lane stop``: end a running lane from outside.

``lane run`` starts its (non-interactive) child as the leader of its own process group and stores the
group in the lane file (``pgid``). Stopping writes a ``stop`` marker into the lane file, sends SIGTERM to
that group, waits up to ``TERM_WAIT_S`` and then sends SIGKILL. ``lane run`` sees the marker when its child
is gone and ends the lane as ``failed`` with the report ``Stopped by <owner|manager>: <reason>``; if
``lane run`` itself is gone, the stop ends the lane. The worktree is kept. See ``docs/CONTRACT-gates.md``.
"""

from __future__ import annotations

import os
import signal
import time
from pathlib import Path

from tirith.home import now_iso
from tirith.store import lane_path, read_json, write_lane

STOPPERS = ("owner", "manager", "steward")  # "steward" is the old name of "manager"
TERM_WAIT_S = 10.0
END_WAIT_S = 5.0
REASON_MAX = 300


class StopError(Exception):
    """A stop that cannot happen; the message says why."""


def alive(pid) -> bool:
    if not pid:
        return False
    try:
        os.kill(int(pid), 0)
    except ProcessLookupError:
        return False
    except (OSError, ValueError):
        return True
    return True


def report_for(by: str, reason: str | None) -> str:
    reason = " ".join(str(reason or "").split())[:REASON_MAX]
    by = "manager" if by == "steward" else by
    return f"Stopped by {by}: {reason}" if reason else f"Stopped by {by}."


def marker(lane: dict) -> dict | None:
    """The ``stop`` marker of a lane file (``{by, reason, at}``), or None."""
    stop = lane.get("stop")
    return stop if isinstance(stop, dict) and stop.get("by") in STOPPERS else None


def _signal(lane: dict, signum: int) -> None:
    pid, pgid = lane.get("pid"), lane.get("pgid")
    try:
        if pgid:
            os.killpg(int(pgid), signum)
        elif pid:
            os.kill(int(pid), signum)
    except (ProcessLookupError, PermissionError):
        pass


def _group_alive(lane: dict) -> bool:
    pgid = lane.get("pgid")
    if not pgid:
        return alive(lane.get("pid"))
    try:
        os.killpg(int(pgid), 0)
    except ProcessLookupError:
        return False
    except OSError:
        return True
    return True


def _reaped(lane: dict) -> bool:
    """The child is gone, or only a zombie that its waiting parent has not collected yet."""
    if not _group_alive(lane):
        return True
    pid = lane.get("pid")
    try:
        from tirith import timeouts as timeouts_mod
        state = timeouts_mod.run(["ps", "-o", "stat=", "-p", str(int(pid))], "generic", text=True,
                                 timeout=5).stdout.strip()
    except (OSError, TypeError, ValueError):
        return False
    return state.startswith("Z") or not state


def stop_lane(home: Path, lane_id: str, by: str = "manager", reason: str | None = None,
              term_wait: float = TERM_WAIT_S, end_wait: float = END_WAIT_S,
              sleep=time.sleep) -> dict:
    """Stop a running lane; returns the stored lane. Raises ``StopError`` for an unknown or ended lane."""
    from tirith.cli import end_lane  # late: cli imports the collector, which imports this module

    if by not in STOPPERS:
        raise StopError(f"--by must be one of {', '.join(STOPPERS)}")
    by = "manager" if by == "steward" else by
    path = lane_path(home, lane_id)
    lane = read_json(path, None)
    if not isinstance(lane, dict):
        raise StopError(f"unknown lane: {lane_id}")
    if lane.get("ended_at"):
        raise StopError(f"lane {lane_id} already ended ({lane.get('result')})")
    lane["stop"] = {"by": by, "reason": " ".join(str(reason or "").split())[:REASON_MAX], "at": now_iso()}
    write_lane(home, lane)
    _signal(lane, signal.SIGTERM)
    deadline = time.monotonic() + term_wait
    while not _reaped(lane) and time.monotonic() < deadline:
        sleep(0.1)
    if not _reaped(lane):
        _signal(lane, signal.SIGKILL)
        deadline = time.monotonic() + 2.0
        while not _reaped(lane) and time.monotonic() < deadline:
            sleep(0.05)
    deadline = time.monotonic() + end_wait
    while time.monotonic() < deadline:  # `lane run` ends the lane itself once its child is gone
        current = read_json(path, None)
        if isinstance(current, dict) and current.get("ended_at"):
            return current
        sleep(0.1)
    return end_lane(home, lane_id, "failed", report_for(by, reason))
