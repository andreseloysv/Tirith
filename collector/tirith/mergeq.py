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
from tirith import selfreload as selfreload_mod
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
DOC_ONLY = ("docs/", "site/", "ios/")  # a lane that changes only these (or *.md) has nothing for the test stages
RUST_STAGE = "rust-test"
FLAKY_FILE = "flaky.json"  # {project: {test id: {"count": n, "last": iso}}}: tests that failed in the suite and passed alone
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


def _may_enqueue(seen: dict) -> bool:
    """False for a lane row that cannot be enqueued, so its file is not read again every cycle."""
    if "ended_at" in seen and not seen["ended_at"]:
        return False
    if "result" in seen and seen["result"] != "done":
        return False
    status = (seen.get("review") or {}).get("status")
    return status in (None, review_mod.PENDING, review_mod.READY)


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


def _claim_many(home: Path, skip: frozenset[str] = frozenset(), limit: int = 1) -> list[dict]:
    """Take the oldest waiting entry and up to ``limit - 1`` more of the same project, oldest first."""
    with _locked(home):
        data = load(home)
        for entry in data["queue"]:
            if entry["status"] == RUNNING and _alive(entry.get("pid")):
                return []  # one at a time
            if entry["status"] == RUNNING:
                entry["status"] = QUEUED  # the merger died: take it again
        waiting = [e for e in data["queue"] if e["status"] == QUEUED and e["lane"] not in skip]
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
                                     f"{stage.run} -n 0 {shlex.quote(test)}", lane_id)
        if not ok:
            return None
    note_flaky(home, project, tests)
    return tests


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


def _land(home: Path, repo: Path, tmp: Path, project: str, base: str, tested: str, main_branch: str,
          subject: str, via: str) -> tuple[str, tuple[str, str] | None]:
    """Fast-forward main to the tested commit under the plan writer lock: ``(commit, None)``, else ``("", (result, message))``."""
    with plangate_mod.writer_lock(home, project):
        if _git(repo, "status", "--porcelain", "--untracked-files=no").stdout.strip():
            return "", (WAITING, f"{repo} has uncommitted changes; the queue waits for it to be clean")
        head = _git(repo, "rev-parse", "HEAD").stdout.strip()
        if head != base:
            tested = _with_plan_moves(repo, tmp, base, head, tested, via, subject)
            if tested is None:
                return "", ("retry", "main moved while the suite ran")
        plangate_mod.recover_git_lock(repo)
        moved = _git(repo, "merge", "--ff-only", "-q", tested)
        if moved.returncode != 0 and "index.lock" in moved.stderr and plangate_mod.recover_git_lock(repo):
            moved = _git(repo, "merge", "--ff-only", "-q", tested)
        if moved.returncode != 0:
            return "", (WAITING, f"fast-forwarding {main_branch} failed: {_tail(moved.stderr or moved.stdout)}")
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


def _run_stages(home: Path, project: str, stages: list, tmp: Path, lane_id: str
                ) -> tuple[str | None, str, str | None, list[str]]:
    """Run the stages: ``(failed "pipeline/stage" | None, output of the failure, passed summary, flaky test ids)``.

    A red stage first repeats its failed tests alone (:func:`rerun_failed`): when they all pass the stage goes on."""
    passed, flaky = None, []
    for pipeline, stage in stages:
        ok, output = review_mod.run_tests(home, project, pipeline, stage.name, tmp, stage.run, lane_id)
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
            _git(tmp, "merge", "--abort")
            what = ("conflicts in " + ", ".join(files)) if files else _tail(merged.stderr or merged.stdout)
            return CONFLICT, f"merging {branch} into {main_branch} failed: {what}", {"main": main_branch, "files": files}
        stages = stages_of(tmp)
        if not stages:
            return WAITING, "the manifest has no build or test stage with a run command", {}
        stages, skipped = select_stages(stages, [needs_of(changed_paths(repo, base, branch))])
        broke, output, passed, flaky = _run_stages(home, project, stages, tmp, lane_id)
        if broke:
            return RED, f"stage {broke} failed on {branch} merged with {main_branch}:\n{_tail(output)}", {"main": main_branch}
        tested = _git(tmp, "rev-parse", "HEAD").stdout.strip()
        tested, refused = _land(home, repo, tmp, project, base, tested, main_branch, feature, branch)
        if refused:
            return refused[0], refused[1], {}
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
            if merged.returncode != 0:
                files = _git(tmp, "diff", "--name-only", "--diff-filter=U").stdout.split()
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
        if broke:
            return RED, f"stage {broke} failed on the train ({ids}) merged with {main_branch}:\n{_tail(output)}", info
        tested = _git(tmp, "rev-parse", "HEAD").stdout.strip()
        tested, refused = _land(home, repo, tmp, str(joined[0][1].get("project") or ""), base, tested, main_branch,
                                f"train of {len(joined)}: {ids}", "train")
        if refused:
            return refused[0], refused[1], info
        info.update(commit=tested, passed=passed, skipped=skipped, flaky=flaky)
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


def _close_merged(home: Path, repo: Path, lane: dict, entry: dict, extra: dict, message: str, files: int | None = None) -> dict:
    """A merged lane: tick its plan item, remove its worktree and branch, record the review and the history row."""
    lane_id = entry["lane"]
    try:
        plangate_mod.review_passed(home, repo, read_json(lane_path(home, lane_id), lane))
    except plangate_mod.PlanGateError as error:
        extra["plan_error"] = str(error)
    workspace = Path(str(lane["workspace"]))
    _remove_worktree(repo, "--force", str(workspace))
    _git(repo, "branch", "-D", str(extra["branch"]))
    if files is None:
        files = len(_git(repo, "diff", "--name-only", f"{extra['commit']}^1", extra["commit"]).stdout.split())
    fresh = read_json(lane_path(home, lane_id), lane)
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
    if result == WAITING:
        return _finish(home, entry, WAITING, message, keep=True)
    if result in (CONFLICT, RED):
        sent = send_back(home, config, lane, result, message, str(extra.get("main") or "main"), relaunch)
        return _finish(home, entry, result, message, sent_to=sent or None)
    if push:  # merged: main moved already; the push can fail on its own (network, rejected)
        failed = extra.get("push_error") if "push_error" in extra else _push(repo, str(extra["main"]))
        if failed:
            extra["push_error"] = failed
    return _close_merged(home, repo, lane, entry, extra, message, files)


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
