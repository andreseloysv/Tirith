"""Lanes must not fill the disk: merged worktrees are removed and new lanes stop when space runs low.

Every collector cycle ``dispatch.run`` calls :func:`cleanup` (a finished lane whose branch is merged into the
project's checked-out branch and whose worktree is clean loses its worktree and its branch; a dirty worktree is
left alone and reported in the lane record) and :func:`check` (free space of the volume holding the lanes;
below ``[disk] min_free_gb`` the autopilot and the dispatcher start no lane and one manager queue item says
why). The shared cargo target folder lives in ``tirith.runstage``. See ``docs/CONTRACT-launch.md``.
"""

from __future__ import annotations

import getpass
import os
import re
import shutil
import subprocess
import tempfile
import time
from datetime import datetime
from pathlib import Path
from typing import Callable

from tirith import asks as asks_mod
from tirith import inbox as inbox_mod
from tirith import projects as projects_mod
from tirith import review as review_mod
from tirith.home import Config, now_iso
from tirith.store import read_lanes, write_lane

GB = 1024 ** 3
DISK_ITEM = "disk"
HEADROOM_GB = 3  # cache is cleared when free space is below the limit plus this
PYTEST_MIN_AGE_S = 3600
_TIRITH_MARK = "tirith-home"  # the home folder the Tirith tests make inside their pytest temp dir


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


# -- freeing space by itself ------------------------------------------------------------------


def lanes_folder(repo: Path) -> Path:
    return repo.parent / f"{repo.name}-lanes"


def _tree_size(folder: Path) -> int:
    total = 0
    for root, _dirs, files in os.walk(folder, onerror=lambda _e: None):
        for name in files:
            try:
                total += os.lstat(os.path.join(root, name)).st_size
            except OSError:
                pass
    return total


def busy_folders() -> list[Path]:
    """Working folders of every running process (``lsof`` cwd listing); empty when it cannot be read."""
    done = subprocess.run(["lsof", "-d", "cwd", "-Fn"], capture_output=True, text=True)
    return [Path(line[1:]) for line in done.stdout.splitlines() if line.startswith("n/")]


def process_running(*names: str) -> bool:
    for name in names:
        if subprocess.run(["pgrep", "-x", name], capture_output=True).returncode == 0:
            return True
    return False


def _inside(folder: Path, parent: Path) -> bool:
    return folder == parent or parent in folder.parents


def _worktrees(repo: Path) -> list[tuple[Path, str]]:
    """``(folder, branch)`` of every linked worktree of ``repo`` that is on a branch."""
    out, folder, found = _git(repo, "worktree", "list", "--porcelain").stdout, None, []
    for line in out.splitlines() + [""]:
        if line.startswith("worktree "):
            folder = Path(line[9:])
        elif line.startswith("branch refs/heads/") and folder is not None:
            found.append((folder, line[18:]))
        elif not line:
            folder = None
    return found


def sweep(home: Path, config: Config, now: datetime, busy: Callable[[], list[Path]] | None = None) -> dict:
    """Remove every merged, clean, idle worktree under the lanes folder of each project, whoever made it.

    Never the main checkout, the shared cargo target, a worktree of a lane still working, a worktree a process
    runs in, or one with uncommitted changes (never ``--force``). The fully merged branch goes too.
    Returns ``{"worktrees": n, "bytes": freed}``."""
    busy_list, removed, freed = None, 0, 0
    working = {Path(str(l["workspace"])).resolve() for l in read_lanes(home)
               if l.get("workspace") and not l.get("ended_at")}
    for proj in config.projects:
        if not proj.repo:
            continue
        repo = projects_mod.real(proj.repo)
        root = lanes_folder(repo).resolve()
        for folder, branch in _worktrees(repo):
            folder = folder.resolve()
            if folder == repo.resolve() or not _inside(folder, root) or folder == root or not folder.is_dir():
                continue
            if any(_inside(folder, w) or _inside(w, folder) for w in working):
                continue
            if _git(repo, "merge-base", "--is-ancestor", branch, "HEAD").returncode != 0:
                continue
            if _git(folder, "status", "--porcelain").stdout.strip():
                continue
            if busy_list is None:
                busy_list = [p.resolve() for p in (busy or busy_folders)()]
            if any(_inside(p, folder) for p in busy_list):
                continue
            size = _tree_size(folder)
            if _git(repo, "worktree", "remove", str(folder)).returncode != 0:
                continue
            _git(repo, "branch", "-d", branch)
            removed += 1
            freed += size
    _git_prune = [proj for proj in config.projects if proj.repo]
    for proj in _git_prune:
        _git(projects_mod.real(proj.repo), "worktree", "prune")
    return {"worktrees": removed, "bytes": freed}


def clear_build_cache(home: Path, config: Config, running: Callable[[], bool] | None = None) -> int:
    """Remove the ``incremental`` folders of each project's shared cargo target when no cargo / rustc runs."""
    from tirith import runstage as runstage_mod
    if (running or (lambda: process_running("cargo", "rustc")))():
        return 0
    freed = 0
    for proj in config.projects:
        target = runstage_mod.cargo_target_dir(home, proj.id, proj.repo or ".") if proj.repo else None
        if target is None or not Path(target).is_dir():
            continue
        for incremental in sorted(Path(target).glob("*/incremental")):
            freed += _tree_size(incremental)
            shutil.rmtree(incremental, ignore_errors=True)
    return freed


def clear_pytest_leftovers(tmp_root: Path | None = None, now_s: float | None = None,
                           running: Callable[[], bool] | None = None) -> int:
    """Remove old ``pytest-N`` run folders that hold a Tirith test home, when no pytest runs."""
    if (running or (lambda: process_running("pytest")))():
        return 0
    base = Path(tmp_root or tempfile.gettempdir()) / f"pytest-of-{getpass.getuser()}"
    now_s = time.time() if now_s is None else now_s
    freed = 0
    for run in base.glob("pytest-*") if base.is_dir() else []:
        try:
            old = now_s - run.stat().st_mtime >= PYTEST_MIN_AGE_S
        except OSError:
            continue
        if run.is_symlink() or not old or not any(run.glob(f"*/{_TIRITH_MARK}")):
            continue
        freed += _tree_size(run)
        shutil.rmtree(run, ignore_errors=True)
    return freed


def _biggest(count: int = 3) -> list[tuple[str, int]]:
    """The biggest folders directly under the user's home (outside Tirith), as ``(name, bytes)``."""
    sizes = []
    for entry in Path.home().iterdir():
        if entry.name.startswith(".") or not entry.is_dir() or entry.is_symlink():
            continue
        done = subprocess.run(["du", "-sk", str(entry)], capture_output=True, text=True, timeout=120)
        match = re.match(r"\d+", done.stdout)
        if match:
            sizes.append((entry.name, int(match.group()) * 1024))
    return sorted(sizes, key=lambda x: -x[1])[:count]


def free_space(home: Path, config: Config, state: dict, now: datetime, *,
               free: Callable[[Path], int] | None = None, busy: Callable[[], list[Path]] | None = None,
               running: Callable[[], bool] | None = None, tmp_root: Path | None = None,
               biggest: Callable[[], list[tuple[str, int]]] | None = None) -> bool:
    """Below the limit plus ``HEADROOM_GB`` clean by itself: merged lane folders, then the build cache and
    pytest leftovers; one calm chat line says what was freed. Nothing left to clean: one information line with
    the biggest folders outside Tirith (never a question). True when it said something."""
    free = free or free_bytes
    try:
        before = min(free(folder) for folder in lanes_volumes(home, config))
    except OSError:
        return False
    if before >= int((config.disk_min_free_gb + HEADROOM_GB) * GB):
        state.pop("disk_nothing_said", None)
        return False
    if not any(proj.repo for proj in config.projects):
        return False  # no project, no lanes folder: nothing of Tirith's to clean or to report
    swept = sweep(home, config, now, busy)
    cache = clear_build_cache(home, config, running)
    leftovers = clear_pytest_leftovers(tmp_root, running=running)
    total = swept["bytes"] + cache + leftovers
    parts = []
    if swept["worktrees"]:
        parts.append(f"{swept['worktrees']} old lane folder{'s' if swept['worktrees'] != 1 else ''}")
    if cache:
        parts.append("build cache")
    if leftovers:
        parts.append("test leftovers")
    if parts:
        state.pop("disk_nothing_said", None)
        line = f"Freed {total / GB:.1f} GB: {', '.join(parts)}"
        inbox_mod.add_say(home, line)
        append_log(home, line)
        return True
    if state.get("disk_nothing_said"):
        return False
    state["disk_nothing_said"] = now_iso(now)
    try:
        folders = (biggest or _biggest)()
    except (OSError, subprocess.SubprocessError):
        folders = []
    line = (f"Disk is low ({before / GB:.1f} GB free) and Tirith has nothing left to clean.")
    if folders:
        line += " Biggest folders outside Tirith: " + ", ".join(f"{n} {b / GB:.1f} GB" for n, b in folders) + "."
    inbox_mod.add_say(home, line)
    append_log(home, line)
    return True


def append_log(home: Path, line: str) -> None:
    from tirith.store import append_jsonl
    append_jsonl(Path(home) / "diskdiet.jsonl", {"at": now_iso(), "text": line})


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
