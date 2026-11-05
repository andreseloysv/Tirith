"""Tirith pushes ``main`` to the project's ``origin`` soon after every change of the plan.

Tirith commits each plan change on the main checkout (``tirith.plangate``), and the merge queue pushes after a merge,
but nothing else sent those commits to GitHub: a plan edit made between merges stayed local. The dispatcher pass
calls :func:`run`; a project whose local ``main`` is ahead of ``origin/main`` is pushed, a plain fast-forward (never
a force). The pass runs every few seconds, so ``main`` is on the origin at most :data:`MAX_DELAY_S` (5 minutes)
after the change; the push runs in a thread outside the pass (a slow hook or network never stalls the dispatcher); a failed push
is tried again after :data:`RETRY_S`, then 2x, 4x... up to :data:`BACKOFF_MAX_S`, and its text stays in the dispatcher state
(``pushed``) for the manager and the self-check. See ``docs/CONTRACT-mergeq.md`` ("Push of main").
"""

from __future__ import annotations

import threading
from datetime import datetime
from pathlib import Path

from tirith import plangate as plangate_mod
from tirith import projects as projects_mod
from tirith import timeouts as timeouts_mod
from tirith.home import Config, now_iso, parse_iso

MAX_DELAY_S = 300.0  # the promise: main is on the origin this long after a change of the plan, at most
RETRY_S = 60.0       # the first failed push waits this long (well inside the promise); each next failure doubles it
BACKOFF_MAX_S = 1800.0  # ...up to 30 minutes
BRANCH = "main"
ERROR_MAX = 400


def project_ids(home: Path, config: Config) -> list[str]:
    ids = [p.id for p in config.projects]
    ids += [str(p["id"]) for p in projects_mod.load_auto(home)["projects"] if str(p["id"]) not in ids]
    return ids


def _git(repo: Path, *args: str):
    return plangate_mod._git(repo, *args)


def pending(repo: Path) -> bool:
    """True when the repo has an ``origin`` and local ``main`` has commits the origin's ``main`` lacks."""
    if _git(repo, "remote", "get-url", "origin").returncode != 0:
        return False
    if _git(repo, "rev-parse", "--verify", "-q", f"refs/heads/{BRANCH}").returncode != 0:
        return False
    if _git(repo, "rev-parse", "--verify", "-q", f"refs/remotes/origin/{BRANCH}").returncode != 0:
        return True  # never pushed
    ahead = _git(repo, "rev-list", "--count", f"refs/remotes/origin/{BRANCH}..refs/heads/{BRANCH}")
    return ahead.returncode == 0 and ahead.stdout.strip() not in ("", "0")


def push(repo: Path) -> str | None:
    """Fast-forward the origin's ``main`` to the local one; the failure text, else None."""
    # a push may run hooks and write to the origin: the commit's limit, like the merge queue's push
    done = timeouts_mod.run(["git", "-C", str(repo), "push", "-q", "origin", f"refs/heads/{BRANCH}:refs/heads/{BRANCH}"],
                            "git-commit", text=True)
    if done.returncode == 0:
        return None
    return (done.stderr or done.stdout or timeouts_mod.why(done)).strip()[-ERROR_MAX:] or "push failed"


def retry_after(fails: int) -> float:
    """Seconds a push that failed ``fails`` times in a row waits: 1, 2, 4... minutes, at most :data:`BACKOFF_MAX_S`."""
    return min(RETRY_S * 2 ** max(fails - 1, 0), BACKOFF_MAX_S)


_lock = threading.Lock()
_running: dict[str, threading.Thread] = {}  # project -> its push in flight
_done: dict[str, dict] = {}                 # project -> result not yet merged into the dispatcher state


def _work(project: str, repo: Path, at: datetime) -> None:
    try:
        error = push(repo)
        head = _git(repo, "rev-parse", f"refs/heads/{BRANCH}").stdout.strip()
    except Exception as exc:  # a push must never take the dispatcher down
        error, head = f"{type(exc).__name__}: {exc}"[-ERROR_MAX:], ""
    with _lock:
        _done[project] = {"at": now_iso(at), "ok": error is None, "commit": head, **({"error": error} if error else {})}
        _running.pop(project, None)


def wait(timeout: float | None = None) -> None:
    """Join the pushes in flight (tests and shutdown; the dispatcher pass never calls it)."""
    with _lock:
        threads = list(_running.values())
    for thread in threads:
        thread.join(timeout)


def run(home: Path, config: Config, now: datetime, state: dict) -> bool:
    """One pass, never waiting for a push: merge the finished pushes into ``state['pushed']``, start a push thread for
    every project whose ``main`` is ahead of its origin. True when ``state['pushed']`` changed."""
    book = state.setdefault("pushed", {})
    changed = False
    with _lock:
        finished, busy = dict(_done), set(_running)
        _done.clear()
    for project, result in finished.items():
        prior = book.get(project) or {}
        fails = 0 if result["ok"] else int(prior.get("fails", 0)) + 1
        book[project] = {**result, "fails": fails, **({"retry_s": retry_after(fails)} if fails else {})}
        changed = True
    for project in project_ids(home, config):
        if project in busy:
            continue
        repo = plangate_mod.repo_of(home, project, config)
        if repo is None or not repo.is_dir():
            continue
        last = book.get(project) or {}
        if not last.get("ok", True):  # a failed push waits longer each time, not every pass
            at = parse_iso(last.get("at")) if last.get("at") else None
            if at is not None and (now - at).total_seconds() < retry_after(int(last.get("fails", 1))):
                continue
        if not pending(repo):
            if last.get("ok") is False:
                book[project] = {"at": now_iso(now), "ok": True, "commit": last.get("commit", ""), "fails": 0}
                changed = True
            continue
        thread = threading.Thread(target=_work, args=(project, repo, now), name=f"pushplan-{project}", daemon=True)
        with _lock:
            _running[project] = thread
        thread.start()
    return changed
