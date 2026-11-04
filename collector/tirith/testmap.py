"""The execution map of the Python suite (pytest-testmon): which tests ran which code.

A lane's test stage and review run only the tests whose recorded code changed. The map is built on the project's
main by ``tirith testmap build`` (the merge queue starts it after a landing when the map is old) in a worktree of
its own under ``<home>/testmon/<project>/``, and copied into a lane's worktree before its first selected run
(the lane then keeps its own copy and updates it). Without a map, ``tirith.fasttests`` selects by names and the
import graph. testmon does not see code that runs in other processes (git hooks, ``tirith`` as a subprocess), so
the merge queue keeps the full suite. See ``docs/CONTRACT-launch.md``."""

from __future__ import annotations

import os
import re
import shutil
import time
from pathlib import Path

from tirith import timeouts as timeouts_mod

MAP = ".testmondata"
MAX_AGE_S = 6 * 3600.0  # the merge queue builds a new map after a landing when the map is older than this
BASE_COMMAND = "uv run -q pytest -q -p no:xdist --testmon"  # without xdist: a selected run under it runs no tests
NO_TESTS = 5  # pytest's exit code when it collected nothing


def folder(home: Path, project: str) -> Path:
    return Path(home) / "testmon" / (re.sub(r"[^A-Za-z0-9._-]", "_", project) or "project")


def map_file(home: Path, project: str) -> Path:
    return folder(home, project) / MAP


def usable(home: Path, project: str) -> bool:
    try:
        return map_file(home, project).stat().st_size > 0
    except OSError:
        return False


def stale(home: Path, project: str, now: float | None = None) -> bool:
    """No map, or one older than ``MAX_AGE_S``."""
    try:
        return (time.time() if now is None else now) - map_file(home, project).stat().st_mtime > MAX_AGE_S
    except OSError:
        return True


def install(home: Path, project: str, worktree: Path) -> bool:
    """Put the map in the lane's ``collector`` folder unless the lane has one already (its own is newer)."""
    target = Path(worktree) / "collector" / MAP
    if target.is_file():
        return True
    if not usable(home, project) or not target.parent.is_dir():
        return False
    try:
        shutil.copyfile(map_file(home, project), target)
    except OSError:
        return False
    return True


def command() -> str:
    """The shell step for a selected Python run: testmon deselects what the change does not reach (exit 5 = none left)."""
    return f"(cd collector && {BASE_COMMAND}; rc=$?; [ $rc -eq 0 ] || [ $rc -eq {NO_TESTS} ])"


def build(home: Path, project: str, repo: Path, base: str = "main") -> tuple[bool, str]:
    """Update the map from ``base`` of ``repo``: a detached worktree of its own is moved to ``base`` and the
    suite runs there without xdist (testmon runs only what changed since the last build). Holds the machine's
    one full-suite slot. ``(ok, output)``."""
    from tirith import loadlimit as loadlimit_mod
    home, repo = Path(home), Path(repo)
    where = folder(home, project)
    tree, data = where / "tree", map_file(home, project)
    where.mkdir(parents=True, exist_ok=True)
    git = lambda *a: timeouts_mod.run(["git", "-C", str(repo if a[0] == "worktree" else tree), *a], "git", text=True)
    git("worktree", "prune")
    done = git("checkout", "-q", "--detach", "-f", base) if tree.is_dir() else \
        git("worktree", "add", "-q", "--detach", str(tree), base)
    if done.returncode != 0:
        return False, f"could not prepare {tree} at {base}: {timeouts_mod.why(done)}"
    env = {**os.environ, "TESTMON_DATAFILE": str(data)}
    env.pop("TIRITH_LANE", None)
    with loadlimit_mod.full_suite(home):
        done = timeouts_mod.run(["/bin/sh", "-c", BASE_COMMAND], "stage:test", cwd=tree / "collector", env=env,
                                home=home, text=True)
    output = ((done.stdout or "") + (done.stderr or "")).strip()
    if done.returncode in (0, NO_TESTS):
        if data.is_file():
            os.utime(data)  # an unchanged map is still current: its age counts from this build
        return True, output
    data.unlink(missing_ok=True)  # a half-written map must not select anything
    return False, output


def refresh(home: Path, project: str, repo: Path, base: str, stage_run: str) -> bool:
    """After a landing: start ``tirith testmap build`` in the background when the project runs pytest and its map
    is missing or old. True when it was started."""
    if "pytest" not in stage_run or not stale(home, project):
        return False
    from tirith import launch as launch_mod
    argv = [*launch_mod.tirith_argv(home), "--home", str(home), "testmap", "build", "--project", project,
            "--repo", str(repo), "--base", base]
    log = folder(home, project) / "build.log"
    log.parent.mkdir(parents=True, exist_ok=True)
    launch_mod.spawn_detached(argv, home, log)
    return True
