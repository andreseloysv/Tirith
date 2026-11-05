"""The single merge queue: agents never touch main, Tirith merges one finished lane at a time.

A lane whose contract is complete (ended ``done`` with the outcome ``done`` in its plan report) is enqueued
(:func:`enqueue`, by the dispatcher or ``tirith merge request``). :func:`process_next` takes the oldest
waiting lane and, in a temporary worktree cut from main: merges the lane branch, runs the project's full
build and test stages, then (under the plan writer lock, with stale ``index.lock`` recovery) fast-forwards
main in the main checkout to exactly that tested commit, pushes, ticks the plan item and cleans up.
A conflict or a red suite goes back to the same agent (:func:`send_back`: a relaunch whose brief continues
the lane's branch), never to the owner. The queue lives in ``<home>/mergeq.json``.

Two things make it fast without dropping a rule: only the stages a lane's own diff needs run
(:func:`select_stages`: a docs-only lane runs the build alone, no ``src-tauri``/``Cargo.*`` change skips the Rust
tests), and up to ``[mergeq] train`` queued lanes of a project are merged together and tested in ONE pass
(:func:`_try_train`); a red train falls back to one lane at a time so only the culprit is rejected.
"""

from __future__ import annotations

import os
import re
import shlex
import shutil
import time
from contextlib import contextmanager
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable

try:
    import fcntl
except ImportError:  # Windows: one dispatcher, one queue consumer
    fcntl = None  # type: ignore[assignment]

from tirith import asks as asks_mod
from tirith import flows as flows_mod
from tirith import manifest as manifest_mod
from tirith import wasted as wasted_mod
from tirith import plangate as plangate_mod
from tirith import review as review_mod
from tirith import selfreload as selfreload_mod
from tirith import stable as stable_mod
from tirith import testmap as testmap_mod
from tirith import reaper as reaper_mod
from tirith import timeouts as timeouts_mod
from tirith.home import Config, now_iso
from tirith.launch import stage_all, worktree_path
from tirith.store import lane_path, read_json, read_lane, write_json_atomic, write_lane

FILE = "mergeq.json"
QUEUED, RUNNING = "queued", "running"
MERGED, CONFLICT, RED, PUSH_FAILED, WAITING, SENT_BACK, CLOSED = (
    "merged", "conflict", "red-suite", "push-failed", "waiting", "sent-back", "closed")
SENT_BACK_STATUS = "sent-back"  # the lane's ``review.status`` once the queue returned it to its agent
MARK = "[merge queue]"
MAX_SENT_BACK = 3  # after this many returns the manager (never the owner) decides
START_WAIT = 20.0  # seconds a returned lane's new agent has to register itself before the manager hears of it
WAIT_NOTICE = 2  # consecutive `waiting` results of one entry after which the manager is told why
MAX_TRIES = 3  # main moved under the merge: start again from the new main
TIMED_OUT = "timed out"  # a git command stopped at its deadline: retried by the queue, never sent back
OUT_OF_TIME = "out-of-time"  # a stage hit its time limit: the suite did not fail
HISTORY = 20
DOC_ONLY = ("docs/", "site/", "ios/")  # a lane that changes only these (or *.md) has nothing for the test stages
RUST_STAGE = "rust-test"
FLAKY_FILE = "flaky.json"  # {project: {test id: {"count": n, "last": iso}}}: tests that failed in the suite and passed alone
WIP_LIMIT = 3  # lanes waiting in the queue at which no new lane starts until it drains below this
OUTPUT_MAX = 1500
PRIORITY_PATHS = frozenset({  # the queue, its stages and their contract: fixes here enter the queue first
    "collector/tirith/mergeq.py", "collector/tirith/runstage.py", "collector/tirith/testmap.py",
    "collector/tirith/timeouts.py", "collector/tests/test_mergeq.py", "docs/CONTRACT-mergeq.md", "tirith.toml"})


def _path(home: Path) -> Path:
    return Path(home) / FILE


@contextmanager
def _locked(home: Path):
    folder = Path(home) / "locks"
    folder.mkdir(parents=True, exist_ok=True)
    handle = open(folder / "mergeq.lock", "a")
    try:
        if fcntl is not None:
            fcntl.flock(handle, fcntl.LOCK_EX)
        yield
    finally:
        handle.close()


def load(home: Path) -> dict:
    data = read_json(_path(home), None)
    if not isinstance(data, dict):
        data = {}
    data.setdefault("queue", [])
    data.setdefault("last", {})
    data.setdefault("history", [])
    return data


def _alive(pid) -> bool:
    return review_mod._alive(pid)


def queued(home: Path, lane_id: str) -> bool:
    return any(e["lane"] == lane_id for e in load(home)["queue"])


def is_priority_path(path: str) -> bool:
    """True for a file of the merge queue or of its stages: a fix there must not wait behind other lanes."""
    return path in PRIORITY_PATHS


def lane_touches_queue(lane: dict) -> bool:
    """True when the lane's work (committed or not, against the merge base with main) changes the merge queue or
    its stages. Any git failure reads as False: the lane then queues in order."""
    workspace = Path(str(lane.get("workspace") or ""))
    if not workspace.is_dir():
        return False
    common = _git(workspace, "rev-parse", "--path-format=absolute", "--git-common-dir").stdout.strip()
    if not common:
        return False
    main = timeouts_mod.run(["git", "--git-dir", common, "rev-parse", "HEAD"], "git", text=True).stdout.strip()
    base = _git(workspace, "merge-base", "HEAD", main).stdout.strip() if main else ""
    if not base:
        return False
    paths = _git(workspace, "diff", "--name-only", base).stdout.split()
    paths += _git(workspace, "ls-files", "--others", "--exclude-standard").stdout.split()
    return any(is_priority_path(p) for p in paths)


def enqueue(home: Path, lane_id: str) -> int:
    """Add the lane to the queue (once); returns its 1-based position. A lane that fixes the queue or its
    stages goes before the waiting lanes that do not (behind the running one and earlier priority lanes)."""
    lane = read_json(lane_path(home, lane_id), None)
    if not isinstance(lane, dict):
        raise review_mod.ReviewError(f"unknown lane: {lane_id}")
    priority = lane_touches_queue(lane)
    with _locked(home):
        data = load(home)
        for n, entry in enumerate(data["queue"], 1):
            if entry["lane"] == lane_id:
                return n
        row = {"lane": lane_id, "project": str(lane.get("project") or ""), "at": now_iso(),
               "status": QUEUED, "feature": str(lane.get("feature") or lane_id)}
        place = len(data["queue"])
        if priority:
            row["priority"] = True
            place = next((n for n, e in enumerate(data["queue"])
                          if e["status"] != RUNNING and not e.get("priority")), place)
        data["queue"].insert(place, row)
        write_json_atomic(_path(home), data)
        return place + 1


def shares_lane_worktree(lane: dict, repo: Path) -> bool:
    """True for a continuation lane: its workspace is another lane's worktree (a folder of ``<repo>-lanes``)."""
    if not lane.get("workspace"):
        return False
    folder = Path(str(lane["workspace"])).resolve()
    return folder.parent == worktree_path(repo, "x").parent.resolve() and not folder.name.startswith(("_", "."))


def ready_to_enqueue(home: Path, lane: dict, repo: Path | None) -> bool:
    """A finished launched lane whose plan outcome is ``done`` and that is not merged or queued yet."""
    if not lane.get("ended_at") or lane.get("result") != "done" or repo is None:
        return False
    if (lane.get("outcome") or {}).get("status") != "done" or not (review_mod.launched(lane, repo)
                                                                   or shares_lane_worktree(lane, repo)):
        return False
    status = (lane.get("review") or {}).get("status")
    return status in (None, review_mod.PENDING, review_mod.READY) and Path(str(lane["workspace"])).is_dir() \
        and not queued(home, str(lane["id"]))


def _may_enqueue(seen: dict) -> bool:
    """False for a lane row that cannot be enqueued, so its file is not read again every cycle."""
    if "ended_at" in seen and not seen["ended_at"]:
        return False
    if "result" in seen and seen["result"] != "done":
        return False
    status = (seen.get("review") or {}).get("status")
    return status in (None, review_mod.PENDING, review_mod.READY)


def wip_block(home: Path) -> str:
    """Why no new lane may start now (empty when it may): ``WIP_LIMIT`` or more lanes wait in the queue."""
    waiting = len(load(home)["queue"])
    if waiting < WIP_LIMIT:
        return ""
    return (f"{waiting} lanes are waiting in the merge queue (limit {WIP_LIMIT}): finish what was started before "
            f"starting more. The item stays open: launch it again when the queue is below {WIP_LIMIT} "
            "(`tirith merge status` shows it), or help it drain by fixing any lane the queue sent back.")


def view(home: Path, project: str | None = None) -> dict:
    """The queue for the plan view: ``{"items": [{lane, position, status, ...}], "last": {...}|None}``."""
    data = load(home)
    items = [{"lane": e["lane"], "position": n, "status": e["status"], "since": e["at"],
              "feature": e.get("feature"), **({"message": e["message"]} if e.get("message") else {})}
             for n, e in enumerate(data["queue"], 1) if project is None or e.get("project") == project]
    rows = [h for h in data["history"] if project is None or h.get("project") == project]
    last = data["last"].get(project) if project else (rows[-1] if rows else None)
    flaky = read_json(Path(home) / FLAKY_FILE, {})
    flaky = flaky.get(project) if project and isinstance(flaky, dict) else flaky
    return {"items": items, "last": last, "recent": rows[-5:], "flaky": flaky if isinstance(flaky, dict) else {}}


# -- taking work -------------------------------------------------------------------------------


CANARY = "canary"  # the end-to-end test project: its own tiny check, its own track, never behind other projects


def _lane_of(entry: dict) -> bool:
    return entry.get("project") == CANARY


def _claim_many(home: Path, skip: frozenset[str] = frozenset(), limit: int = 1) -> list[dict]:
    """Take the oldest waiting entry and up to ``limit - 1`` more of the same project, oldest first.

    One merger at a time per track: the canary has its own track, so a busy tirith merge never holds it back."""
    with _locked(home):
        data = load(home)
        for entry in data["queue"]:
            if entry["status"] == RUNNING and not _alive(entry.get("pid")):
                entry["status"] = QUEUED  # the merger died: take it again
        live = {_lane_of(e) for e in data["queue"] if e["status"] == RUNNING}
        waiting = []
        for track in (True, False):  # the canary track first: it is tiny and must not wait for the others
            if track not in live:
                waiting = [e for e in data["queue"] if e["status"] == QUEUED and e["lane"] not in skip
                           and _lane_of(e) == track]
                if waiting:
                    break
        if not waiting:
            write_json_atomic(_path(home), data)
            return []
        taken = [e for e in waiting if e.get("project") == waiting[0].get("project")][:max(1, limit)]
        for entry in taken:
            entry.update(status=RUNNING, pid=os.getpid(), started=now_iso())
            entry.pop("message", None)
        write_json_atomic(_path(home), data)
        return [dict(e) for e in taken]


def _claim(home: Path, skip: frozenset[str] = frozenset()) -> dict | None:
    taken = _claim_many(home, skip, 1)
    return taken[0] if taken else None


def _finish(home: Path, entry: dict, result: str, message: str = "", *, keep: bool = False, **extra) -> dict:
    """Leave the queue (or wait in it again with ``keep``) and remember the result."""
    with _locked(home):
        data = load(home)
        row = {"lane": entry["lane"], "project": entry["project"], "result": result, "at": now_iso(),
               "message": message[:OUTPUT_MAX], **extra}
        if keep:
            for e in data["queue"]:
                if e["lane"] == entry["lane"]:
                    e.update(status=QUEUED, message=message[:OUTPUT_MAX], **extra)
                    e.pop("pid", None)
        else:
            data["queue"] = [e for e in data["queue"] if e["lane"] != entry["lane"]]
            data["history"] = (data["history"] + [row])[-HISTORY:]
            wasted_mod.record(home, row)
        data["last"][entry["project"]] = row
        write_json_atomic(_path(home), data)
    return row


def note_merged(home: Path, project: str, lane: str, commit: str, message: str, **extra) -> dict:
    """Remember a merge made outside the queue (``tirith lane review`` merges on its own), so ``last`` on the
    plan page is the newest merge and not an old failure of the queue."""
    with _locked(home):
        data = load(home)
        row = {"lane": lane, "project": project, "result": "merged", "at": now_iso(),
               "message": message[:OUTPUT_MAX], "commit": commit, **extra}
        data["history"] = (data["history"] + [row])[-HISTORY:]
        data["last"][project] = row
        write_json_atomic(_path(home), data)
    return row


# -- the merge ---------------------------------------------------------------------------------


def _git(folder: Path, *args: str):
    return plangate_mod._git(folder, *args)

def _remove_worktree(repo: Path, *args: str):
    """``git worktree remove`` deletes every file of the lane, so it has its own (minutes-long) limit, not ``git``'s."""
    return timeouts_mod.run(["git", "-C", str(repo), "worktree", "remove", *args], "git-worktree-remove", text=True)



def _tail(text: str) -> str:
    text = (text or "").strip()
    return text if len(text) <= OUTPUT_MAX else "…" + text[-OUTPUT_MAX:]


def failed_tests(output: str) -> list[str]:
    """The pytest ids a red test stage reports as failed; empty when they cannot be told (an error, no report)."""
    if re.search(r"^ERROR ", output or "", re.M):
        return []
    return list(dict.fromkeys(re.findall(r"^FAILED (.+?)(?: - |$)", output or "", re.M)))


def note_flaky(home: Path, project: str, tests: list[str]) -> None:
    """Count each test that failed in the suite and passed on its own repeat."""
    with _locked(home):
        data = read_json(Path(home) / FLAKY_FILE, None)
        data = data if isinstance(data, dict) else {}
        rows = data.setdefault(project, {})
        for test in tests:
            row = rows.setdefault(test, {"count": 0})
            row.update(count=int(row.get("count", 0)) + 1, last=now_iso())
        write_json_atomic(Path(home) / FLAKY_FILE, data)


def rerun_failed(home: Path, project: str, pipeline: str, stage, tmp: Path, lane_id: str, output: str) -> list[str] | None:
    """Repeat the failed tests one at a time with ``-n 0``; the flaky ids when all pass, None for a real red."""
    tests = failed_tests(output)
    if not tests:
        return None
    for test in tests:
        ok, _ = review_mod.run_tests(home, project, pipeline, f"{stage.name}-rerun", tmp,
                                     f"{stage.run} -n 0 {shlex.quote(test)}", lane_id, full=False)
        if not ok:
            return None
    note_flaky(home, project, tests)
    return tests


def refresh_map(home: Path, repo: Path, project: str, base: str, stages: list) -> None:
    """After a landing, start the build of the execution map when it is missing or old (see ``tirith.testmap``)."""
    for _, stage in stages:
        if manifest_mod.stage_kind(stage) == "test" and testmap_mod.refresh(home, project, repo, base, stage.run):
            return


def stages_of(folder: Path) -> list[tuple[str, manifest_mod.Stage]]:
    """``(pipeline id, stage)`` of every build and test stage with a command of the gating pipelines, in manifest order."""
    return [(pipe.id, stage) for pipe in manifest_mod.load(folder).pipelines if pipe.gating for stage in pipe.stages
            if stage.run and manifest_mod.stage_kind(stage) in ("build", "test")]


def changed_paths(repo: Path, base: str, branch: str) -> list[str]:
    """The files the lane's own commits changed: its diff against the merge base with main."""
    mb = _git(repo, "merge-base", base, branch).stdout.strip()
    return _git(repo, "diff", "--name-only", mb, branch).stdout.split() if mb else []


def needs_of(paths: list[str]) -> tuple[bool, bool]:
    """``(test stages, rust stage)`` a lane with these changed files needs; an unknown diff needs everything."""
    if not paths:
        return True, True
    if all(p.startswith(DOC_ONLY) or p.endswith(".md") for p in paths):
        return False, False
    rust = any(p.startswith("app/src-tauri/") or p.rsplit("/", 1)[-1].startswith("Cargo.") for p in paths)
    return True, rust


def select_stages(stages: list[tuple[str, manifest_mod.Stage]], needs: list[tuple[bool, bool]]
                  ) -> tuple[list[tuple[str, manifest_mod.Stage]], list[str]]:
    """The stages to run for lanes with these ``needs`` (the union), and the names skipped. Builds always run."""
    tests, rust = any(n[0] for n in needs), any(n[1] for n in needs)
    kept, skipped = [], []
    for pipeline, stage in stages:
        is_test = manifest_mod.stage_kind(stage) == "test"
        if is_test and not (rust if stage.name == RUST_STAGE else tests):
            skipped.append(stage.name)
        else:
            kept.append((pipeline, stage))
    return kept, skipped


def flaky_note(flaky: list[str]) -> str:
    return f" (flaky, passed on repeat: {', '.join(flaky)})" if flaky else ""


def skip_note(skipped: list[str]) -> str:
    return f" (skipped {', '.join(skipped)}: nothing it tests changed)" if skipped else ""


def branch_exists(repo: Path, branch: str) -> bool:
    return _git(repo, "rev-parse", "--verify", "-q", f"refs/heads/{branch}").returncode == 0


def branch_of(lane: dict, workspace: Path) -> str:
    """The branch the lane works on: the one recorded on it, else the one checked out in its workspace (a
    continuation lane uses the original lane's worktree), else ``lane/<id>``."""
    recorded = str(lane.get("branch") or "")
    if recorded:
        return recorded
    if workspace.is_dir():
        current = _git(workspace, "rev-parse", "--abbrev-ref", "HEAD").stdout.strip()
        if current.startswith("lane/"):
            return current
    return f"lane/{lane['id']}"


def absorbed_by(home: Path, repo: Path, lane_id: str, branch: str) -> str:
    """The id of another queued lane whose branch already contains every commit of ``branch``, else "". A lane
    continued from this one holds its commits, so testing both is a second suite for the same code; when two
    branches are the same commit the one queued first is kept."""
    queue = [e for e in load(home)["queue"]]
    mine = next((n for n, e in enumerate(queue) if e["lane"] == lane_id), len(queue))
    for n, other in enumerate(queue):
        if other["lane"] == lane_id:
            continue
        found = read_json(lane_path(home, other["lane"]), None)
        if not isinstance(found, dict) or not found.get("workspace"):
            continue
        theirs = branch_of(found, Path(str(found["workspace"])))
        if theirs == branch or not branch_exists(repo, theirs):
            continue
        if _git(repo, "merge-base", "--is-ancestor", branch, theirs).returncode != 0:
            continue
        if _git(repo, "merge-base", "--is-ancestor", theirs, branch).returncode == 0 and n > mine \
                and not _git(Path(str(found["workspace"])), "status", "--porcelain").stdout.strip():
            continue  # the same commit and nothing more in its worktree: the first in the queue stays
        return str(other["lane"])
    return ""


def _commit_workspace(workspace: Path, lane_id: str, feature: str) -> str | None:
    """Commit what the agent left in its worktree; the error text when that fails."""
    added = stage_all(workspace)
    if added.returncode != 0 and "index.lock" in added.stderr and plangate_mod.recover_git_lock(workspace):
        added = stage_all(workspace)  # a crashed git left its lock in the lane's worktree
    if _git(workspace, "diff", "--cached", "--quiet").returncode != 0:
        done = _git(workspace, "commit", "-q", "-m", f"Lane {lane_id}: {feature}")
        if done.returncode != 0 and "index.lock" in done.stderr and plangate_mod.recover_git_lock(workspace):
            done = _git(workspace, "commit", "-q", "-m", f"Lane {lane_id}: {feature}")
        if done.returncode != 0:
            return f"{TIMED_OUT}: {done.error}" if done.timed_out else _tail(done.stderr or done.stdout)
    elif added.returncode != 0:
        return _tail(added.stderr or added.stdout)
    return None


def _plan_paths(repo: Path) -> set[str]:
    """Repo-relative paths of the project's plan files (the roadmap and the manifest ``plans``)."""
    found = [plangate_mod.plan_file(repo), *(repo / p for p in manifest_mod.load(repo).plans)]
    out = set()
    for path in found:
        try:
            out.add(path.resolve().relative_to(repo.resolve()).as_posix()) if path else None
        except ValueError:
            pass
    return out


def _with_plan_moves(repo: Path, tmp: Path, base: str, head: str, tested: str, branch: str, feature: str) -> str | None:
    """Main moved while the suite ran: when only plan files changed, merge them into the tested
    result without a second suite run and return the new commit; None means retry from the new main."""
    plans = _plan_paths(repo)
    if not plans or _git(repo, "merge-base", "--is-ancestor", base, head).returncode != 0:
        return None
    moved = _git(repo, "diff", "--name-only", base, head).stdout.split()
    if any(name not in plans for name in moved):
        return None
    if _git(tmp, "merge", "--no-ff", "-q", "-m", f"Merge {branch}: {feature}", head).returncode != 0:
        _git(tmp, "merge", "--abort")
        return None
    landed = _git(tmp, "rev-parse", "HEAD").stdout.strip()
    differs = _git(tmp, "diff", "--name-only", tested, landed).stdout.split()
    if any(name not in plans for name in differs):
        return None
    return landed


def resolve_plan_conflict(repo: Path, tmp: Path, lane: dict, files: list[str]) -> bool:
    """A merge in ``tmp`` stopped on conflicts: when they are all in plan files (docs/TODO.md), keep main's
    version of them, write the lane's plan report again (``tirith plan report``) and finish the merge.
    False leaves the merge unfinished (the caller aborts it)."""
    plans = _plan_paths(repo)
    if not files or any(name not in plans for name in files):
        return False
    linked = plangate_mod.lane_item(lane)
    mutate = plangate_mod.outcome_mutation(lane, *linked) if linked else None
    for name in files:
        if _git(tmp, "checkout", "--ours", "--", name).returncode != 0:
            return False
        if mutate:
            path = tmp / name
            try:
                path.write_text(mutate(path.read_text(encoding="utf-8")), encoding="utf-8")
            except Exception:  # the item is gone from main's plan: main's version stands
                _git(tmp, "checkout", "--ours", "--", name)
        if _git(tmp, "add", "--", name).returncode != 0:
            return False
    if _git(tmp, "diff", "--name-only", "--diff-filter=U").stdout.split():
        return False
    return _git(tmp, "commit", "-q", "--no-edit").returncode == 0


def _land(home: Path, repo: Path, tmp: Path, project: str, base: str, tested: str, main_branch: str,
          subject: str, via: str) -> tuple[str, tuple[str, str] | None]:
    """Fast-forward main to the tested commit under the plan writer lock: ``(commit, None)``, else ``("", (result, message))``."""
    with plangate_mod.writer_lock(home, project):
        head = _git(repo, "rev-parse", "HEAD").stdout.strip()
        if head != base:
            tested = _with_plan_moves(repo, tmp, base, head, tested, via, subject)
            if tested is None:
                return "", ("retry", "main moved while the suite ran")
        # A dirty main checkout only blocks the landing when a dirty file is one the fast-forward would change;
        # the tested tree is remembered, so waiting for it does not run the suite again.
        dirty = set(_git(repo, "diff", "--name-only", "HEAD").stdout.split())
        if dirty:
            clash = sorted(dirty & set(_git(repo, "diff", "--name-only", "HEAD", tested).stdout.split()))
            if clash:
                return "", (WAITING, f"{repo} has uncommitted changes in {', '.join(clash)}, which the merge changes; "
                                     "the queue waits for them to be committed or dropped")
        plangate_mod.recover_git_lock(repo)
        moved = _git(repo, "merge", "--ff-only", "-q", tested)
        if moved.returncode != 0 and "index.lock" in moved.stderr and plangate_mod.recover_git_lock(repo):
            moved = _git(repo, "merge", "--ff-only", "-q", tested)
        if moved.returncode != 0:
            return "", (WAITING, f"fast-forwarding {main_branch} failed: {timeouts_mod.why(moved, OUTPUT_MAX)}")
    return tested, None


def _make_tmp(repo: Path, name: str, base: str):
    """A clean temporary worktree at ``base``: ``(path, error text or None)``."""
    tmp = worktree_path(repo, name)
    if tmp.exists():
        _remove_worktree(repo, "--force", str(tmp))
        shutil.rmtree(tmp, ignore_errors=True)
    _git(repo, "worktree", "prune")
    made = _git(repo, "worktree", "add", "-q", "--detach", str(tmp), base)
    return tmp, (None if made.returncode == 0 else _tail(made.stderr or made.stdout))


def _drop_tmp(repo: Path, tmp: Path) -> None:
    _remove_worktree(repo, "--force", str(tmp))
    shutil.rmtree(tmp, ignore_errors=True)
    _git(repo, "worktree", "prune")


class Cancelled(Exception):
    """The merge in progress became unnecessary (another queued lane holds all of its commits)."""


def superseded_message(home: Path, repo: Path, lane_id: str, branch: str) -> str:
    """Why the running merge of ``branch`` is no longer needed (empty while it is): another queued lane's branch
    now contains every commit of it. Checked between stages, so a lane queued meanwhile cancels the merge."""
    inside = absorbed_by(home, repo, lane_id, branch)
    return f"nothing to merge: {branch} is already in the queued branch of {inside}" if inside else ""


def _run_stages(home: Path, project: str, stages: list, tmp: Path, lane_id: str,
                stop: Callable[[], str] | None = None) -> tuple[str | None, str, str | None, list[str]]:
    """Run the stages: ``(failed "pipeline/stage" | None, output of the failure, passed summary, flaky test ids)``.

    A red stage first repeats its failed tests alone (:func:`rerun_failed`): when they all pass the stage goes on."""
    passed, flaky = None, []
    for pipeline, stage in stages:
        if stop and (why := stop()):
            raise Cancelled(why)
        suite = (manifest_mod.stage_kind(stage) == "test" and stage.name != RUST_STAGE
                 and project != CANARY)  # one full suite at a time; the canary's tiny check never waits for one
        ok, output = review_mod.run_tests(home, project, pipeline, stage.name, tmp, stage.run, lane_id, full=suite)
        if not ok and output.startswith(review_mod.DID_NOT_FINISH):  # out of time: nothing to repeat
            return f"{pipeline}/{stage.name}", output, passed, flaky
        if not ok:
            again = rerun_failed(home, project, pipeline, stage, tmp, lane_id, output)
            if again is None:
                return f"{pipeline}/{stage.name}", output, passed, flaky
            flaky += again
            continue
        passed = review_mod.tests_passed(output) or passed
    return None, "", passed, flaky


def _try(home: Path, repo: Path, lane: dict, entry: dict) -> tuple[str, str, dict]:
    """One attempt: ``(result, message, extra)``; ``waiting`` / ``retry`` mean try again later / at once."""
    lane_id, project = str(lane["id"]), str(lane.get("project") or "")
    feature = str(lane.get("feature") or lane_id)
    workspace = Path(str(lane["workspace"]))
    branch = branch_of(lane, workspace)
    failed = _commit_workspace(workspace, lane_id, feature)
    if failed:
        if failed.startswith(TIMED_OUT):  # git was cut off, the lane is not at fault
            return WAITING, f"committing the lane {failed}", {}
        return RED, f"committing the lane failed: {failed}", {}
    if not branch_exists(repo, branch):
        return WAITING, f"branch not found: {branch}", {}
    if _git(repo, "merge-base", "--is-ancestor", branch, "HEAD").returncode == 0:
        return plangate_mod.WORK_NONE, f"nothing to merge: {branch} has no commits main lacks", {"branch": branch}
    inside = absorbed_by(home, repo, lane_id, branch)
    if inside:
        return plangate_mod.WORK_NONE, f"nothing to merge: {branch} is already in the queued branch of {inside}", \
            {"branch": branch, "wasted": "absorbed"}
    base = _git(repo, "rev-parse", "HEAD").stdout.strip()
    main_branch = _git(repo, "rev-parse", "--abbrev-ref", "HEAD").stdout.strip()
    tmp, error = _make_tmp(repo, f"_mergeq-{lane_id}", base)
    if error:
        return WAITING, f"could not make the temporary worktree: {error}", {}
    try:
        merged = _git(tmp, "merge", "--no-ff", "-q", "-m", f"Merge {branch}: {feature}", branch)
        if merged.timed_out:  # git was cut off by its time limit: not a conflict, try again, never the lane
            _git(tmp, "merge", "--abort")
            return "retry", f"merging {branch} into {main_branch} {TIMED_OUT}; trying again", {}
        if merged.returncode != 0:
            files = _git(tmp, "diff", "--name-only", "--diff-filter=U").stdout.split()
            if not resolve_plan_conflict(repo, tmp, lane, files):
                _git(tmp, "merge", "--abort")
                what = ("conflicts in " + ", ".join(files)) if files else _tail(merged.stderr or merged.stdout)
                return CONFLICT, f"merging {branch} into {main_branch} failed: {what}", \
                    {"main": main_branch, "files": files}
        stages = stages_of(tmp)
        if not stages:
            return WAITING, "the manifest has no build or test stage with a run command", {}
        stages, skipped = select_stages(stages, [needs_of(changed_paths(repo, base, branch))])
        try:
            broke, output, passed, flaky = _run_stages(home, project, stages, tmp, lane_id,
                                                       lambda: superseded_message(home, repo, lane_id, branch))
        except Cancelled as cancelled:  # another lane made this merge unnecessary: stop the suite, leave main alone
            return plangate_mod.WORK_NONE, str(cancelled), {"branch": branch, "wasted": "discarded"}
        if broke and output.startswith(review_mod.DID_NOT_FINISH):  # out of time, not red: never sent back
            return OUT_OF_TIME, f"stage {broke} ran out of time on {branch}: {_tail(output)}", {}
        if broke:
            return RED, f"stage {broke} failed on {branch} merged with {main_branch}:\n{_tail(output)}", {"main": main_branch}
        if why := superseded_message(home, repo, lane_id, branch):  # last look before main moves
            return plangate_mod.WORK_NONE, why, {"branch": branch, "wasted": "discarded"}
        tested = _git(tmp, "rev-parse", "HEAD").stdout.strip()
        tested, refused = _land(home, repo, tmp, project, base, tested, main_branch, feature, branch)
        if refused:
            return refused[0], refused[1], {}
        refresh_map(home, repo, project, main_branch, stages)
        return MERGED, "tests passed; merged" + skip_note(skipped) + flaky_note(flaky), \
            {"commit": tested, "main": main_branch, "passed": passed, "branch": branch, "skipped": skipped,
             **({"flaky": flaky} if flaky else {})}
    finally:
        _drop_tmp(repo, tmp)


def _try_train(home: Path, repo: Path, members: list[tuple[dict, dict]]) -> tuple[str, str, dict]:
    """One attempt for several lanes at once: ``(result, message, info)``.

    ``info["joined"]`` are the ``(entry, lane)`` pairs merged into the temp worktree, ``info["left"]`` the
    ``(entry, lane, result, message, extra)`` of those that could not join (a real conflict, a failed commit).
    The result is MERGED / RED (the joined lanes together are red) / WAITING / ``retry`` as for :func:`_try`."""
    joined: list[tuple[dict, dict]] = []
    left: list[tuple] = []
    info: dict = {"joined": joined, "left": left}
    for entry, lane in members:
        lane_id = str(lane["id"])
        workspace = Path(str(lane["workspace"]))
        branch = branch_of(lane, workspace)
        failed = _commit_workspace(workspace, lane_id, str(lane.get("feature") or lane_id))
        if failed:
            timed = failed.startswith(TIMED_OUT)
            left.append((entry, lane, WAITING if timed else RED,
                         f"committing the lane {failed}" if timed else f"committing the lane failed: {failed}", {}))
        elif not branch_exists(repo, branch):
            left.append((entry, lane, WAITING, f"branch not found: {branch}", {}))
        elif _git(repo, "merge-base", "--is-ancestor", branch, "HEAD").returncode == 0:  # main has all of it
            left.append((entry, lane, plangate_mod.WORK_NONE,
                         f"nothing to merge: {branch} has no commits main lacks", {"branch": branch}))
        elif absorbed_by(home, repo, lane_id, branch):
            left.append((entry, lane, plangate_mod.WORK_NONE,
                         f"nothing to merge: {branch} is already in the queued branch of "
                         f"{absorbed_by(home, repo, lane_id, branch)}", {"branch": branch, "wasted": "absorbed"}))
        else:
            joined.append((entry, lane))
    if not joined:
        return WAITING, "no lane could join the train", info
    base = _git(repo, "rev-parse", "HEAD").stdout.strip()
    main_branch = _git(repo, "rev-parse", "--abbrev-ref", "HEAD").stdout.strip()
    tmp, error = _make_tmp(repo, f"_mergeq-train-{joined[0][0]['lane']}", base)
    if error:
        return WAITING, f"could not make the temporary worktree: {error}", info
    try:
        needs, branches = [], {}
        for pair in list(joined):
            entry, lane = pair
            branch = branch_of(lane, Path(str(lane["workspace"])))
            feature = str(lane.get("feature") or lane["id"])
            merged = _git(tmp, "merge", "--no-ff", "-q", "-m", f"Merge {branch}: {feature}", branch)
            if merged.timed_out:
                _git(tmp, "merge", "--abort")
                return "retry", f"merging {branch} into {main_branch} {TIMED_OUT}; trying again", info
            files = _git(tmp, "diff", "--name-only", "--diff-filter=U").stdout.split() if merged.returncode != 0 else []
            if merged.returncode != 0 and not resolve_plan_conflict(repo, tmp, lane, files):
                _git(tmp, "merge", "--abort")
                what = ("conflicts in " + ", ".join(files)) if files else _tail(merged.stderr or merged.stdout)
                joined.remove(pair)
                left.append((entry, lane, CONFLICT, f"merging {branch} into {main_branch} failed: {what}",
                             {"main": main_branch, "files": files}))
                continue
            paths = changed_paths(repo, base, branch)
            needs.append(needs_of(paths))
            branches[str(lane["id"])] = (branch, len(paths))
        info["branches"], info["main"] = branches, main_branch
        if not joined:
            return WAITING, "no lane could join the train", info
        stages = stages_of(tmp)
        if not stages:
            return WAITING, "the manifest has no build or test stage with a run command", info
        stages, skipped = select_stages(stages, needs)
        ids = ", ".join(str(lane["id"]) for _, lane in joined)
        broke, output, passed, flaky = _run_stages(home, str(joined[0][1].get("project") or ""), stages, tmp,
                                                   str(joined[0][1]["id"]))
        if broke and output.startswith(review_mod.DID_NOT_FINISH):  # out of time, not red: the train stays whole
            return OUT_OF_TIME, f"stage {broke} ran out of time on the train ({ids}) merged with {main_branch}: {_tail(output)}", info
        if broke:
            return RED, f"stage {broke} failed on the train ({ids}) merged with {main_branch}:\n{_tail(output)}", info
        tested = _git(tmp, "rev-parse", "HEAD").stdout.strip()
        tested, refused = _land(home, repo, tmp, str(joined[0][1].get("project") or ""), base, tested, main_branch,
                                f"train of {len(joined)}: {ids}", "train")
        if refused:
            return refused[0], refused[1], info
        info.update(commit=tested, passed=passed, skipped=skipped, flaky=flaky)
        refresh_map(home, repo, str(joined[0][1].get("project") or ""), main_branch, stages)
        return MERGED, f"tests passed; merged in a train of {len(joined)}" + skip_note(skipped) + flaky_note(flaky), info
    finally:
        _drop_tmp(repo, tmp)


def _push(repo: Path, branch: str) -> str | None:
    """Push main when the repo has an ``origin``; the failure text, else None."""
    if _git(repo, "remote", "get-url", "origin").returncode != 0:
        return None
    # a push writes to the origin and may run hooks: it takes the commit's limit, not the read-only `git` one (30 s
    # is too short for a loaded machine, and a timed-out push read as push-failed)
    done = timeouts_mod.run(["git", "-C", str(repo), "push", "-q", "origin", branch], "git-commit", text=True)
    return None if done.returncode == 0 else _tail(done.stderr or done.stdout)


def _sent_back_count(lane: dict) -> int:
    try:
        return Path(str(lane.get("brief") or "")).read_text(encoding="utf-8", errors="replace").count(MARK)
    except OSError:
        return 0


def feedback_note(result: str, message: str, main: str) -> str:
    if result == CONFLICT:
        todo = (f"Bring `{main}` into your branch and resolve the conflicts: in your worktree run `git merge {main}`, "
                "fix the files, commit")
    else:
        todo = f"Fix what failed (the suite ran on your branch merged with `{main}`), commit"
    return (f"## {MARK} Your work was not merged\n\n{message}\n\n{todo}. Do not merge into or push to the main "
            "checkout: when the suite passes locally run `tirith plan report --lane $TIRITH_LANE --status done` and "
            "Tirith's merge queue merges you.")


def agent_started(home: Path, lane_id: str, wait: float | None = None) -> bool:
    """The relaunched agent registered its lane (``tirith lane run`` writes it when the agent starts), waiting
    up to ``wait`` seconds (default :data:`START_WAIT`)."""
    deadline = time.monotonic() + (START_WAIT if wait is None else wait)
    while True:
        if read_lane(home, lane_id) is not None:
            return True
        if time.monotonic() >= deadline:
            return False
        time.sleep(0.25)


def _reopen_item(home: Path, config: Config, lane: dict, result: str, message: str) -> str:
    """Reopen the lane's plan item with the reason it came back; the error text when that fails, else ""."""
    repo = plangate_mod.repo_of(home, str(lane.get("project") or ""), config)
    if repo is None:
        return ""
    try:
        plangate_mod.review_failed(home, repo, lane, f"{result}: {message.splitlines()[0] if message else ''}")
    except plangate_mod.PlanGateError as error:
        return str(error)
    return ""


def send_back(home: Path, config: Config, lane: dict, result: str, message: str, main: str,
              relaunch: Callable | None = None) -> str:
    """Return the lane to its agent with the conflict or failure; the id of the new lane, else the manager item."""
    lane_id, project = str(lane["id"]), str(lane.get("project") or "")
    note = feedback_note(result, message, main)
    alias = str(lane.get("agent") or "")
    if relaunch is None:
        def relaunch(home, lane, alias, note):  # late import: cli imports this module
            from tirith.cli import relaunch_lane
            return relaunch_lane(home, lane, alias, by_tirith=True, note=note)
    sent, why = None, ""
    if alias and alias not in config.agents:
        why = f" (its agent '{alias}' is not in the configuration)"
    elif _sent_back_count(lane) >= MAX_SENT_BACK:
        why = f" (sent back {MAX_SENT_BACK} times already)"
    else:  # no registered agent (null): the configured worker takes it
        try:
            sent = relaunch(home, lane, alias or None, note)
        except (Exception, SystemExit) as error:
            why = f" (sending it back failed: {error})"
    reopen_error = _reopen_item(home, config, lane, result, message)
    if reopen_error:
        why += f" (its plan item could not be reopened: {reopen_error})"
    started = bool(sent) and agent_started(home, sent)
    fresh = read_json(lane_path(home, lane_id), lane)
    fresh["review"] = {"status": SENT_BACK_STATUS, "at": now_iso(), "by": "mergeq", "message": message[:OUTPUT_MAX]}
    if sent:
        fresh["review"].update(sent_to=sent, started=started)
    write_lane(home, fresh)
    if sent and started:
        return sent
    if sent:
        why += f" (the agent did not start: lane {sent} never registered)"
    text = (f"Merge queue: {lane.get('feature') or lane_id} ({lane_id}) {result}: " + message.splitlines()[0][:200]
            + why + f"\n- Send it back: tirith merge request --lane {lane_id}\n- Drop the lane: tirith lane drop {lane_id}")
    asks_mod.queue_add(home, project, lane_id, text)
    return sent or ""


def _tell_manager(home: Path, entry: dict, lane: dict, result: str, message: str, hint: str) -> None:
    """A manager queue item with the reason a merge-queue attempt did not land (never the owner)."""
    lane_id = str(entry["lane"])
    first = message.strip().splitlines()[0][:200] if message.strip() else "no reason given"
    asks_mod.queue_add(home, str(entry.get("project") or ""), lane_id,
                       f"Merge queue: {lane.get('feature') or lane_id} ({lane_id}) {result}: {first}\n- {hint}")


def _close_gone(home: Path, entry: dict, lane: dict | None, repo: Path | None) -> dict:
    """An entry whose lane, worktree or project is gone leaves the queue with its reason (flow + history), so
    the lanes behind it go on."""
    lane_id = entry["lane"]
    if lane is None:
        why = "its lane is gone"
    elif repo is None:
        why = "its project is gone"
    else:
        branch = branch_of(lane, Path(str(lane.get("workspace") or "")))
        merged = plangate_mod.lane_work(home, repo, lane)[0] == plangate_mod.WORK_MERGED
        why = ("already merged into main" if merged else "its worktree is gone")
        if merged:
            fresh = dict(lane)
            fresh["review"] = {"status": review_mod.MERGED, "at": now_iso(), "by": "mergeq",
                               "message": "already merged into main"}
            write_lane(home, fresh)
    message = f"left the queue: {why}"
    flows_mod.append_flow(home, "tirith", "manager", "alert", f"{MARK} {lane_id}: {message}.", session=None,
                          project=entry.get("project") or None)
    return _finish(home, entry, CLOSED, message)


def _close_merged(home: Path, repo: Path, lane: dict, entry: dict, extra: dict, message: str, files: int | None = None) -> dict:
    """A merged lane: tick its plan item, remove its worktree and branch, record the review and the history row."""
    lane_id = entry["lane"]
    try:
        plangate_mod.review_passed(home, repo, read_json(lane_path(home, lane_id), lane))
    except plangate_mod.PlanGateError as error:
        extra["plan_error"] = str(error)
    workspace = Path(str(lane["workspace"]))
    reaper_mod.reap_lane_leftovers(workspace)
    _remove_worktree(repo, "--force", str(workspace))
    _git(repo, "branch", "-D", str(extra["branch"]))
    if files is None:
        files = len(_git(repo, "diff", "--name-only", f"{extra['commit']}^1", extra["commit"]).stdout.split())
    fresh = read_json(lane_path(home, lane_id), lane)
    fresh["branch"] = extra["branch"]  # the branch is gone: :func:`branch_of` can no longer ask the worktree
    fresh["review"] = {"status": review_mod.MERGED, "at": now_iso(), "by": "mergeq", "pid": os.getpid(),
                       "message": message, "commit": extra["commit"], "branch": extra["main"],
                       "passed": extra.get("passed"), "files": files}
    write_lane(home, fresh)
    status = PUSH_FAILED if extra.get("push_error") else MERGED
    more = {k: extra[k] for k in ("skipped", "train", "flaky") if extra.get(k)}
    return _finish(home, entry, status, extra.get("push_error") or message, commit=extra["commit"], **more)


def _load_entry(home: Path, config: Config, entry: dict):
    """``(lane, repo, closed row)``: an entry whose lane, worktree or project is gone is closed here."""
    lane = read_json(lane_path(home, entry["lane"]), None)
    repo = plangate_mod.repo_of(home, entry["project"], config) if entry["project"] else None
    if not isinstance(lane, dict) or repo is None or not Path(str(lane.get("workspace") or "")).is_dir():
        return None, None, _close_gone(home, entry, lane if isinstance(lane, dict) else None, repo)
    return lane, repo, None


def _process_one(home: Path, config: Config, entry: dict, lane: dict, repo: Path, relaunch: Callable | None,
                 push: bool) -> dict:
    result, message, extra = "retry", "", {}
    for _ in range(MAX_TRIES):
        result, message, extra = _try(home, repo, lane, entry)
        if result != "retry":
            break
    return _settle(home, config, entry, lane, repo, result, message, extra, relaunch, push)


def _settle(home: Path, config: Config, entry: dict, lane: dict, repo: Path, result: str, message: str, extra: dict,
            relaunch: Callable | None, push: bool, *, files: int | None = None) -> dict:
    """Act on the outcome for one lane: wait, send back, or close it as merged (pushing first when ``push``)."""
    if result == "retry":
        return _finish(home, entry, WAITING, message, keep=True, retried=True)
    if result == WAITING:  # the same reason twice in a row is not luck: the manager hears of it (once)
        count = int(entry.get("waits") or 0) + 1
        if count == WAIT_NOTICE:
            _tell_manager(home, entry, lane, WAITING, message, "It waits in the queue and is tried again; fix the reason "
                          "above (or `tirith lane drop <lane>`).")
        return _finish(home, entry, WAITING, message, keep=True, waits=count)
    if result == OUT_OF_TIME:  # the suite ran out of time, it did not fail: never sent back
        count = int(entry.get("timeouts") or 0) + 1
        if count == 2:
            asks_mod.queue_add(home, entry["project"], entry["lane"], f"Merge queue: {lane.get('feature') or entry['lane']} "
                               f"({entry['lane']}) {message.splitlines()[0][:200]}\n- It waits in the queue. Raise the limit "
                               "with: tirith timeout set --class stage:test --reason \"…\"")
        return _finish(home, entry, WAITING, message, keep=True, timeouts=count)
    if result == plangate_mod.WORK_NONE:  # committed what was left and main already has it all: no merge, no tick here
        _remove_worktree(repo, "--force", str(lane["workspace"]))
        _git(repo, "branch", "-D", str(extra["branch"]))
        fresh = read_json(lane_path(home, entry["lane"]), lane)
        fresh["review"] = {"status": plangate_mod.WORK_NONE, "at": now_iso(), "by": "mergeq", "message": message}
        write_lane(home, fresh)
        return _finish(home, entry, CLOSED, message, wasted=extra.get("wasted"))
    if result in (CONFLICT, RED):
        sent = send_back(home, config, lane, result, message, str(extra.get("main") or "main"), relaunch)
        return _finish(home, entry, result, message, sent_to=sent or None)
    if push:  # merged: main moved already; the push can fail on its own (network, rejected)
        failed = extra.get("push_error") if "push_error" in extra else _push(repo, str(extra["main"]))
        if failed:
            extra["push_error"] = failed
    row = _close_merged(home, repo, lane, entry, extra, message, files)
    if row["result"] == PUSH_FAILED:  # main has the lane but origin does not: nobody else would see it
        _tell_manager(home, entry, lane, PUSH_FAILED, str(row["message"]),
                      "The lane is merged into main; push it with `git push origin <main branch>` once the cause is fixed.")
    return row


def _process_train(home: Path, config: Config, members: list[tuple[dict, dict]], repo: Path,
                   relaunch: Callable | None, push: bool) -> list[dict]:
    """Test ``members`` (``(entry, lane)``, oldest first) together; a red train is redone one lane at a time."""
    result, message, info = "retry", "", {}
    for _ in range(MAX_TRIES):
        result, message, info = _try_train(home, repo, members)
        if result != "retry":
            break
    rows = []
    for entry, lane, left_result, left_message, left_extra in info["left"]:  # real conflicts leave; the rest go on
        rows.append(_settle(home, config, entry, lane, repo, left_result, left_message, left_extra, relaunch, push))
    joined = info["joined"]
    if result == RED and len(joined) > 1:  # who is the culprit? the same lanes, one at a time
        return rows + [_process_one(home, config, entry, lane, repo, relaunch, push) for entry, lane in joined]
    if result != MERGED:
        return rows + [_settle(home, config, entry, lane, repo, result, message, info, relaunch, push)
                       for entry, lane in joined]
    if push:  # one push for the whole train
        failed = _push(repo, str(info["main"]))
        info["push_error"] = failed or None
    ids = [str(lane["id"]) for _, lane in joined]
    for entry, lane in joined:
        branch, files = info["branches"][str(lane["id"])]
        extra = {"commit": info["commit"], "main": info["main"], "passed": info["passed"], "branch": branch,
                 "skipped": info["skipped"], "flaky": info["flaky"], "train": ids, "push_error": info["push_error"] if push else None}
        rows.append(_settle(home, config, entry, lane, repo, MERGED, message, extra, relaunch, False, files=files))
    return rows


def process(home: Path, config: Config, *, relaunch: Callable | None = None, push: bool = True,
            skip: frozenset[str] = frozenset()) -> list[dict]:
    """Merge the oldest waiting lane together with up to ``[mergeq] train - 1`` more of its project; the result rows."""
    entries = _claim_many(home, skip, config.mergeq_train)
    members, rows, repo = [], [], None
    for entry in entries:
        lane, found, closed = _load_entry(home, config, entry)
        if closed:
            rows.append(closed)
        else:
            members.append((entry, lane))
            repo = found
    if len(members) == 1:
        rows.append(_process_one(home, config, *members[0], repo, relaunch, push))
    elif members:
        rows += _process_train(home, config, members, repo, relaunch, push)
    if repo is not None and any(r["result"] == MERGED for r in rows):
        stable_mod.promote_after_merge(home, repo)  # main moved: the stable copy follows once the full test passes
    place = {e["lane"]: n for n, e in enumerate(entries)}
    return sorted(rows, key=lambda r: place.get(r["lane"], len(place)))  # queue order, whoever left the train


def process_next(home: Path, config: Config, *, relaunch: Callable | None = None, push: bool = True,
                 skip: frozenset[str] = frozenset()) -> dict | None:
    """:func:`process`, returning the row of the oldest lane (or None when the queue is empty or busy)."""
    rows = process(home, config, relaunch=relaunch, push=push, skip=skip)
    return rows[0] if rows else None


def drain(home: Path, config: Config, *, watch: selfreload_mod.CodeWatch | None = None, **kwargs) -> list[dict]:
    """Merge every waiting lane, one pass (a train) after the other; stops after a pass once the code on disk
    changed (the dispatcher starts a fresh runner), so a long queue never merges with code that main has already replaced."""
    out, tried = [], set()
    watch = watch or selfreload_mod.CodeWatch(stable_s=0.0)
    while True:
        if out and watch.changed():
            return out
        rows = process(home, config, skip=frozenset(tried), **kwargs)
        if not rows:
            return out
        out += rows
        if any(r["result"] == WAITING and not r.get("retried") for r in rows):
            return out
        tried.update(r["lane"] for r in rows if r["result"] == WAITING and r.get("retried"))  # main moved: next entry


# -- the dispatcher ----------------------------------------------------------------------------


def run(home: Path, config: Config, lanes: list[dict], *, spawn: Callable | None = None,
        dry_run: bool = False) -> bool:
    """One dispatcher pass: enqueue complete lanes, start a detached ``tirith merge run`` when work waits
    (``spawn`` defaults to ``launch.spawn_detached``; only ``dry_run`` skips starting it)."""
    from tirith import autoreview as autoreview_mod, launch as launch_mod
    spawn = spawn or launch_mod.spawn_detached
    from tirith.store import read_lane
    changed = False
    for seen in lanes:
        if not _may_enqueue(seen):
            continue
        lane = read_lane(home, str(seen.get("id") or ""))
        if isinstance(lane, dict) and ready_to_enqueue(home, lane, autoreview_mod.repo_of(config, lane)):
            enqueue(home, str(lane["id"]))
            changed = True
    data = load(home)
    def track_busy(canary: bool) -> bool:
        return any(e["status"] == RUNNING and _alive(e.get("pid")) and _lane_of(e) == canary for e in data["queue"])

    work = any(e["status"] in (QUEUED, RUNNING) and not _lane_of(e) for e in data["queue"]) and not track_busy(False)
    work = work or (any(e["status"] in (QUEUED, RUNNING) and _lane_of(e) for e in data["queue"]) and not track_busy(True))
    if work:
        stamp = datetime.now(timezone.utc).timestamp()
        if stamp - float(data.get("spawned") or 0) > 30:  # a runner that just started has not claimed yet
            if dry_run:  # tests, dry runs: only enqueue
                return changed
            with _locked(home):
                data = load(home)
                data["spawned"] = stamp
                write_json_atomic(_path(home), data)
            spawn([*launch_mod.tirith_argv(home), "merge", "run"], home, home / "logs" / "mergeq.log")
            changed = True
    return changed
