"""The ``tirith`` command line: lanes, pipelines, notes, waits, guard, brain.

Every subcommand only touches files under TIRITH_HOME (plus, for
``lane run``/``run``, the child command it wraps and, for ``guard install``,
the hooks folder of the repo the owner named). No command opens a port.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import secrets
import signal
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

from tirith.collector import (
    Collector, collect_tree, git_head, git_shortstat, heuristic_architecture, lane_status,
)
from tirith.guardwrap import default_words, guard_upgrade_steps, install_guard, verify_guard
from tirith import agentrun as agentrun_mod
from tirith import flows as flows_mod
from tirith import gates as gates_mod
from tirith import manifest as manifest_mod
from tirith import hooks as hooks_mod
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith import review as review_mod
from tirith import projects as projects_mod
from tirith import rooms as rooms_mod
from tirith.home import ensure_home, load_config, now_iso, parse_iso, tirith_home
from tirith.store import (
    append_jsonl, final_answer, lane_path, read_json, read_jsonl, read_lanes, write_json_atomic,
    write_lane,
)


def _home(args) -> Path:
    return ensure_home(tirith_home(getattr(args, "home", None)))


def _slug(text: str) -> str:
    slug = re.sub(r"[^a-z0-9]+", "-", (text or "").lower()).strip("-")
    return slug[:32] or "lane"


def _unique_lane_id(home: Path, wanted: str) -> str:
    candidate, suffix = wanted, 2
    while lane_path(home, candidate).exists():
        candidate = f"{wanted}-{suffix}"
        suffix += 1
    return candidate


REQUEST_MAX = 2048
REQUEST_WINDOW_S = 6 * 3600
REPORT_MAX = 16 * 1024


def bound_text(text, limit: int) -> str:
    """``text`` cut at a line boundary under ``limit`` characters (whole lines only)."""
    text = str(text or "")
    if len(text) <= limit:
        return text
    cut = text[:limit - 3]
    head = cut.rsplit("\n", 1)[0] if "\n" in cut else cut
    return head.rstrip() + "\n\n…"


def owner_request(home: Path, environ=None) -> dict | None:
    """The owner's request this lane answers: the parent's agent session (``TIRITH_AGENT_SESSION``)
    else the most recent request within the last 6 hours, as ``{text, at, agent_session_id}``."""
    env = os.environ if environ is None else environ
    folder = home / hooks_mod.REQUESTS_DIR
    wanted = re.sub(r"[^A-Za-z0-9._-]", "_", env.get("TIRITH_AGENT_SESSION") or "")
    best, best_at = None, None
    for path in sorted(folder.glob("*.json")) if folder.is_dir() else []:
        data = read_json(path, None)
        when = parse_iso(data.get("at")) if isinstance(data, dict) else None
        if not when or not data.get("prompt"):
            continue
        if wanted:
            if path.stem == wanted:
                best, best_at = data, when
                break
            continue
        if best_at is None or when > best_at:
            best, best_at = data, when
    if best is None and wanted:
        return owner_request(home, {})
    if best is None or (not wanted and (datetime.now(timezone.utc) - best_at).total_seconds()
                        > REQUEST_WINDOW_S):
        return None
    return {"text": bound_text(best["prompt"], REQUEST_MAX), "at": best["at"],
            "agent_session_id": str(best.get("agent_session_id") or "")}


def register_lane(home: Path, *, lane_id: str | None, project: str, hero_class: str,
                  feature: str, workspace: str | None, log: str | None,
                  pid: int | None, brief: str | None, step: str | None = None) -> dict:
    """Write a new lane registration; returns the stored dict."""
    if not lane_id:
        lane_id = _unique_lane_id(home, _slug(feature))
    elif lane_path(home, lane_id).exists():
        raise SystemExit(f"lane already exists: {lane_id}")
    workspace_abs = str(Path(workspace).expanduser().resolve()) if workspace else None
    parent = os.environ.get("TIRITH_LANE") or "steward"
    lane = {
        "id": lane_id, "parent": parent, "project": project, "class": hero_class, "feature": feature,
        "workspace": workspace_abs,
        "log": str(Path(log).expanduser().resolve()) if log else None,
        "pid": pid,
        "brief": str(Path(brief).expanduser().resolve()) if brief else None,
        "started_at": now_iso(), "ended_at": None, "result": None,
        "start_commit": git_head(workspace_abs), "report": None,
        "session": projects_mod.current_session(home),
        "step": str(step) if step else None,
        "request": owner_request(home),
    }
    write_lane(home, lane)
    source = "steward" if parent == "steward" else f"agent:{parent}"
    flows_mod.append_flow(home, source, f"agent:{lane_id}", "brief",
                          f"Brief: {feature}", lane["brief"], project=project)
    return lane


def end_lane(home: Path, lane_id: str, result: str, report: str | None = None) -> dict:
    """Mark a lane ended with done/failed; returns the stored dict."""
    lane = read_json(lane_path(home, lane_id), None)
    if not isinstance(lane, dict):
        raise SystemExit(f"unknown lane: {lane_id}")
    lane["ended_at"] = now_iso()
    lane["result"] = result
    lane.pop("token", None)  # the shims accept the token of a running lane only
    stored = agentrun_mod.store_diff(home, lane)
    if stored:
        lane["diff"] = stored
    if report is not None:
        lane["report"] = bound_text(report, REPORT_MAX)
    write_lane(home, lane)
    flows_mod.append_flow(
        home, f"agent:{lane_id}", "steward", "report",
        f"{result}: " + (flows_mod.first_line(report) or str(lane.get("feature") or "")),
        session=lane.get("session") or projects_mod.current_session(home),
        project=str(lane.get("project") or "") or None)
    return lane


def _cmd_lane_start(args) -> int:
    home = _home(args)
    lane = register_lane(
        home, lane_id=args.id, project=args.project, hero_class=args.hero_class,
        feature=args.feature, workspace=args.workspace, log=args.log,
        pid=args.pid, brief=args.brief, step=args.step)
    print(lane["id"])
    return 0


def _lane_repo(home: Path, lane_id: str) -> Path | None:
    lane = read_json(lane_path(home, lane_id), None)
    if not isinstance(lane, dict):
        return None
    _, proj = _project_or_exit(home, str(lane.get("project") or ""))
    return projects_mod.real(proj.repo) if proj.repo else None


def _cmd_lane_review(args) -> int:
    home = _home(args)
    try:
        review = review_mod.review_lane(home, args.lane_id, _lane_repo(home, args.lane_id),
                                        merge=not args.tests_only)
    except review_mod.ReviewError as error:
        print(f"tirith lane review: {error}", file=sys.stderr)
        return 3
    print(f"lane {args.lane_id}: {review['status']}" + (f"\n{review['message']}" if review.get("message") else ""))
    return 0 if review["status"] in (review_mod.READY, review_mod.MERGED) else 1


def _cmd_lane_end(args) -> int:
    home = _home(args)
    report = args.report
    if args.report_file:
        report = Path(args.report_file).expanduser().read_text(encoding="utf-8")
    end_lane(home, args.lane_id, args.result, report)
    print(f"lane {args.lane_id}: {args.result}")
    return 0


def _wrapper_path_env(home: Path | None = None, lane_id: str | None = None,
                      token: str | None = None) -> dict:
    """Environment with the owner's git wrapper folder first in PATH (and the lane, if any)."""
    env = dict(os.environ)
    if lane_id:
        env["TIRITH_LANE"] = lane_id
    if token:
        env[gates_mod.TOKEN_ENV] = token
    session = projects_mod.current_session(home or tirith_home())
    if session:
        env["TIRITH_SESSION"] = session
    wrapper_dir = str(Path.home() / ".local" / "bin")
    env["PATH"] = wrapper_dir + os.pathsep + env.get("PATH", "")
    return env


def _project_or_exit(home: Path, project_id: str):
    config = load_config(home)
    proj = config.project(project_id)
    if proj is None:
        auto = next((p for p in projects_mod.load_auto(home)["projects"]
                     if str(p["id"]) == project_id), None)
        if auto is not None:
            from tirith.home import ProjectConfig
            return config, ProjectConfig(id=project_id, name=str(auto.get("name") or project_id),
                                         repo=Path(auto["folder"]), color=str(auto.get("color") or "#888888"))
        raise SystemExit(f"unknown project '{project_id}' (not in {home / 'config.toml'})")
    return config, proj


def _lane_run_refusal(args, config, proj, workspace: Path) -> str | None:
    """Why ``lane run`` must not start (agent-style text, exit 3), else None."""
    brief = Path(args.brief).expanduser() if args.brief else None
    if brief is None or not brief.is_file() or not brief.read_text(
            encoding="utf-8", errors="replace").strip():
        shown = f" ({brief})" if brief else ""
        return (f"tirith: lane-brief: lane run needs a brief: the file is missing or empty{shown}. "
                "Do this instead: write the task (goal, files, rules, how to report) into a file "
                "and pass it with --brief <file>")
    problems = verify_guard(workspace, config.words_for(proj, default_words()))
    if problems:
        return "refusing to start: commit guard is not intact:\n" + \
            "\n".join(f"  {problem}" for problem in problems)
    failed = [p for p in manifest_mod.check(workspace, config.words_for(proj, default_words()))
              if p.rule != "commit-guard"]  # the guard was verified just above
    if failed:
        return ("tirith: project-check: tirith check fails for '" + proj.id + "', so no lane may "
                "start. Do this instead: fix these, then run `tirith check --project "
                + proj.id + "`:\n" + "\n".join(f"  {p}" for p in failed))
    return None


def _agent_command(config, args) -> list[str]:
    if args.prompt and args.prompt_file:
        raise SystemExit("lane run: pass --prompt or --prompt-file, not both")
    if args.prompt_file:
        prompt = Path(args.prompt_file).expanduser().read_text(encoding="utf-8")
    elif args.prompt:
        prompt = args.prompt
    elif args.brief and Path(args.brief).expanduser().is_file():
        prompt = Path(args.brief).expanduser().read_text(encoding="utf-8")  # the brief is the task
    else:
        raise SystemExit("lane run --agent needs --prompt or --prompt-file")
    if not prompt.strip():
        raise SystemExit("lane run: the prompt is empty")
    return agentrun_mod.build_command(config, args.agent, args.project, prompt,
                                      model=args.model, effort=args.effort)


def _cmd_doctor(args) -> int:
    home = _home(args)
    config = load_config(home)
    env = _wrapper_path_env(home)
    wrapper_dir = Path.home() / ".local" / "bin"
    failed = 0
    for name, ok, detail in agentrun_mod.doctor_agent(config, args.agent, env["PATH"], wrapper_dir):
        print(("ok   " if ok else "FAIL ") + f"{name}: {detail}")
        failed += 0 if ok else 1
    return 1 if failed else 0


def _cmd_lane_run(args) -> int:
    home = _home(args)
    command = list(args.command or [])
    if command and command[0] == "--":
        command = command[1:]
    config, proj = _project_or_exit(home, args.project)
    if args.agent:
        if command:
            raise SystemExit("lane run: use either --agent or -- <command>, not both")
        command = _agent_command(config, args)
    elif not command:
        raise SystemExit("lane run needs a command after -- (or --agent <alias>)")
    workspace = Path(args.workspace).expanduser().resolve() if args.workspace else proj.repo
    if workspace is None:
        raise SystemExit(f"project '{args.project}' has no repo; pass --workspace")
    refusal = _lane_run_refusal(args, config, proj, Path(workspace))
    if refusal:
        print(refusal, file=sys.stderr)
        return 3
    lane_id = args.id or _unique_lane_id(home, _slug(args.feature))
    interactive = bool(getattr(args, "interactive", False))
    log_path = None if interactive else (
        Path(args.log).expanduser().resolve() if args.log else home / "logs" / f"{lane_id}.log")
    if log_path is not None:
        log_path.parent.mkdir(parents=True, exist_ok=True)
    lane = register_lane(
        home, lane_id=lane_id, project=args.project, hero_class=args.hero_class,
        feature=args.feature, workspace=str(workspace),
        log=str(log_path) if log_path else None,
        pid=None, brief=args.brief, step=args.step)
    lane["token"] = secrets.token_urlsafe(24)
    write_lane(home, lane)
    began = time.monotonic()
    env = _wrapper_path_env(home, lane_id, lane["token"])
    if interactive:
        exit_code = _run_interactive(home, lane, command, workspace, env)
        answer = _last_stop_message(home, lane_id, lane.get("started_at"))
    else:
        with log_path.open("ab") as log_handle:
            proc = subprocess.Popen(
                command, cwd=str(workspace), env=env,
                stdout=log_handle, stderr=subprocess.STDOUT)
            lane["pid"] = proc.pid
            write_lane(home, lane)
            exit_code = proc.wait()
        answer, _ = final_answer(log_path)
    result = "done" if exit_code == 0 else "failed"
    end_lane(home, lane_id, result, answer or None)
    if result == "failed":
        add_wait(home, args.project, f"lane {lane_id} failed")
    elif not answer:
        add_wait(home, args.project, f"lane {lane_id} ended without an answer")
    files, plus, minus = git_shortstat(str(workspace), lane.get("start_commit"))
    print(f"lane {lane_id}: {result}, {files} files +{plus} -{minus}, "
          f"{_duration(time.monotonic() - began)}")
    if answer:
        print()
        print(answer)
    return exit_code


FORWARDED_SIGNALS = tuple(getattr(signal, name) for name in ("SIGINT", "SIGTERM", "SIGHUP")
                          if hasattr(signal, name))


def _run_interactive(home: Path, lane: dict, command: list[str], workspace: Path, env: dict) -> int:
    """Run ``command`` on the owner's terminal; signals go to the child, which is always awaited.

    Returns the exit code (128 + signal number when the child died of a signal).
    """
    proc = subprocess.Popen(command, cwd=str(workspace), env=env)  # inherits stdin/stdout/stderr
    lane["pid"] = proc.pid
    write_lane(home, lane)

    # Ctrl+C on a terminal already reaches the child (same foreground process group): forwarding
    # SIGINT too would interrupt it twice, so it is only forwarded when there is no terminal.
    on_terminal = sys.stdin.isatty()

    def forward(signum, _frame):
        if on_terminal and signum == getattr(signal, "SIGINT", None):
            return
        try:
            proc.send_signal(signum)
        except (OSError, ProcessLookupError):
            pass

    previous = {}
    for signum in FORWARDED_SIGNALS:
        try:
            previous[signum] = signal.signal(signum, forward)
        except ValueError:  # not the main thread: no forwarding, still waits
            break
    try:
        while True:
            try:
                code = proc.wait()
                break
            except KeyboardInterrupt:  # SIGINT raced our handler install
                forward(signal.SIGINT, None)
    finally:
        for signum, handler in previous.items():
            signal.signal(signum, handler)
    return code if code >= 0 else 128 - code


def _last_stop_message(home: Path, lane_id: str, since: str | None) -> str:
    """Message of the lane's last ``Stop`` hook event (the final answer of an interactive lane)."""
    rows = hooks_mod.read_hook_rows(home / hooks_mod.EVENTS_FILE)
    for row in reversed(rows):
        if row.get("lane") == lane_id and row.get("event") == "Stop" \
                and (not since or str(row.get("ts") or "") >= since):
            text = str(row.get("summary") or "").strip()
            return "" if text in ("", "Stop") else text
    return ""


def _duration(seconds: float) -> str:
    total = int(seconds)
    if total < 60:
        return f"{total}s"
    minutes, secs = divmod(total, 60)
    if minutes < 60:
        return f"{minutes}m{secs:02d}s"
    return f"{minutes // 60}h{minutes % 60:02d}m"


def _pipeline_book(home: Path) -> dict:
    data = read_json(home / "pipelines.json", {})
    return data if isinstance(data, dict) else {}


def _cmd_pipeline_declare(args) -> int:
    home = _home(args)
    stages = [s.strip() for s in (args.stages or "").split(",") if s.strip()]
    if not stages:
        raise SystemExit("pipeline declare needs --stages a,b,c")
    book = _pipeline_book(home)
    key = f"{args.project}/{args.pipeline}" if args.project else f"*/{args.pipeline}"
    entry = book.get(key) if isinstance(book.get(key), dict) else {}
    entry.update({"stages": stages})
    entry.setdefault("counter", 0)
    book[key] = entry
    write_json_atomic(home / "pipelines.json", book)
    print(f"{args.pipeline}: {','.join(stages)}")
    return 0


def _resolve_run(home: Path, project: str, pipeline: str,
                 explicit: str | None, new_run: bool) -> str:
    if explicit:
        return explicit
    book = _pipeline_book(home)
    key = f"{project}/{pipeline}"
    entry = book.get(key) if isinstance(book.get(key), dict) else None
    if entry is None:
        entry = book.get(f"*/{pipeline}") if isinstance(book.get(f"*/{pipeline}"), dict) else {}
        key = f"{project}/{pipeline}"
    counter = int(entry.get("counter") or 0)
    if new_run or counter == 0:
        counter += 1
        entry = {**entry, "counter": counter, "current_run": f"{pipeline}#{counter}"}
        book[key] = entry
        write_json_atomic(home / "pipelines.json", book)
    return str(entry.get("current_run") or f"{pipeline}#{counter}")


def _wrapper_env_session(home: Path) -> dict:
    """Child environment that carries the current session to nested tirith calls."""
    env = dict(os.environ)
    session = projects_mod.current_session(home)
    if session:
        env["TIRITH_SESSION"] = session
    return env


def _project_folder_of(home: Path, project: str | None) -> tuple[str, Path | None]:
    """``(project id, folder)`` for an id, or for the project containing the cwd."""
    config = load_config(home)
    items = projects_mod.entries(config, projects_mod.load_auto(home))
    if project:
        found = next((e for e in items if e.id == project), None)
    else:
        found = projects_mod.match(Path.cwd(), items)
    return (found.id if found else project or ""), (found.folder if found else None)


def _manifest_stage(args, home: Path, command: list[str]) -> list[str]:
    """Fill project, pipeline and command from ``tirith.toml`` when ``run`` leaves them out."""
    args.project, folder = _project_folder_of(home, args.project)
    if args.stage in manifest_mod.GIT_EVENTS and command and not args.pipeline:
        # a git hook's `[[hooks]]` entry: one "git-hooks" pipeline per project
        args.pipeline = manifest_mod.GIT_HOOKS_PIPELINE
        args.project = args.project or Path.cwd().name
    if not args.project:
        raise SystemExit("tirith run needs --project (cwd is in no known project)")
    if args.pipeline and command:
        return command
    loaded = manifest_mod.load(folder) if folder else None
    if loaded is None or not loaded.exists:
        raise SystemExit("tirith run needs --pipeline and a command after --, "
                         f"or a {manifest_mod.FILENAME} in the project folder")
    matches = [(p, s) for p in loaded.pipelines for s in p.stages
               if s.name == args.stage and (not args.pipeline or p.id == args.pipeline)]
    if not matches:
        raise SystemExit(f"stage '{args.stage}' is not in {loaded.path}")
    if len(matches) > 1:
        ids = ", ".join(p.id for p, _ in matches)
        raise SystemExit(f"stage '{args.stage}' is in several pipelines ({ids}): add --pipeline <id>")
    pipe, stage = matches[0]
    args.pipeline = pipe.id
    if command:
        return command
    if not stage.run:
        raise SystemExit(f"stage '{args.stage}' has no run command in {loaded.path}")
    args.stage_cwd = str(Path(loaded.path).parent)  # manifest commands run from the project root
    return ["/bin/sh", "-c", stage.run]


def _cmd_run(args) -> int:
    home = _home(args)
    command = list(args.command or [])
    if command and command[0] == "--":
        command = command[1:]
    command = _manifest_stage(args, home, command)
    run_id = _resolve_run(home, args.project, args.pipeline, args.run, args.new_run)
    safe_run = run_id.replace("#", "-").replace("/", "-")
    log_path = Path(args.log).expanduser().resolve() if args.log \
        else home / "logs" / f"{safe_run}-{args.stage}.log"
    log_path.parent.mkdir(parents=True, exist_ok=True)
    base = {"run": run_id, "project": args.project, "pipeline": args.pipeline,
            "stage": args.stage, "log": str(log_path),
            "cwd": getattr(args, "stage_cwd", None) or str(Path.cwd().resolve()),
            "session": projects_mod.current_session(home)}
    if args.agent:
        base["agent"] = args.agent
    append_jsonl(home / "events.jsonl",
                 {"at": now_iso(), **base, "event": "start", "status": "running"})
    exit_code = 1
    try:
        with log_path.open("ab") as log_handle:
            proc = subprocess.Popen(command, stdout=subprocess.PIPE, cwd=getattr(args, "stage_cwd", None),
                                    stderr=subprocess.STDOUT, env=_wrapper_env_session(home))
            assert proc.stdout is not None
            for chunk in proc.stdout:
                log_handle.write(chunk)
                log_handle.flush()
                buffer = getattr(sys.stdout, "buffer", None)
                if buffer is not None:
                    buffer.write(chunk)
                    buffer.flush()
                else:
                    sys.stdout.write(chunk.decode(errors="replace"))
                    sys.stdout.flush()
            exit_code = proc.wait()
    finally:
        append_jsonl(home / "events.jsonl", {
            "at": now_iso(), **base, "event": "end",
            "status": "passed" if exit_code == 0 else "failed"})
    return exit_code


def _cmd_note(args) -> int:
    home = _home(args)
    append_jsonl(home / "notes.jsonl",
                 {"at": now_iso(), "project": args.project, "text": args.text,
                  "session": projects_mod.current_session(home)})
    print("noted")
    return 0


def _next_wait_id(home: Path) -> str:
    highest = 0
    for row in read_jsonl(home / "waits.jsonl"):
        match = re.match(r"w-(\d+)$", str(row.get("id") or ""))
        if match:
            highest = max(highest, int(match.group(1)))
    return f"w-{highest + 1}"


def add_wait(home: Path, project: str, text: str, link: str | None = None,
             wait_id: str | None = None) -> str:
    """Add an item to the owner's waiting list; returns its id."""
    wait_id = wait_id or _next_wait_id(home)
    append_jsonl(home / "waits.jsonl", {
        "id": wait_id, "at": now_iso(), "project": project,
        "text": text, "link": link, "done_at": None,
        "session": projects_mod.current_session(home)})
    flows_mod.append_flow(home, "steward", "owner", "request", text, link, project=project)
    return wait_id


def _cmd_wait_add(args) -> int:
    print(add_wait(_home(args), args.project, args.text, args.link, args.id))
    return 0


def _cmd_wait_done(args) -> int:
    home = _home(args)
    rows = read_jsonl(home / "waits.jsonl")
    original = next((r for r in reversed(rows) if str(r.get("id")) == args.wait_id), None)
    if original is None:
        raise SystemExit(f"unknown wait: {args.wait_id}")
    append_jsonl(home / "waits.jsonl", {
        **original, "done_at": now_iso(), "session": original.get("session"),
        "done_session": projects_mod.current_session(home)})
    flows_mod.append_flow(home, "owner", "steward", "answer",
                          f"Done: {original.get('text') or args.wait_id}",
                          session=original.get("session") or projects_mod.current_session(home),
                          project=original.get("project"))
    print(f"{args.wait_id}: done")
    return 0


def _cmd_flow(args) -> int:
    home = _home(args)
    for label, value in (("--from", args.src), ("--to", args.dst)):
        if not flows_mod.valid_endpoint(value):
            raise SystemExit(
                f"{label} must be steward, owner, guard, agent:<lane>, room:<room> or "
                f"pipeline:<run>, got {value!r}")
    if not args.text.strip():
        raise SystemExit("--text is empty")
    row = flows_mod.append_flow(home, args.src, args.dst, args.kind, args.text, args.ref)
    print(row["id"])
    return 0


def _cmd_decide(args) -> int:
    """Record a decision and its reason: agent -> steward, or steward -> owner without a lane."""
    home = _home(args)
    if not args.decision.strip():
        raise SystemExit("the decision is empty")
    if not args.why.strip():
        raise SystemExit("--why is empty")
    lane_id = args.lane or os.environ.get("TIRITH_LANE")
    lane = read_json(lane_path(home, lane_id), None) if lane_id else None
    if args.lane and not isinstance(lane, dict):
        raise SystemExit(f"unknown lane: {args.lane}")
    lane = lane if isinstance(lane, dict) else {}
    src, dst = (f"agent:{lane_id}", "steward") if lane else ("steward", "owner")
    row = flows_mod.append_flow(
        home, src, dst, "decision", args.decision, why=args.why,
        session=lane.get("session") or projects_mod.current_session(home),
        project=str(lane.get("project") or "") or None)
    print(row["id"])
    return 0


def _cmd_project_add(args) -> int:
    home = _home(args)
    words = [w.strip() for w in (args.guard_words or "").split(",") if w.strip()] or None
    try:
        pid, folder, created = projects_mod.add_project(
            home, args.folder, args.name, args.color, words)
    except ValueError as exc:
        raise SystemExit(str(exc))
    print(f"{pid}: {folder}" if created else f"{folder} is already registered as {pid}")
    return 0


def _cmd_project_remove(args) -> int:
    home = _home(args)
    try:
        where = projects_mod.remove_project(home, args.project_id)
    except ValueError as exc:
        raise SystemExit(str(exc))
    if where is None:
        print(f"unknown project: {args.project_id}", file=sys.stderr)
        return 1
    print(f"removed {args.project_id}" + (" (auto-discovered; it will not come back)"
                                          if where == "auto" else ""))
    return 0


def _cmd_project_list(args) -> int:
    home = _home(args)
    items = projects_mod.entries(load_config(home), projects_mod.load_auto(home))
    if not items:
        print("no projects; try: tirith project add <folder>")
        return 0
    for item in items:
        print(f"{item.id}  {item.folder or '-'}  {item.name}  {item.color}"
              + ("  [auto]" if item.auto else ""))
    return 0


def _cmd_session_start(args) -> int:
    home = _home(args)
    title = " ".join((args.title or "").split())
    if not title:
        raise SystemExit("session start needs a title")
    try:
        row = projects_mod.start_session(home, title, args.id)
    except ValueError as exc:
        raise SystemExit(str(exc))
    print(row["id"])
    print(f"export TIRITH_SESSION={row['id']}")
    return 0


def _cmd_session_end(args) -> int:
    home = _home(args)
    row = projects_mod.end_session(home, args.session_id)
    if row is None:
        print(f"unknown session: {args.session_id}" if args.session_id
              else "no current session", file=sys.stderr)
        return 1
    print(f"session {row['id']}: ended")
    return 0


def _cmd_session_list(args) -> int:
    home = _home(args)
    rows = projects_mod.read_sessions(home)
    if not rows:
        print("no sessions; try: tirith session start \"<title>\"")
        return 0
    current = projects_mod.current_session(home)
    for row in sorted(rows, key=lambda r: (str(r.get("started_at") or ""), str(r["id"])),
                      reverse=True):
        mark = "*" if row["id"] == current else " "
        print(f"{mark} {row['id']}  {row.get('started_at') or '-'}  "
              f"{'ended ' + row['ended_at'] if row.get('ended_at') else 'open'}  "
              f"{row.get('title') or ''}")
    return 0


def _cmd_steward(args) -> int:
    home = _home(args)
    write_json_atomic(home / "orchestrator.json",
                      {"activity": args.activity, "since": now_iso()})
    print("steward updated")
    return 0


def _cmd_map(args) -> int:
    home = _home(args)
    config, proj = _project_or_exit(home, args.project)
    collector = Collector(home)
    target = home / "brain" / f"architecture-{proj.id}.json"
    if collector.worker.enabled and proj.repo is not None:
        tree_text, _count = collect_tree(proj.repo)
        if tree_text.strip():
            from tirith import brain as brain_mod
            try:
                result = brain_mod.job_architecture(collector.worker.client, tree_text)
            except Exception as exc:
                print(f"brain map failed ({exc}); using folder map", file=sys.stderr)
                result = {}
            if result.get("nodes"):
                arch = {**result, "generated_at": now_iso(), "source": "muse"}
                write_json_atomic(target, arch)
                collector.cache.put("architecture", tree_text, result)
                print(f"{proj.id}: {len(arch['nodes'])} nodes (brain) -> {target}")
                return 0
    arch = heuristic_architecture(proj.repo)
    write_json_atomic(target, arch)
    print(f"{proj.id}: {len(arch['nodes'])} nodes (folder map) -> {target}")
    return 0


def _cmd_collect(args) -> int:
    home = _home(args)
    collector = Collector(home)
    try:
        collector.run(interval=args.interval, once=args.once)
    except KeyboardInterrupt:
        return 130
    if args.once:
        print(home / "state.json")
    return 0


def _cmd_status(args) -> int:
    home = _home(args)
    state = read_json(home / "state.json", None)
    if not isinstance(state, dict):
        print(f"no state yet at {home / 'state.json'}; run: tirith collect --once",
              file=sys.stderr)
        return 1
    brain = state.get("brain") or {}
    steward = state.get("steward") or {}
    print(f"Tirith state {state.get('generated_at')}  brain: "
          f"{'on' if brain.get('enabled') else 'off'} ({brain.get('model')})"
          + (f"  last error: {brain['last_error']}" if brain.get("last_error") else ""))
    print(f"Steward: {steward.get('activity') or '(idle)'}")
    waits = state.get("waiting_on_you") or []
    print(f"Waiting on you: {len(waits)}")
    for wait in waits:
        print(f"  [{wait.get('project')}] {wait.get('text')}")
    print("Agents:")
    for agent in state.get("agents") or []:
        print(f"  {agent['name']} {agent['epithet']}, {agent['rank']} {agent['class']} "
              f"[{agent['status']}] {agent['feature']} -- {agent['summary']}")
    print("Pipelines:")
    for pipe in state.get("pipelines") or []:
        stages = " ".join(f"{s['name']}:{s['status']}" for s in pipe.get("stages") or [])
        print(f"  {pipe['run']} [{pipe['status']}] {stages}")
    print("Projects:")
    for proj in state.get("projects") or []:
        guard = proj.get("guard") or {}
        print(f"  {proj['id']} guard: {'OK' if guard.get('ok') else 'NOT OK'}"
              + ("" if guard.get("ok") else f" ({'; '.join(guard.get('problems') or [])})"))
    return 0


def _guard_targets(args, home: Path) -> list[tuple[str, Path, list[str]]]:
    config = load_config(home)
    defaults = default_words()
    targets: list[tuple[str, Path, list[str]]] = []
    if args.all or (not args.project and config.projects):
        for proj in config.projects:
            if proj.repo is not None:
                targets.append((proj.id, proj.repo, config.words_for(proj, defaults)))
        return targets
    if not args.project:
        raise SystemExit("guard needs a project id, a repo path, or --all")
    proj = config.project(args.project)
    if proj is not None:
        if proj.repo is None:
            raise SystemExit(f"project '{proj.id}' has no repo in config.toml")
        return [(proj.id, proj.repo, config.words_for(proj, defaults))]
    candidate = Path(args.project).expanduser()
    if candidate.exists():
        return [(str(candidate), candidate.resolve(), config.words_for(None, defaults))]
    raise SystemExit(f"unknown project or path: {args.project}")


def _cmd_guard(args) -> int:
    home = _home(args)
    targets = _guard_targets(args, home)
    if not targets:
        print("no configured projects with a repo", file=sys.stderr)
        return 1
    exit_code = 0
    for label, repo, words in targets:
        problems = install_guard(repo, words) if args.action == "install" \
            else verify_guard(repo, words)
        if problems:
            exit_code = 1
            print(f"{label}: guard NOT OK")
            for problem in problems:
                print(f"  {problem}")
        else:
            print(f"{label}: guard OK")
            for line in guard_upgrade_steps(repo, words):
                print(f"  {line}")
    return exit_code


def _cmd_check(args) -> int:
    home = _home(args)
    config = load_config(home)
    if args.folder or (getattr(args, "ci", False) and not args.project):
        folder = projects_mod.real(args.folder or ".")
        proj = next((p for p in config.projects if p.repo and projects_mod.real(p.repo) == folder), None)
    else:
        pid, found = _project_folder_of(home, args.project)
        if found is None:
            raise SystemExit(f"unknown project '{pid}'" if args.project
                             else "tirith check: cwd is in no known project; pass a folder or --project")
        folder, proj = found, config.project(pid)
    words = config.words_for(proj, default_words())
    problems = manifest_mod.check(folder, words, ci=bool(getattr(args, "ci", False)))
    for problem in problems:
        print(problem)
    if not problems:
        print(f"{folder}: all checks passed")
    return 1 if problems else 0


def launch_step(home: Path, project_id: str, step_id: str, agent: str | None = None,
                dry_run: bool = False, out=None) -> str:
    """Start a sub-agent lane for one open roadmap step; returns the lane id.

    Raises ``launch_mod.LaunchError`` with the reason when it must not start.
    """
    out = out or sys.stdout
    config, proj = _project_or_exit(home, project_id)
    repo = projects_mod.real(proj.repo) if proj.repo else None
    if repo is None:
        raise launch_mod.LaunchError(f"project '{project_id}' has no repo folder")
    step, intro = launch_mod.find_step(repo, step_id)
    title = step["title"]
    open_items = [i["text"] for i in step["items"] if not i["done"]]
    if not open_items:
        raise launch_mod.LaunchError(f"step {step['id']} ({title}) is already done" if step["items"]
                                     else f"step {step['id']} ({title}) has no items to finish")
    busy = launch_mod.working_lane(read_lanes(home), project_id, step["id"], lane_status)
    if busy:
        raise launch_mod.LaunchError(f"step {step['id']} already has a working lane: {busy['id']}")
    failed = manifest_mod.check(repo, config.words_for(proj, default_words()))
    if failed:
        raise launch_mod.LaunchError(
            f"tirith check fails for '{project_id}', so no lane may start. Fix these, then run "
            f"`tirith check --project {project_id}`:\n" + "\n".join(f"  {p}" for p in failed))
    lane_id = launch_mod.lane_id_for(title, {str(l["id"]) for l in read_lanes(home)}, repo)
    brief_path = home / "briefs" / f"{lane_id}.md"
    brief = launch_mod.build_brief(title, intro, open_items, step["id"])
    workspace = launch_mod.worktree_path(repo, lane_id)
    command = launch_mod.agent_command(config, agent, brief_path, workspace)
    argv = launch_mod.lane_run_argv(project_id, step["id"], launch_mod.pick_class(title), title,
                                    brief_path, workspace, lane_id, command)
    if dry_run:
        print(f"lane {lane_id} (dry run)\nworktree: {workspace} (branch lane/{lane_id})\n"
              f"brief: {brief_path}\ncommand: {' '.join(argv)}\n\n{brief}", file=out)
        return lane_id
    launch_mod.create_worktree(repo, lane_id)
    brief_path.parent.mkdir(parents=True, exist_ok=True)
    brief_path.write_text(brief, encoding="utf-8")
    launch_mod.spawn_detached(argv, home, home / "logs" / f"{lane_id}.launch.log")
    print(f"lane {lane_id}", file=out)
    return lane_id


def _cmd_launch(args) -> int:
    home = _home(args)
    try:
        launch_step(home, args.project, str(args.step), args.agent, args.dry_run)
    except launch_mod.LaunchError as error:
        print(f"tirith launch: {error}", file=sys.stderr)
        return 3
    return 0


def _cmd_url_open(args) -> int:
    home = _home(args)
    try:
        link = launch_mod.parse_url(args.url)
        if link["action"] not in ("launch", "inbox", "review"):
            raise launch_mod.LaunchError(f"unknown action '{link['action']}' in {args.url!r}")
        if not launch_mod.token_ok(home, link["token"]):
            gates_mod.record_violation(
                home, gates_mod.Violation(
                    "launch-token", "a tirith:// launch link had a wrong or missing token",
                    "open the link from your own Tirith page; it carries the token of "
                    + str(home / launch_mod.LAUNCH_FILE)),
                ts=now_iso(), agent=None, lane=None, session=None,
                project=link["project"] or None, tool=None, cwd=None)
            raise launch_mod.LaunchError("refusing: wrong or missing token")
        if link["action"] == "inbox":
            fields = inbox_mod.parse_url(args.url)
            inbox_mod.add_request(home, fields["kind"], fields["project"], fields["step"], fields["text"])
            return 0
        if link["action"] == "review":
            lane_id = link["lane"]
            review_mod.check_ready(home, lane_id, _lane_repo(home, lane_id))
            launch_mod.spawn_detached([*launch_mod.tirith_argv(), "lane", "review", lane_id], home,
                                      home / "logs" / f"{lane_id}.review.log")
            print(f"reviewing {lane_id}")
            return 0
        launch_step(home, link["project"], link["step"], link["agent"])
    except (launch_mod.LaunchError, inbox_mod.InboxError, review_mod.ReviewError) as error:
        print(f"tirith url open: {error}", file=sys.stderr)
        return 3
    return 0


def _cmd_inbox(args) -> int:
    home = _home(args)
    action = args.inbox_command
    try:
        if action == "add":
            text = args.text if args.text is not None else ""
            print(inbox_mod.add_request(home, args.kind, args.project, str(args.step), text)["id"])
        elif action == "list":
            for req in inbox_mod.requests(home):
                if args.open and req["status"] != "open":
                    continue
                print(json.dumps(req) if args.json else
                      f"{req['id']}  {req['status']:<8} {req['kind']:<6} {req['project']}#{req['step']}  "
                      + " ".join(req["text"].split())[:80])
        elif action == "answer":
            inbox_mod.add_answer(home, args.id, args.text, by=args.by)
        elif action == "done":
            inbox_mod.mark_done(home, args.id)
        elif action == "watch":
            def emit(req: dict) -> None:
                print(inbox_mod.watch_line(req), flush=True)
            try:
                inbox_mod.watch(home, emit, poll_s=args.interval, include_existing=not args.new_only)
            except KeyboardInterrupt:
                pass
    except inbox_mod.InboxError as error:
        print(f"tirith inbox {action}: {error}", file=sys.stderr)
        return 3
    return 0


def _cmd_url(args) -> int:
    action = args.url_command
    base = Path(args.apps_dir).expanduser() if args.apps_dir else None
    try:
        if action == "install":
            app = launch_mod.install_handler(base, home_env=os.environ.get("TIRITH_HOME"))
            print(f"installed {app}")
        elif action == "uninstall":
            print("removed" if launch_mod.uninstall_handler(base) else "was not installed")
        else:
            problems = launch_mod.verify_handler(base)
            for problem in problems:
                print(problem)
            if not problems:
                print(f"{launch_mod.app_path(base)}: handler OK")
            return 1 if problems else 0
    except launch_mod.LaunchError as error:
        print(f"tirith url {action}: {error}", file=sys.stderr)
        return 3
    return 0


def _transport(args):
    """The room transport; tests replace this to avoid the network."""
    return rooms_mod.HttpTransport()


def _room_id(args) -> str:
    room_id = args.room.strip()
    if not rooms_mod.valid_room_id(room_id):
        raise SystemExit(f"invalid room id: {room_id!r} (letters, digits, . _ - only)")
    return room_id


def _cmd_room_add(args) -> int:
    home = _home(args)
    room_id = _room_id(args)
    aliases: dict[str, str] = {}
    for item in args.alias or []:
        name, sep, lane = item.partition("=")
        if not sep or not name.strip() or not lane.strip():
            raise SystemExit(f"--alias needs NAME=LANE-ID, got {item!r}")
        aliases[name.strip()] = lane.strip()
    if rooms_mod.add_room_to_config(home, room_id, args.project, aliases):
        print(f"watching {rooms_mod.room_url(room_id)}")
    else:
        print(f"{room_id} is already in config.toml")
    return 0


def _cmd_room_remove(args) -> int:
    home = _home(args)
    room_id = _room_id(args)
    if rooms_mod.remove_room_from_config(home, room_id):
        print(f"no longer watching {room_id} (cached messages kept in "
              f"{rooms_mod.room_cache_path(home, room_id)})")
        return 0
    print(f"{room_id} is not in config.toml", file=sys.stderr)
    return 1


def _cmd_room_list(args) -> int:
    home = _home(args)
    config = load_config(home)
    if not config.rooms:
        print("no rooms configured; try: tirith room add <room>")
        return 0
    for room in config.rooms:
        cache = rooms_mod.read_room_cache(home, room.id)
        last = (cache.get("messages") or [{}])[-1].get("at") if cache.get("messages") else None
        extras = []
        if room.project:
            extras.append(f"project={room.project}")
        if room.aliases:
            extras.append("aliases=" + ",".join(f"{k}->{v}" for k, v in room.aliases.items()))
        print(f"{room.id}  messages={cache.get('count', 0)}  last={last or '-'}  "
              f"polled={cache.get('polled_at') or 'never'}"
              + (f"  error={cache['error']}" if cache.get("error") else "")
              + ("  " + " ".join(extras) if extras else ""))
    return 0


def _cmd_room_tail(args) -> int:
    home = _home(args)
    room_id = _room_id(args)
    config = load_config(home)
    if config.room(room_id) is not None:
        poller = rooms_mod.RoomPoller(home, _transport(args))
        if not poller.poll_room(room_id):
            print("could not reach the room; showing cached messages", file=sys.stderr)
        messages = rooms_mod.read_room_cache(home, room_id).get("messages") or []
    else:
        try:
            data = _transport(args).get(room_id, None)
        except rooms_mod.RoomError as exc:
            print(str(exc), file=sys.stderr)
            return 1
        messages = [m for m in map(rooms_mod.normalize_message, data.get("messages") or []) if m]
    for message in messages[-max(1, args.lines):]:
        reply = " re " + ",".join(f"#{n}" for n in message["re"]) if message["re"] else ""
        agreed = f"  [agreed: {', '.join(message['agreed'])}]" if message["agreed"] else ""
        text = " ".join(message["text"].split())
        print(f"#{message['n']} {message['at'] or '?'} {message['from']}{reply}: "
              f"{text[:400]}{agreed}")
    return 0


def _cmd_room_say(args) -> int:
    """The only command that writes to a room (an explicit POST by the owner)."""
    room_id = _room_id(args)
    name = rooms_mod.clean_name(args.as_name)
    text = args.text.strip()
    if not name:
        raise SystemExit("--as needs a name (letters, digits, . _ -)")
    if not text:
        raise SystemExit("nothing to say: the text is empty")
    if len(text) > rooms_mod.MAX_SAY:
        raise SystemExit(f"text too long ({len(text)} > {rooms_mod.MAX_SAY} characters)")
    payload: dict = {"from": name, "text": text}
    if args.re:
        payload["re"] = sorted(set(args.re))
    try:
        answer = _transport(args).post(room_id, payload)
    except rooms_mod.RoomError as exc:
        print(str(exc), file=sys.stderr)
        return 1
    posted = answer.get("message") if isinstance(answer.get("message"), dict) else answer
    number = posted.get("n")
    print(f"said in {room_id} as {name}" + (f" (#{number})" if number else ""))
    return 0


def _cmd_hook(args) -> int:
    if args.action in ("install", "uninstall"):
        try:
            fn = hooks_mod.install if args.action == "install" else hooks_mod.uninstall
            kwargs = {"tirith": args.tirith} if args.action == "install" else {}
            kwargs["agents"] = load_config(_home(args)).agents
            path = fn(args.agent, Path(args.settings).expanduser() if args.settings else None,
                      **kwargs)
        except (OSError, ValueError) as err:
            raise SystemExit(f"tirith hook {args.action}: {err}")
        print(f"{args.action}ed tirith hooks for {args.agent}: {path}")
        return 0
    try:  # a hook call must never break the agent
        home = tirith_home(getattr(args, "home", None))
        return hooks_mod.run_hook(home, args.agent, args.event, sys.stdin)
    except BaseException:
        return 0


def _cmd_shim(args) -> int:
    config = load_config(_home(args))
    shim_dir = Path(args.dir).expanduser() if args.dir else Path.home() / ".local" / "bin"
    if not any(str(t.get("command") or "").strip() for t in config.agents.values()):
        raise SystemExit("tirith shim: no [agents.<alias>] command in config.toml; add "
                         "[agents.<alias>] command = \"<executable name>\"")
    if args.action == "verify":
        problems = gates_mod.verify_shims(config, shim_dir)
        for problem in problems:
            print(problem)
        if not problems:
            print(f"shims in {shim_dir}: all intact")
        return 1 if problems else 0
    rows = (gates_mod.install_shims if args.action == "install"
            else lambda c, d: gates_mod.uninstall_shims(c, d))(config, shim_dir)
    for alias, status, detail in rows:
        print(f"{alias}: {status} {detail}")
    return 1 if any(status == "skipped" for _, status, _ in rows) else 0


def build_parser() -> argparse.ArgumentParser:
    """The full ``tirith`` argument parser."""
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--home", default=argparse.SUPPRESS,
                        help="TIRITH_HOME override (default: env or ~/.tirith)")
    parser = argparse.ArgumentParser(prog="tirith", parents=[common],
                                     description="Tirith: watch your coding agents.")
    sub = parser.add_subparsers(dest="command", required=True)

    # NOTE: parent options such as --home must come before the subcommand
    # when given on the top level; they are also accepted per subcommand.
    lane = sub.add_parser("lane", parents=[common], help="agent lanes")
    lane_sub = lane.add_subparsers(dest="lane_command", required=True)
    start = lane_sub.add_parser("start", parents=[common], help="register a lane")
    start.add_argument("--project", required=True)
    start.add_argument("--class", dest="hero_class", required=True)
    start.add_argument("--feature", required=True)
    start.add_argument("--id", default=None)
    start.add_argument("--workspace", default=None)
    start.add_argument("--log", default=None)
    start.add_argument("--pid", type=int, default=None)
    start.add_argument("--brief", default=None)
    start.add_argument("--step", default=None, help="roadmap step this lane works on")
    start.set_defaults(func=_cmd_lane_start)
    end = lane_sub.add_parser("end", parents=[common], help="finish a lane")
    end.add_argument("lane_id")
    end.add_argument("--result", choices=["done", "failed"], required=True)
    end.add_argument("--report", default=None)
    end.add_argument("--report-file", default=None)
    end.set_defaults(func=_cmd_lane_end)
    review = lane_sub.add_parser("review", parents=[common],
                                 help="run the tests of a launched lane, then merge it")
    review.add_argument("lane_id")
    review.add_argument("--tests-only", action="store_true", help="run the tests, do not merge")
    review.set_defaults(func=_cmd_lane_review)
    lane_run = lane_sub.add_parser("run", parents=[common],
                                   help="start an agent command as a lane")
    lane_run.add_argument("--project", required=True)
    lane_run.add_argument("--class", dest="hero_class", required=True)
    lane_run.add_argument("--feature", required=True)
    lane_run.add_argument("--id", default=None)
    lane_run.add_argument("--workspace", default=None)
    lane_run.add_argument("--log", default=None)
    lane_run.add_argument("--brief", default=None)
    lane_run.add_argument("--step", default=None, help="roadmap step this lane works on")
    lane_run.add_argument("--interactive", action="store_true",
                          help="give the agent the terminal (no log capture); activity comes from hooks")
    lane_run.add_argument("--agent", default=None, help="agent alias from [agents.<alias>]")
    lane_run.add_argument("--model", default=None)
    lane_run.add_argument("--effort", default=None)
    lane_run.add_argument("--prompt", default=None)
    lane_run.add_argument("--prompt-file", default=None)
    lane_run.add_argument("command", nargs=argparse.REMAINDER)
    lane_run.set_defaults(func=_cmd_lane_run)

    run = sub.add_parser("run", parents=[common], help="run one pipeline stage")
    run.add_argument("--project", default=None, help="default: the project containing the cwd")
    run.add_argument("--pipeline", default=None, help="default: from tirith.toml by stage name")
    run.add_argument("--stage", required=True)
    run.add_argument("--run", dest="run", default=None, help="explicit run id")
    run.add_argument("--new-run", action="store_true", help="start a new run number")
    run.add_argument("--log", default=None)
    run.add_argument("--agent", default=None, help="lane id to credit renown to")
    run.add_argument("command", nargs=argparse.REMAINDER)
    run.set_defaults(func=_cmd_run)

    pipe = sub.add_parser("pipeline", parents=[common], help="pipeline declarations")
    pipe_sub = pipe.add_subparsers(dest="pipeline_command", required=True)
    declare = pipe_sub.add_parser("declare", parents=[common], help="fix stage order")
    declare.add_argument("pipeline")
    declare.add_argument("--project", default=None)
    declare.add_argument("--stages", required=True, help="comma-separated stage names")
    declare.set_defaults(func=_cmd_pipeline_declare)

    note = sub.add_parser("note", parents=[common], help="record a steward note")
    note.add_argument("text")
    note.add_argument("--project", default=None)
    note.set_defaults(func=_cmd_note)

    wait = sub.add_parser("wait", parents=[common], help="things waiting on you")
    wait_sub = wait.add_subparsers(dest="wait_command", required=True)
    wait_add = wait_sub.add_parser("add", parents=[common])
    wait_add.add_argument("text")
    wait_add.add_argument("--project", default=None)
    wait_add.add_argument("--link", default=None)
    wait_add.add_argument("--id", default=None)
    wait_add.set_defaults(func=_cmd_wait_add)
    wait_done = wait_sub.add_parser("done", parents=[common])
    wait_done.add_argument("wait_id")
    wait_done.set_defaults(func=_cmd_wait_done)

    flow = sub.add_parser("flow", parents=[common],
                          help="record information passed between endpoints")
    flow.add_argument("--from", dest="src", required=True)
    flow.add_argument("--to", dest="dst", required=True)
    flow.add_argument("--kind", choices=flows_mod.KINDS, default="message")
    flow.add_argument("--text", required=True)
    flow.add_argument("--ref", default=None)
    flow.set_defaults(func=_cmd_flow)

    decide = sub.add_parser("decide", parents=[common],
                            help="record a decision and why (shown in the Flow view)")
    decide.add_argument("decision")
    decide.add_argument("--why", required=True)
    decide.add_argument("--lane", default=None, help="lane id (default: $TIRITH_LANE)")
    decide.set_defaults(func=_cmd_decide)

    project = sub.add_parser("project", parents=[common], help="projects are folders")
    project_sub = project.add_subparsers(dest="project_command", required=True)
    p_add = project_sub.add_parser("add", parents=[common], help="register a folder")
    p_add.add_argument("folder")
    p_add.add_argument("--name", default=None)
    p_add.add_argument("--color", default=None)
    p_add.add_argument("--guard-words", default=None, help="comma-separated")
    p_add.set_defaults(func=_cmd_project_add)
    p_rm = project_sub.add_parser("remove", parents=[common], help="unregister a project")
    p_rm.add_argument("project_id")
    p_rm.set_defaults(func=_cmd_project_remove)
    p_ls = project_sub.add_parser("list", parents=[common], help="registered and auto projects")
    p_ls.set_defaults(func=_cmd_project_list)

    session = sub.add_parser("session", parents=[common], help="orchestration sessions")
    session_sub = session.add_subparsers(dest="session_command", required=True)
    s_start = session_sub.add_parser("start", parents=[common], help="start and select a session")
    s_start.add_argument("title")
    s_start.add_argument("--id", default=None)
    s_start.set_defaults(func=_cmd_session_start)
    s_end = session_sub.add_parser("end", parents=[common], help="close a session")
    s_end.add_argument("session_id", nargs="?", default=None)
    s_end.set_defaults(func=_cmd_session_end)
    s_ls = session_sub.add_parser("list", parents=[common], help="all sessions")
    s_ls.set_defaults(func=_cmd_session_list)

    steward = sub.add_parser("steward", parents=[common], help="set steward activity")
    steward.add_argument("activity")
    steward.set_defaults(func=_cmd_steward)

    map_cmd = sub.add_parser("map", parents=[common], help="map project architecture")
    map_cmd.add_argument("project")
    map_cmd.set_defaults(func=_cmd_map)

    collect = sub.add_parser("collect", parents=[common], help="run the collector")
    collect.add_argument("--once", action="store_true")
    collect.add_argument("--interval", type=float, default=2.0)
    collect.set_defaults(func=_cmd_collect)

    status = sub.add_parser("status", parents=[common], help="summarize state.json")
    status.set_defaults(func=_cmd_status)

    room = sub.add_parser("room", parents=[common], help="agentsmessenger.com rooms")
    room_sub = room.add_subparsers(dest="room_command", required=True)
    room_add = room_sub.add_parser("add", parents=[common], help="watch a room")
    room_add.add_argument("room")
    room_add.add_argument("--project", default=None)
    room_add.add_argument("--alias", action="append", metavar="NAME=LANE-ID",
                          help="room name that speaks for a Tirith lane (repeatable)")
    room_add.set_defaults(func=_cmd_room_add)
    room_rm = room_sub.add_parser("remove", parents=[common], help="stop watching a room")
    room_rm.add_argument("room")
    room_rm.set_defaults(func=_cmd_room_remove)
    room_ls = room_sub.add_parser("list", parents=[common], help="configured rooms")
    room_ls.set_defaults(func=_cmd_room_list)
    room_tail = room_sub.add_parser("tail", parents=[common], help="latest messages (GET)")
    room_tail.add_argument("room")
    room_tail.add_argument("-n", "--lines", type=int, default=20)
    room_tail.set_defaults(func=_cmd_room_tail)
    room_say = room_sub.add_parser("say", parents=[common],
                                   help="post a message (the only command that writes)")
    room_say.add_argument("room")
    room_say.add_argument("--as", dest="as_name", required=True)
    room_say.add_argument("--re", type=int, action="append", metavar="N")
    room_say.add_argument("text")
    room_say.set_defaults(func=_cmd_room_say)

    doctor = sub.add_parser("doctor", parents=[common], help="check that an agent can run in a lane")
    doctor.add_argument("--agent", required=True, help="agent alias from [agents.<alias>]")
    doctor.set_defaults(func=_cmd_doctor)

    hook = sub.add_parser("hook", parents=[common],
                          help="record an agent hook call (stdin JSON) / install hooks")
    hook.add_argument("action", nargs="?", choices=["install", "uninstall"], default=None)
    hook.add_argument("--agent", required=True, help="agent alias: anthropic, muse, or any other name")
    hook.add_argument("--event", default=None, help="event name if the payload has none")
    hook.add_argument("--settings", default=None, help="agent settings file (install/uninstall)")
    hook.add_argument("--tirith", default="tirith", help="tirith executable to write on install")
    hook.set_defaults(func=_cmd_hook)

    guard = sub.add_parser("guard", parents=[common], help="commit guard")
    guard.add_argument("action", choices=["install", "verify"])
    guard.add_argument("project", nargs="?", default=None,
                       help="project id or repo path")
    guard.add_argument("--all", action="store_true")
    guard.set_defaults(func=_cmd_guard)

    shim = sub.add_parser("shim", parents=[common],
                          help="agent CLI shims: start only through tirith lane run")
    shim.add_argument("action", choices=["install", "uninstall", "verify"])
    shim.add_argument("--dir", default=None, help="shim folder (default: ~/.local/bin)")
    shim.set_defaults(func=_cmd_shim)

    check = sub.add_parser("check", parents=[common],
                           help="check a project is set up (tirith.toml, pipelines, hooks, guard)")
    check.add_argument("folder", nargs="?", default=None)
    check.add_argument("--project", default=None, help="project id")
    check.add_argument("--ci", action="store_true",
                       help="running in CI, not a local clone with hooks: skip the commit-guard rule")
    check.set_defaults(func=_cmd_check)

    launch = sub.add_parser("launch", parents=[common],
                            help="start a sub-agent to finish one open step of the plan")
    launch.add_argument("--project", required=True)
    launch.add_argument("--step", required=True, help="roadmap step number")
    launch.add_argument("--agent", default=None, help="agent alias (its [agents.<alias>] launch template)")
    launch.add_argument("--dry-run", action="store_true", help="print the brief and command, start nothing")
    launch.set_defaults(func=_cmd_launch)

    inbox = sub.add_parser("inbox", parents=[common], help="the owner's requests on plan steps")
    inbox_sub = inbox.add_subparsers(dest="inbox_command", required=True)
    i_add = inbox_sub.add_parser("add", parents=[common], help="add a request (prints its id)")
    i_add.add_argument("--kind", required=True, choices=inbox_mod.KINDS)
    i_add.add_argument("--project", required=True)
    i_add.add_argument("--step", required=True, help="roadmap step number")
    i_add.add_argument("--text", default=None, help="the question (required for ask)")
    i_ls = inbox_sub.add_parser("list", parents=[common], help="list requests")
    i_ls.add_argument("--open", action="store_true", help="only open requests")
    i_ls.add_argument("--json", action="store_true", help="one JSON object per line")
    i_ans = inbox_sub.add_parser("answer", parents=[common], help="answer a request (markdown)")
    i_ans.add_argument("id")
    i_ans.add_argument("text")
    i_ans.add_argument("--by", choices=inbox_mod.ANSWERERS, default="steward")
    i_done = inbox_sub.add_parser("done", parents=[common], help="close a request")
    i_done.add_argument("id")
    i_watch = inbox_sub.add_parser("watch", parents=[common],
                                   help="block; print each open request as one JSON line, then each new one")
    i_watch.add_argument("--new-only", action="store_true", help="skip the requests already in the file")
    i_watch.add_argument("--interval", type=float, default=1.0, help="seconds between reads")
    for i_cmd in (i_add, i_ls, i_ans, i_done, i_watch):
        i_cmd.set_defaults(func=_cmd_inbox)

    url = sub.add_parser("url", parents=[common], help="the tirith:// link handler (macOS)")
    url_sub = url.add_subparsers(dest="url_command", required=True)
    for name, text in (("install", "build and register the handler app"),
                       ("uninstall", "remove the handler app"),
                       ("verify", "check the handler app")):
        url_cmd = url_sub.add_parser(name, parents=[common], help=text)
        url_cmd.add_argument("--apps-dir", default=None, help="folder for the app (default ~/Applications)")
        url_cmd.set_defaults(func=_cmd_url)
    url_open = url_sub.add_parser("open", parents=[common], help="handle one tirith:// link")
    url_open.add_argument("url")
    url_open.set_defaults(func=_cmd_url_open)
    return parser


def main(argv: list[str] | None = None) -> int:
    """Console entry point; returns the process exit code."""
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return int(args.func(args))
    except BrokenPipeError:
        return 1


if __name__ == "__main__":
    sys.exit(main())
