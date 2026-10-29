"""Lanes must not fill the disk: merged worktrees are removed and new lanes stop when space runs low.

Every collector cycle ``dispatch.run`` calls :func:`cleanup` (a finished lane whose branch is merged into the
project's checked-out branch and whose worktree is clean loses its worktree and its branch; a dirty worktree is
left alone and reported in the lane record) and :func:`check` (free space of the volume holding the lanes;
below ``[disk] min_free_gb`` the autopilot and the dispatcher start no lane and one manager queue item says
why). The shared cargo target folder lives in ``tirith.runstage``. See ``docs/CONTRACT-launch.md``.
"""

from __future__ import annotations

import shutil
import subprocess
from datetime import datetime
from pathlib import Path
from typing import Callable

from tirith import asks as asks_mod
from tirith import projects as projects_mod
from tirith import review as review_mod
from tirith.home import Config, now_iso
from tirith.store import read_lanes, write_lane

GB = 1024 ** 3
DISK_ITEM = "disk"


def _git(folder: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", "-C", str(folder), *args], capture_output=True, text=True)


def _repo(config: Config, lane: dict) -> Path | None:
    for proj in config.projects:
        if proj.id == lane.get("project") and proj.repo:
            return projects_mod.real(proj.repo)
    return None


# -- cleanup of merged worktrees --------------------------------------------------------------


def _note(home: Path, lane: dict, status: str, now: datetime, **extra) -> bool:
    """Store ``lane["cleanup"]``; True when it changed (a dirty lane is not rewritten every cycle)."""
    old = lane.get("cleanup") or {}
    if old.get("status") == status and old.get("message") == extra.get("message"):
        return False
    lane["cleanup"] = {"status": status, "at": now_iso(now), **extra}
    write_lane(home, lane)
    return True


def cleanup(home: Path, config: Config, now: datetime) -> bool:
    """Remove the worktree (never ``--force``) and local branch of every merged lane; True when a lane changed.

    Only launched lanes (never the manager lane or the main checkout) that ended ``done``."""
    changed = False
    for lane in read_lanes(home):
        if not lane.get("ended_at") or lane.get("result") != "done":
            continue
        if (lane.get("cleanup") or {}).get("status") == "removed":
            continue
        repo = _repo(config, lane)
        if repo is None or not review_mod.launched(lane, repo):
            continue
        review = lane.get("review") or {}
        if review.get("status") == review_mod.RUNNING and review_mod._alive(review.get("pid")):
            continue
        workspace, branch = Path(str(lane["workspace"])), f"lane/{lane['id']}"
        if not workspace.is_dir():
            continue
        if _git(repo, "merge-base", "--is-ancestor", branch, "HEAD").returncode != 0:
            continue  # unmerged (or no such branch): the review's business
        if _git(workspace, "status", "--porcelain").stdout.strip():
            changed |= _note(home, lane, "dirty", now, message="the worktree has uncommitted changes; left in place")
            continue
        done = _git(repo, "worktree", "remove", str(workspace))
        if done.returncode != 0:
            changed |= _note(home, lane, "blocked", now, message=(done.stderr or done.stdout).strip()[:300])
            continue
        deleted = _git(repo, "branch", "-d", branch).returncode == 0
        _note(home, lane, "removed", now, branch_deleted=deleted)
        changed = True
    return changed


# -- the disk guard ---------------------------------------------------------------------------


def lanes_volumes(home: Path, config: Config) -> list[Path]:
    """Existing folders on the volumes that hold the projects' lane worktrees (else the Tirith home)."""
    found = []
    for proj in config.projects:
        if proj.repo:
            folder = projects_mod.real(proj.repo).parent
            if folder.is_dir() and folder not in found:
                found.append(folder)
    return found or [home]


def free_bytes(folder: Path) -> int:
    return shutil.disk_usage(folder).free


def _open(home: Path, item_id: str | None) -> bool:
    return bool(item_id) and any(i["id"] == item_id for i in asks_mod.queue_items(home, only_open=True))


def check(home: Path, config: Config, state: dict, now: datetime,
          free: Callable[[Path], int] | None = None) -> bool:
    """Record free space in ``state["disk"]`` and keep the manager queue item in step; True when it changed.

    ``low`` is True below ``config.disk_min_free_gb``; the item opens when it turns low and closes when
    it recovers."""
    free = free or free_bytes
    try:
        smallest = min(free(folder) for folder in lanes_volumes(home, config))
    except OSError:
        return False
    limit = int(config.disk_min_free_gb * GB)
    old = state.get("disk") if isinstance(state.get("disk"), dict) else {}
    disk = {"free_bytes": smallest, "min_free_bytes": limit, "low": smallest < limit, "at": now_iso(now)}
    queue = old.get("queue")
    if disk["low"] and not _open(home, queue):
        text = (f"Disk is almost full: {smallest / GB:.1f} GB free, below the {limit / GB:.1f} GB limit. "
                "Tirith starts no new lanes until space is freed (merged lane worktrees are removed by "
                "themselves; also check the lanes folder and its .cargo-target).")
        queue = asks_mod.queue_add(home, "", "", text, kind=DISK_ITEM)["id"]
    elif not disk["low"] and queue:
        if _open(home, queue):
            asks_mod.queue_done(home, queue, f"{smallest / GB:.1f} GB free again")
        queue = None
    if queue:
        disk["queue"] = queue
    changed = (not old or disk["low"] != old.get("low") or disk.get("queue") != old.get("queue")
               or abs(smallest - int(old.get("free_bytes") or 0)) >= GB // 10)
    if changed:
        state["disk"] = disk
    return changed


def low(state: dict) -> bool:
    return bool((state.get("disk") or {}).get("low"))
