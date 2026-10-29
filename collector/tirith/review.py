"""``tirith lane review``: tests, then merge, for a lane that ``tirith launch`` started.

The lane's agent leaves its work uncommitted in the worktree (``<repo>-lanes/<lane-id>``). Review runs
the project's test stage there; when it passes, it commits the work on ``lane/<lane-id>``, merges that
branch into the branch checked out in the repo and removes the worktree. See ``docs/CONTRACT-launch.md``.
"""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

from tirith import manifest as manifest_mod
from tirith.home import now_iso
from tirith.launch import worktree_path
from tirith.store import lane_path, read_json, write_lane

OUTPUT_MAX = 4000
TEST_TIMEOUT_S = 1800

PENDING = "pending"          # launched lane, done, not reviewed yet
RUNNING = "running"
TESTS_FAILED = "tests-failed"
READY = "ready"              # tests passed, not merged (--tests-only)
MERGE_FAILED = "merge-failed"
MERGED = "merged"


class ReviewError(Exception):
    """A review that must not start or finish; the message says why and what to do instead."""


def _alive(pid) -> bool:
    if not pid:
        return False
    try:
        os.kill(int(pid), 0)
    except ProcessLookupError:
        return False
    except (OSError, ValueError):
        return True
    return True


def launched(lane: dict, repo: Path | None) -> bool:
    """True for a lane whose workspace is the worktree ``tirith launch`` made for it."""
    if repo is None or not lane.get("workspace"):
        return False
    return Path(str(lane["workspace"])).resolve() == worktree_path(repo, str(lane["id"])).resolve()


def public(lane: dict, repo: Path | None) -> dict | None:
    """The ``review`` object for state.json: None unless the lane is a finished launched lane."""
    review = lane.get("review")
    if isinstance(review, dict) and review.get("status"):
        out = {k: review[k] for k in ("status", "at", "message") if review.get(k)}
        if out["status"] == RUNNING and not _alive(review.get("pid")):
            out["status"], out["message"] = MERGE_FAILED, "the review was interrupted; run it again"
        return out
    if lane.get("ended_at") and lane.get("result") == "done" and launched(lane, repo):
        return {"status": PENDING}
    return None


def test_command(folder: Path) -> str | None:
    """The ``run`` of the first test stage of the project's manifest."""
    for pipe in manifest_mod.load(folder).pipelines:
        for stage in pipe.stages:
            if stage.run and manifest_mod.stage_kind(stage) == "test":
                return stage.run
    return None


def check_ready(home: Path, lane_id: str, repo: Path | None) -> dict:
    """The lane when it may be reviewed now; else raises ``ReviewError`` with the reason."""
    lane = read_json(lane_path(home, lane_id), None)
    if not isinstance(lane, dict):
        raise ReviewError(f"unknown lane: {lane_id}")
    if not lane.get("ended_at"):
        raise ReviewError(f"lane {lane_id} is still running; review it when it ends")
    if lane.get("result") != "done":
        raise ReviewError(f"lane {lane_id} ended with result '{lane.get('result')}', not done")
    if repo is None or not launched(lane, repo):
        raise ReviewError(f"lane {lane_id} was not started by `tirith launch`, so there is no worktree to merge")
    review = lane.get("review") or {}
    if review.get("status") == MERGED:
        raise ReviewError(f"lane {lane_id} is already merged")
    if review.get("status") == RUNNING and _alive(review.get("pid")):
        raise ReviewError(f"lane {lane_id} is already being reviewed")
    if not Path(str(lane["workspace"])).is_dir():
        raise ReviewError(f"the worktree {lane['workspace']} is gone")
    return lane


def _git(folder: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", "-C", str(folder), *args], capture_output=True, text=True)


def _tail(text: str) -> str:
    text = text.strip()
    return text if len(text) <= OUTPUT_MAX else "…" + text[-OUTPUT_MAX:]


def _set(home: Path, lane: dict, status: str, message: str | None = None, **extra) -> dict:
    lane["review"] = {"status": status, "at": now_iso(), "pid": os.getpid(), **extra}
    if message:
        lane["review"]["message"] = message
    write_lane(home, lane)
    return lane["review"]


def run_tests(workspace: Path, command: str) -> tuple[bool, str]:
    env = {k: v for k, v in os.environ.items() if k != "TIRITH_LANE"}
    try:
        done = subprocess.run(command, shell=True, cwd=workspace, capture_output=True, text=True,
                              timeout=TEST_TIMEOUT_S, env=env)
    except subprocess.TimeoutExpired:
        return False, f"tests did not finish in {TEST_TIMEOUT_S} s"
    return done.returncode == 0, _tail(done.stdout + "\n" + done.stderr)


def merge_lane(repo: Path, workspace: Path, lane_id: str, feature: str) -> str:
    """Commit the worktree, merge ``lane/<id>`` into the repo's current branch; returns the merge commit."""
    branch = f"lane/{lane_id}"
    if _git(repo, "status", "--porcelain", "--untracked-files=no").stdout.strip():
        raise ReviewError(f"{repo} has uncommitted changes, so nothing can be merged into it. "
                          "Fix: commit or stash them, then run the review again")
    _git(workspace, "add", "-A")
    if _git(workspace, "diff", "--cached", "--quiet").returncode != 0:
        done = _git(workspace, "commit", "-q", "-m", f"Lane {lane_id}: {feature}")
        if done.returncode != 0:
            raise ReviewError(f"committing the lane failed: {_tail(done.stderr or done.stdout)}")
    done = _git(repo, "merge", "--no-ff", "-q", "-m", f"Merge {branch}: {feature}", branch)
    if done.returncode != 0:
        _git(repo, "merge", "--abort")
        raise ReviewError(f"merging {branch} failed (conflicts?): {_tail(done.stderr or done.stdout)}. "
                          f"Fix: merge {branch} by hand")
    head = _git(repo, "rev-parse", "HEAD").stdout.strip()
    _git(repo, "worktree", "remove", "--force", str(workspace))
    _git(repo, "branch", "-d", branch)
    return head


def review_lane(home: Path, lane_id: str, repo: Path | None, merge: bool = True) -> dict:
    """Tests, then (when ``merge``) the merge. Returns the stored review; failures are stored too."""
    lane = check_ready(home, lane_id, repo)
    workspace = Path(str(lane["workspace"]))
    command = test_command(workspace)
    if not command:
        raise ReviewError("the manifest has no test stage with a run command. "
                          "Fix: add a [[pipelines.stages]] with kind = \"test\" and run = \"...\" to tirith.toml")
    _set(home, lane, RUNNING)
    ok, output = run_tests(workspace, command)
    if not ok:
        return _set(home, lane, TESTS_FAILED, output or "tests failed")
    if not merge:
        return _set(home, lane, READY, "tests passed")
    try:
        head = merge_lane(repo, workspace, lane_id, str(lane.get("feature") or lane_id))
    except ReviewError as error:
        return _set(home, lane, MERGE_FAILED, str(error))
    return _set(home, lane, MERGED, "tests passed; merged", commit=head)
