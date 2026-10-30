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
import threading
import time
from datetime import datetime
from pathlib import Path
from typing import Callable

from tirith import timeouts as timeouts_mod
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
TEMP_CLEANUP_ENABLED = True  # the test suite turns it off: no pass touches the real temp folder


def _git(folder: Path, *args: str) -> subprocess.CompletedProcess:
    return timeouts_mod.run(["git", "-C", str(folder), *args], "git", text=True)

def _remove_worktree(repo: Path, *args: str):
    """``git worktree remove`` deletes every file of the lane, so it has its own (minutes-long) limit, not ``git``'s."""
    return timeouts_mod.run(["git", "-C", str(repo), "worktree", "remove", *args], "git-worktree-remove", text=True)



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
        done = _remove_worktree(repo, str(workspace))
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
            if _remove_worktree(repo, str(folder)).returncode != 0:
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


def _guard():
    from tirith import guardwrap
    return guardwrap.guard_module()


def _within(path: Path, root: Path) -> bool:
    """True when ``path`` itself (a link is not followed) lies strictly below ``root``."""
    try:
        return path.parent.resolve() == root or root in path.parent.resolve().parents
    except OSError:
        return False


def _remove_tree(folder: Path, root: Path, deadline: float) -> int:
    """Delete ``folder`` (below the temp ``root`` only), clearing the guard's flag on what it finds there.

    Links are removed, never followed. Stops at ``deadline`` (monotonic); the rest goes in a later pass.
    Returns the bytes freed."""
    if not _within(folder, root) or folder.is_symlink() or not folder.is_dir():
        return 0
    guard, freed, dirs = _guard(), 0, []

    def clear(path: Path) -> None:
        try:
            if not path.is_symlink() and guard._is_locked(path):
                guard._unlock(path)
        except OSError:
            pass

    for current, names, files in os.walk(folder, followlinks=False, onerror=lambda _e: None):
        here = Path(current)
        clear(here)
        dirs.append(here)
        for name in files + [n for n in names if (here / n).is_symlink()]:
            if time.monotonic() > deadline:
                return freed
            path = here / name
            clear(path)
            try:
                size = os.lstat(path).st_size
                path.unlink()
                freed += size
            except OSError:
                pass
    for here in reversed(dirs):
        if time.monotonic() > deadline:
            return freed
        try:
            here.rmdir()
        except OSError:
            pass
    return freed


def pytest_in_use(base: Path) -> set[Path]:
    """``pytest-N`` folders under ``base`` that a running pytest uses as its basetemp (open files or ``--basetemp``;
    with any pytest running, also the newest run, which ``pytest-current`` names)."""
    pids = subprocess.run(["pgrep", "-f", "pytest"], capture_output=True, text=True).stdout.split()
    if not pids:
        return set()
    text = subprocess.run(["lsof", "-p", ",".join(pids), "-Fn"], capture_output=True, text=True).stdout
    text += subprocess.run(["ps", "-p", ",".join(pids), "-ww", "-o", "command="], capture_output=True, text=True).stdout
    used = set()
    for match in re.finditer(re.escape(str(base)) + r"/(pytest-\d+)", text):
        used.add(base / match.group(1))
    current = base / "pytest-current"
    if current.is_symlink():
        try:
            used.add(current.resolve())
        except OSError:
            pass
    return used


def clear_temp_leftovers(tmp_root: Path | None = None, now_s: float | None = None,
                         in_use: Callable[[Path], set[Path]] | None = None, budget_s: float = 60.0) -> int:
    """Remove, under the temp root only, the old test folders that pile up: ``pytest-of-<user>/pytest-N`` not in
    use and over an hour old, ``garbage-*`` and ``pytest-old-archive``. Returns the bytes freed; bounded by
    ``budget_s`` (a big pile takes several passes)."""
    root = Path(tmp_root or tempfile.gettempdir()).resolve()
    now_s = time.time() if now_s is None else now_s
    deadline = time.monotonic() + budget_s
    base = root / f"pytest-of-{getpass.getuser()}"
    freed = 0
    candidates: list[Path] = [root / "pytest-old-archive", *sorted(root.glob("garbage-*"))]
    if base.is_dir() and not base.is_symlink():
        used = {p.resolve() for p in (in_use or pytest_in_use)(base)}
        for run in sorted(base.glob("pytest-*")):
            try:
                old = now_s - run.lstat().st_mtime >= PYTEST_MIN_AGE_S
            except OSError:
                continue
            if old and re.fullmatch(r"pytest-\d+", run.name) and run.resolve() not in used:
                candidates.append(run)
    for folder in candidates:
        if time.monotonic() > deadline:
            break
        freed += _remove_tree(folder, root, deadline)
    return freed


_cleanup_lock = threading.Lock()
_cleanup_state: dict = {"at": None, "thread": None}


def start_temp_cleanup(home: Path, free: int | None = None, *, tmp_root: Path | None = None,
                       every_s: float = 900.0, low_bytes: int = 20 * GB, budget_s: float = 60.0,
                       in_use: Callable[[Path], set[Path]] | None = None, wait: bool = False) -> bool:
    """Start :func:`clear_temp_leftovers` on its own thread, never blocking the cycle: at most once every
    ``every_s`` seconds, at once when ``free`` bytes are under ``low_bytes``, never two at a time.
    What it freed goes to ``diskdiet.jsonl``. True when a pass started (``wait`` joins it, for tests)."""
    if not TEMP_CLEANUP_ENABLED:
        return False
    with _cleanup_lock:
        thread, last = _cleanup_state["thread"], _cleanup_state["at"]
        if thread is not None and thread.is_alive():
            return False
        short = free is not None and free < low_bytes
        if last is not None and time.monotonic() - last < every_s and not short:
            return False

        def work() -> None:
            try:
                freed = clear_temp_leftovers(tmp_root, in_use=in_use, budget_s=budget_s)
                if freed:
                    append_log(home, f"Freed {freed / GB:.1f} GB: old test temp folders")
            except Exception as exc:  # a background pass never takes the collector down
                append_log(home, f"Temp cleanup failed: {exc}")

        thread = threading.Thread(target=work, name="tirith-temp-cleanup", daemon=True)
        _cleanup_state.update(at=time.monotonic(), thread=thread)
        thread.start()
    if wait:
        thread.join()
    return True


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
               running: Callable[[], bool] | None = None,
               biggest: Callable[[], list[tuple[str, int]]] | None = None) -> bool:
    """Below the limit plus ``HEADROOM_GB`` clean by itself: merged lane folders, then the build cache; one calm chat line says what was freed. Nothing left to clean: one information line with
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
    total = swept["bytes"] + cache
    parts = []
    if swept["worktrees"]:
        parts.append(f"{swept['worktrees']} old lane folder{'s' if swept['worktrees'] != 1 else ''}")
    if cache:
        parts.append("build cache")
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


def smallest_free(home: Path, config: Config, free: Callable[[Path], int] | None = None) -> int | None:
    """Free bytes of the emptiest volume holding lanes; None when it cannot be read."""
    try:
        return min((free or free_bytes)(folder) for folder in lanes_volumes(home, config))
    except OSError:
        return None


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
