"""Kill what a lane leaves running: every process whose executable or working folder is under the lane's
worktree (``Tirith-lanes/<lane>/``), SIGTERM first and SIGKILL after ``TERM_WAIT_S`` for what ignored it.

Called when a lane ends (done, stopped, removed) and when a full suite finishes. Tirith's own process and its
ancestors are never touched (``lane end`` is often run from inside the lane). See ``docs/CONTRACT-gates.md``.
"""

from __future__ import annotations

import os
import signal
import time
from pathlib import Path
from typing import Callable

TERM_WAIT_S = 3.0
LANES_DIR = "Tirith-lanes"


def _under(path: str, root: str) -> bool:
    return bool(path) and (path == root or path.startswith(root.rstrip("/") + "/"))


def _roots(workspace: Path) -> list[str]:
    roots = {str(workspace), os.path.realpath(workspace)}  # /var vs /private/var
    return sorted(roots)


def _protected(table) -> set[int]:
    """Us, our ancestors and everything in their process groups (``tirith run | tail``, the agent's shell)."""
    parent = {p.pid: p.ppid for p in table}
    keep, pid = {os.getpid()}, os.getppid()
    while pid > 1 and pid not in keep:
        keep.add(pid)
        pid = parent.get(pid, 0)
    groups = {p.pgid for p in table if p.pid in keep}
    return keep | {p.pid for p in table if p.pgid in groups and p.pgid > 1}


def leftovers(workspace: Path) -> list[int]:
    """Pids (not zombies, not us or our ancestors) whose exe or cwd is under ``workspace``."""
    from tirith import procs
    roots = _roots(workspace)
    table = procs.snapshot()
    keep = _protected(table)
    found = []
    for proc in table:
        if proc.pid in keep or proc.zombie:
            continue
        exe, cwd = procs.locations(proc.pid)
        if any(_under(where, root) for where in (exe, cwd) for root in roots):
            found.append(proc.pid)
    return found


def _alive(pid: int) -> bool:
    from tirith import procs
    return any(p.pid == pid and not p.zombie for p in procs.snapshot())


def reap_lane_leftovers(workspace: Path | str | None, term_wait: float = TERM_WAIT_S,
                        sleep: Callable[[float], None] = time.sleep) -> list[int]:
    """SIGTERM every process left under ``workspace``, SIGKILL those still there after ``term_wait`` s.
    Returns the pids that were signalled. Never raises."""
    if not workspace or not str(workspace).strip():
        return []
    workspace = Path(str(workspace))
    if LANES_DIR not in workspace.parts or workspace.name == LANES_DIR:
        return []  # only worktrees under Tirith-lanes/<lane>/; never a main checkout
    try:
        pids = leftovers(workspace)
        if not pids:
            return []
        for pid in pids:
            _send(pid, signal.SIGTERM)
        deadline = time.monotonic() + term_wait
        alive = pids
        while alive and time.monotonic() < deadline:
            sleep(0.05)
            alive = [pid for pid in alive if _alive(pid)]
        for pid in alive:
            _send(pid, signal.SIGKILL)
        return pids
    except Exception:  # cleanup must never break ending a lane
        return []


def _send(pid: int, sig: int) -> None:
    try:
        os.kill(pid, sig)
    except OSError:
        pass


SWEEP_EVERY_S = 180.0


def reap_ended_lanes(home: Path | str, term_wait: float = TERM_WAIT_S,
                     sleep: Callable[[float], None] = time.sleep) -> list[int]:
    """Sweep: reap what is still running under the worktree of every lane that already ended
    (``ended_at`` set). Run at collector start and every ``SWEEP_EVERY_S``. Returns the pids signalled. Never raises."""
    from tirith.store import read_lanes
    killed: list[int] = []
    try:
        lanes = read_lanes(Path(home))
    except Exception:
        return killed
    for lane in lanes:
        if lane.get("ended_at") and lane.get("workspace"):
            killed += reap_lane_leftovers(lane["workspace"], term_wait=term_wait, sleep=sleep)
    return killed
