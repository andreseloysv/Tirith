"""The ``tirith`` command line: lanes, pipelines, notes, waits, guard, brain.

Every subcommand only touches files under TIRITH_HOME (plus, for
``lane run``/``run``, the child command it wraps and, for ``guard install``,
the hooks folder of the repo the owner named). No command opens a port.
"""

from __future__ import annotations

import argparse
import json
from urllib.parse import parse_qs, urlsplit
import os
import re
import secrets
import shlex
import signal
import subprocess
import sys
import time
from datetime import date, datetime, timezone
from pathlib import Path

from tirith import timeouts as timeouts_mod
from tirith.collector import (
    Collector, collect_tree, git_head, git_shortstat, heuristic_architecture, lane_status,
)
from tirith.guardwrap import (
    default_words, guard_upgrade_steps, install_guard, reinstall_guard, verify_guard,
)
from tirith import actions as actions_mod
from tirith import agentrun as agentrun_mod
from tirith import asks as asks_mod
from tirith import fallback as fallback_mod
from tirith import filerefs as filerefs_mod
from tirith import flows as flows_mod
from tirith import gates as gates_mod
from tirith import manifest as manifest_mod
from tirith import hooks as hooks_mod
from tirith import brain as brain_mod
from tirith import collectlock as collectlock_mod
from tirith import selfreload as selfreload_mod
from tirith import inbox as inbox_mod
from tirith import setup as setup_mod
from tirith import keepgoing as keepgoing_mod
from tirith import lanestop as lanestop_mod
from tirith import managerd as managerd_mod
from tirith import launch as launch_mod
from tirith import models as models_mod
from tirith import howto as howto_mod
from tirith import ownergate as ownergate_mod
from tirith import planlane as planlane_mod
from tirith import plangate as plangate_mod
from tirith import verifylane as verifylane_mod
from tirith import roadmap as roadmap_mod
from tirith import review as review_mod
from tirith import roadmap as roadmap_mod
from tirith import runstage as runstage_mod
from tirith import projects as projects_mod
from tirith import projenv as projenv_mod
from tirith import rooms as rooms_mod
from tirith.home import ensure_home, load_config, now_iso, parse_iso, tirith_home
from tirith.store import (
    append_jsonl, final_answer, lane_path, read_json, read_jsonl, read_lane, read_lanes, write_json_atomic,
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
REPO_BIN = gates_mod.REPO_BIN


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


def lane_request(home: Path, brief: str | None) -> dict | None:
    """The request a lane answers: the brief it was given (its own task), else the owner's latest request."""
    path = Path(brief).expanduser() if brief else None
    try:
        text = path.read_text(encoding="utf-8", errors="replace").strip() if path and path.is_file() else ""
    except OSError:
        text = ""
    if text:
        return {"text": bound_text(text, REQUEST_MAX), "at": now_iso(), "agent_session_id": ""}
    return owner_request(home)


def register_lane(home: Path, *, lane_id: str | None, project: str, hero_class: str,
                  feature: str, workspace: str | None, log: str | None,
                  pid: int | None, brief: str | None, step: str | None = None,
                  request_id: str | None = None, item: str | None = None, item_step: str | None = None,
                  retry_of: str | None = None) -> dict:
    """Write a new lane registration; returns the stored dict."""
    if not lane_id:
        lane_id = _unique_lane_id(home, _slug(feature))
    elif lane_path(home, lane_id).exists():
        raise SystemExit(f"lane already exists: {lane_id}")
    workspace_abs = str(Path(workspace).expanduser().resolve()) if workspace else None
    parent = os.environ.get("TIRITH_LANE") or "manager"
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
        "request_id": str(request_id) if request_id else None,
        "request": lane_request(home, brief),
    }
    if item:  # the plan item this lane works on (``tirith.plangate``)
        lane["item"] = str(item)
        lane["item_step"] = str(item_step or step or "")
    if retry_of:
        lane["retry_of"] = retry_of
    write_lane(home, lane)
    source = "manager" if parent in ("manager", "steward") else f"agent:{parent}"
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
    lane["result"] = result = plangate_mod.lane_ended(home, lane)  # ``no-outcome``: no plan report
    lane.pop("token", None)  # the shims accept the token of a running lane only
    stored = agentrun_mod.store_diff(home, lane)
    if stored:
        lane["diff"] = stored
    if report is not None:
        lane["report"] = bound_text(report, REPORT_MAX)
    write_lane(home, lane)
    flows_mod.append_flow(
        home, f"agent:{lane_id}", "manager", "report",
        f"{result}: " + (flows_mod.first_line(report) or str(lane.get("feature") or "")),
        session=lane.get("session") or projects_mod.current_session(home),
        project=str(lane.get("project") or "") or None)
    _lane_follow_up(home, lane, result, report)
    return lane


def _lane_follow_up(home: Path, lane: dict, result: str, report: str | None) -> None:
    """A finished lane whose report has an "open / needs the owner" section: one review item for the
    manager, never raw items for the owner. The manager re-asks with ``tirith wait add --option``."""
    section = asks_mod.open_section(report) if result == "done" else ""
    if section:
        asks_mod.queue_add(home, str(lane.get("project") or ""), str(lane["id"]), section)


def _log_tail(log: Path | None, lines: int = 12) -> str:
    try:
        text = log.read_text(encoding="utf-8", errors="replace") if log else ""
    except OSError:
        return ""
    return "\n".join(text.strip().splitlines()[-lines:])[-1500:]


def add_lane_ask(home: Path, lane: dict, what: str, log_tail: str = "") -> str:
    """A lane that failed or said nothing is one item in the manager queue (``sq-lane-<id>``) with the log
    tail: retrying or dropping it is never a question for the owner."""
    text = f"Lane **{lane.get('feature') or lane['id']}** ({lane['id']}) {what}."
    if log_tail:
        text += f"\nLog tail:\n```\n{log_tail}\n```"
    asks_mod.queue_add(home, str(lane.get("project") or ""), str(lane["id"]), text,
                       kind="lane-failed", item_id=f"sq-lane-{lane['id']}")
    return f"sq-lane-{lane['id']}"


def _plan_start(home: Path, args, lane_id: str | None) -> tuple[str | None, str]:
    """The plan check of an agent start (``tirith.plangate``): ``(clean item, "")`` or ``(None, refusal text)``.

    The step is ``--item-step`` (Tirith's own work lives in another step than the lane's) else ``--step``."""
    step = getattr(args, "item_step", None) or args.step
    try:
        repo = plangate_mod.repo_of(home, args.project)
        if repo is None:
            raise plangate_mod.PlanGateError(f"project '{args.project}' has no repo folder, so it has no plan")
        return plangate_mod.check_start(home, repo, args.project, step, getattr(args, "item", None), lane_id,
                                        getattr(args, "hero_class", None)), ""
    except plangate_mod.PlanGateError as error:
        return None, f"tirith: plan-item: no agent starts without a plan item: {error}"


def _cmd_lane_start(args) -> int:
    home = _home(args)
    item = None
    if getattr(args, "item", None):  # `lane start` only records an agent that already runs; with an item it is linked
        item, refusal = _plan_start(home, args, args.id)
        if item is None:
            print(refusal, file=sys.stderr)
            return 3
    lane = register_lane(
        home, lane_id=args.id, project=args.project, hero_class=args.hero_class,
        feature=args.feature, workspace=args.workspace, log=args.log,
        pid=args.pid, brief=args.brief, step=args.step,
        request_id=getattr(args, "request", None), item=item,
        item_step=getattr(args, "item_step", None))
    _plan_begin(home, lane)
    print(lane["id"])
    return 0


def _plan_begin(home: Path, lane: dict) -> None:
    """Mark the lane's item "(en curso: <lane>)"; the check already passed, so a plan that cannot be written
    right now (uncommitted edits) only leaves the note out."""
    linked = plangate_mod.lane_item(lane)
    repo = plangate_mod.repo_of(home, str(lane.get("project") or ""))
    if linked is None or repo is None:
        return
    try:
        plangate_mod.begin(home, repo, str(lane["project"]), linked[0], linked[1], str(lane["id"]))
    except plangate_mod.PlanGateError as error:
        print(f"tirith: the plan was not marked: {error}", file=sys.stderr)


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


def _cmd_lane_stop(args) -> int:
    home = _home(args)
    by = args.by or ("manager" if os.environ.get("TIRITH_LANE") or not sys.stdin.isatty() else "owner")
    try:
        lane = lanestop_mod.stop_lane(home, args.lane_id, by, args.reason)
    except lanestop_mod.StopError as error:
        print(f"tirith lane stop: {error}", file=sys.stderr)
        return 3
    print(f"lane {args.lane_id}: {lane.get('result')} ({lane.get('report')})")
    return 0


def _cmd_lane_drop(args) -> int:
    home = _home(args)
    try:
        review_mod.drop_lane(home, args.lane_id, _lane_repo(home, args.lane_id))
    except review_mod.ReviewError as error:
        print(f"tirith lane drop: {error}", file=sys.stderr)
        return 3
    print(f"lane {args.lane_id}: dropped")
    return 0


def _cmd_lane_end(args) -> int:
    home = _home(args)
    report = args.report
    if args.report_file:
        report = Path(args.report_file).expanduser().read_text(encoding="utf-8")
    end_lane(home, args.lane_id, args.result, report)
    print(f"lane {args.lane_id}: {args.result}")
    return 0


def _wrapper_path_env(home: Path | None = None, lane_id: str | None = None,
                      token: str | None = None, project: str | None = None) -> dict:
    """Environment with the owner's git wrapper folder first in PATH (and the lane, if any).

    The project's own variables (``tirith project env``) are laid over the inherited ones."""
    env = projenv_mod.apply(home or tirith_home(), project)
    if lane_id:
        env["TIRITH_LANE"] = lane_id
        env["TIRITH_HOME"] = str(home or tirith_home())  # the shims look the lane file up here
    if token:
        env[gates_mod.TOKEN_ENV] = token
    session = projects_mod.current_session(home or tirith_home())
    if session:
        env["TIRITH_SESSION"] = session
    env["PATH"] = gates_mod.lane_path(home or tirith_home(), env.get("PATH", ""))  # shims, git wrapper, repo
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
    problems = verify_guard(workspace, config.words_for(proj, default_words())) if proj.guard else []
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
    stdin = agentrun_mod.wants_stdin(config, args.agent, args.project, prompt)
    args.stdin_text = prompt if stdin else None  # written to the agent's stdin by _cmd_lane_run
    return agentrun_mod.build_command(config, args.agent, args.project, prompt,
                                      model=args.model, effort=args.effort, step=args.step,
                                      via_stdin=stdin)


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


def in_manager_session(environ=None) -> bool:
    """True inside the manager (the on-call lane or a session with ``TIRITH_ROLE=manager``)."""
    env = os.environ if environ is None else environ
    return (str(env.get("TIRITH_LANE") or "").startswith(managerd_mod.LANE_PREFIX)
            or env.get("TIRITH_ROLE") == "manager")


def _lane_workspace(repo: Path, lane_id: str, explicit: bool) -> Path:
    """A lane never runs in the main checkout: a new worktree ``<repo>-lanes/<lane-id>`` on branch
    ``lane/<lane-id>`` from the current ``main`` (else HEAD); an existing one is reused."""
    path = launch_mod.worktree_path(repo, lane_id)
    if path.exists():
        return path
    try:
        return launch_mod.create_worktree(repo, lane_id, base="main" if launch_mod.branch_exists(repo, "main") else None)
    except launch_mod.LaunchError as error:
        raise SystemExit(f"lane run: {error}. Pass --workspace <folder> to run elsewhere.")


def _detached_argv(args, home: Path, lane_id: str, workspace: Path, command: list[str]) -> list[str]:
    """``tirith lane run`` again with the same arguments, resolved, that runs in the foreground (``--no-detach``)."""
    argv = [*launch_mod.tirith_argv(home), "lane", "run", "--id", lane_id, "--project", args.project,
            "--class", args.hero_class, "--feature", args.feature, "--workspace", str(workspace),
            "--no-detach"]
    for flag, value in (("--log", args.log), ("--brief", args.brief), ("--step", args.step),
                        ("--item", args.item), ("--item-step", args.item_step), ("--retry-of", args.retry_of),
                        ("--request", args.request), ("--agent", args.agent), ("--as-agent", args.as_agent),
                        ("--fallback-from", args.fallback_from), ("--tried", args.tried),
                        ("--model", args.model), ("--effort", args.effort), ("--prompt", args.prompt),
                        ("--prompt-file", args.prompt_file)):
        if value:
            argv += [flag, str(value)]
    if args.by_tirith:
        argv.append("--by-tirith")
    return [*argv, "--", *command]


def _duplicate_lane(home: Path, args, item: str | None) -> dict | None:
    """The working lane of the same project and plan item that ``lane run --step`` would duplicate, else None.
    A lane that continues another (``--fallback-from``: fallback, relaunch, claim re-send) is no duplicate."""
    if not args.step or args.fallback_from:
        return None
    others = [l for l in read_lanes(home) if l.get("id") != args.id]
    return launch_mod.working_lane(others, args.project, str(args.step), lane_status, item)


def _run_child(home: Path, lane: dict, lane_id: str, command: list[str], workspace: Path, env: dict,
               log_path: Path, stdin_text: str | None) -> tuple[int, str]:
    """One run of the lane's agent process: ``(exit code, final answer)``."""
    with log_path.open("ab") as log_handle:
        proc = subprocess.Popen(
            command, cwd=str(workspace), env=env,
            stdin=subprocess.PIPE if stdin_text else subprocess.DEVNULL,
            stdout=log_handle, stderr=subprocess.STDOUT,
            start_new_session=os.name == "posix")  # its own process group: `lane stop` ends all of it
        lane = read_json(lane_path(home, lane_id), None) or lane  # keeps what the agent recorded (outcome)
        lane["pid"] = proc.pid
        if os.name == "posix":
            lane["pgid"] = proc.pid
        write_lane(home, lane)
        if stdin_text:  # a long prompt (the brief) is read from stdin, not from argv
            try:
                proc.stdin.write(stdin_text.encode("utf-8"))
                proc.stdin.close()
            except OSError:  # the agent exited before reading it; its exit code tells
                pass
        watch = timeouts_mod.supervise(proc, "agent-turn", home=home, what=f"lane {lane_id}")
        exit_code = proc.wait()
        ended = watch()
    answer, _ = final_answer(log_path)
    if ended["timed_out"]:
        exit_code, answer = timeouts_mod.EXIT_TIMED_OUT, ended["message"]
    return exit_code, answer


def _resume_child(home: Path, config, args, lane: dict, command: list[str], log_path: Path, answer: str,
                  text: str) -> tuple[list[str], str | None] | None:
    """The command (and stdin text) that sends ``text`` to the lane's agent again, or None."""
    alias = str(args.agent or getattr(args, "as_agent", None) or "")
    try:
        brief = Path(str(lane.get("brief") or "")).read_text(encoding="utf-8", errors="replace")
    except OSError:
        brief = ""
    return keepgoing_mod.resume_command(config, alias, args.project, command,
                                        keepgoing_mod.session_id(log_path), text, answer, brief)


def _cmd_lane_run(args) -> int:
    home = _home(args)
    command = list(args.command or [])
    if command and command[0] == "--":
        command = command[1:]
    raw_command = list(command)
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
    item, refusal = _plan_start(home, args, args.id)
    if item is None:
        print(refusal, file=sys.stderr)
        return 3
    busy = _duplicate_lane(home, args, item)
    if busy:
        print(f"{busy['id']} is already working on item \"{item}\" of step {args.step}", file=sys.stderr)
        return 3
    args.item = item
    interactive = bool(getattr(args, "interactive", False))
    manager_lane = bool(args.id and args.id.startswith(managerd_mod.LANE_PREFIX)
                        and args.hero_class == managerd_mod.HERO_CLASS)
    lane_id = args.id or _unique_lane_id(home, _slug(args.feature))
    if not args.workspace and not manager_lane:
        repo = Path(workspace)
        while not args.id and (launch_mod.worktree_path(repo, lane_id).exists()
                               or launch_mod.branch_exists(repo, f"lane/{lane_id}")):
            lane_id = _unique_lane_id(home, f"{lane_id}-2")
        workspace = _lane_workspace(repo, lane_id, False)
    if (not interactive and not args.no_detach and (args.detach or (
            not args.by_tirith and in_manager_session()))):  # a manager's turn must end right away
        launch_mod.spawn_detached(_detached_argv(args, home, lane_id, Path(workspace), raw_command), home,
                                  home / "logs" / f"{lane_id}.launch.log")
        print(lane_id)
        print(f"detached; worktree: {workspace}")
        return 0
    log_path = None if interactive else (
        Path(args.log).expanduser().resolve() if args.log else home / "logs" / f"{lane_id}.log")
    if log_path is not None:
        log_path.parent.mkdir(parents=True, exist_ok=True)
    lane = register_lane(
        home, lane_id=lane_id, project=args.project, hero_class=args.hero_class,
        feature=args.feature, workspace=str(workspace),
        log=str(log_path) if log_path else None,
        pid=None, brief=args.brief, step=args.step,
        request_id=getattr(args, "request", None), item=item, item_step=args.item_step,
        retry_of=args.retry_of)
    _plan_begin(home, lane)
    lane["token"] = secrets.token_urlsafe(24)
    if interactive:
        lane["interactive"] = True  # the owner's own conversation: no plan report is asked of it
    lane["ran"] = models_mod.command_choice(command)  # the model and effort this lane was started with
    lane["agent"] = args.agent or getattr(args, "as_agent", None) or None  # the alias that runs it
    if getattr(args, "by_tirith", False) or lane_id.startswith(managerd_mod.LANE_PREFIX):
        lane["by_tirith"] = True  # started by Tirith itself: a failure goes to the manager queue
    if getattr(args, "fallback_from", None):
        before = read_json(lane_path(home, args.fallback_from), None) or {}
        lane["fallback_from"] = before.get("agent")
        lane["fallback_lane"] = args.fallback_from
    lane["tried"] = [a for a in str(getattr(args, "tried", "") or "").split(",") if a]
    write_lane(home, lane)
    began = time.monotonic()
    env = _wrapper_path_env(home, lane_id, lane["token"], args.project)
    if args.agent:
        env = agentrun_mod.child_env(config, args.agent, args.project, env)
    if interactive:
        exit_code = _run_interactive(home, lane, command, workspace, env)
        answer = _last_stop_message(home, lane_id, lane.get("started_at"))
    else:
        stdin_text = getattr(args, "stdin_text", None)
        rounds, missing = 0, []
        while True:
            exit_code, answer = _run_child(home, lane, lane_id, command, workspace, env, log_path, stdin_text)
            stopped = lanestop_mod.marker(read_json(lane_path(home, lane_id), None) or {})
            if stopped or exit_code != 0 or not (args.agent or getattr(args, "as_agent", None)):
                break
            missing = keepgoing_mod.gaps(home, lane, proj.repo, answer)
            if not missing or rounds >= keepgoing_mod.MAX_ROUNDS:
                break
            rounds += 1
            keepgoing_mod.note_round(home, lane_id, rounds, missing)
            resumed = _resume_child(home, config, args, lane, command, log_path, answer,
                                    keepgoing_mod.message(missing, rounds))
            if resumed is None:
                break
            command, stdin_text = resumed
        if missing and not stopped and exit_code == 0:
            keepgoing_mod.queue_gaps(home, lane, missing)
    stopped = lanestop_mod.marker(read_json(lane_path(home, lane_id), None) or {})
    if stopped:  # `tirith lane stop`: failed, with who stopped it and why
        exit_code, answer = exit_code or 1, lanestop_mod.report_for(stopped["by"], stopped.get("reason"))
    result = "done" if exit_code == 0 else "failed"
    end_lane(home, lane_id, result, answer or None)
    if stopped:
        pass
    elif result == "failed":
        add_lane_ask(home, lane, "failed", log_tail=_log_tail(log_path))
    elif not answer and not (interactive and manager_lane):  # the manager session has no final answer
        add_lane_ask(home, lane, "ended without an answer")
    files, plus, minus = git_shortstat(str(workspace), lane.get("start_commit"))
    print(f"lane {lane_id}: {result}, {files} files +{plus} -{minus}, "
          f"{_duration(time.monotonic() - began)}")
    over = plangate_mod.budget_note(files, plus, minus)
    if over:
        print(f"tirith: {over}")
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
    key = f"{args.project}/{args.pipeline}" if args.project else f"*/{args.pipeline}"
    with runstage_mod.pipelines_lock(home):
        book = _pipeline_book(home)
        entry = book.get(key) if isinstance(book.get(key), dict) else {}
        entry.update({"stages": stages})
        entry.setdefault("counter", 0)
        book[key] = entry
        write_json_atomic(home / "pipelines.json", book)
    print(f"{args.pipeline}: {','.join(stages)}")
    return 0


def _resolve_run(home: Path, project: str, pipeline: str,
                 explicit: str | None, new_run: bool) -> str:
    return runstage_mod.resolve_run(home, project, pipeline, explicit, new_run)


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
    info = runstage_mod.worktree_info(Path.cwd())
    if info and folder and (Path(info["worktree"]) / manifest_mod.FILENAME).is_file() \
            and projects_mod.canonical(info["worktree"]) == projects_mod.real(folder):
        folder = Path(info["worktree"])  # in a lane's worktree the stages run from the worktree
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
    cwd = getattr(args, "stage_cwd", None) or str(Path.cwd().resolve())
    tags = runstage_mod.lane_tags(home, cwd)
    code, _ = runstage_mod.run_stage(
        home, args.project, args.pipeline, args.stage, command, cwd,
        lane=tags.get("lane") or (os.environ.get("TIRITH_LANE") or None), run_id=args.run,
        worktree=tags.get("worktree"), branch=tags.get("branch"), agent=args.agent,
        log=args.log, new_run=args.new_run)
    return code


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
             wait_id: str | None = None, options=None, command: str | None = None,
             kind: str | None = None, actions: list[dict] | None = None,
             why_owner: str | None = None, why_detail: str | None = None, howto=None) -> str:
    """Add an ask to the owner's list; returns its id (``""`` when the gate kept it from the owner).

    ``why_owner`` (a key of ``asks.WHY_OWNER``) is required by the owner gate: without it the text is
    queued for the manager as "agent can do" and nothing reaches the owner (``tirith.ownergate``).

    ``howto`` is the concrete how-to (``tirith.howto``); one with a placeholder goes back to the manager.

    ``options`` are one-click answers (``{"label", "value"}`` or ``"Label|value"``), ``command`` is a
    shell line the owner can copy, ``kind`` is decision|action|approval|info, ``actions`` are commands
    the owner runs with a button (see ``tirith.actions``)."""
    options = asks_mod.clean_options(options)
    if not options and not any(isinstance(a, dict) and a.get("command") for a in actions or []):
        raise SystemExit(
            "tirith: call-to-action: an item for the owner needs at least one one-click answer or Run action. "
            'Do this instead: add --option "Label|value" (repeatable) or --run "<command>" --label "<what it does>"; '
            "what only you must see goes to your own queue, not to the owner")
    kind = kind or asks_mod.default_kind(options, command)
    if kind not in asks_mod.ASK_KINDS:
        raise SystemExit(f"wait add: --kind must be one of {', '.join(asks_mod.ASK_KINDS)}")
    wait_id = wait_id or _next_wait_id(home)
    verdict = ownergate_mod.admit(home, wait_id, project, text, why_owner, via="wait-add", howto=howto,
                                  why_detail=why_detail)
    if verdict != ownergate_mod.OWNER:
        return ""
    why = asks_mod.why_key(why_owner)
    row = {"id": wait_id, "at": now_iso(), "project": project,
           "text": text, "link": link, "done_at": None,
           "session": projects_mod.current_session(home),
           "why_owner": why, **({"why_detail": why_detail} if why_detail else {}),
           "reason_tag": asks_mod.reason_tag(why, why_detail)}
    if howto_mod.parse(howto):
        row["howto"] = howto_mod.parse(howto)
    if options or command or kind != "info":
        row.update({"kind": kind, "options": options, "command": command or None})
    if actions:
        row["actions"] = actions
    append_jsonl(home / "waits.jsonl", row)
    flows_mod.append_flow(home, "manager", "owner", "request", text, link, project=project)
    return wait_id


OWNER_GATE_NOTE = (
    "tirith: owner-gate: not an owner item, so the owner was not asked. Only these reach the owner: "
    + ", ".join(asks_mod.WHY_OWNER) + " (pass --why-owner KEY). The text went to the manager queue as "
    '"agent can do" (or was dropped when out of plan); an agent does it.')


def _add_why_args(parser) -> None:
    parser.add_argument("--howto", default=None, metavar="JSON|FILE",
                        help='concrete how-to: {"steps":[{"text","url"?,"terminal"?}],"done_check":"<command>",'
                        '"already_done":["fact"]}; no placeholders, every command runnable as it is')
    parser.add_argument("--why-owner", default=None, metavar="KEY",
                        help="why only the owner can do it: " + ", ".join(asks_mod.WHY_OWNER)
                        + " (without it the item goes to the manager queue, not to the owner)")
    parser.add_argument("--why-detail", default=None, metavar="WORDS",
                        help='plain words for the chip, e.g. "Apple account" -> "Needs your Apple account"')


def _cmd_wait_add(args) -> int:
    text = args.text or args.text_opt
    if not text:
        raise SystemExit("wait add needs the question (positional or --text)")
    try:
        options = [asks_mod.parse_option(o) for o in args.option or []]
    except asks_mod.AskError as error:
        raise SystemExit(f"wait add: {error}")
    try:
        actions_mod.check_text(text, "wait")
        runs = list(args.run or []) + ([args.command] if args.command else [])  # never text to copy
        actions = actions_mod.make_actions(runs, args.label, args.cwd)
    except actions_mod.ActionError as error:
        print(error, file=sys.stderr)
        return 2
    wait_id = add_wait(_home(args), args.project, text, args.link, args.id, options=options,
                       command=args.command, kind=args.kind, actions=actions,
                       why_owner=args.why_owner, why_detail=args.why_detail, howto=args.howto)
    return _print_wait(wait_id)


def _print_wait(wait_id: str) -> int:
    if wait_id:
        print(wait_id)
        return 0
    print(OWNER_GATE_NOTE, file=sys.stderr)
    return 0


def _cmd_chat_say(args) -> int:
    """The manager writes to the owner in the chat: an owner item with options or Run actions, else a plain message."""
    text = (args.text or "").strip()
    if not text:
        raise SystemExit("chat say needs the message")
    try:
        options = [asks_mod.parse_option(o) for o in args.option or []]
    except asks_mod.AskError as error:
        raise SystemExit(f"chat say: {error}")
    try:
        actions_mod.check_text(text, "wait")
        actions = actions_mod.make_actions(list(args.run or []), args.label, args.cwd)
    except actions_mod.ActionError as error:
        print(error, file=sys.stderr)
        return 2
    home = _home(args)
    if options or actions:
        return _print_wait(add_wait(home, args.project, text, options=options, actions=actions,
                                    why_owner=args.why_owner, why_detail=args.why_detail))
    try:
        print(inbox_mod.add_say(home, text, args.project or "")["id"])
    except inbox_mod.InboxError as error:
        raise SystemExit(f"chat say: {error}")
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
    flows_mod.append_flow(home, "owner", "manager", "answer",
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
                f"{label} must be manager, owner, guard, agent:<lane>, room:<room> or "
                f"pipeline:<run>, got {value!r}")
    if not args.text.strip():
        raise SystemExit("--text is empty")
    row = flows_mod.append_flow(home, args.src, args.dst, args.kind, args.text, args.ref)
    print(row["id"])
    return 0


def _lane_of_caller(home: Path) -> str | None:
    """The lane the caller works for: ``$TIRITH_LANE`` when it names a lane that has not ended."""
    lane_id = os.environ.get("TIRITH_LANE") or None
    lane = read_lane(home, lane_id) if lane_id else None
    return lane_id if isinstance(lane, dict) and not lane.get("ended_at") else None


def _cmd_timeout(args) -> int:
    """``timeout get|set``: the value in force per command class, and a bounded change with a reason."""
    home = _home(args)
    config = load_config(home)
    if args.timeout_command == "get":
        classes = [args.cls] if args.cls else list(timeouts_mod.POLICY)
        for cls in classes:
            row = timeouts_mod.describe(home, config, cls)
            print(f"{cls}: {row['seconds']:g} s ({row['source']}; allowed {row['min']:g} to {row['max']:g} s)")
        return 0
    lane = _lane_of_caller(home)
    try:
        row = timeouts_mod.set_timeout(home, config, args.cls, args.seconds, args.reason or "", lane=lane)
    except timeouts_mod.TimeoutRefused as error:
        raise SystemExit(f"tirith timeout: {error}")
    flows_mod.append_flow(
        home, f"agent:{lane}" if lane else "owner", "guard", "decision",
        f"timeout of '{args.cls}' set to {args.seconds:g} s", why=row["reason"] or None)
    print(f"{args.cls}: {args.seconds:g} s" + (" until the lane ends" if lane else ""))
    return 0


def _cmd_decide(args) -> int:
    """Record a decision and its reason: agent -> manager, or manager -> owner without a lane."""
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
    src, dst = (f"agent:{lane_id}", "manager") if lane else ("manager", "owner")
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


def _env_project(args, home: Path) -> str:
    project, _ = _project_folder_of(home, args.project)
    if not project:
        raise SystemExit("project env: no project; pass --project <id> (see: tirith project list)")
    _project_or_exit(home, project)
    return project


def _cmd_project_env(args) -> int:
    """``tirith project env set|unset|list --project P``: the variables every run of the project gets."""
    home = _home(args)
    project = _env_project(args, home)
    try:
        if args.env_command == "list":
            env = projenv_mod.load(home, project)
            if args.json:
                print(json.dumps(env, indent=2, sort_keys=True))
            else:
                for key, value in env.items():
                    print(f"{key}={value}")
            return 0
        if args.env_command == "unset":
            projenv_mod.unset_vars(home, project, args.keys)
            print(f"{project}: unset {', '.join(args.keys)}")
            return 0
        if args.replace_stdin:
            try:
                given = json.loads(sys.stdin.read() or "{}")
            except ValueError:
                raise SystemExit("project env set: stdin is not a JSON object")
            if not isinstance(given, dict):
                raise SystemExit("project env set: stdin is not a JSON object")
            projenv_mod.save(home, project, given)
            print(f"{project}: {len(given)} variable(s) saved")
            return 0
        pairs = {}
        for item in args.pairs:
            key, eq, value = item.partition("=")
            if not eq:
                raise SystemExit(f"project env set: '{item}' is not KEY=VALUE")
            pairs[key] = value
        projenv_mod.set_vars(home, project, pairs)
        print(f"{project}: set {', '.join(pairs)}")
    except projenv_mod.EnvError as exc:
        print(f"project env: {exc}", file=sys.stderr)
        return 3
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


MANAGER_VERBS = ("say", "interrupt", "stop", "restart", "status")


def _cmd_manager_session(args, words: list[str]) -> int:
    """``tirith manager say <text> | interrupt | stop | restart [--fresh] | status`` (the persistent session)."""
    from tirith import msession as msession_mod
    home = _home(args)
    verb, rest = words[0], words[1:]
    if verb == "say":
        text = " ".join(rest).strip()
        if not text:
            raise SystemExit("manager say needs the message")
        print(inbox_mod.add_request(home, "chat", text=text)["id"])  # queued into the same session
        return 0
    if verb == "status":
        from tirith import dispatch as dispatch_mod
        book = (dispatch_mod.load_state(home).get("manager") or {}).get("session") or {}
        print(json.dumps({k: book.get(k) for k in ("lane", "session_id", "turns_total", "stopped", "error")}))
        return 0
    fresh = verb == "restart" and "--fresh" in rest
    msession_mod.send_control(home, verb, "fresh" if fresh else "")
    print(f"manager {verb}: sent")
    return 0


def _cmd_manager(args) -> int:
    home = _home(args)
    words = list(args.activity)
    if words[0] in MANAGER_VERBS and (words[0] == "say" or len(words) == 1 or words[1:] == ["--fresh"]):
        return _cmd_manager_session(args, words)
    write_json_atomic(home / "orchestrator.json",
                      {"activity": " ".join(words), "since": now_iso()})
    print("manager updated")
    return 0


def _cmd_manager_note(args) -> int:
    from tirith import managerd as managerd_mod
    home = _home(args)
    managerd_mod.add_note(home, args.text, datetime.now(timezone.utc))
    print(f"noted in {home / managerd_mod.NOTES_FILE}")
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
    if not args.once and hasattr(signal, "SIGTERM"):
        # the desktop app stops the collector with SIGTERM: unwind so the lock file handle closes cleanly
        # a second SIGTERM (e.g. forwarded by uv) must not raise again while the interpreter shuts down
        def _terminate(signum, frame):
            signal.signal(signal.SIGTERM, signal.SIG_IGN)
            raise SystemExit(0)
        signal.signal(signal.SIGTERM, _terminate)
    reload_now = False
    try:
        if args.once:  # a single cycle may overlap a running collector
            collector.run(interval=args.interval, once=True)
        else:
            with collectlock_mod.hold_collector_lock(home):
                try:
                    collector.run(interval=args.interval, once=False,
                                  reload_check=selfreload_mod.CodeWatch().changed)
                except selfreload_mod.ReloadRequested:
                    reload_now = True  # re-exec only after the lock is released
        if reload_now:
            selfreload_mod.reexec()
    except collectlock_mod.CollectorRunning as running:
        print(running)
        return 0
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
    manager = state.get("manager") or state.get("steward") or {}
    print(f"Tirith state {state.get('generated_at')}  brain: "
          f"{'on' if brain.get('enabled') else 'off'} ({brain.get('model')})"
          + (f"  last error: {brain['last_error']}" if brain.get("last_error") else ""))
    print(f"Manager: {manager.get('activity') or '(idle)'}")
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
        if args.action == "install":
            problems = install_guard(repo, words, chain=args.chain)
        elif args.action == "reinstall":  # owner only: the guard-reinstall gate blocks agents
            problems = reinstall_guard(repo, words, chain=args.chain)
        else:
            problems = verify_guard(repo, words)
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
                dry_run: bool = False, out=None, request_id: str | None = None,
                by_tirith: bool = False, item: str | None = None, auto_item: bool = False) -> str:
    """Start a sub-agent lane for one open item of a roadmap step; returns the lane id.

    ``item`` is the exact text of the open plan item (``tirith.plangate``); without one, only a caller that
    is Tirith itself (``by_tirith``: the autopilot; ``auto_item``: a click on a step) takes the first open item
    an agent can do. Raises ``launch_mod.LaunchError`` with the reason when it must not start.
    """
    out = out or sys.stdout
    config, proj = _project_or_exit(home, project_id)
    repo = projects_mod.real(proj.repo) if proj.repo else None
    if repo is None:
        raise launch_mod.LaunchError(f"project '{project_id}' has no repo folder")
    step, intro = launch_mod.find_step(repo, step_id)
    title = step["title"]
    open_items = roadmap_mod.agent_items(step, date.today())
    if not open_items:
        if any(not i["done"] for i in step["items"]):
            raise launch_mod.LaunchError(f"step {step['id']} ({title}) has no items an agent can do now "
                                         "(they wait for the owner or for a date)")
        raise launch_mod.LaunchError(f"step {step['id']} ({title}) is already done" if step["items"]
                                     else f"step {step['id']} ({title}) has no items to finish")
    failed = manifest_mod.check(repo, config.words_for(proj, default_words()))
    if failed:
        raise launch_mod.LaunchError(
            f"tirith check fails for '{project_id}', so no lane may start. Fix these, then run "
            f"`tirith check --project {project_id}`:\n" + "\n".join(f"  {p}" for p in failed))
    if not item and (by_tirith or auto_item):
        item = open_items[0]
    try:
        item = plangate_mod.check_start(home, repo, project_id, step["id"], item)
    except plangate_mod.PlanGateError as error:
        raise launch_mod.LaunchError(f"no agent starts without a plan item: {error}")
    busy = launch_mod.working_lane(read_lanes(home), project_id, step["id"], lane_status, item)
    if busy:
        raise launch_mod.LaunchError(f"that item of step {step['id']} already has a working lane: {busy['id']}")
    lane_id = launch_mod.lane_id_for(title, {str(l["id"]) for l in read_lanes(home)}, repo)
    brief_path = home / "briefs" / f"{lane_id}.md"
    brief = launch_mod.build_brief(title, intro, open_items, step["id"], item)
    workspace = launch_mod.worktree_path(repo, lane_id)
    if agent is None:  # the first agent of the chain that did not run out of credits in the last hour
        agent = fallback_mod.pick_alias(config, read_lanes(home), datetime.now(timezone.utc), project_id, step["id"])
    command = launch_mod.agent_command(config, agent, brief_path, workspace, project_id, step["id"])
    argv = launch_mod.lane_run_argv(project_id, step["id"], launch_mod.pick_class(title), title,
                                    brief_path, workspace, lane_id, command, request_id,
                                    alias=agent or _worker_alias(config, project_id, step["id"]),
                                    by_tirith=by_tirith, item=item)
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


def launch_plan(home: Path, project_id: str, kind: str, request_id: str | None = None,
                out=None, by_tirith: bool = False) -> str:
    """Start the auditor (``plan-refresh``) or planner (``plan-new``) lane for a project; returns the lane id.

    Raises ``launch_mod.LaunchError`` with the reason when it must not start."""
    out = out or sys.stdout
    config, proj = _project_or_exit(home, project_id)
    repo = projects_mod.real(proj.repo) if proj.repo else None
    if repo is None:
        raise launch_mod.LaunchError(f"project '{project_id}' has no repo folder")
    step = planlane_mod.LANE_STEPS[kind]
    lanes = read_lanes(home)
    busy = launch_mod.working_lane(lanes, project_id, step, lane_status) if kind == "plan-refresh" else None
    if busy:
        raise launch_mod.LaunchError(f"{project_id} already has a plan update running: lane {busy['id']}")
    failed = manifest_mod.check(repo, config.words_for(proj, default_words()))
    if failed:
        raise launch_mod.LaunchError(
            f"tirith check fails for '{project_id}', so no lane may start. Fix these, then run "
            f"`tirith check --project {project_id}`:\n" + "\n".join(f"  {p}" for p in failed))
    request = inbox_mod.find_request(home, request_id) if request_id else None
    manifest = manifest_mod.load(repo)
    if kind == "plan-new":
        plan = (request or {}).get("plan")
        if not plan:
            raise launch_mod.LaunchError("a new plan needs its request (title, goal, deadline, first steps)")
        slug = planlane_mod.slug_for(plan["title"], planlane_mod.taken_slugs(repo))
        brief = planlane_mod.new_plan_brief(project_id, plan, slug, manifest.plans)
    else:
        plan, path = None, roadmap_mod.find_file(repo, manifest.roadmap)
        if path is None:
            raise launch_mod.LaunchError(f"{project_id} has no plan file to update (docs/TODO.md)")
        merged = [l for l in lanes if l.get("project") == project_id
                  and (l.get("review") or {}).get("status") == "merged"]
        merged.sort(key=lambda l: str(l.get("started_at") or ""))
        rows = [r for r in inbox_mod.requests(home) if r["project"] == project_id and r["kind"] != "setup"]
        rel = str(path.relative_to(repo)) if path.is_relative_to(repo) else str(path)
        only = str((request or {}).get("step") or "")
        brief = planlane_mod.refresh_brief(project_id, rel, path.read_text(encoding="utf-8"), merged, rows, home,
                                           step=only if only and only != inbox_mod.PLAN_STEP else None)
    title = planlane_mod.lane_title(kind, plan)
    lane_id = launch_mod.lane_id_for(title, {str(l["id"]) for l in lanes}, repo)
    brief_path = home / "briefs" / f"{lane_id}.md"
    workspace = launch_mod.worktree_path(repo, lane_id)
    command = launch_mod.agent_command(config, None, brief_path, workspace, project_id, step)
    item_text = (f"Revisar el plan ({date.today().isoformat()})" if kind == "plan-refresh"
                 else f"Plan nuevo: {plan['title']}")
    item = _ops_item(home, repo, project_id, item_text, lane_id)
    argv = launch_mod.lane_run_argv(project_id, step, planlane_mod.HERO_CLASS, title, brief_path, workspace,
                                    lane_id, command, request_id, by_tirith=by_tirith, item=item,
                                    item_step=plangate_mod.OPS_STEP)
    launch_mod.create_worktree(repo, lane_id)
    brief_path.parent.mkdir(parents=True, exist_ok=True)
    brief_path.write_text(brief, encoding="utf-8")
    launch_mod.spawn_detached(argv, home, home / "logs" / f"{lane_id}.launch.log")
    print(f"lane {lane_id}", file=out)
    return lane_id


def launch_verify(home: Path, project_id: str, text: str, out=None, by_tirith: bool = False) -> str:
    """Start the verification lane for one "agent can do" check (``tirith.verifylane``); returns the lane id.

    Raises ``launch_mod.LaunchError`` with the reason when it must not start."""
    out = out or sys.stdout
    config, proj = _project_or_exit(home, project_id)
    repo = projects_mod.real(proj.repo) if proj.repo else None
    if repo is None:
        raise launch_mod.LaunchError(f"project '{project_id}' has no repo folder")
    failed = manifest_mod.check(repo, config.words_for(proj, default_words()))
    if failed:
        raise launch_mod.LaunchError(f"tirith check fails for '{project_id}', so no lane may start")
    lanes = read_lanes(home)
    title = verifylane_mod.lane_title(text)
    lane_id = launch_mod.lane_id_for(title, {str(l["id"]) for l in lanes}, repo)
    brief_path = home / "briefs" / f"{lane_id}.md"
    workspace = launch_mod.worktree_path(repo, lane_id)
    command = launch_mod.agent_command(config, None, brief_path, workspace, project_id, verifylane_mod.LANE_STEP)
    item = _ops_item(home, repo, project_id, f"Verificar: {verifylane_mod.lane_title(text)}", lane_id)
    argv = launch_mod.lane_run_argv(project_id, verifylane_mod.LANE_STEP, verifylane_mod.HERO_CLASS, title,
                                    brief_path, workspace, lane_id, command, None, by_tirith=by_tirith, item=item,
                                    item_step=plangate_mod.OPS_STEP)
    launch_mod.create_worktree(repo, lane_id)
    brief_path.parent.mkdir(parents=True, exist_ok=True)
    brief_path.write_text(verifylane_mod.brief(project_id, text), encoding="utf-8")
    launch_mod.spawn_detached(argv, home, home / "logs" / f"{lane_id}.launch.log")
    print(f"lane {lane_id}", file=out)
    return lane_id


def _ops_item(home: Path, repo: Path, project_id: str, text: str, lane_id: str) -> str:
    """The plan item for work Tirith starts by itself (created, already "(en curso)", before the lane starts)."""
    try:
        return plangate_mod.ops_item(home, repo, project_id, text, lane_id)[1]
    except plangate_mod.PlanGateError as error:
        raise launch_mod.LaunchError(f"the plan item for \"{text}\" could not be added: {error}")


def _worker_alias(config, project_id: str, step_id: str) -> str | None:
    alias = models_mod.resolve_workers(config, project_id, step_id).agent
    return alias if alias in config.agents else None


def relaunch_lane(home: Path, lane: dict, alias: str | None, by_tirith: bool = False,
                  note: str | None = None) -> str:
    """Start the brief of a failed ``lane`` again with agent ``alias`` (None: the configured worker); returns
    the new lane id. The worktree is kept when it has no uncommitted changes, else a new one is made from the
    repo. With a ``note`` (a failed claim check, or a lane that ended without a plan outcome) the brief gets the
    note appended and the new lane continues the old lane's work: it is committed on the old branch and the new
    worktree starts from it."""
    config, proj = _project_or_exit(home, str(lane.get("project") or ""))
    repo = projects_mod.real(proj.repo) if proj.repo else None
    brief = Path(str(lane.get("brief") or ""))
    if repo is None or not brief.is_file():
        raise launch_mod.LaunchError(f"lane {lane.get('id')} has no repo or brief to start again from")
    title = str(lane.get("feature") or lane["id"])
    lane_id = launch_mod.lane_id_for(title, {str(l["id"]) for l in read_lanes(home)}, repo)
    old = Path(str(lane.get("workspace") or ""))
    if note:
        timeouts_mod.run(["git", "-C", str(old), "add", "-A"], "git")
        timeouts_mod.run(["git", "-C", str(old), "commit", "-q", "-m", f"Lane {lane['id']}: work before the claim check"],
                         "git-commit")
        workspace = launch_mod.create_worktree(repo, lane_id, base=f"lane/{lane['id']}")
        brief = brief.with_name(f"{brief.stem}-{lane_id}.md")
        brief.write_text(Path(str(lane["brief"])).read_text(encoding="utf-8") + "\n\n" + note, encoding="utf-8")
    else:
        dirty = timeouts_mod.run(["git", "-C", str(old), "status", "--porcelain"], "git", text=True)
        if old.is_dir() and dirty.returncode == 0 and not dirty.stdout.strip():
            workspace = old
        else:
            workspace = launch_mod.create_worktree(repo, lane_id)
    step = lane.get("step")
    command = launch_mod.agent_command(config, alias or None, brief, workspace, str(lane.get("project") or ""), step)
    linked = plangate_mod.lane_item(lane)
    argv = launch_mod.lane_run_argv(
        str(lane.get("project") or ""), str(step or ""), str(lane.get("class") or launch_mod.DEFAULT_CLASS),
        title, brief, workspace, lane_id, command, lane.get("request_id"), alias=alias or None,
        fallback_from=str(lane["id"]), tried=fallback_mod.tried_of(lane), by_tirith=by_tirith,
        item=linked[1] if linked else None, item_step=linked[0] if linked else None,
        retry_of=str(lane["id"]) if note else None)
    launch_mod.spawn_detached(argv, home, home / "logs" / f"{lane_id}.launch.log")
    return lane_id


def _cmd_plan_add(args) -> int:
    home = _home(args)
    repo = plangate_mod.repo_of(home, args.project)
    if repo is None:
        print(f"tirith plan add: unknown project '{args.project}'", file=sys.stderr)
        return 3
    who = os.environ.get("TIRITH_LANE") or ("manager" if in_manager_session() else "owner")
    try:
        text = plangate_mod.add_item(home, repo, args.project, str(args.step), args.text, who, args.title,
                                      getattr(args, "check", None))
    except plangate_mod.PlanGateError as error:
        print(f"tirith plan add: {error}", file=sys.stderr)
        return 3
    print(f"added to step {args.step}: {text}")
    return 0


def _cmd_plan_check(args) -> int:
    home = _home(args)
    repo = plangate_mod.repo_of(home, args.project)
    if repo is None:
        print(f"tirith plan check: unknown project '{args.project}'", file=sys.stderr)
        return 3
    try:
        passed, output = plangate_mod.check_item(repo, str(args.step), args.item)
    except plangate_mod.PlanGateError as error:
        print(f"tirith plan check: {error}", file=sys.stderr)
        return 3
    print(("check passed" if passed else "check failed") + (f"\n{output}" if output.strip() else ""))
    return 0 if passed else 1


def _cmd_plan_done(args) -> int:
    home = _home(args)
    repo = plangate_mod.repo_of(home, args.project)
    if repo is None:
        print(f"tirith plan done: unknown project '{args.project}'", file=sys.stderr)
        return 3
    try:
        sha = plangate_mod.mark_done(home, repo, args.project, str(args.step), args.item, args.commit)
    except plangate_mod.PlanGateError as error:
        print(f"tirith plan done: {error}", file=sys.stderr)
        return 3
    print(f"step {args.step}: ticked (hecho: {sha})")
    return 0


def _cmd_plan_report(args) -> int:
    home = _home(args)
    lane_id = args.lane or os.environ.get("TIRITH_LANE")
    if not lane_id:
        print("tirith plan report: pass --lane <id> (or run it inside a lane)", file=sys.stderr)
        return 3
    try:
        lane = plangate_mod.report(home, lane_id, args.status, args.add, args.reason)
    except plangate_mod.PlanGateError as error:
        print(f"tirith plan report: {error}", file=sys.stderr)
        return 3
    outcome = lane["outcome"]
    new = f", {len(outcome['added'])} new item(s)" if outcome["added"] else ""
    later = "" if outcome.get("applied") else f" (recorded; the plan will be updated: {outcome.get('error')})"
    print(f"lane {lane_id}: {args.status}{new}{later}")
    return 0


def _cmd_launch(args) -> int:
    home = _home(args)
    try:
        launch_step(home, args.project, str(args.step), args.agent, args.dry_run,
                    request_id=args.request, item=args.item or "")
    except launch_mod.LaunchError as error:
        print(f"tirith launch: {error}", file=sys.stderr)
        return 3
    return 0


def _open_file(home: Path, project: str, path: str, line: int | None, col: int | None) -> None:
    """Open one file named in the chat, checked against the registered project folders."""
    config = load_config(home)
    entries = projects_mod.entries(config, projects_mod.load_auto(home))
    target = filerefs_mod.open_file(
        [{"id": e.id, "folder": str(e.folder)} for e in entries if e.folder], project, path, line, col,
        config.app_editor)
    print(f"opened {target}")


def _cmd_open(args) -> int:
    """``tirith open --project P --path X [--line N]``: the desktop app's click on a file in the chat."""
    try:
        _open_file(_home(args), args.project or "", args.path, args.line, args.col)
    except filerefs_mod.FileRefError as error:
        print(f"tirith open: {error}", file=sys.stderr)
        return 3
    return 0


def _cmd_url_open(args) -> int:
    home = _home(args)
    launch_mod.log_handler(home, "received " + launch_mod.redact_url(args.url))
    code = _url_open(args, home)
    launch_mod.log_handler(home, f"result exit {code}")
    return code


def _url_open(args, home: Path) -> int:
    try:
        link = launch_mod.parse_url(args.url)
        if link["action"] not in ("launch", "inbox", "review", "run", "stop", "open", "env-set"):
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
        if link["action"] == "env-set":  # tirith://env-set?project=P&env=<json object>&token=T: replaces the set
            _project_or_exit(home, link["project"])
            raw = parse_qs(urlsplit(args.url).query).get("env", ["{}"])[-1]
            try:
                given = json.loads(raw)
            except ValueError:
                given = None
            if not isinstance(given, dict):
                raise launch_mod.LaunchError("env-set needs env=<JSON object>")
            projenv_mod.save(home, link["project"], given)
            print(f"{link['project']}: {len(given)} variable(s) saved")
            return 0
        if link["action"] == "run":
            return _run_action(home, actions_mod.parse_run_url(args.url))
        if link["action"] == "inbox":
            fields = inbox_mod.parse_url(args.url)
            if fields["kind"] == "reply":
                inbox_mod.add_reply(home, fields["wait"], fields["project"], fields["value"], fields["text"])
                return 0
            inbox_mod.add_request(home, fields["kind"], fields["project"], fields["step"], fields["text"],
                                  cid=fields["cid"])
            return 0
        if link["action"] == "open":
            ref = filerefs_mod.parse_open_url(args.url)
            _open_file(home, ref["project"], ref["path"], ref["line"], ref["col"])
            return 0
        if link["action"] == "stop":
            lane = read_json(lane_path(home, link["lane"]), None)
            if not isinstance(lane, dict) or lane.get("ended_at"):
                raise launch_mod.LaunchError(f"{link['lane'] or 'that lane'} is not a running lane")
            launch_mod.spawn_detached([*launch_mod.tirith_argv(home), "lane", "stop", link["lane"], "--by", "owner"],
                                      home, home / "logs" / f"{link['lane']}.stop.log")
            print(f"stopping {link['lane']}")
            return 0
        if link["action"] == "review":
            lane_id = link["lane"]
            review_mod.check_ready(home, lane_id, _lane_repo(home, lane_id))
            launch_mod.spawn_detached([*launch_mod.tirith_argv(home), "lane", "review", lane_id], home,
                                      home / "logs" / f"{lane_id}.review.log")
            print(f"reviewing {lane_id}")
            return 0
        launch_step(home, link["project"], link["step"], link["agent"], item=link["item"] or None,
                    auto_item=True)  # the step button names a step: Tirith takes its first open item
    except (launch_mod.LaunchError, inbox_mod.InboxError, review_mod.ReviewError,
            actions_mod.ActionError, filerefs_mod.FileRefError, projenv_mod.EnvError) as error:
        print(f"tirith url open: {error}", file=sys.stderr)
        return 3
    return 0


def _run_action(home: Path, action_id: str) -> int:
    action = actions_mod.find_action(home, action_id)
    if action is None:
        raise actions_mod.ActionError("no action " + action_id)
    _, folder = _project_folder_of(home, action["project"] or None)
    result = actions_mod.run_action(home, action_id, folder)
    print(action_id + ": " + result["status"] + " (exit " + str(result["exit_code"]) + ")")
    return 0 if result["status"] == "passed" else 1


def _cmd_action(args) -> int:
    home = _home(args)
    try:
        if args.action_command == "run":
            return _run_action(home, args.id)
        for action in actions_mod.list_actions(home):
            run = action["last_run"]
            if args.json:
                print(json.dumps(action))
            else:
                print(action["id"] + "  " + (run["status"] if run else "never run").ljust(9) + " "
                      + (action["project"] or "-") + "  " + action["label"] + "  $ " + action["command"][:60])
    except actions_mod.ActionError as error:
        print("tirith action " + args.action_command + ": " + str(error), file=sys.stderr)
        return 3
    return 0


def _cmd_inbox(args) -> int:
    home = _home(args)
    action = args.inbox_command
    try:
        if action == "add":
            text = args.text if args.text is not None else ""
            print(inbox_mod.add_request(home, args.kind, args.project, str(args.step or ""), text,
                                        attachments=inbox_mod.attachment_list(args.attach),
                                        cid=args.cid)["id"])
        elif action == "list":
            for req in inbox_mod.requests(home):
                if args.open and req["status"] != "open":
                    continue
                print(json.dumps(req) if args.json else
                      f"{req['id']}  {req['status']:<8} {req['kind']:<6} {req['project']}#{req['step']}  "
                      + (inbox_mod.setup_summary(req["setup"]) if req.get("setup")
                         else " ".join(req["text"].split())[:80]))
        elif action == "reply":
            project = args.project or next((str(r.get("project") or "") for r in reversed(
                read_jsonl(home / "waits.jsonl")) if str(r.get("id")) == args.wait), "")
            print(inbox_mod.add_reply(home, args.wait, project, args.value, args.text)["id"])
        elif action == "answer":
            try:
                actions_mod.check_text(args.text)
                actions = actions_mod.make_actions(args.run, args.label, args.cwd, created_by=args.by)
            except actions_mod.ActionError as error:
                print(error, file=sys.stderr)
                return 2
            inbox_mod.add_answer(home, args.id, args.text, by=args.by, lane=args.lane, actions=actions,
                                 attachments=inbox_mod.attachment_list(args.attach))
        elif action == "done":
            inbox_mod.mark_done(home, args.id)
        elif action == "brain":
            req = inbox_mod.find_request(home, args.id)
            client = brain_mod.make_client(load_config(home))
            if req is None or client is None:
                raise inbox_mod.InboxError(f"no request {args.id}" if req is None else "the brain is not enabled")
            state = read_json(home / "state.json", {})
            project = next((p for p in state.get("projects") or [] if p.get("id") == req["project"]), None)
            text = brain_mod.job_inbox_answer(client, inbox_mod.answer_prompt(
                req, project, state.get("agents") or [], state.get("flows") or []))
            inbox_mod.add_answer(home, args.id, text, by="brain", auto=True)
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


def _cmd_queue(args) -> int:
    home = _home(args)
    if args.queue_command == "done":
        try:
            asks_mod.queue_done(home, args.id)
        except asks_mod.AskError as error:
            print(f"tirith queue done: {error}", file=sys.stderr)
            return 3
        return 0
    for item in asks_mod.queue_items(home, only_open=not args.all):
        print(json.dumps(item) if args.json else
              f"{item['id']}  {item['status']:<5} {item['project']}  {item['lane']}  "
              + " ".join(item["text"].split())[:80])
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
    if args.action == "verify":
        config = load_config(_home(args))
        try:
            path = hooks_mod.default_settings(args.agent, config.agents)
        except ValueError as err:
            raise SystemExit(f"tirith hook verify: {err}")
        if not hooks_mod.has_hooks(path, args.agent):
            print(f"Hooks missing for {args.agent} in {path}. Run: tirith hook install --agent {args.agent}")
            return 1
        saved = hooks_mod.protect_state(_home(args)).get(args.agent) or {}
        try:
            secs = max(0, int((datetime.now(timezone.utc) - datetime.fromisoformat(
                str(saved["checked_at"]).replace("Z", "+00:00"))).total_seconds()))
            ago = f" · last check {secs} s ago" if secs < 120 else f" · last check {secs // 60} min ago"
        except (KeyError, ValueError):
            ago = " · the collector has not checked yet"
        print(f"Hooks protected{ago}")
        return 0
    if args.action in ("install", "uninstall"):
        try:
            fn = hooks_mod.install if args.action == "install" else hooks_mod.uninstall
            kwargs = {"tirith": args.tirith, "role": args.role} if args.action == "install" else {}
            kwargs["agents"] = load_config(_home(args)).agents
            path = fn(args.agent, Path(args.settings).expanduser() if args.settings else None,
                      **kwargs)
        except (OSError, ValueError) as err:
            raise SystemExit(f"tirith hook {args.action}: {err}")
        print(f"{args.action}ed tirith hooks for {args.agent}: {path}")
        if args.action == "install" and not args.settings:
            _record_agent_settings(_home(args), args.agent)
        return 0
    try:  # a hook call must never break the agent
        home = tirith_home(getattr(args, "home", None))
        return hooks_mod.run_hook(home, args.agent, args.event, sys.stdin)
    except BaseException:
        return 0


def _record_agent_settings(home: Path, only: str | None = None) -> None:
    try:
        for alias, value in hooks_mod.record_settings(home, load_config(home).agents, only):
            print(f"config.toml: [agents.{alias}] settings = {value}")
    except OSError as err:
        print(f"tirith: could not record settings in config.toml ({err})", file=sys.stderr)


def _cmd_agents(args) -> int:
    """``tirith agents set-settings`` / ``install``: guard every configured agent for real."""
    home = _home(args)
    config = load_config(home)
    if args.action == "set-settings":
        _record_agent_settings(home)
        return 0
    status = 0
    for alias in config.agents:
        try:
            path = hooks_mod.install(alias, None, args.tirith, config.agents)
        except (OSError, ValueError) as err:
            print(f"{alias}: {err}")
            status = 1
            continue
        print(f"{alias}: hooks installed in {path}")
    _record_agent_settings(home)
    return status


def _cmd_shim(args) -> int:
    config = load_config(_home(args))
    # their own folder: the agents' real executables often live in ~/.local/bin themselves
    shim_dir = Path(args.dir).expanduser() if args.dir else _home(args) / "shims"
    if not any(str(t.get("command") or "").strip() for t in config.agents.values()):
        raise SystemExit("tirith shim: no [agents.<alias>] command in config.toml; add "
                         "[agents.<alias>] command = \"<executable name>\"")
    if args.action == "verify":
        # shims only have to come first inside lanes, whose PATH `lane run` builds
        problems = gates_mod.verify_home_shims(config, _home(args), shim_dir if args.dir else None)
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


def _cmd_safegit(args) -> int:
    from tirith import safegit as safegit_mod
    shim_dir = Path(args.dir).expanduser() if args.dir else _home(args) / "shims"
    if args.action == "verify":
        lane_path = _wrapper_path_env(_home(args))["PATH"] if not args.dir else None
        problems = safegit_mod.verify(shim_dir, lane_path)
        for problem in problems:
            print(problem)
        if not problems:
            print(f"safe-git in {shim_dir}: intact")
        return 1 if problems else 0
    status, detail = (safegit_mod.install(shim_dir) if args.action == "install"
                      else safegit_mod.uninstall(shim_dir))
    print(f"git: {status} {detail}")
    return 1 if status == "skipped" else 0


def _add_action_args(parser) -> None:
    parser.add_argument("--run", action="append", default=[], metavar="COMMAND",
                        help="a command for the owner: shown as a Run button (repeatable)")
    parser.add_argument("--label", action="append", default=[], help="what the matching --run does")
    parser.add_argument("--cwd", action="append", default=[], help="folder for the matching --run")


def _cmd_setup(args) -> int:
    home = _home(args)
    action = args.setup_command
    if action == "apply":
        return _setup_apply(home, args.id)
    pid, folder = _project_folder_of(home, args.project)
    if folder is None:
        raise SystemExit(f"tirith setup: unknown project '{pid}'" if args.project
                         else "tirith setup: cwd is in no known project; pass --project")
    if action == "show":
        _setup_show(manifest_mod.load(folder), folder)
        return 0
    before = {(p.rule, p.message) for p in manifest_mod.check(folder, [], ci=True)}
    try:
        if action == "add-stage":
            done = setup_mod.add_stage(folder, args.pipeline, args.name, args.run, args.kind, args.after)
        elif action == "remove-stage":
            done = setup_mod.remove_stage(folder, args.name, args.pipeline)
        elif action == "add-rule":
            done = setup_mod.add_rule(folder, setup_mod.build_rule(
                args.id, args.action, args.message, args.instead, args.match, args.regex,
                [x.strip() for x in (args.paths or "").split(",") if x.strip()], args.description))
        elif action == "describe-rule":
            done = setup_mod.describe_rule(folder, args.id, args.description)
        else:
            done = setup_mod.remove_rule(folder, args.id)
    except setup_mod.SetupError as error:
        print(f"tirith setup {action}: {error}", file=sys.stderr)
        return 3
    print(f"{done} ({folder / manifest_mod.FILENAME})")
    problems = manifest_mod.check(folder, [], ci=True)
    for problem in problems:
        print(problem)
    print("tirith check: ready" if not problems else f"tirith check: {len(problems)} problem(s)")
    return 1 if {(p.rule, p.message) for p in problems} - before else 0


def _setup_show(loaded, folder: Path) -> None:
    if not loaded.exists:
        print(f"{folder}: no {manifest_mod.FILENAME}")
        return
    for pipe in loaded.pipelines:
        print(f"pipeline {pipe.id}: " + ", ".join(
            f"{s.name} [{manifest_mod.stage_kind(s) or '-'}] {s.run}" for s in pipe.stages))
    for hook in loaded.hooks:
        print(f"hook {hook.event}: {hook.run}")
    for rule in loaded.rules:
        print(f"rule {rule.id} ({rule.action}): {rule.description}")


def _setup_apply(home: Path, request_id: str) -> int:
    """Apply a ``setup`` inbox request (edit or run), answer it and close it."""
    req = inbox_mod.find_request(home, request_id)
    if req is None or req["kind"] != "setup" or not req.get("setup"):
        print(f"tirith setup apply: no setup request {request_id}", file=sys.stderr)
        return 3
    if req["setup"].get("action") in inbox_mod.MODELS_ACTIONS:
        return _models_apply(home, request_id, req["setup"])
    pid, folder = _project_folder_of(home, req["project"])
    if folder is None:
        print(f"tirith setup apply: unknown project '{pid}'", file=sys.stderr)
        return 3
    setup = req["setup"]
    try:
        if setup["action"] == "run-stage":
            argv = [sys.executable, "-m", "tirith", "--home", str(home), "run", "--project", pid,
                    "--stage", str(setup.get("name") or "")]
            if setup.get("pipeline"):
                argv += ["--pipeline", str(setup["pipeline"])]
            code = timeouts_mod.run(argv, "stage", cwd=folder, capture_output=False).returncode
            note = f"ran stage `{setup.get('name')}`: " + ("passed" if code == 0 else f"failed (exit {code})")
        else:
            note, code = setup_mod.apply_request(folder, setup), 0
    except setup_mod.SetupError as error:
        inbox_mod.add_answer(home, request_id, f"Could not apply: {error}")
        inbox_mod.mark_done(home, request_id)
        print(f"tirith setup apply: {error}", file=sys.stderr)
        return 3
    inbox_mod.add_answer(home, request_id, note[0].upper() + note[1:] + ".")
    inbox_mod.mark_done(home, request_id)
    print(note)
    return 0 if code == 0 else 1


def _models_apply(home: Path, request_id: str, setup: dict) -> int:
    """Apply a ``models-set`` or ``apply-manager`` request of the Settings page, answer it, close it."""
    try:
        if setup["action"] == "apply-manager":
            note = "; ".join(models_mod.apply_manager(load_config(home)))
        else:
            note = models_mod.set_models(
                home, str(setup.get("role") or ""), model=setup.get("model") or None,
                effort=setup.get("effort") or None, agent=setup.get("agent") or None,
                project=setup.get("project") or None, step=setup.get("step") or None,
                clear=bool(setup.get("clear")),
                fallback=[str(a) for a in setup["fallback"]] if isinstance(setup.get("fallback"), list) else None)
    except models_mod.ModelsError as error:
        inbox_mod.add_answer(home, request_id, f"Could not apply: {error}")
        inbox_mod.mark_done(home, request_id)
        print(f"tirith setup apply: {error}", file=sys.stderr)
        return 3
    inbox_mod.add_answer(home, request_id, note[0].upper() + note[1:] + ".")
    inbox_mod.mark_done(home, request_id)
    print(note)
    return 0


def _fallback_arg(value: str | None) -> list[str] | None:
    """``--fallback a,b`` -> ``["a", "b"]``; ``--fallback ""`` -> ``[]`` (no fallback); absent -> None."""
    return None if value is None else [a.strip() for a in value.split(",") if a.strip()]


def _cmd_models(args) -> int:
    home = _home(args)
    config = load_config(home)
    try:
        if args.models_command == "show":
            for line in models_mod.show_lines(config):
                print(line)
        elif args.models_command == "apply-manager":
            for line in models_mod.apply_manager(config):
                print(line)
        else:
            print(models_mod.set_models(home, args.role, model=args.model, effort=args.effort,
                                        agent=args.agent, project=args.project, step=args.step,
                                        clear=args.clear, fallback=_fallback_arg(args.fallback)))
    except models_mod.ModelsError as error:
        print(f"tirith models {args.models_command}: {error}", file=sys.stderr)
        return 3
    return 0


def _item_args(parser) -> None:
    parser.add_argument("--item", default=None,
                        help="the exact text (or i-xxxxxxxx id) of the open plan item this agent works on; "
                             "no agent starts without one (add it first: tirith plan add)")
    parser.add_argument("--item-step", default=None, help=argparse.SUPPRESS)


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
    start.add_argument("--request", default=None, help="inbox request id this lane answers")
    _item_args(start)
    start.set_defaults(func=_cmd_lane_start)
    end = lane_sub.add_parser("end", parents=[common], help="finish a lane")
    end.add_argument("lane_id")
    end.add_argument("--result", choices=["done", "failed"], required=True)
    end.add_argument("--report", default=None)
    end.add_argument("--report-file", default=None)
    end.set_defaults(func=_cmd_lane_end)
    stop = lane_sub.add_parser("stop", parents=[common],
                               help="stop a running lane (SIGTERM, then SIGKILL after 10 s); it ends as failed")
    stop.add_argument("lane_id")
    stop.add_argument("--reason", default=None)
    stop.add_argument("--by", choices=list(lanestop_mod.STOPPERS), default=None,
                      help="who stops it (default: manager from a script or an agent, owner at a terminal)")
    stop.set_defaults(func=_cmd_lane_stop)
    review = lane_sub.add_parser("review", parents=[common],
                                 help="run the tests of a launched lane, then merge it")
    review.add_argument("lane_id")
    review.add_argument("--tests-only", action="store_true", help="run the tests, do not merge")
    review.set_defaults(func=_cmd_lane_review)
    drop = lane_sub.add_parser("drop", parents=[common], help="throw away a launched lane's worktree and branch")
    drop.add_argument("lane_id")
    drop.set_defaults(func=_cmd_lane_drop)
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
    lane_run.add_argument("--request", default=None, help="inbox request id this lane answers")
    _item_args(lane_run)
    lane_run.add_argument("--retry-of", default=None, help=argparse.SUPPRESS)
    lane_run.add_argument("--interactive", action="store_true",
                          help="give the agent the terminal (no log capture); activity comes from hooks")
    lane_run.add_argument("--agent", default=None, help="agent alias from [agents.<alias>]")
    lane_run.add_argument("--as-agent", default=None, help="alias this command runs (recorded on the lane)")
    lane_run.add_argument("--fallback-from", default=None, help="lane id this one continues (out of credits)")
    lane_run.add_argument("--by-tirith", action="store_true", help=argparse.SUPPRESS)
    lane_run.add_argument("--detach", action="store_true",
                          help="start the lane in its own session and return its id at once "
                               "(the default inside the manager)")
    lane_run.add_argument("--no-detach", action="store_true", help=argparse.SUPPRESS)
    lane_run.add_argument("--tried", default="", help="comma list of aliases already used for this brief")
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

    note = sub.add_parser("note", parents=[common], help="record a manager note")
    note.add_argument("text")
    note.add_argument("--project", default=None)
    note.set_defaults(func=_cmd_note)

    wait = sub.add_parser("wait", parents=[common], help="things waiting on you")
    wait_sub = wait.add_subparsers(dest="wait_command", required=True)
    wait_add = wait_sub.add_parser("add", parents=[common])
    wait_add.add_argument("text", nargs="?", default=None, help="the question (markdown)")
    wait_add.add_argument("--text", dest="text_opt", default=None, help="the question, as an option")
    wait_add.add_argument("--option", action="append", metavar="LABEL|VALUE",
                          help="a one-click answer; repeat for more (max 6)")
    wait_add.add_argument("--command", default=None, help="a shell command the owner runs with a button (same as --run)")
    wait_add.add_argument("--kind", default=None, choices=asks_mod.ASK_KINDS)
    wait_add.add_argument("--project", default=None)
    wait_add.add_argument("--link", default=None)
    wait_add.add_argument("--id", default=None)
    _add_why_args(wait_add)
    _add_action_args(wait_add)
    wait_add.set_defaults(func=_cmd_wait_add)
    wait_done = wait_sub.add_parser("done", parents=[common])
    wait_done.add_argument("wait_id")
    wait_done.set_defaults(func=_cmd_wait_done)

    chat = sub.add_parser("chat", parents=[common], help="write to the owner in the chat")
    chat_sub = chat.add_subparsers(dest="chat_command", required=True)
    chat_say = chat_sub.add_parser("say", parents=[common],
                                   help="a manager message (an owner item when it has options or actions)")
    chat_say.add_argument("text", help="the message (markdown)")
    chat_say.add_argument("--project", default=None)
    chat_say.add_argument("--option", action="append", metavar="LABEL|VALUE",
                          help="a one-click answer; repeat for more (max 6)")
    _add_action_args(chat_say)
    _add_why_args(chat_say)
    chat_say.set_defaults(func=_cmd_chat_say)

    flow = sub.add_parser("flow", parents=[common],
                          help="record information passed between endpoints")
    flow.add_argument("--from", dest="src", required=True)
    flow.add_argument("--to", dest="dst", required=True)
    flow.add_argument("--kind", choices=flows_mod.KINDS, default="message")
    flow.add_argument("--text", required=True)
    flow.add_argument("--ref", default=None)
    flow.set_defaults(func=_cmd_flow)

    tmo = sub.add_parser("timeout", parents=[common], help="the timeout of each class of command")
    tmo_sub = tmo.add_subparsers(dest="timeout_command", required=True)
    t_get = tmo_sub.add_parser("get", parents=[common], help="show the value in force")
    t_get.add_argument("--class", dest="cls", default=None)
    t_get.set_defaults(func=_cmd_timeout)
    t_set = tmo_sub.add_parser("set", parents=[common], help="raise or lower it within its limits, with a reason")
    t_set.add_argument("--class", dest="cls", required=True)
    t_set.add_argument("--seconds", type=float, required=True)
    t_set.add_argument("--reason", default=None)
    t_set.set_defaults(func=_cmd_timeout)

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
    p_env = project_sub.add_parser("env", parents=[common], help="environment variables of a project")
    env_sub = p_env.add_subparsers(dest="env_command", required=True)
    env_set = env_sub.add_parser("set", parents=[common], help="set KEY=VALUE pairs (or replace all from stdin)")
    env_set.add_argument("pairs", nargs="*", metavar="KEY=VALUE")
    env_set.add_argument("--project", default=None)
    env_set.add_argument("--replace-stdin", action="store_true",
                         help="read a JSON object from stdin; it becomes the whole set")
    env_unset = env_sub.add_parser("unset", parents=[common], help="remove variables")
    env_unset.add_argument("keys", nargs="+", metavar="KEY")
    env_unset.add_argument("--project", default=None)
    env_list = env_sub.add_parser("list", parents=[common], help="show the variables")
    env_list.add_argument("--project", default=None)
    env_list.add_argument("--json", action="store_true")
    for env_cmd in (env_set, env_unset, env_list):
        env_cmd.set_defaults(func=_cmd_project_env)

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

    manager = sub.add_parser("manager", parents=[common], help="set manager activity, or control the "
                             "persistent session: say <text> | interrupt | stop | restart [--fresh] | status")
    manager.add_argument("activity", nargs="+")
    manager.set_defaults(func=_cmd_manager)

    manager_note = sub.add_parser("manager-note", parents=[common],
                                  help="append a line to the on-call manager's rolling notes")
    manager_note.add_argument("text")
    manager_note.set_defaults(func=_cmd_manager_note)

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
    hook.add_argument("action", nargs="?", choices=["install", "uninstall", "verify"], default=None)
    hook.add_argument("--agent", required=True, help="agent alias: anthropic, muse, codex, copilot, or any other name")
    hook.add_argument("--event", default=None, help="event name if the payload has none")
    hook.add_argument("--settings", default=None, help="agent settings file (install/uninstall)")
    hook.add_argument("--tirith", default="tirith", help="tirith executable to write on install")
    hook.add_argument("--role", choices=["manager", "steward"], default=None,
                      help="install: mark the hooks as the manager's (TIRITH_ROLE=manager in the command; "
                           "steward is the old spelling)")
    hook.set_defaults(func=_cmd_hook)

    agents = sub.add_parser("agents", parents=[common],
                            help="guard every [agents.<alias>] in config.toml (hooks + settings paths)")
    agents.add_argument("action", choices=["set-settings", "install"],
                        help="set-settings: record each agent's hook file in config.toml; "
                             "install: install the hooks of every agent, then record the paths")
    agents.add_argument("--tirith", default="tirith", help="tirith executable to write on install")
    agents.set_defaults(func=_cmd_agents)

    guard = sub.add_parser("guard", parents=[common], help="commit guard")
    guard.add_argument("action", choices=["install", "reinstall", "verify"])
    guard.add_argument("project", nargs="?", default=None,
                       help="project id or repo path")
    guard.add_argument("--all", action="store_true")
    guard.add_argument("--chain", action="store_true",
                       help="install: keep the repository's own hooks as <hook>.local and run them after the guard")
    guard.set_defaults(func=_cmd_guard)

    shim = sub.add_parser("shim", parents=[common],
                          help="agent CLI shims: start only through tirith lane run")
    shim.add_argument("action", choices=["install", "uninstall", "verify"])
    shim.add_argument("--dir", default=None, help="shim folder (default: ~/.tirith/shims, first in every lane's PATH)")
    shim.set_defaults(func=_cmd_shim)

    safegit = sub.add_parser("safegit", parents=[common],
                             help="Windows safe-git shim: refuses git flags that skip hooks (git.cmd + Python core)")
    safegit.add_argument("action", choices=["install", "uninstall", "verify"])
    safegit.add_argument("--dir", default=None, help="shim folder (default: ~/.tirith/shims, first in every lane's PATH)")
    safegit.set_defaults(func=_cmd_safegit)

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
    launch.add_argument("--item", default=None, help="the exact text (or i-xxxxxxxx id) of the open plan item; "
                        "required: add it first with `tirith plan add`")
    launch.add_argument("--agent", default=None, help="agent alias (its [agents.<alias>] launch template)")
    launch.add_argument("--request", default=None, help="inbox request id this lane answers")
    launch.add_argument("--dry-run", action="store_true", help="print the brief and command, start nothing")
    launch.set_defaults(func=_cmd_launch)

    plan = sub.add_parser("plan", parents=[common],
                          help="the plan is the only way to start and finish an agent: add items, report outcomes")
    plan_sub = plan.add_subparsers(dest="plan_command", required=True)
    pl_add = plan_sub.add_parser("add", parents=[common], help="add an open item to a step of the plan (Tirith writes it)")
    pl_add.add_argument("--project", required=True)
    pl_add.add_argument("--step", required=True, help="step number")
    pl_add.add_argument("--text", required=True, help="the new item")
    pl_add.add_argument("--title", default=None, help="create the step with this title when it is not in the plan")
    pl_add.add_argument("--check", default=None, help="a command that passes when the item is done")
    pl_add.set_defaults(func=_cmd_plan_add)
    pl_check = plan_sub.add_parser("check", parents=[common], help="run an item's done check (exit 0 = done)")
    pl_check.add_argument("--project", required=True)
    pl_check.add_argument("--step", required=True, help="step number")
    pl_check.add_argument("--item", required=True, help="the exact text of the item")
    pl_check.set_defaults(func=_cmd_plan_check)
    pl_rep = plan_sub.add_parser("report", parents=[common],
                                 help="the outcome of a lane's item: done, partial or blocked")
    pl_rep.add_argument("--lane", default=None, help="lane id (default: $TIRITH_LANE)")
    pl_rep.add_argument("--status", required=True, choices=list(plangate_mod.STATUSES))
    pl_rep.add_argument("--add", action="append", default=[], help="a new item for the same step (repeatable)")
    pl_rep.add_argument("--reason", default=None, help="why (needed for blocked)")
    pl_rep.set_defaults(func=_cmd_plan_report)
    pl_done = plan_sub.add_parser("done", parents=[common],
                                  help="tick an item whose work is already on main (the manager names the commit)")
    pl_done.add_argument("--project", required=True)
    pl_done.add_argument("--step", required=True, help="step number")
    pl_done.add_argument("--item", required=True, help="the exact text of the item")
    pl_done.add_argument("--commit", required=True, help="the commit on main that did the work")
    pl_done.set_defaults(func=_cmd_plan_done)

    inbox = sub.add_parser("inbox", parents=[common], help="the owner's requests on plan steps")
    inbox_sub = inbox.add_subparsers(dest="inbox_command", required=True)
    i_add = inbox_sub.add_parser("add", parents=[common], help="add a request (prints its id)")
    i_add.add_argument("--kind", required=True, choices=inbox_mod.KINDS)
    i_add.add_argument("--project", default="", help="required, except for a chat message")
    i_add.add_argument("--step", default=None, help="roadmap step number (not for setup)")
    i_add.add_argument("--text", default=None,
                       help="the question (required for ask); for setup the request as JSON, see CONTRACT-setup")
    i_add.add_argument("--attach", action="append", default=[], metavar="FILE",
                       help="a file to attach (an image or any other file; up to 6, 10 MB each); repeatable")
    i_add.add_argument("--cid", default=None, help="client id; a second add with the same cid is ignored (safe retry)")
    i_ls = inbox_sub.add_parser("list", parents=[common], help="list requests")
    i_ls.add_argument("--open", action="store_true", help="only open requests")
    i_ls.add_argument("--json", action="store_true", help="one JSON object per line")
    i_ans = inbox_sub.add_parser("answer", parents=[common], help="answer a request (markdown)")
    i_ans.add_argument("id")
    i_ans.add_argument("text")
    i_ans.add_argument("--by", choices=inbox_mod.ANSWERERS, default="manager")
    i_ans.add_argument("--lane", default=None, help="id of the lane (agent) that took the request")
    i_ans.add_argument("--attach", action="append", default=[], metavar="FILE",
                       help="an image or other file the answer refers to (up to 6); repeatable")
    i_reply = inbox_sub.add_parser("reply", parents=[common], help="the owner's answer to an ask")
    i_reply.add_argument("--wait", required=True, help="id of the ask (tirith wait add)")
    i_reply.add_argument("--project", default=None)
    i_reply.add_argument("--value", default="", help="the option chosen")
    i_reply.add_argument("--text", default="", help="free text")
    _add_action_args(i_ans)
    i_done = inbox_sub.add_parser("done", parents=[common], help="close a request or a reply")
    i_done.add_argument("id")
    i_brain = inbox_sub.add_parser("brain", parents=[common], help="answer a request with the brain now")
    i_brain.add_argument("id")
    i_watch = inbox_sub.add_parser("watch", parents=[common],
                                   help="block; print each open request as one JSON line, then each new one")
    i_watch.add_argument("--new-only", action="store_true", help="skip the requests already in the file")
    i_watch.add_argument("--interval", type=float, default=1.0, help="seconds between reads")
    for i_cmd in (i_add, i_ls, i_ans, i_reply, i_done, i_brain, i_watch):
        i_cmd.set_defaults(func=_cmd_inbox)

    queue = sub.add_parser("queue", parents=[common], help="the manager's own review queue")
    queue_sub = queue.add_subparsers(dest="queue_command", required=True)
    q_ls = queue_sub.add_parser("list", parents=[common], help="open review items")
    q_ls.add_argument("--all", action="store_true", help="include closed items")
    q_ls.add_argument("--json", action="store_true", help="one JSON object per line")
    q_done = queue_sub.add_parser("done", parents=[common], help="close a review item")
    q_done.add_argument("id")
    for q_cmd in (q_ls, q_done):
        q_cmd.set_defaults(func=_cmd_queue)
    action = sub.add_parser("action", parents=[common], help="commands the owner runs with a button")
    action_sub = action.add_subparsers(dest="action_command", required=True)
    a_run = action_sub.add_parser("run", parents=[common], help="run a declared action (owner's click)")
    a_run.add_argument("id")
    a_ls = action_sub.add_parser("list", parents=[common], help="list declared actions with their last run")
    a_ls.add_argument("--json", action="store_true", help="one JSON object per line")
    for a_cmd in (a_run, a_ls):
        a_cmd.set_defaults(func=_cmd_action)
    setup = sub.add_parser("setup", parents=[common], help="see and change a project's tirith.toml setup")
    setup_sub = setup.add_subparsers(dest="setup_command", required=True)
    se_show = setup_sub.add_parser("show", parents=[common], help="pipelines, hooks and rules of a project")
    se_add = setup_sub.add_parser("add-stage", parents=[common], help="add a stage to a pipeline")
    se_add.add_argument("--pipeline", required=True)
    se_add.add_argument("--name", required=True)
    se_add.add_argument("--run", required=True, help="the shell command of the stage")
    se_add.add_argument("--kind", choices=manifest_mod.KINDS, default=None)
    se_add.add_argument("--after", default=None, help="insert after this stage (default: last)")
    se_rm = setup_sub.add_parser("remove-stage", parents=[common], help="remove a stage")
    se_rm.add_argument("--name", required=True)
    se_rm.add_argument("--pipeline", default=None)
    se_rule = setup_sub.add_parser("add-rule", parents=[common], help="add a custom rule ([[rules]])")
    se_rule.add_argument("--id", required=True)
    se_rule.add_argument("--match", default=None, help="command glob, e.g. 'git push --force*'")
    se_rule.add_argument("--regex", default=None, help="regular expression on the command line")
    se_rule.add_argument("--paths", default=None, help="comma-separated globs of files an agent may not edit")
    se_rule.add_argument("--action", choices=manifest_mod.RULE_ACTIONS, default="block")
    se_rule.add_argument("--message", required=True)
    se_rule.add_argument("--instead", required=True)
    se_rule.add_argument("--description", required=True,
                         help="one plain sentence saying what the rule does, for someone who does not code")
    se_dr = setup_sub.add_parser("describe-rule", parents=[common],
                                 help="set a custom rule's description and mark it as up to date")
    se_dr.add_argument("--id", required=True)
    se_dr.add_argument("--description", required=True)
    se_rr = setup_sub.add_parser("remove-rule", parents=[common], help="remove a custom rule")
    se_rr.add_argument("--id", required=True)
    se_apply = setup_sub.add_parser("apply", parents=[common],
                                    help="apply a setup request of the inbox, answer it and close it")
    se_apply.add_argument("id")
    for se_cmd in (se_show, se_add, se_rm, se_rule, se_dr, se_rr):
        se_cmd.add_argument("--project", default=None, help="project id (default: the one of the cwd)")
    for se_cmd in (se_show, se_add, se_rm, se_rule, se_dr, se_rr, se_apply):
        se_cmd.set_defaults(func=_cmd_setup)

    models = sub.add_parser("models", parents=[common],
                            help="the model and effort of the manager and of the agents it starts")
    models_sub = models.add_subparsers(dest="models_command", required=True)
    mo_show = models_sub.add_parser("show", parents=[common], help="the configured models")
    mo_set = models_sub.add_parser("set", parents=[common], help="edit [models] in config.toml")
    mo_set.add_argument("role", choices=models_mod.ROLES)
    mo_set.add_argument("--model", default=None)
    mo_set.add_argument("--effort", default=None)
    mo_set.add_argument("--agent", default=None, help="agent alias")
    mo_set.add_argument("--fallback", default=None,
                        help="workers: agents to use, in order, when the agent is out of credits (a,b; empty = none)")
    mo_set.add_argument("--project", default=None, help="workers: override for this project")
    mo_set.add_argument("--step", default=None, help="workers: override for this step (with --project)")
    mo_set.add_argument("--clear", action="store_true", help="remove the project or step override")
    mo_apply = models_sub.add_parser("apply-manager", parents=[common],
                                     help="write the manager model into the manager agent's user settings")
    for mo_cmd in (mo_show, mo_set, mo_apply):
        mo_cmd.set_defaults(func=_cmd_models)

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

    open_cmd = sub.add_parser("open", parents=[common],
                              help="open a file named in the chat (inside a registered project folder)")
    open_cmd.add_argument("--project", default="", help="the message's project ('' = the tirith repo)")
    open_cmd.add_argument("--path", required=True, help="the path as the message names it")
    open_cmd.add_argument("--line", type=int, default=None)
    open_cmd.add_argument("--col", type=int, default=None)
    open_cmd.set_defaults(func=_cmd_open)
    return parser


def main(argv: list[str] | None = None) -> int:
    """Console entry point; returns the process exit code."""
    parser = build_parser()
    argv = list(sys.argv[1:] if argv is None else argv)
    if argv and argv[0] == "steward":  # hidden alias: the old name of "manager"
        argv[0] = "manager"
    args = parser.parse_args(argv)
    try:
        return int(args.func(args))
    except BrokenPipeError:
        return 1


if __name__ == "__main__":
    sys.exit(main())
