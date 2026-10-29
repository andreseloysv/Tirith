"""No finished lane is left unreviewed: the dispatcher reviews it, by code.

Every collector cycle ``dispatch.run`` calls :func:`run`. A launched lane that ended ``done`` gets one
detached ``tirith lane review <lane>`` (``--tests-only`` when ``[review] auto_merge = false``), once per lane,
state in ``dispatch.json`` (``reviews``). When the review has a result the dispatcher settles it: a merge
answers the owner's request, closes the manager queue item and ticks the plan; a failure opens an owner
item with Run actions and keeps the queue item open. A lane that ended 30 minutes ago with unmerged work
and no review result is a ``review-overdue`` violation. See ``docs/CONTRACT-launch.md``.
"""

from __future__ import annotations

import re
import subprocess
from datetime import datetime
from pathlib import Path
from typing import Callable

from tirith import actions as actions_mod
from tirith import asks as asks_mod
from tirith import claimcheck as claimcheck_mod
from tirith import flows as flows_mod
from tirith import tirithcmd
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith import ownergate as ownergate_mod
from tirith import planlane as planlane_mod
from tirith import projects as projects_mod
from tirith import review as review_mod
from tirith import roadmap as roadmap_mod
from tirith.home import Config, now_iso, parse_iso
from tirith.store import read_lane

OVERDUE_AFTER_S = 1800.0
AUTO_WINDOW_S = 86400.0  # older finished lanes are not merged behind the owner's back; they are overdue
TICK_AT = 0.7            # share of an item's words found in the report's "What changed" lines
CANDIDATE_AT = 0.3
TERMINAL = (review_mod.MERGED, review_mod.TESTS_FAILED, review_mod.CLAIMS_FAILED, review_mod.MERGE_FAILED, review_mod.READY,
            review_mod.DROPPED)
_WORD = re.compile(r"[a-z0-9]{4,}")
_CHANGED = re.compile(r"^\s*(?:#{1,6}\s*|\*\*|__)?\s*what\s+changed\b[^\n]*$", re.IGNORECASE)
_LABEL = re.compile(r"^[A-Za-z][A-Za-z ]{0,30}:(?:\s|$)")  # "Open: none", "Tests: 3 passed"
_HEADING = re.compile(r"^\s*(?:#{1,6}\s+\S|\*\*[^*\n]+\*\*:?\s*$|__[^_\n]+__:?\s*$)")


def repo_of(config: Config, lane: dict) -> Path | None:
    for proj in config.projects:
        if proj.id == lane.get("project") and proj.repo:
            return projects_mod.real(proj.repo)
    return None


def _git(folder: Path, *args: str) -> str:
    done = subprocess.run(["git", "-C", str(folder), *args], capture_output=True, text=True)
    return done.stdout.strip() if done.returncode == 0 else ""


def _age_s(iso: str | None, now: datetime) -> float:
    at = parse_iso(iso)
    return (now - at).total_seconds() if at else 0.0


def close_queue(home: Path, rec: dict, result: str) -> None:
    """Close the review item the dispatcher opened for the lane (not the report's open-points item)."""
    if rec.get("queue") and any(i["id"] == rec["queue"] for i in asks_mod.queue_items(home, only_open=True)):
        asks_mod.queue_done(home, rec["queue"], result)


def unmerged_work(repo: Path, lane: dict) -> bool:
    """True while the lane's worktree has uncommitted changes or commits the repo's branch lacks."""
    workspace = Path(str(lane.get("workspace") or ""))
    if not workspace.is_dir():
        return False
    if _git(workspace, "status", "--porcelain"):
        return True
    return _git(repo, "rev-list", "--count", f"HEAD..lane/{lane['id']}") not in ("", "0")


# -- the plan ---------------------------------------------------------------------------------


def brief_items(brief: str | None) -> list[str]:
    """The open items a generated brief lists under "What to build"."""
    try:
        text = Path(str(brief)).read_text(encoding="utf-8") if brief else ""
    except (OSError, UnicodeDecodeError):
        return []
    items, inside = [], False
    for line in text.splitlines():
        if line.startswith("## "):
            inside = line.strip().lower() == "## what to build"
        elif inside and line.startswith("- "):
            items.append(line[2:].strip())
    return items


def changed_lines(report: str | None) -> str:
    """The "What changed" part of a lane report: from that heading to the next one (inline text counts)."""
    lines = str(report or "").splitlines()
    for index, line in enumerate(lines):
        if not _CHANGED.match(line):
            continue
        inline = line.partition(":")[2].strip(" *_")
        body = [inline] if inline else []
        for later in lines[index + 1:]:
            if (_HEADING.match(later) or _LABEL.match(later)) and not later.lstrip().startswith(("- ", "* ")):
                break
            body.append(later)
        return "\n".join(body)
    return ""


def _words(text: str) -> set[str]:
    return set(_WORD.findall(text.lower()))


def match_items(items: list[str], report: str | None) -> tuple[list[str], list[str]]:
    """``(ticked, candidates)``: items the report's "What changed" lines clearly claim, and the unsure ones."""
    changed = _words(changed_lines(report))
    ticked, candidates = [], []
    for item in items:
        words = _words(item)
        share = len(words & changed) / len(words) if words else 0.0
        if share >= TICK_AT:
            ticked.append(item)
        elif share >= CANDIDATE_AT or not changed:
            candidates.append(item)
    return ticked, candidates


def update_plan(home: Path, repo: Path, lane: dict) -> str:
    """Tick what the report claims in the repo's roadmap and commit it; returns a line for the owner/manager."""
    step = str(lane.get("step") or "")
    items = brief_items(lane.get("brief"))
    if not step or not items or step in planlane_mod.LANE_STEPS.values():
        return ""
    folder = repo
    from tirith import manifest as manifest_mod
    path = roadmap_mod.find_file(folder, manifest_mod.load(folder).roadmap)
    if path is None:
        return ""
    ticked, candidates = match_items(items, lane.get("report"))
    note = ""
    if ticked:
        text = path.read_text(encoding="utf-8")
        new, done = roadmap_mod.tick(text, step, set(ticked))
        rel = str(path.relative_to(repo)) if path.is_relative_to(repo) else str(path)
        if done and new != text and not _git(repo, "status", "--porcelain", "--", rel):
            path.write_text(new, encoding="utf-8")
            subprocess.run(["git", "-C", str(repo), "add", "--", rel], capture_output=True)
            made = subprocess.run(
                ["git", "-C", str(repo), "commit", "-q", "-m", f"Roadmap: step {step} progress from {lane['id']}",
                 "--", rel], capture_output=True, text=True)
            if made.returncode != 0:  # the guard (or git) refused: leave the plan as it was
                subprocess.run(["git", "-C", str(repo), "reset", "-q", "--", rel], capture_output=True)
                subprocess.run(["git", "-C", str(repo), "checkout", "--", rel], capture_output=True)
                candidates = [*done, *candidates]
                note = f"committing the plan failed: {flows_mod.one_line(made.stderr or made.stdout, 200)}"
        elif done:
            candidates = [*done, *candidates]  # the plan has uncommitted edits: do not touch them
            note = "the plan file has uncommitted changes"
    if candidates or not ticked:
        listed = "\n".join(f"- {c}" for c in (candidates or items))
        asks_mod.queue_add(home, str(lane.get("project") or ""), str(lane["id"]),
                           f"update the plan for step {step}: lane {lane['id']} merged, "
                           f"these items may be done (not ticked, check the report)"
                           + (f" ({note})" if note else "") + f":\n{listed}", kind="plan")
    return f"ticked {len(ticked)} of {len(items)} item(s) of step {step}" if ticked and not note else ""


# -- the cycle --------------------------------------------------------------------------------


def _log_hint(home: Path, review: dict, lane_id: str) -> str:
    run = str(review.get("run") or "").replace("#", "-").replace("/", "-")
    found = sorted((home / "logs").glob(f"{run}-*.log")) if run else []
    return str(found[-1] if found else home / "logs" / f"{lane_id}.review.log")


def _actions(rows: list[tuple[str, str]], at: str) -> list[dict]:
    return actions_mod.make_actions([tirithcmd.rewrite(c) for _, c in rows], [label for label, _ in rows],
                                    created_by="tirith", at=at)


def _out_items(repo: Path | None) -> list[str]:
    """The texts of the plan items marked out of this plan (what a report must not ask the owner about)."""
    if repo is None:
        return []
    from tirith import manifest as manifest_mod
    path = roadmap_mod.find_file(repo, manifest_mod.load(repo).roadmap)
    steps = roadmap_mod.parse(path.read_text(encoding="utf-8")) if path else []
    return [i["text"] for step in steps for i in step["items"] if i.get("who") == "out"]


def _owner_item(home: Path, rec: dict, item_id: str, project: str | None, text: str,
                rows: list[tuple[str, str]], at: str) -> None:
    from tirith import dispatch
    if rec.get("owner_item") and rec["owner_item"] != item_id:
        dispatch.close_owner_item(home, rec["owner_item"], at)
    dispatch.add_owner_item(home, item_id, project, text, [], at, actions=_actions(rows, at),
                            why_owner="owner-decision", why_detail="choice: merge it or drop it")
    rec["owner_item"] = item_id


def settle(home: Path, config: Config, lane: dict, review: dict, rec: dict, now: datetime,
           repo: Path | None) -> None:
    """Act on a finished review once (per review result): answer, close, or ask the owner."""
    from tirith import dispatch
    at, lane_id = now_iso(now), str(lane["id"])
    project = str(lane.get("project") or "") or None
    status, step = review["status"], str(lane.get("step") or "")
    feature = str(lane.get("feature") or lane_id)
    if status == review_mod.MERGED:
        passed = review.get("passed")
        text = (f"Merged {lane_id} into {review.get('branch') or 'the project branch'}: "
                f"{review.get('files', 0)} files, tests {f'{passed} passed' if passed is not None else 'passed'}")
        plan = update_plan(home, repo, lane) if repo is not None else ""
        answer = text + (f". {plan}." if plan else ".")
        if step in planlane_mod.LANE_STEPS.values() and str(lane.get("report") or "").strip():
            # the auditor's summary / the planner's report; its "needs the owner" list passes the owner gate
            answer += "\n\n" + ownergate_mod.filter_report(
                home, project, lane_id, str(lane["report"]).strip(), out_items=_out_items(repo), at=at)
        if lane.get("request_id") and inbox_mod.find_request(home, str(lane["request_id"])):
            inbox_mod.add_answer(home, str(lane["request_id"]), answer,
                                 by="tirith", auto=True, at=at, lane=lane_id)
        close_queue(home, rec, text)
        if rec.get("owner_item"):
            dispatch.close_owner_item(home, rec["owner_item"], at)
        flows_mod.append_flow(home, "tirith", "owner", "answer", text, session=None, project=project, at=at)
        return
    if status == review_mod.DROPPED:
        close_queue(home, rec, f"lane {lane_id} dropped")
        if rec.get("owner_item"):
            dispatch.close_owner_item(home, rec["owner_item"], at)
        return
    if status == review_mod.CLAIMS_FAILED:
        _claims_failed(home, config, lane, review, rec, project, at)
        return
    key = f"review-{lane_id}-{re.sub(r'[^0-9]', '', str(review.get('at') or ''))}"
    drop = ("Drop the lane", f"tirith lane drop {lane_id}")
    if status == review_mod.READY:  # auto_merge is off: the owner decides
        text = f"Tests passed for **{feature}** ({lane_id}). Merge it into the project branch?"
        rows = [("Merge", f"tirith lane review {lane_id}"), ("Drop", drop[1])]
    else:
        why = "the tests failed" if status == review_mod.TESTS_FAILED else "the merge did not go through"
        text = f"Review of **{feature}** ({lane_id}): {why}. " + flows_mod.one_line(review.get("message") or "", 240)
        rows = [("Retry the review", f"tirith lane review {lane_id}")]
        if step and project:
            rows.append(("Start an agent to fix it", f"tirith launch --project {project} --step {step}"))
        rows += [("Show the log", f"tail -n 80 {_log_hint(home, review, lane_id)}"), drop]
        # employees report to the manager: a failed review is the manager's job, never the owner's
        _manager_item(home, rec, project, lane_id, text, rows)
        flows_mod.append_flow(home, "tirith", "manager", "alert", text, session=None, project=project, at=at)
        return
    _owner_item(home, rec, key, project, text, rows, at)  # auto_merge off: the owner chose to decide


def _brief_text(lane: dict) -> str:
    try:
        return Path(str(lane.get("brief") or "")).read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def _claims_failed(home: Path, config: Config, lane: dict, review: dict, rec: dict, project: str | None,
                   at: str) -> None:
    """The report said things the diff does not show: the lane goes back to its agent once with the list
    (never to the owner); a lane that was already sent back goes to the manager's queue."""
    lane_id = str(lane["id"])
    note = str(review.get("message") or "")
    text = (f"Review of {lane.get('feature') or lane_id} ({lane_id}): the report does not match the change. "
            + flows_mod.one_line(note.replace(claimcheck_mod.MARK, ""), 400))
    sent = None
    if claimcheck_mod.MARK not in _brief_text(lane):
        alias = str(lane.get("agent") or "")
        if alias in config.agents:
            try:
                sent = relauncher(home, lane, alias, note)
            except (Exception, SystemExit) as error:  # the manager decides when it cannot be sent back
                text += f" (sending it back failed: {flows_mod.one_line(str(error), 120)})"
    if sent:
        rec["claims_sent"] = sent
        close_queue(home, rec, f"claims did not match; sent back to the agent as {sent}")
        flows_mod.append_flow(home, "tirith", f"agent:{sent}", "brief", f"Sent back: {text}", session=None,
                              project=project, at=at)
        return
    rows = [("Retry the review", f"tirith lane review {lane_id}"), ("Drop the lane", f"tirith lane drop {lane_id}")]
    _manager_item(home, rec, project, lane_id, text, rows)
    flows_mod.append_flow(home, "tirith", "manager", "alert", text, session=None, project=project, at=at)


def relauncher(home: Path, lane: dict, alias: str, note: str) -> str:
    from tirith.cli import relaunch_lane  # late: cli imports this module
    return relaunch_lane(home, lane, alias, by_tirith=True, note=note)


def _manager_item(home: Path, rec: dict, project: str | None, lane_id: str, text: str,
                  rows: list[tuple[str, str]]) -> None:
    """One open item in the manager's queue with the commands it can run (once per review result)."""
    commands = "\n".join(f"- {label}: {command}" for label, command in rows)
    if rec.get("queue"):  # the review's own queue item is replaced by this one, which says what failed
        try:
            asks_mod.queue_done(home, rec["queue"])
        except asks_mod.AskError:
            pass
    item = asks_mod.queue_add(home, project or "", lane_id, f"{text}\n{commands}")
    rec["queue"] = item["id"]


def _result_key(review: dict) -> str:
    """Identity of a review result: status plus time (a retry can finish in the same second as the failure)."""
    return f"{review.get('status') or ''}@{review.get('at') or ''}" if review.get("at") else ""


def run(home: Path, config: Config, now: datetime, state: dict, lanes: list[dict], *,
        spawn: Callable | None = None, notify: Callable[[str, str], None] | None = None) -> bool:
    """One pass over the finished lanes; returns True when ``state`` changed."""
    spawn = spawn or launch_mod.spawn_detached
    changed = False
    book = state.setdefault("reviews", {})
    overdue = state.setdefault("overdue", {})
    for seen in lanes:
        lane_id = str(seen.get("id") or "")
        lane = read_lane(home, lane_id) if lane_id else None
        if not isinstance(lane, dict) or not lane.get("ended_at") or lane.get("result") != "done":
            continue
        repo = repo_of(config, lane)
        if repo is None or not review_mod.launched(lane, repo):
            continue
        review = review_mod.public(lane, repo) or {}
        rec = book.get(lane_id)
        age = _age_s(lane.get("ended_at"), now)
        if rec is None:
            if review.get("status") not in (review_mod.PENDING, None) or not Path(str(lane["workspace"])).is_dir():
                book[lane_id] = {"seen": now_iso(now), "settled": _result_key(review) or "gone"}  # nothing to start
                changed = True
                continue
            if not unmerged_work(repo, lane):  # merged by hand: nothing to review, settled
                book[lane_id] = {"seen": now_iso(now), "settled": "merged"}
                changed = True
                continue
            if age <= AUTO_WINDOW_S:
                item = asks_mod.queue_add(home, str(lane.get("project") or ""), lane_id,
                                          f"Review lane {lane_id} ({lane.get('feature') or ''}): tests, then merge")
                argv = [*launch_mod.tirith_argv(home), "lane", "review", lane_id]
                if not config.review_auto_merge:
                    argv.append("--tests-only")
                spawn(argv, home, home / "logs" / f"{lane_id}.review.log")
                book[lane_id] = {"started": now_iso(now), "queue": item["id"]}
                changed = True
                continue
        elif review.get("status") in TERMINAL and rec.get("settled") != _result_key(review):
            settle(home, config, lane, review, rec, now, repo)
            rec["settled"] = _result_key(review)
            changed = True
            continue
        stale = review.get("status") in (None, review_mod.PENDING) or (
            review.get("status") == review_mod.MERGE_FAILED and "interrupted" in str(review.get("message")))
        if stale and age >= OVERDUE_AFTER_S and lane_id not in overdue:
            if unmerged_work(repo, lane):
                flag_overdue(home, lane, age, now, notify)
                overdue[lane_id] = now_iso(now)
                changed = True
    return changed


def flag_overdue(home: Path, lane: dict, age: float, now: datetime, notify: Callable | None) -> None:
    from tirith import dispatch, gates
    lane_id, project = str(lane["id"]), str(lane.get("project") or "") or None
    at = now_iso(now)
    minutes = int(age // 60)
    text = f"Lane {lane_id} ended {minutes} min ago with unmerged work and no review result"
    gates.record_violation(
        home, gates.Violation("review-overdue", text, f"tirith lane review {lane_id}"),
        ts=at, agent="anthropic", lane=lane_id, session=None, project=project, tool=None, cwd=None)
    (notify or dispatch.macos_notify)("Tirith", text)  # a no-op: no OS notifications
    _manager_item(home, {}, project, lane_id, text + ". Review it now or drop it.",
                  [("Review it now", f"tirith lane review {lane_id}"), ("Drop the lane", f"tirith lane drop {lane_id}")])
    flows_mod.append_flow(home, "tirith", "manager", "alert", text, session=None, project=project, at=at)
