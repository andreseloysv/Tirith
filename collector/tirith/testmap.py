"""The execution map of the Python suite (pytest-testmon): which tests ran which code.

A lane's test stage and review run only the tests whose recorded code changed. The map is built on the project's
main by ``tirith testmap build`` (the merge queue starts it after a landing when the map is old) in a worktree of
its own under ``<home>/testmon/<project>/``, and copied into a lane's worktree before its first selected run
(the lane then keeps its own copy and updates it). Without a map, ``tirith.fasttests`` selects by names and the
import graph. testmon does not see code that runs in other processes (git hooks, ``tirith`` as a subprocess), so
the merge queue keeps the full suite. A selected run asks testmon what it picks first (``--collect-only``): a few
tests run in one process with testmon, more run in parallel under xdist (testmon cannot select under xdist, and a
map some merges old can pick half the suite). See ``docs/CONTRACT-launch.md``."""

from __future__ import annotations

import os
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from tirith import timeouts as timeouts_mod

MAP = ".testmondata"
MAX_AGE_S = 6 * 3600.0  # the merge queue builds a new map after a landing when the map is older than this
BASE_COMMAND = "uv run -q pytest -q -p no:xdist --testmon"  # without xdist: a selected run under it runs no tests
NO_TESTS = 5  # pytest's exit code when it collected nothing
TESTS_FAILED = 1  # pytest's exit code when the run finished with failing tests
SERIAL_MAX = 20  # a selection this small runs in one process with testmon; a larger one runs in parallel
PARALLEL_COMMAND = "uv run -q pytest -q -n auto --maxprocesses 8 -p no:testmon"  # the picked tests, by node id


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


def command(tirith: str | None = None) -> str:
    """The shell step for a selected Python run (:func:`run_selected` from ``collector``), through the ``tirith`` of
    the main checkout, not the lane's (a lane branched before this code would not know the command)."""
    if tirith is None:
        from tirith import tirithcmd
        tirith = tirithcmd.resolved()
    return f"(cd collector && {tirith} testmap run)"


def picked(collector: Path, run=subprocess.run, env: dict | None = None) -> tuple[int, list[str], str]:
    """``(exit code, node ids, output)``: the tests testmon picks for the change in ``collector``, none run."""
    done = run([*shlex.split(BASE_COMMAND), "--collect-only"], cwd=str(collector), capture_output=True, text=True,
               env=env)
    out = done.stdout or ""
    ids = [line.strip() for line in out.splitlines() if "::" in line and not line[:1].isspace()]
    return done.returncode, ids, (out + (done.stderr or "")).strip()


def run_selected(collector: Path, run=subprocess.run, out=None) -> int:
    """Run what testmon picks in ``collector``: nothing picked passes, up to ``SERIAL_MAX`` tests run with testmon in
    one process, more run in parallel by node id (pytest reads them from an ``@file``). Returns pytest's exit code,
    with "nothing collected" as a pass."""
    out = out or sys.stdout
    env = {k: v for k, v in os.environ.items() if k != "VIRTUAL_ENV"}  # the lane's venv, not the one tirith runs in
    code, ids, output = picked(collector, run, env)
    if code not in (0, NO_TESTS):
        print(output[-4000:], file=out, flush=True)
        return code
    if not ids:
        print("testmon: no test reaches the change", file=out, flush=True)
        return 0
    if len(ids) <= SERIAL_MAX:
        done = run(shlex.split(BASE_COMMAND), cwd=str(collector), env=env)
        return 0 if done.returncode == NO_TESTS else done.returncode
    print(f"testmon picked {len(ids)} tests: they run in parallel", file=out, flush=True)
    with tempfile.NamedTemporaryFile("w", suffix=".args", prefix="testmon-", delete=False, encoding="utf-8") as fh:
        fh.write("\n".join(ids) + "\n")
    try:
        done = run([*shlex.split(PARALLEL_COMMAND), f"@{fh.name}"], cwd=str(collector), env=env)
    finally:
        os.unlink(fh.name)
    return 0 if done.returncode == NO_TESTS else done.returncode


def build(home: Path, project: str, repo: Path, base: str = "main", if_stale: bool = False) -> tuple[bool, str]:
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
        if if_stale and not stale(home, project):  # waited for the slot behind another build: that one is enough
            return True, "the map was built meanwhile"
        done = timeouts_mod.run(["/bin/sh", "-c", BASE_COMMAND], "stage:test", cwd=tree / "collector", env=env,
                                home=home, text=True)
    output = ((done.stdout or "") + (done.stderr or "")).strip()
    if done.returncode in (0, NO_TESTS, TESTS_FAILED) and data.is_file():
        # a run that finished is a whole map, failing tests included (testmon picks them again in every lane)
        os.utime(data)  # an unchanged map is still current: its age counts from this build
        return done.returncode != TESTS_FAILED, output
    data.unlink(missing_ok=True)  # a half-written map (interrupted, timed out, crashed) must not select anything
    return False, output


def refresh(home: Path, project: str, repo: Path, base: str, stage_run: str) -> bool:
    """After a landing: start ``tirith testmap build`` in the background when the project runs pytest and its map
    is missing or old. True when it was started."""
    if "pytest" not in stage_run or not stale(home, project):
        return False
    from tirith import launch as launch_mod
    argv = [*launch_mod.tirith_argv(home), "--home", str(home), "testmap", "build", "--project", project,
            "--repo", str(repo), "--base", base, "--if-stale"]
    log = folder(home, project) / "build.log"
    log.parent.mkdir(parents=True, exist_ok=True)
    launch_mod.spawn_detached(argv, home, log)
    return True
