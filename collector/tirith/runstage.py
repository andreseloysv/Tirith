"""One recorded pipeline stage run: the single code path behind ``tirith run`` and ``tirith lane review``.

Everything Tirith (or an agent) builds or tests goes through :func:`run_stage`, so the owner sees it as a
pipeline run with its stage, log and result. In a lane's git worktree the run is tagged with the lane,
the worktree and the branch. See ``docs/CONTRACT-manifest.md`` ("Runs").
"""

from __future__ import annotations

import contextlib
import fcntl
import os
import subprocess
import sys
from pathlib import Path

from tirith import timeouts as timeouts_mod
from tirith import projects as projects_mod
from tirith import projenv as projenv_mod
from tirith.home import now_iso
from tirith.store import append_jsonl, read_json, read_lanes, write_json_atomic

OUTPUT_MAX = 4000


def pipeline_book(home: Path) -> dict:
    data = read_json(home / "pipelines.json", {})
    return data if isinstance(data, dict) else {}


@contextlib.contextmanager
def pipelines_lock(home: Path):
    """Exclusive lock on ``pipelines.json`` (a lock file next to it) for a read-modify-write."""
    home.mkdir(parents=True, exist_ok=True)
    with (home / "pipelines.json.lock").open("a") as handle:
        fcntl.flock(handle, fcntl.LOCK_EX)
        try:
            yield
        finally:
            fcntl.flock(handle, fcntl.LOCK_UN)


def resolve_run(home: Path, project: str, pipeline: str, explicit: str | None, new_run: bool) -> str:
    """The run id of ``project/pipeline``: ``explicit``, the current run, or the next one."""
    if explicit:
        return explicit
    key = f"{project}/{pipeline}"
    with pipelines_lock(home):  # read, increment and write as one step: parallel runs get distinct numbers
        book = pipeline_book(home)
        entry = book.get(key) if isinstance(book.get(key), dict) else None
        if entry is None:
            entry = book.get(f"*/{pipeline}") if isinstance(book.get(f"*/{pipeline}"), dict) else {}
        counter = int(entry.get("counter") or 0)
        if new_run or counter == 0:
            counter += 1
            entry = {**entry, "counter": counter, "current_run": f"{pipeline}#{counter}"}
            book[key] = entry
            write_json_atomic(home / "pipelines.json", book)
        return str(entry.get("current_run") or f"{pipeline}#{counter}")


def _git(folder: Path, *args: str) -> str | None:
    try:
        done = timeouts_mod.run(["git", "-C", str(folder), *args], "git", text=True)
    except (OSError, subprocess.SubprocessError):
        return None
    return done.stdout.strip() if done.returncode == 0 else None


def worktree_info(folder: str | Path) -> dict | None:
    """``{"worktree", "branch"}`` when ``folder`` is inside a linked git worktree, else None."""
    folder = Path(folder)
    if not folder.is_dir():
        return None
    top = _git(folder, "rev-parse", "--show-toplevel")
    git_dir = _git(folder, "rev-parse", "--path-format=absolute", "--git-dir")
    common = _git(folder, "rev-parse", "--path-format=absolute", "--git-common-dir")
    if not (top and git_dir and common) or projects_mod.real(git_dir) == projects_mod.real(common):
        return None
    return {"worktree": str(projects_mod.real(top)),
            "branch": _git(folder, "branch", "--show-current") or None}


def lane_of_worktree(home: Path, worktree: str | Path) -> str | None:
    """Id of the lane whose ``workspace`` is that worktree (the newest one when several)."""
    wanted = projects_mod.real(worktree)
    found = [lane for lane in read_lanes(home)
             if lane.get("workspace") and projects_mod.real(str(lane["workspace"])) == wanted]
    found.sort(key=lambda lane: str(lane.get("started_at") or ""))
    return str(found[-1]["id"]) if found else None


def lane_tags(home: Path, folder: str | Path | None) -> dict:
    """``lane``/``worktree``/``branch`` tags for a run started in ``folder`` (``{}`` outside a lane worktree)."""
    info = worktree_info(folder) if folder else None
    if info is None:
        return {}
    lane = lane_of_worktree(home, info["worktree"])
    return {k: v for k, v in {"lane": lane, **info}.items() if v}


def cargo_target_dir(home: Path, project: str, folder: str | Path) -> Path | None:
    """The one ``CARGO_TARGET_DIR`` every cargo stage of ``project`` shares, across its lane worktrees:
    the project's ``cargo_target_dir`` from ``config.toml``, else ``<repo>-lanes/.cargo-target``."""
    from tirith.home import load_config
    proj = load_config(home).project(project)
    if proj and proj.cargo_target_dir:
        return proj.cargo_target_dir
    repo = projects_mod.real(proj.repo) if proj and proj.repo else None
    if repo is None:
        common = _git(Path(folder), "rev-parse", "--path-format=absolute", "--git-common-dir") \
            if Path(folder).is_dir() else None
        if not common or Path(common).name != ".git":
            return None
        repo = projects_mod.real(Path(common).parent)
    return repo.parent / f"{repo.name}-lanes" / ".cargo-target"


def _tail(text: str) -> str:
    text = text.strip()
    return text if len(text) <= OUTPUT_MAX else "…" + text[-OUTPUT_MAX:]


def run_stage(home: Path, project: str, pipeline: str, stage: str, command: list[str] | str,
              cwd: str | Path | None, lane: str | None = None, run_id: str | None = None, *,
              worktree: str | None = None, branch: str | None = None, agent: str | None = None,
              log: str | Path | None = None, echo: bool = True, env: dict | None = None,
              timeout: float | None = None, new_run: bool = False) -> tuple[int, str]:
    """Run one stage command, recording start and end in ``events.jsonl``; returns ``(exit code, output tail)``.

    ``command`` is an argv list, or a shell line. ``echo`` copies the output to this process's stdout.
    The run id is resolved from ``pipelines.json`` when not given."""
    run_id = resolve_run(home, project, pipeline, run_id, new_run)
    safe_run = run_id.replace("#", "-").replace("/", "-")
    log_path = Path(log).expanduser().resolve() if log else home / "logs" / f"{safe_run}-{stage}.log"
    log_path.parent.mkdir(parents=True, exist_ok=True)
    work = str(Path(cwd).resolve()) if cwd else str(Path.cwd().resolve())
    base = {"run": run_id, "project": project, "pipeline": pipeline, "stage": stage,
            "log": str(log_path), "cwd": work, "session": projects_mod.current_session(home)}
    for key, value in (("agent", agent), ("lane", lane), ("worktree", worktree), ("branch", branch)):
        if value:
            base[key] = value
    child_env = projenv_mod.apply(home, project, env)
    session = projects_mod.current_session(home)
    if session:
        child_env["TIRITH_SESSION"] = session
    if "CARGO_TARGET_DIR" not in child_env:  # worktrees share compiled dependencies instead of 2 GB each
        shared = cargo_target_dir(home, project, work)
        if shared:
            child_env["CARGO_TARGET_DIR"] = str(shared)
    limit = timeout or timeouts_mod.effective(home, None, f"stage:{stage}")
    started = {"pid": os.getpid(), "timeout": int(limit)}
    append_jsonl(home / "events.jsonl", {"at": now_iso(), **base, **started, "event": "start", "status": "running"})
    exit_code, kept = 1, b""
    stop = None
    try:
        with log_path.open("ab") as log_handle:
            proc = subprocess.Popen(command, shell=isinstance(command, str), stdout=subprocess.PIPE,
                                    stderr=subprocess.STDOUT, cwd=work, env=child_env,
                                    start_new_session=os.name == "posix")  # its own group: a timeout ends all of it
            stop = timeouts_mod.supervise(proc, f"stage:{stage}", home=home, timeout=limit,
                                          what=f"{pipeline}/{stage}")
            assert proc.stdout is not None
            for chunk in proc.stdout:
                log_handle.write(chunk)
                log_handle.flush()
                kept = (kept + chunk)[-OUTPUT_MAX * 4:]
                if echo:
                    buffer = getattr(sys.stdout, "buffer", None)
                    if buffer is not None:
                        buffer.write(chunk)
                        buffer.flush()
                    else:
                        sys.stdout.write(chunk.decode(errors="replace"))
                        sys.stdout.flush()
            exit_code = proc.wait()
    finally:
        ended = stop() if stop else {"timed_out": False}
        append_jsonl(home / "events.jsonl", {
            "at": now_iso(), **base, "event": "end", "status": "passed" if exit_code == 0 and not ended["timed_out"] else "failed",
            **({"timed_out": True} if ended["timed_out"] else {})})
    output = _tail(kept.decode(errors="replace"))
    if ended["timed_out"]:
        output = f"{ended['message']}\n{output}".strip()
        exit_code = timeouts_mod.EXIT_TIMED_OUT
    return exit_code, output
