"""The single merge queue: agents never touch main, Tirith merges one finished lane at a time.

A lane whose contract is complete (ended ``done`` with the outcome ``done`` in its plan report) is enqueued
(:func:`enqueue`, by the dispatcher or ``tirith merge request``). :func:`process_next` takes the oldest
waiting lane and, in a temporary worktree cut from main: merges the lane branch, runs the project's full
build and test stages, then (under the plan writer lock, with stale ``index.lock`` recovery) fast-forwards
main in the main checkout to exactly that tested commit, pushes, ticks the plan item and cleans up.
A conflict or a red suite goes back to the same agent (:func:`send_back`: a relaunch whose brief continues
the lane's branch), never to the owner. The queue lives in ``<home>/mergeq.json``.
"""

from __future__ import annotations

import os
import shutil
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
from tirith import plangate as plangate_mod
from tirith import review as review_mod
from tirith import timeouts as timeouts_mod
from tirith.home import Config, now_iso
from tirith.launch import worktree_path
from tirith.store import lane_path, read_json, write_json_atomic, write_lane

FILE = "mergeq.json"
QUEUED, RUNNING = "queued", "running"
MERGED, CONFLICT, RED, PUSH_FAILED, WAITING, SENT_BACK, CLOSED = (
    "merged", "conflict", "red-suite", "push-failed", "waiting", "sent-back", "closed")
SENT_BACK_STATUS = "sent-back"  # the lane's ``review.status`` once the queue returned it to its agent
MARK = "[merge queue]"
MAX_SENT_BACK = 3  # after this many returns the manager (never the owner) decides
MAX_TRIES = 3  # main moved under the merge: start again from the new main
TIMED_OUT = "timed out"  # a git command stopped at its deadline: retried by the queue, never sent back
HISTORY = 20
OUTPUT_MAX = 1500


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


def enqueue(home: Path, lane_id: str) -> int:
    """Add the lane at the end of the queue (once); returns its 1-based position."""
    lane = read_json(lane_path(home, lane_id), None)
    if not isinstance(lane, dict):
        raise review_mod.ReviewError(f"unknown lane: {lane_id}")
    with _locked(home):
        data = load(home)
        for n, entry in enumerate(data["queue"], 1):
            if entry["lane"] == lane_id:
                return n
        data["queue"].append({"lane": lane_id, "project": str(lane.get("project") or ""), "at": now_iso(),
                              "status": QUEUED, "feature": str(lane.get("feature") or lane_id)})
        write_json_atomic(_path(home), data)
        return len(data["queue"])


def ready_to_enqueue(home: Path, lane: dict, repo: Path | None) -> bool:
    """A finished launched lane whose plan outcome is ``done`` and that is not merged or queued yet."""
    if not lane.get("ended_at") or lane.get("result") != "done" or repo is None:
        return False
    if (lane.get("outcome") or {}).get("status") != "done" or not review_mod.launched(lane, repo):
        return False
    status = (lane.get("review") or {}).get("status")
    return status in (None, review_mod.PENDING, review_mod.READY) and Path(str(lane["workspace"])).is_dir() \
        and not queued(home, str(lane["id"]))


def view(home: Path, project: str | None = None) -> dict:
    """The queue for the plan view: ``{"items": [{lane, position, status, ...}], "last": {...}|None}``."""
    data = load(home)
    items = [{"lane": e["lane"], "position": n, "status": e["status"], "since": e["at"],
              "feature": e.get("feature"), **({"message": e["message"]} if e.get("message") else {})}
             for n, e in enumerate(data["queue"], 1) if project is None or e.get("project") == project]
    rows = [h for h in data["history"] if project is None or h.get("project") == project]
    last = data["last"].get(project) if project else (rows[-1] if rows else None)
    return {"items": items, "last": last, "recent": rows[-5:]}


# -- taking work -------------------------------------------------------------------------------


def _claim(home: Path, skip: frozenset[str] = frozenset()) -> dict | None:
    with _locked(home):
        data = load(home)
        for entry in data["queue"]:
            if entry["status"] == RUNNING and _alive(entry.get("pid")):
                return None  # one at a time
            if entry["status"] == RUNNING:
                entry["status"] = QUEUED  # the merger died: take it again
        waiting = [e for e in data["queue"] if e["status"] == QUEUED and e["lane"] not in skip]
        if not waiting:
            write_json_atomic(_path(home), data)
            return None
        entry = waiting[0]
        entry.update(status=RUNNING, pid=os.getpid(), started=now_iso())
        entry.pop("message", None)
        write_json_atomic(_path(home), data)
        return dict(entry)


def _finish(home: Path, entry: dict, result: str, message: str = "", *, keep: bool = False, **extra) -> dict:
    """Leave the queue (or wait in it again with ``keep``) and remember the result."""
    with _locked(home):
        data = load(home)
        row = {"lane": entry["lane"], "project": entry["project"], "result": result, "at": now_iso(),
               "message": message[:OUTPUT_MAX], **extra}
        if keep:
            for e in data["queue"]:
                if e["lane"] == entry["lane"]:
                    e.update(status=QUEUED, message=message[:OUTPUT_MAX])
                    e.pop("pid", None)
        else:
            data["queue"] = [e for e in data["queue"] if e["lane"] != entry["lane"]]
            data["history"] = (data["history"] + [row])[-HISTORY:]
        data["last"][entry["project"]] = row
        write_json_atomic(_path(home), data)
    return row


# -- the merge ---------------------------------------------------------------------------------


def _git(folder: Path, *args: str):
    return timeouts_mod.run(["git", "-C", str(folder), *args], "git", text=True)

def _remove_worktree(repo: Path, *args: str):
    """``git worktree remove`` deletes every file of the lane, so it has its own (minutes-long) limit, not ``git``'s."""
    return timeouts_mod.run(["git", "-C", str(repo), "worktree", "remove", *args], "git-worktree-remove", text=True)



def _tail(text: str) -> str:
    text = (text or "").strip()
    return text if len(text) <= OUTPUT_MAX else "…" + text[-OUTPUT_MAX:]


def stages_of(folder: Path) -> list[tuple[str, manifest_mod.Stage]]:
    """``(pipeline id, stage)`` of every build and test stage with a command of the gating pipelines, in manifest order."""
    return [(pipe.id, stage) for pipe in manifest_mod.load(folder).pipelines if pipe.gating for stage in pipe.stages
            if stage.run and manifest_mod.stage_kind(stage) in ("build", "test")]


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


def _commit_workspace(workspace: Path, lane_id: str, feature: str) -> str | None:
    """Commit what the agent left in its worktree; the error text when that fails."""
    _git(workspace, "add", "-A")
    if _git(workspace, "diff", "--cached", "--quiet").returncode != 0:
        done = _git(workspace, "commit", "-q", "-m", f"Lane {lane_id}: {feature}")
        if done.returncode != 0:
            return f"{TIMED_OUT}: {done.error}" if done.timed_out else _tail(done.stderr or done.stdout)
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
    base = _git(repo, "rev-parse", "HEAD").stdout.strip()
    main_branch = _git(repo, "rev-parse", "--abbrev-ref", "HEAD").stdout.strip()
    tmp = worktree_path(repo, f"_mergeq-{lane_id}")
    if tmp.exists():
        _remove_worktree(repo, "--force", str(tmp))
        shutil.rmtree(tmp, ignore_errors=True)
    _git(repo, "worktree", "prune")
    made = _git(repo, "worktree", "add", "-q", "--detach", str(tmp), base)
    if made.returncode != 0:
        return WAITING, f"could not make the temporary worktree: {_tail(made.stderr or made.stdout)}", {}
    try:
        merged = _git(tmp, "merge", "--no-ff", "-q", "-m", f"Merge {branch}: {feature}", branch)
        if merged.timed_out:  # git was cut off by its time limit: not a conflict, try again, never the lane
            _git(tmp, "merge", "--abort")
            return "retry", f"merging {branch} into {main_branch} {TIMED_OUT}; trying again", {}
        if merged.returncode != 0:
            files = _git(tmp, "diff", "--name-only", "--diff-filter=U").stdout.split()
            _git(tmp, "merge", "--abort")
            what = ("conflicts in " + ", ".join(files)) if files else _tail(merged.stderr or merged.stdout)
            return CONFLICT, f"merging {branch} into {main_branch} failed: {what}", {"main": main_branch, "files": files}
        stages = stages_of(tmp)
        if not stages:
            return WAITING, "the manifest has no build or test stage with a run command", {}
        passed = None
        for pipeline, stage in stages:
            ok, output = review_mod.run_tests(home, project, pipeline, stage.name, tmp, stage.run, lane_id)
            if not ok:
                return RED, f"stage {pipeline}/{stage.name} failed on {branch} merged with {main_branch}:\n{_tail(output)}", \
                    {"main": main_branch}
            passed = review_mod.tests_passed(output) or passed
        tested = _git(tmp, "rev-parse", "HEAD").stdout.strip()
        with plangate_mod.writer_lock(home, project):
            if _git(repo, "status", "--porcelain", "--untracked-files=no").stdout.strip():
                return WAITING, f"{repo} has uncommitted changes; the queue waits for it to be clean", {}
            head = _git(repo, "rev-parse", "HEAD").stdout.strip()
            if head != base:
                tested = _with_plan_moves(repo, tmp, base, head, tested, branch, feature)
                if tested is None:
                    return "retry", "main moved while the suite ran", {}
            plangate_mod.recover_git_lock(repo)
            moved = _git(repo, "merge", "--ff-only", "-q", tested)
            if moved.returncode != 0 and "index.lock" in moved.stderr and plangate_mod.recover_git_lock(repo):
                moved = _git(repo, "merge", "--ff-only", "-q", tested)
            if moved.returncode != 0:
                return WAITING, f"fast-forwarding {main_branch} failed: {_tail(moved.stderr or moved.stdout)}", {}
        return MERGED, "tests passed; merged", {"commit": tested, "main": main_branch, "passed": passed, "branch": branch}
    finally:
        _remove_worktree(repo, "--force", str(tmp))
        shutil.rmtree(tmp, ignore_errors=True)
        _git(repo, "worktree", "prune")


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
    if alias in config.agents and _sent_back_count(lane) < MAX_SENT_BACK:
        try:
            sent = relaunch(home, lane, alias, note)
        except (Exception, SystemExit) as error:
            why = f" (sending it back failed: {error})"
    elif alias in config.agents:
        why = f" (sent back {MAX_SENT_BACK} times already)"
    fresh = read_json(lane_path(home, lane_id), lane)
    fresh["review"] = {"status": SENT_BACK_STATUS, "at": now_iso(), "by": "mergeq", "message": message[:OUTPUT_MAX]}
    write_lane(home, fresh)
    if sent:
        return sent
    text = (f"Merge queue: {lane.get('feature') or lane_id} ({lane_id}) {result}: " + message.splitlines()[0][:200]
            + why + f"\n- Send it back: tirith merge request --lane {lane_id}\n- Drop the lane: tirith lane drop {lane_id}")
    asks_mod.queue_add(home, project, lane_id, text)
    return ""


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
        merged = _git(repo, "merge-base", "--is-ancestor", branch, "HEAD").returncode == 0
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


def process_next(home: Path, config: Config, *, relaunch: Callable | None = None, push: bool = True,
                 skip: frozenset[str] = frozenset()) -> dict | None:
    """Merge the oldest waiting lane; the result row, or None when the queue is empty or busy."""
    entry = _claim(home, skip)
    if entry is None:
        return None
    lane_id = entry["lane"]
    lane = read_json(lane_path(home, lane_id), None)
    repo = plangate_mod.repo_of(home, entry["project"], config) if entry["project"] else None
    if not isinstance(lane, dict) or repo is None or not Path(str(lane.get("workspace") or "")).is_dir():
        return _close_gone(home, entry, lane if isinstance(lane, dict) else None, repo)
    result, message, extra = "retry", "", {}
    for _ in range(MAX_TRIES):
        result, message, extra = _try(home, repo, lane, entry)
        if result != "retry":
            break
    if result == "retry":
        return _finish(home, entry, WAITING, message, keep=True, retried=True)
    if result == WAITING:
        return _finish(home, entry, WAITING, message, keep=True)
    if result in (CONFLICT, RED):
        sent = send_back(home, config, lane, result, message, str(extra.get("main") or "main"), relaunch)
        return _finish(home, entry, result, message, sent_to=sent or None)
    if push:  # merged: main moved already; the push can fail on its own (network, rejected)
        failed = _push(repo, str(extra["main"]))
        if failed:
            extra["push_error"] = failed
    try:
        plangate_mod.review_passed(home, repo, read_json(lane_path(home, lane_id), lane))
    except plangate_mod.PlanGateError as error:
        extra["plan_error"] = str(error)
    workspace = Path(str(lane["workspace"]))
    _remove_worktree(repo, "--force", str(workspace))
    _git(repo, "branch", "-D", str(extra["branch"]))
    fresh = read_json(lane_path(home, lane_id), lane)
    fresh["review"] = {"status": review_mod.MERGED, "at": now_iso(), "by": "mergeq", "pid": os.getpid(),
                       "message": message, "commit": extra["commit"], "branch": extra["main"],
                       "passed": extra.get("passed"),
                       "files": len(_git(repo, "diff", "--name-only", f"{extra['commit']}^1", extra["commit"]).stdout.split())}
    write_lane(home, fresh)
    status = PUSH_FAILED if extra.get("push_error") else MERGED
    row = _finish(home, entry, status, extra.get("push_error") or message, commit=extra["commit"])
    return row


def drain(home: Path, config: Config, **kwargs) -> list[dict]:
    """Merge every waiting lane, one after the other."""
    out, tried = [], set()
    while True:
        row = process_next(home, config, skip=frozenset(tried), **kwargs)
        if row is None:
            return out
        if row["result"] == WAITING and row.get("retried"):  # main moved in code: back in the queue, next entry
            tried.add(row["lane"])
            out.append(row)
            continue
        if row["result"] == WAITING:
            return out + [row]
        out.append(row)


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
        lane = read_lane(home, str(seen.get("id") or ""))
        if isinstance(lane, dict) and ready_to_enqueue(home, lane, autoreview_mod.repo_of(config, lane)):
            enqueue(home, str(lane["id"]))
            changed = True
    data = load(home)
    busy = any(e["status"] == RUNNING and _alive(e.get("pid")) for e in data["queue"])
    if not busy and any(e["status"] in (QUEUED, RUNNING) for e in data["queue"]):
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
