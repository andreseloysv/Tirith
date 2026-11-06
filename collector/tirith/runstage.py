"""One recorded pipeline stage run: the single code path behind ``tirith run`` and ``tirith lane review``.

Everything Tirith (or an agent) builds or tests goes through :func:`run_stage`, so the owner sees it as a
pipeline run with its stage, log and result. In a lane's git worktree the run is tagged with the lane,
the worktree and the branch. See ``docs/CONTRACT-manifest.md`` ("Runs").
"""

from __future__ import annotations

import contextlib
import fcntl
import hashlib
import json
import re
import os
import shutil
import tempfile
import subprocess
import sys
from pathlib import Path

from tirith import timeouts as timeouts_mod
from tirith import projects as projects_mod
from tirith import projenv as projenv_mod
from tirith import netproxy as netproxy_mod
from tirith.home import now_iso
from tirith.store import append_jsonl, read_json, read_lanes, write_json_atomic

OUTPUT_MAX = 4000
TESTED_FILE = "tested.json"  # {key: [{tree, at, run, log}]}: trees whose test stage already passed
TESTED_KEEP = 40  # trees remembered per stage key
FORCE_ENV = "TIRITH_RETEST"  # set to 1 to run a test stage even when this exact tree already passed


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


def resolve_run(home: Path, project: str, pipeline: str, explicit: str | None, new_run: bool,
                worktree: str | None = None) -> str:
    """The run id of ``project/pipeline``: ``explicit``, the current run, or the next one.

    The current run belongs to the worktree that last took it; a run from another worktree gets the next number
    (so two worktrees never share a run number, and with it a log file). ``worktree`` is None outside a lane."""
    if explicit:
        return explicit
    key = f"{project}/{pipeline}"
    with pipelines_lock(home):  # read, increment and write as one step: parallel runs get distinct numbers
        book = pipeline_book(home)
        entry = book.get(key) if isinstance(book.get(key), dict) else None
        if entry is None:
            entry = book.get(f"*/{pipeline}") if isinstance(book.get(f"*/{pipeline}"), dict) else {}
        counter = int(entry.get("counter") or 0)
        owner = entry.get("worktree") if "worktree" in entry else None  # no owner recorded: anyone may reuse
        elsewhere = "worktree" in entry and (owner or None) != (worktree or None)
        if new_run or counter == 0 or elsewhere:
            counter += 1
            entry = {**entry, "counter": counter, "current_run": f"{pipeline}#{counter}", "worktree": worktree or None}
            book[key] = entry
            write_json_atomic(home / "pipelines.json", book)
        elif "worktree" not in entry and worktree:
            book[key] = {**entry, "worktree": worktree}
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
    lines = (_git(folder, "rev-parse", "--path-format=absolute", "--show-toplevel", "--git-dir",
                  "--git-common-dir") or "").splitlines()  # one git process answers all three
    top, git_dir, common = (lines + ["", "", ""])[:3]
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


def tree_of(folder: str | Path) -> str | None:
    """The git tree hash of everything in ``folder``'s worktree (tracked and untracked files, working-tree
    content, ignored files left out), computed in a copy of the index so the real one is never touched;
    None outside a git worktree."""
    folder = Path(folder)
    if not folder.is_dir():
        return None
    lines = (_git(folder, "rev-parse", "--path-format=absolute", "--git-path", "index", "--show-toplevel")
             or "").splitlines()  # one git process answers both
    index, top = (lines + ["", ""])[:2]
    if not index or not top:
        return None
    fd, temp = tempfile.mkstemp(prefix="tirith-tree-")
    os.close(fd)
    try:
        if Path(index).is_file():
            shutil.copyfile(index, temp)
        else:
            os.unlink(temp)  # no index yet: git builds one
        env = {**os.environ, "GIT_INDEX_FILE": temp}
        for args in (("add", "-A"), ("write-tree",)):
            done = timeouts_mod.run(["git", "-C", top, *args], "git", text=True, env=env)
            if done.returncode != 0:
                return None
        return done.stdout.strip() or None
    except (OSError, subprocess.SubprocessError):
        return None
    finally:
        with contextlib.suppress(OSError):
            os.unlink(temp)


def _tested_key(project: str, pipeline: str, stage: str, command, env: dict | None = None) -> str:
    line = (command if isinstance(command, str) else json.dumps(list(command))) + json.dumps(env or {}, sort_keys=True)
    return f"{project}|{pipeline}|{stage}|{hashlib.sha1(line.encode()).hexdigest()[:16]}"


def cacheable(stage: str) -> bool:
    return "test" in stage and not stage.endswith("-rerun") and os.environ.get(FORCE_ENV) != "1"


def tested_before(home: Path, key: str, tree: str) -> dict | None:
    """The record of the earlier passing run of this stage on exactly ``tree``, else None."""
    book = read_json(home / TESTED_FILE, {})
    for row in book.get(key, []) if isinstance(book, dict) else []:
        if isinstance(row, dict) and row.get("tree") == tree:
            return row
    return None


def remember_tested(home: Path, key: str, tree: str, run_id: str, log: Path) -> None:
    home.mkdir(parents=True, exist_ok=True)
    with (home / (TESTED_FILE + ".lock")).open("a") as handle:
        fcntl.flock(handle, fcntl.LOCK_EX)
        try:
            book = read_json(home / TESTED_FILE, {})
            book = book if isinstance(book, dict) else {}
            rows = [r for r in book.get(key, []) if isinstance(r, dict) and r.get("tree") != tree]
            book[key] = (rows + [{"tree": tree, "at": now_iso(), "run": run_id, "log": str(log)}])[-TESTED_KEEP:]
            write_json_atomic(home / TESTED_FILE, book)
        finally:
            fcntl.flock(handle, fcntl.LOCK_UN)


def _reuse(home: Path, base: dict, before: dict, tree: str, log_path: Path, echo: bool) -> tuple[int, str]:
    """Record a passing run that did not execute anything: the tree already passed (``before``). Its log keeps the
    earlier output, so the claim check still reads the counts."""
    try:
        earlier = Path(str(before.get("log"))).read_text(encoding="utf-8", errors="replace")
    except OSError:
        earlier = ""
    note = f"tirith: tree {tree[:12]} already passed this stage in run {before.get('run')} ({before.get('at')}); not run again"
    text = _tail(f"{note}\n{earlier}")
    log_path.write_text(text + "\n", encoding="utf-8")
    if echo:
        sys.stdout.write(note + "\n")
        sys.stdout.flush()
    append_jsonl(home / "events.jsonl", {"at": now_iso(), **base, "pid": os.getpid(), "event": "start", "status": "running",
                                         "cached": True})
    append_jsonl(home / "events.jsonl", {"at": now_iso(), **base, "event": "end", "status": "passed", "cached": True})
    return 0, text


_PARTIAL = re.compile(r"(?:^|\s)(?:-k\s|-k\S|--deselect\b)|pytest\b[^&|;]*?\s[\w./-]*test_\w*\.py\b")


def is_partial(command) -> bool:
    """Whether a test command runs a selection (pytest ``-k`` or named test files), not the whole suite."""
    line = command if isinstance(command, str) else " ".join(map(str, command))
    return bool(_PARTIAL.search(line))


def _safe(text: str) -> str:
    return re.sub(r"[^\w.-]+", "-", str(text)).strip("-") or "x"


def stage_log_prefix(project: str, run_id: str) -> str:
    """``ci-<project>-<n>`` for run ``ci#<n>``: the project is in the name, so two projects never share a log."""
    kind, sep, number = str(run_id).partition("#")
    if not sep:
        return f"{_safe(project)}-{_safe(run_id)}" if project else _safe(run_id)
    return "-".join(_safe(part) for part in (kind, project, number) if part)


def stage_log_path(home: Path, project: str, run_id: str, stage: str) -> Path:
    return Path(home) / "logs" / f"{stage_log_prefix(project, run_id)}-{stage}.log"


def run_stage(home: Path, project: str, pipeline: str, stage: str, command: list[str] | str,
              cwd: str | Path | None, lane: str | None = None, run_id: str | None = None, *,
              worktree: str | None = None, branch: str | None = None, agent: str | None = None,
              log: str | Path | None = None, echo: bool = True, env: dict | None = None,
              timeout: float | None = None, new_run: bool = False, reuse: bool = False) -> tuple[int, str]:
    """Run one stage command, recording start and end in ``events.jsonl``; returns ``(exit code, output tail)``.

    ``command`` is an argv list, or a shell line. ``echo`` copies the output to this process's stdout.
    The run id is resolved from ``pipelines.json`` when not given. With ``reuse`` (the lane's ``tirith run``, the
    review and the merge queue) a test stage is not run again on a tree that already passed it."""
    run_id = resolve_run(home, project, pipeline, run_id, new_run, worktree)
    log_path = Path(log).expanduser().resolve() if log else stage_log_path(home, project, run_id, stage)
    log_path.parent.mkdir(parents=True, exist_ok=True)
    work = str(Path(cwd).resolve()) if cwd else str(Path.cwd().resolve())
    base = {"run": run_id, "project": project, "pipeline": pipeline, "stage": stage,
            "log": str(log_path), "cwd": work, "session": projects_mod.current_session(home)}
    for key, value in (("agent", agent), ("lane", lane), ("worktree", worktree), ("branch", branch)):
        if value:
            base[key] = value
    tree = key = None
    if reuse and timeout is None and cacheable(stage):  # a tree that already passed this stage is not tested again
        tree = tree_of(work)
        key = _tested_key(project, pipeline, stage, command, env)
        before = tested_before(home, key, tree) if tree else None
        if before:
            return _reuse(home, base, before, tree, log_path, echo)
    child_env = projenv_mod.apply(home, project, env)
    session = projects_mod.current_session(home)
    if session:
        child_env["TIRITH_SESSION"] = session
    if "CARGO_TARGET_DIR" not in child_env:  # worktrees share compiled dependencies instead of 2 GB each
        shared = cargo_target_dir(home, project, work)
        if shared:
            child_env["CARGO_TARGET_DIR"] = str(shared)
    netproxy_mod.swift_shim(home, child_env)  # swift reached through a script or make gets the flag too
    command = netproxy_mod.swiftpm(command, child_env)  # SwiftPM cannot nest a sandbox-exec inside a lane's
    limit = timeout or timeouts_mod.effective(home, None, f"stage:{stage}")
    started = {"pid": os.getpid(), "timeout": int(limit)}
    if is_partial(command):
        started["partial"] = True  # a -k or file selection: the claim check accepts its count (claimcheck.py)
    append_jsonl(home / "events.jsonl", {"at": now_iso(), **base, **started, "event": "start", "status": "running"})
    exit_code, kept = 1, b""
    stop = None
    try:
        with log_path.open("ab") as log_handle:
            proc = subprocess.Popen(command, shell=isinstance(command, str), stdout=subprocess.PIPE,
                                    stderr=subprocess.STDOUT, cwd=work, env=child_env,
                                    start_new_session=os.name == "posix")  # its own group: a timeout ends all of it
            stop = timeouts_mod.supervise(proc, f"stage:{stage}", home=home, timeout=limit,
                                          what=f"{pipeline}/{stage}",
                                          idle=timeouts_mod.effective_idle(home, None, f"stage:{stage}"),
                                          probe=timeouts_mod.Probe(workspace=work, pgid=proc.pid))
            assert proc.stdout is not None
            for chunk in proc.stdout:
                stop.touch()
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
    elif exit_code == 0 and tree and key:
        remember_tested(home, key, tree, run_id, log_path)
    return exit_code, output
