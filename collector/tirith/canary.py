"""An end-to-end test lane every hour: the whole way from the plan to the push, in a throwaway project.

Unit tests of each piece passed while nothing merged for hours; only a run through the real wiring catches
that. ``<home>/canary/`` holds a small git repo (``repo``, a manifest with trivial build and test stages and a
``docs/TODO.md``) and a local bare repo as its ``origin`` (``origin.git``); no network, no server. It is
registered as the project ``canary`` so the real dispatcher handles it, and left out of the owner's lists.

:func:`run` (``tirith canary run``, started detached by the dispatcher through :func:`tick`) goes the real
way: ``tirith plan add``, ``tirith lane run`` with a shell command as the agent (edits a file, reports ``done``,
commits; no model, no cost), then waits for the merge queue **the collector's own dispatcher** starts to merge
into the canary's main, push to ``origin`` and tick the plan item. Each step is written to ``canary/runs.json``
as it is reached, so a run that stops says where. :func:`public` is what the selfcheck "canary" check reads.
See ``docs/CONTRACT-canary.md``.
"""

from __future__ import annotations

import hashlib
import os
import shutil
import subprocess
import time
from datetime import datetime
from pathlib import Path
from typing import Callable

from tirith import launch as launch_mod
from tirith import mergeq as mergeq_mod
from tirith import projects as projects_mod
from tirith import selfreload as selfreload_mod
from tirith.home import load_config, now_iso, parse_iso
from tirith.store import lane_path, read_json, write_json_atomic

PROJECT = "canary"
STEP = "1"
EVERY_S = 3600.0   # a run every hour
LIMIT_S = 900.0    # from the start of a run to the push and the tick
KEPT = 10          # results kept
POLL_S = 2.0
MANIFEST = ('deploy = "none"\n\n[[pipelines]]\nid = "ci"\n\n[[pipelines.stages]]\nname = "build"\nrun = "true"\n\n'
            '[[pipelines.stages]]\nname = "test"\nrun = "echo 1 passed"\n')
PLAN = "# Canary plan\n\n## 1. Canary\n\n"
AGENT = ('echo "$TIRITH_LANE" > canary.txt && eval "$TIRITH_BIN plan report --status done" '
         '&& git add -A && git commit -q -m "canary run"')
STEPS = {  # where a run can stop, in order, and what it means
    "spawn": "the canary run never started",
    "setup": "the canary project could not be made",
    "plan-add": "`tirith plan add` failed",
    "lane": "the lane did not end done",
    "queued": "the dispatcher did not put the finished lane in the merge queue",
    "merge-start": "the merge queue never started (nothing ran `tirith merge run`)",
    "merge": "the merge queue did not merge the lane",
    "push": "main was not pushed to the canary's origin",
    "tick": "the plan item was not ticked",
}


def folder(home: Path) -> Path:
    return Path(home) / "canary"


def repo_of(home: Path) -> Path:
    return folder(home) / "repo"


def origin_of(home: Path) -> Path:
    return folder(home) / "origin.git"


def _file(home: Path) -> Path:
    return folder(home) / "runs.json"


def _git(where: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", "-C", str(where), *args], capture_output=True, text=True)


def load(home: Path) -> dict:
    data = read_json(_file(home), None)
    data = data if isinstance(data, dict) else {}
    if not isinstance(data.get("runs"), list):
        data["runs"] = []
    return data


def _save(home: Path, data: dict) -> None:
    data["runs"] = data["runs"][-KEPT:]
    write_json_atomic(_file(home), data)


def code_id() -> str:
    """Which code the collector runs: a new value once it reloads with new code."""
    files = selfreload_mod.code_fingerprint(Path(__file__).resolve().parent)
    return hashlib.sha1(repr(files).encode()).hexdigest()[:12]


# -- the project -------------------------------------------------------------------------------


def ensure(home: Path) -> Path:
    """Make the canary repo, its bare origin and its project row on first use; the repo."""
    home, repo, origin = Path(home), repo_of(home), origin_of(home)
    if not (repo / ".git").exists():
        folder(home).mkdir(parents=True, exist_ok=True)
        subprocess.run(["git", "init", "-q", "-b", "main", str(repo)], check=True, capture_output=True)
        (repo / "docs").mkdir()
        (repo / "tirith.toml").write_text(MANIFEST, encoding="utf-8")
        (repo / "docs" / "TODO.md").write_text(PLAN, encoding="utf-8")
        for key, value in (("user.name", "Tirith canary"), ("user.email", "canary@tirith.invalid")):
            _git(repo, "config", key, value)
        _git(repo, "add", "-A")
        subprocess.run(["git", "-C", str(repo), "commit", "-q", "-m", "canary: start"], check=True, capture_output=True)
    if not origin.exists():
        subprocess.run(["git", "init", "-q", "--bare", str(origin)], check=True, capture_output=True)
    if _git(repo, "remote", "get-url", "origin").returncode != 0:
        _git(repo, "remote", "add", "origin", str(origin))
        _git(repo, "push", "-q", "origin", "main")
    proj = load_config(home).project(PROJECT)
    if proj is None or proj.repo is None or projects_mod.real(proj.repo) != projects_mod.real(repo):
        _register(home, repo)
    return repo


def _register(home: Path, repo: Path) -> None:
    if load_config(home).project(PROJECT) is None:
        projects_mod.add_project(home, repo, name=PROJECT, color="#8a8a8a")
    projects_mod.set_project_guard(home, PROJECT, False)


def hidden(project_id: str) -> bool:
    """The canary is a test project: shown under Health, never on the owner's list."""
    return project_id == PROJECT


# -- one run -----------------------------------------------------------------------------------


class _Run:
    def __init__(self, home: Path, stamp: str) -> None:
        self.home = home
        self.row = {"id": stamp, "started": now_iso(), "step": "setup", "state": "running", "lane": None,
                    "log": str(home / "logs" / f"canary-{stamp}.log"), "code": code_id()}
        self.log_path = Path(self.row["log"])
        self.log_path.parent.mkdir(parents=True, exist_ok=True)
        data = load(home)
        data["runs"] = [r for r in data["runs"] if not str(r.get("id")).startswith("pending-")] + [self.row]
        _save(home, data)

    def note(self, text: str) -> None:
        with self.log_path.open("a", encoding="utf-8") as handle:
            handle.write(f"{now_iso()} {text}\n")

    def reach(self, step: str, **extra) -> None:
        self.row.update(step=step, **extra)
        self.note(f"step {step}")
        self._store()

    def finish(self, ok: bool, step: str, reason: str = "") -> dict:
        self.row.update(state="ok" if ok else "failed", step=step, ended=now_iso(), reason=reason)
        self.note(("ok" if ok else f"FAILED at {step}: {reason}"))
        self._store()
        return self.row

    def _store(self) -> None:
        data = load(self.home)
        data["runs"] = [self.row if r.get("id") == self.row["id"] else r for r in data["runs"]]
        _save(self.home, data)


def _tirith(home: Path, *args: str, cwd: Path | None = None, env: dict | None = None) -> subprocess.CompletedProcess:
    return subprocess.run([*launch_mod.tirith_argv(home), "--home", str(home), *args], capture_output=True,
                          text=True, cwd=str(cwd) if cwd else None, env=env)


def _plan_item_done(repo: Path, text: str) -> bool:
    plan = (repo / "docs" / "TODO.md")
    try:
        return any(line.lstrip().startswith("- [x]") and text in line
                   for line in plan.read_text(encoding="utf-8").splitlines())
    except OSError:
        return False


def _cleanup(home: Path, repo: Path, lane_id: str | None) -> None:
    """Remove the run's worktrees and branches (the disk guard must not grow)."""
    lanes = repo.parent / "repo-lanes"
    if lane_id:
        _git(repo, "worktree", "remove", "--force", str(launch_mod.worktree_path(repo, lane_id)))
        _git(repo, "worktree", "remove", "--force", str(launch_mod.worktree_path(repo, f"_mergeq-{lane_id}")))
    if lanes.is_dir():
        for leftover in lanes.iterdir():
            _git(repo, "worktree", "remove", "--force", str(leftover))
            shutil.rmtree(leftover, ignore_errors=True)
    _git(repo, "worktree", "prune")
    for branch in _git(repo, "for-each-ref", "--format=%(refname:short)", "refs/heads/lane/").stdout.split():
        _git(repo, "branch", "-D", branch)


def run(home: Path, *, wait_s: float = LIMIT_S, pump: Callable[[], None] | None = None,
        sleep: Callable[[float], None] = time.sleep, stamp: str | None = None) -> dict:
    """One canary run; its result row. ``pump`` is called while waiting for the merge queue (tests stand in for
    the collector's dispatch passes there; in production the running collector does that by itself)."""
    home = Path(home)
    stamp = stamp or datetime.now().strftime("%Y%m%d-%H%M%S")
    work = _Run(home, stamp)
    lane_id = None
    try:
        try:
            repo = ensure(home)
        except (OSError, subprocess.SubprocessError, ValueError) as error:
            return work.finish(False, "setup", str(error))
        item = f"Canary run {stamp}"
        done = _tirith(home, "plan", "add", "--project", PROJECT, "--step", STEP, "--text", item, "--title", "Canary")
        work.note((done.stdout + done.stderr).strip())
        work.reach("plan-add")
        if done.returncode != 0:
            return work.finish(False, "plan-add", (done.stderr or done.stdout).strip()[-300:])
        brief = folder(home) / "brief.md"
        brief.write_text("Canary: edit one file, report done, commit.\n", encoding="utf-8")
        lane_id = f"canary-{stamp}"
        tirith_bin = " ".join(f"'{part}'" for part in launch_mod.tirith_argv(home))
        env = {**os.environ, "TIRITH_BIN": f"{tirith_bin} --home '{home}'"}
        work.reach("lane", lane=lane_id)
        ran = _tirith(home, "lane", "run", "--project", PROJECT, "--class", "canary", "--feature", item, "--id", lane_id,
                      "--brief", str(brief), "--step", STEP, "--item", item, "--no-detach", "--",
                      "sh", "-c", AGENT, env=env)
        work.note((ran.stdout + ran.stderr).strip()[-2000:])
        lane = read_json(lane_path(home, lane_id), None) or {}
        if ran.returncode != 0 or lane.get("result") != "done" or (lane.get("outcome") or {}).get("status") != "done":
            return work.finish(False, "lane", f"exit {ran.returncode}: {(ran.stderr or ran.stdout).strip()[-300:]}")
        work.reach("queued")
        deadline = time.monotonic() + wait_s
        seen = {"queued": False, "running": False}
        while True:
            if pump is not None:
                pump()
            queue = mergeq_mod.load(home)
            entry = next((e for e in queue["queue"] if e["lane"] == lane_id), None)
            seen["queued"] = seen["queued"] or entry is not None
            seen["running"] = seen["running"] or bool(entry and entry["status"] == mergeq_mod.RUNNING)
            last = next((h for h in reversed(queue["history"]) if h.get("lane") == lane_id), None)
            if last:
                break
            if time.monotonic() >= deadline:
                step = "queued" if not seen["queued"] else "merge-start" if not seen["running"] else "merge"
                what = (entry or {}).get("message") or ""
                return work.finish(False, step, what or STEPS[step])
            sleep(POLL_S)
        if last["result"] not in (mergeq_mod.MERGED, mergeq_mod.PUSH_FAILED):
            work.reach("merge")
            return work.finish(False, "merge", f"{last['result']}: {last.get('message', '')[:300]}")
        work.reach("push")
        if last["result"] == mergeq_mod.PUSH_FAILED or _git(origin_of(home), "cat-file", "-e", "main:canary.txt").returncode:
            return work.finish(False, "push", last.get("message") or "canary.txt is not on the origin's main")
        work.reach("tick")
        if not _plan_item_done(repo, item):
            return work.finish(False, "tick", "the plan item is not ticked in the canary's docs/TODO.md")
        return work.finish(True, "tick")
    finally:
        try:
            _cleanup(home, repo_of(home), lane_id)
        except OSError:
            pass


# -- the schedule and the check ----------------------------------------------------------------


def running(row: dict | None, now: datetime) -> bool:
    return bool(row and row.get("state") == "running" and (now - (parse_iso(row["started"]) or now)).total_seconds() < LIMIT_S)


def due(home: Path, now: datetime) -> bool:
    """Every hour, and once after the collector reloads with new code; never while a run is under way."""
    runs = load(home)["runs"]
    last = runs[-1] if runs else None
    if running(last, now):
        return False
    if last is None:
        return True
    started = parse_iso(last.get("started"))
    return started is None or (now - started).total_seconds() >= EVERY_S or last.get("code") != code_id()


def tick(home: Path, now: datetime, spawn: Callable) -> bool:
    """Start a detached ``tirith canary run`` when one is due; True when it started."""
    if not due(home, now):
        return False
    spawn([*launch_mod.tirith_argv(home), "--home", str(home), "canary", "run"], home,
          home / "logs" / "canary-spawn.log")
    # the run writes its own row; this one stops a second start before it has
    data = load(home)
    stamp = now.strftime("%Y%m%d-%H%M%S")
    data["runs"].append({"id": f"pending-{stamp}", "started": now_iso(now), "step": "spawn", "state": "running",
                         "code": code_id(), "log": str(home / "logs" / "canary-spawn.log")})
    folder(home).mkdir(parents=True, exist_ok=True)
    _save(home, data)
    return True


def verdict(home: Path, now: datetime) -> tuple[str, str, dict | None]:
    """``(level, plain words, run)``: ``ok`` / ``failed``; the last runs decide."""
    runs = load(home)["runs"]
    if not runs:
        return "ok", "The first end-to-end test run has not happened yet.", None
    last = runs[-1]
    age = (now - (parse_iso(last.get("started")) or now)).total_seconds()
    if last.get("state") == "running":
        if age < LIMIT_S:
            return "ok", "An end-to-end test run is under way.", last
        return "failed", _failure(last), last
    if last.get("state") == "ok":
        return "ok", "The last end-to-end test run reached the push and the plan tick.", last
    return "failed", _failure(last), last


def _failure(run: dict) -> str:
    step = str(run.get("step") or "setup")
    return (f"The end-to-end test run stopped at \"{step}\": {STEPS.get(step, step)}. "
            f"Log: {run.get('log')}")
