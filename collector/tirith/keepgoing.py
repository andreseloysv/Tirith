"""Keep going until done: a lane that ends with its contract unmet is resumed by code.

When the agent of a ``lane run --as-agent <alias>`` ends, ``gaps`` checks what the owner's rules ask of a
finished lane: a plan report (``tirith plan report``), a report whose claims hold (``claimcheck``), the
quick tests recorded green after the last change, the work committed on the lane's branch and a branch
that merges cleanly with main. A lane whose agent ends while a stage it started is still running is not
finished either: ``wait_for_stages`` waits for the stage and ``stage_message`` tells the same session its result. Anything missing is sent back to the SAME agent session (``resume_command``:
the alias's resume flag with the session id read from the stream log; an alias without one starts again
with the brief and its previous report) with a short message that lists exactly what is missing. Up to
``MAX_ROUNDS`` rounds; after them one manager-queue item holds the gaps (``queue_gaps``). The checks only
read; no gate, claim check or plan rule is relaxed by this.
"""

from __future__ import annotations

import os
import re
import time
from pathlib import Path

from tirith import agentrun as agentrun_mod
from tirith import asks as asks_mod
from tirith import claimcheck as claimcheck_mod
from tirith import plangate as plangate_mod
from tirith import timeouts as timeouts_mod
from tirith.home import Config, parse_iso
from tirith.store import lane_path, read_json, read_jsonl, write_lane

MAX_ROUNDS = 3
QUEUE_KIND = "lane-incomplete"
RESUME_FLAGS = {"anthropic": "--resume"}  # alias preset -> flag that continues a session by id
_DOC = re.compile(r"(^docs/|\.(md|txt|rst)$)", re.IGNORECASE)
MAX_STAGE_WAITS = 20  # stage-result resumes per lane; each one is progress, this only stops a runaway
STAGE_GRACE_S = 120  # past the stage's own timeout (which kills it) before the wait gives up
POLL_S = 2.0
_FAILED_TEST = re.compile(r"^(?:FAILED|ERROR)\s+(\S+)", re.MULTILINE)
_SESSION = re.compile(r'"session_id"\s*:\s*"([^"]+)"')


def _git(workspace: Path, *args: str):
    return timeouts_mod.run(["git", "-C", str(workspace), *args], "git", text=True)


def _base_branch(workspace: Path) -> str | None:
    for name in ("main", "master"):
        if _git(workspace, "rev-parse", "--verify", "-q", name).returncode == 0:
            return name
    return None


def session_id(log: str | Path | None) -> str | None:
    """The agent's session id: the last ``session_id`` of the JSON stream in its log."""
    if not log:
        return None
    try:
        text = Path(log).read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None
    found = _SESSION.findall(text)
    return found[-1] if found else None


def gaps(home: Path, lane: dict, repo: Path | None, report: str) -> list[str]:
    """What is missing for ``lane`` to be finished, one sentence with the command to fix it each."""
    out: list[str] = []
    lane_id = str(lane.get("id") or "")
    workspace = Path(str(lane.get("workspace") or ""))
    fresh = read_json(lane_path(home, lane_id), None)
    lane = fresh if isinstance(fresh, dict) else lane
    if plangate_mod.lane_item(lane) is not None and not plangate_mod.standing(lane) and not lane.get("outcome"):
        out.append("no plan report: run `tirith plan report --status done` (or `partial --add \"<what is "
                   "left>\"` / `blocked --reason \"<why>\"`) when the work is finished")
    if not workspace.is_dir():
        return out
    if report.strip():
        out += [f"claim check: {m}" for m in claimcheck_mod.verify(home, lane, repo, report=report).mismatches]
    _, paths = claimcheck_mod.lane_diff(workspace, lane.get("start_commit"))
    code = [p for p in paths if not _DOC.search(p)]
    if code:
        runs = claimcheck_mod.test_runs(home, workspace)
        since = claimcheck_mod.last_change(workspace, paths)
        if not any(r["at"] + 1 >= since for r in runs):
            out.append("no passing test run recorded after your last change: run "
                       "`tirith run --project <project> --stage test` (and `--stage build`) in your worktree")
    dirty = _git(workspace, "status", "--porcelain")
    if dirty.returncode == 0 and dirty.stdout.strip():
        out.append("uncommitted changes in your worktree: commit them on your branch with a normal "
                   "`git commit` (the guard runs; never skip hooks)")
    branch = _git(workspace, "rev-parse", "--abbrev-ref", "HEAD").stdout.strip()
    base = _base_branch(workspace)
    if base and branch and branch not in (base, "HEAD"):
        merged = _git(workspace, "merge-tree", "--write-tree", base, branch)
        if merged.returncode == 1:
            out.append(f"branch {branch} does not merge cleanly with {base}: merge {base} into your branch, "
                       "solve the conflicts and run the tests again")
    return out


def has_work(workspace: Path, start_commit: str | None) -> bool:
    """True when the lane's worktree holds work: uncommitted changes, or commits since the lane started."""
    if not Path(workspace).is_dir():
        return False
    dirty = _git(workspace, "status", "--porcelain")
    if dirty.returncode == 0 and dirty.stdout.strip():
        return True
    if start_commit:
        ahead = _git(workspace, "rev-list", "--count", f"{start_commit}..HEAD")
        return ahead.returncode == 0 and ahead.stdout.strip() not in ("", "0")
    return False


def idle_message(minutes: int, round_no: int) -> str:
    """Sent to a lane's agent that produced no progress for ``minutes``."""
    return (f"You were idle for {minutes} min (no output, file change or command activity; attempt {round_no} of "
            f"{MAX_ROUNDS}). Your work in the worktree is kept. Continue where you stopped, or report if you are "
            "finished or blocked.")


def note_idle(home: Path, lane_id: str, round_no: int, minutes: int) -> None:
    """Keep on the lane that it was resumed after being idle."""
    lane = read_json(lane_path(home, lane_id), None)
    if isinstance(lane, dict):
        lane["idle_resumes"] = round_no
        lane["idle_minutes"] = minutes
        write_lane(home, lane)


def message(missing: list[str], round_no: int) -> str:
    """The short message sent to the agent: what is missing and what to do."""
    listed = "\n".join(f"{n}. {m}" for n, m in enumerate(missing, 1))
    return (f"Tirith checked your finished work (round {round_no} of {MAX_ROUNDS}) and it is not complete yet. "
            f"Missing:\n{listed}\n\nDo exactly these, then finish with your final report. Do not redo work "
            "that is already done.")


def prompt_flag_index(command: list[str], flag: str) -> int | None:
    try:
        return command.index(flag)
    except ValueError:
        return None


def resume_command(config: Config, alias: str, project: str | None, command: list[str], sid: str | None,
                   text: str, previous_report: str, brief: str, left: str = "") -> tuple[list[str], str | None] | None:
    """``(argv, stdin text)`` that sends ``text`` to the session ``sid`` of ``alias``; without a resume flag
    (or an id) a fresh run of the alias with the brief, its previous report and ``text``. None: not buildable."""
    try:
        settings = agentrun_mod.agent_settings(config, alias, project)
    except SystemExit:
        settings = {}
    flag = settings.get("resume_flag") or RESUME_FLAGS.get(alias)
    print_flag = settings.get("print_flag")
    if flag and sid and print_flag and (at := prompt_flag_index(command, str(print_flag))) is not None:
        rest = command[at + 1:]
        inline = bool(rest) and not rest[0].startswith("-")
        head = command[:at + 1] + ([text] if inline else [])
        tail = rest[1:] if inline else rest
        return [*head, str(flag), sid, *tail], (None if inline else text)
    prompt = (f"{brief}\n\n--- Your previous run on this brief ended with this report ---\n{previous_report}\n\n"
              + (f"--- Worktree ---\n{left}\n\n" if left else "") + f"--- Tirith ---\n{text}")
    try:
        stdin = agentrun_mod.wants_stdin(config, alias, project, prompt)
        built = agentrun_mod.build_command(config, alias, project, prompt, via_stdin=stdin)
    except SystemExit:
        return None
    return built, (prompt if stdin else None)


def queue_gaps(home: Path, lane: dict, missing: list[str]) -> None:
    """After ``MAX_ROUNDS`` rounds: one manager-queue item with the gaps that remain."""
    listed = "\n".join(f"- {m}" for m in missing)
    asks_mod.queue_add(
        home, str(lane.get("project") or ""), str(lane["id"]),
        f"Lane **{lane.get('feature') or lane['id']}** ({lane['id']}) still has gaps after {MAX_ROUNDS} "
        f"rounds of being sent back:\n{listed}\nLook at it: tirith lane review {lane['id']}",
        kind=QUEUE_KIND, item_id=f"sq-incomplete-{lane['id']}")


def round_label(lane: dict) -> str | None:
    """``"round 2/3"`` for the Agents view, or None when the lane was never sent back."""
    info = lane.get("keep_going")
    if isinstance(info, dict) and info.get("round"):
        return f"round {int(info['round'])}/{int(info.get('max') or MAX_ROUNDS)}"
    return None


def note_round(home: Path, lane_id: str, round_no: int, missing: list[str]) -> dict:
    """Record the round in the lane file (what the Agents view shows)."""
    lane = read_json(lane_path(home, lane_id), None) or {}
    lane["keep_going"] = {"round": round_no, "max": MAX_ROUNDS, "missing": missing}
    write_lane(home, lane)
    return lane


def infer_alias(config: Config, command: list[str]) -> str | None:
    """The alias whose ``[agents.<alias>] command`` is the executable of ``command``, else None."""
    if not command:
        return None
    exe = Path(str(command[0])).name
    for alias, table in config.agents.items():
        if isinstance(table, dict) and table.get("command") and Path(str(table["command"])).name == exe:
            return str(alias)
    return None


def _alive(pid) -> bool:
    try:
        os.kill(int(pid), 0)
    except (OSError, TypeError, ValueError):
        return False
    return True


def _in_lane(event: dict, lane_id: str, workspace: Path) -> bool:
    if lane_id and event.get("lane") == lane_id:
        return True
    for key in ("worktree", "cwd"):
        where = event.get(key)
        if where:
            try:
                if Path(str(where)).resolve().is_relative_to(workspace.resolve()):
                    return True
            except (OSError, ValueError):
                pass
    return False


def stage_runs(home: Path, lane_id: str, workspace: Path, since: float = 0.0,
               skip: set | None = None) -> list[dict]:
    """Stage runs recorded for the lane or its worktree after ``since``: the start event, plus ``end``
    (the end event or None) and ``alive`` (its process still exists)."""
    open_runs: dict[tuple, dict] = {}
    for event in read_jsonl(home / "events.jsonl"):
        if event.get("event") not in ("start", "end") or not event.get("stage"):
            continue
        if (skip and event.get("log") in skip) or not _in_lane(event, lane_id, workspace):
            continue
        key = (event.get("run"), event.get("stage"), event.get("log"))
        if event["event"] == "start":
            at = parse_iso(event.get("at"))
            if at and at.timestamp() >= since:
                open_runs[key] = {**event, "end": None, "alive": _alive(event.get("pid"))}
        elif key in open_runs:
            open_runs[key]["end"] = event
    return list(open_runs.values())


def running_stages(runs: list[dict]) -> list[dict]:
    return [r for r in runs if r["end"] is None and r["alive"]]


def wait_for_stages(home: Path, lane_id: str, workspace: Path, since: float = 0.0,
                    skip: set | None = None) -> list[dict]:
    """Block while a stage of the lane is running; the stage runs of the lane when none is left.

    The stage's own timeout policy ends it (and records its end); this only gives up ``STAGE_GRACE_S``
    after that limit. Empty: the agent started no stage."""
    runs = stage_runs(home, lane_id, workspace, since, skip)
    while (live := running_stages(runs)):
        now = time.time()
        started = min((parse_iso(r.get("at")) for r in live if parse_iso(r.get("at"))), default=None)
        limit = max(float(r.get("timeout") or 0) for r in live) + STAGE_GRACE_S
        if started and now - started.timestamp() > limit:
            break
        time.sleep(POLL_S)
        runs = stage_runs(home, lane_id, workspace, since, skip)
    return runs


def _failing_tests(log) -> list[str]:
    try:
        text = Path(str(log)).read_text(encoding="utf-8", errors="replace")[-200_000:]
    except OSError:
        return []
    return list(dict.fromkeys(_FAILED_TEST.findall(text)))[:10]


def stage_message(runs: list[dict]) -> str:
    """What the agent is told when it ended while its stage ran: stage, result, failing tests, log path."""
    lines = []
    for run in runs:
        end = run["end"]
        if end is None:
            result = "did not finish (no end recorded)"
        elif end.get("timed_out"):
            result = "timed out"
        else:
            result = str(end.get("status") or "unknown")
        failing = _failing_tests(run.get("log")) if result != "passed" else []
        line = f"- stage `{run.get('stage')}` of {run.get('project') or 'the project'}: {result}; log: {run.get('log')}"
        if failing:
            line += "; failing tests: " + ", ".join(failing)
        lines.append(line)
    return ("You ended your turn while a stage you started was still running. Tirith waited for it. Result:\n"
            + "\n".join(lines) + "\n\nContinue from this result and finish with your final report.")


def previous_run_note(workspace: Path, start_commit: str | None) -> str:
    """For a relaunch from scratch: what the previous run left in the worktree."""
    if not Path(workspace).is_dir():
        return ""
    parts = []
    if start_commit:
        log = _git(workspace, "log", "--oneline", f"{start_commit}..HEAD")
        if log.returncode == 0 and log.stdout.strip():
            parts.append("commits on the branch:\n" + log.stdout.strip())
    status = _git(workspace, "status", "--porcelain")
    if status.returncode == 0 and status.stdout.strip():
        parts.append("uncommitted changes:\n" + status.stdout.strip())
    if not parts:
        return "The previous run left nothing in the worktree."
    return "The previous run left this in the worktree (keep it, do not redo it):\n" + "\n".join(parts)
