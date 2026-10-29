"""Keep going until done: a lane that ends with its contract unmet is resumed by code.

When the agent of a ``lane run --as-agent <alias>`` ends, ``gaps`` checks what the owner's rules ask of a
finished lane: a plan report (``tirith plan report``), a report whose claims hold (``claimcheck``), the
quick tests recorded green after the last change, the work committed on the lane's branch and a branch
that merges cleanly with main. Anything missing is sent back to the SAME agent session (``resume_command``:
the alias's resume flag with the session id read from the stream log; an alias without one starts again
with the brief and its previous report) with a short message that lists exactly what is missing. Up to
``MAX_ROUNDS`` rounds; after them one manager-queue item holds the gaps (``queue_gaps``). The checks only
read; no gate, claim check or plan rule is relaxed by this.
"""

from __future__ import annotations

import re
from pathlib import Path

from tirith import agentrun as agentrun_mod
from tirith import asks as asks_mod
from tirith import claimcheck as claimcheck_mod
from tirith import plangate as plangate_mod
from tirith import timeouts as timeouts_mod
from tirith.home import Config
from tirith.store import lane_path, read_json, write_lane

MAX_ROUNDS = 3
QUEUE_KIND = "lane-incomplete"
RESUME_FLAGS = {"anthropic": "--resume"}  # alias preset -> flag that continues a session by id
_DOC = re.compile(r"(^docs/|\.(md|txt|rst)$)", re.IGNORECASE)
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
                   text: str, previous_report: str, brief: str) -> tuple[list[str], str | None] | None:
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
              f"--- Tirith ---\n{text}")
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
