"""Tirith checks itself, and fixes what it can before the owner notices.

Every dispatcher pass ``dispatch.run`` calls :func:`run` (cheap: no process is started unless a remedy needs
one). Five checks, results in ``dispatch.json`` under ``health`` and, in plain words, ``state.health``:

* ``collector``: the gap since the last pass (the app supervisor restarts the collector; here the gap is kept);
* ``manager``: the oldest chat message without an answer while no manager turn is working on it, or a manager
  process still alive after its ``result``;
* ``autopilot``: open plan items nobody works on and nobody was told about, or proposals the manager leaves
  unanswered for 10 min;
* ``disk``: below the disk-guard limit;
* ``reviews``: a review stuck in ``running``, or ``tests-failed`` on tests the lane's diff does not touch.

A failing check gets a remedy by code where that is safe (restart the manager turn, end a lingering manager
process, run a review again), at most ``MAX_TRIES`` times per episode and never twice within ``GRACE_S``;
otherwise (or when the tries are used up) one manager queue item with the evidence, never an owner item.
After a remedy the manager says in the chat, in one calm line, what happened. The header dot is green, amber
while a remedy is working and red only when a remedy failed. See ``docs/CONTRACT-selfcheck.md``.
"""

from __future__ import annotations

import re
import subprocess
from datetime import datetime
from pathlib import Path
from typing import Callable

from tirith import timeouts as timeouts_mod
from tirith import asks as asks_mod
from tirith import autopilot as autopilot_mod
from tirith import diskdiet as diskdiet_mod
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith import managerd as managerd_mod
from tirith import review as review_mod
from tirith import timeouts as timeouts_mod
from tirith.home import Config, now_iso, parse_iso
from tirith.store import read_lane

MANAGER_STUCK_S = 120.0      # an owner message without an answer, nobody working on it
LINGER_S = 60.0              # a manager process alive this long after its result
AUTOPILOT_IDLE_S = 120.0     # an open item nobody works on and no proposal for it
PROPOSAL_STUCK_S = 600.0     # a proposal the manager has not answered
REVIEW_STUCK_S = 1200.0      # a review in "running"
TIMEOUTS_WINDOW_S = 3600.0   # timeouts of one class are counted over this long
TIMEOUTS_REPEATED = 3        # this many in the window is not chance
COLLECTOR_GAP_S = 150.0      # no pass for this long: the collector was down
GAP_SHOWN_S = 600.0          # how long the gap stays in the result
GRACE_S = 90.0               # a remedy gets this long to work before the next try
MAX_TRIES = 3
RESTARTS_WINDOW_S = 3600.0   # fresh starts of the manager session are counted over this long
MAX_RESTARTS_PER_HOUR = 2    # more than this is a problem
NOTICE_KEPT_S = 1800.0       # a notice whose messages were never answered is dropped after this
SAVE_EVERY_S = 30.0
QUEUE_KIND = "selfcheck"
LEVELS = ("ok", "healing", "failed")
LABELS = {"collector": "Tirith itself", "manager": "The manager", "autopilot": "The autopilot",
          "disk": "Disk space", "reviews": "Reviews", "timeouts": "Command timeouts"}
OK_TEXT = {"collector": "Tirith is collecting normally.", "manager": "The manager is answering normally.",
           "autopilot": "The autopilot is on track.", "disk": "There is enough free disk space.",
           "reviews": "Reviews are moving.", "timeouts": "No command keeps running out of time."}
_TEST_FILE = re.compile(r"[\w./-]*(?:test_\w+\.py|[\w-]+\.test\.mjs)")


def _book(state: dict) -> dict:
    book = state.setdefault("health", {})
    for key in ("tries", "queued", "notices"):
        if not isinstance(book.get(key), dict):
            book[key] = {}
    return book


def _age_s(iso: str | None, now: datetime) -> float:
    at = parse_iso(iso) if iso else None
    return (now - at).total_seconds() if at else 0.0


def _minutes(seconds: float) -> int:
    return max(1, round(seconds / 60))


def _check(level: str, text: str, **extra) -> dict:
    return {"level": level, "text": text, **extra}


# -- remedies and the manager queue -----------------------------------------------------------


def _attempt(book: dict, key: str, now: datetime, act: Callable[[], None], max_tries: int = MAX_TRIES) -> str:
    """Try the remedy ``act`` unless one ran less than ``GRACE_S`` ago: ``acted`` (it ran now), ``healing`` (it
    ran lately) or ``failed`` (the tries are used up)."""
    rec = book["tries"].setdefault(key, {"n": 0, "at": None})
    if rec["at"] and _age_s(rec["at"], now) < GRACE_S:
        return "healing"
    if rec["n"] >= max_tries:
        return "failed"
    rec["n"] += 1
    rec["at"] = now_iso(now)
    act()
    return "acted"


def _queue(home: Path, book: dict, key: str, project: str, lane: str, text: str) -> None:
    """One manager queue item per episode (never an owner item)."""
    if key not in book["queued"]:
        slug = re.sub(r"[^a-z0-9]+", "-", key.lower()).strip("-")
        book["queued"][key] = asks_mod.queue_add(home, project, lane, text, kind=QUEUE_KIND,
                                                 item_id=f"sq-selfcheck-{slug}")["id"]


def _notice(book: dict, key: str, text: str, now: datetime, waiting: list[str] | None = None) -> None:
    """Something to tell the owner in the chat: now, or once ``waiting`` (request ids) are answered."""
    book["notices"].setdefault(key, {"text": text, "wait": list(waiting or []), "at": now_iso(now)})


def _announce(home: Path, book: dict, now: datetime) -> bool:
    open_ids = {r["id"] for r in inbox_mod.requests(home) if r["manager_pending"]}
    changed = False
    for key, note in list(book["notices"].items()):
        if _age_s(note["at"], now) > NOTICE_KEPT_S:
            del book["notices"][key]
            changed = True
        elif not open_ids.intersection(note["wait"]):
            inbox_mod.add_say(home, note["text"].format(n=len(note["wait"])), at=now_iso(now))
            del book["notices"][key]
            changed = True
    return changed


# -- the checks -------------------------------------------------------------------------------


def _collector(book: dict, now: datetime) -> dict:
    gap = _age_s(book.get("at"), now) if book.get("at") else 0.0
    if gap > COLLECTOR_GAP_S:
        book["gap"] = {"s": int(gap), "at": now_iso(now)}
    last = book.get("gap")
    if last and _age_s(last["at"], now) < GAP_SHOWN_S:
        return _check("ok", f"Tirith was paused for {_minutes(last['s'])} min and is running again.",
                      gap_s=last["s"])
    return _check("ok", OK_TEXT["collector"])


def _chat_waiting(home: Path, now: datetime) -> list[dict]:
    """Chat messages the manager owes an answer, and autopilot proposals unanswered for ``PROPOSAL_STUCK_S``."""
    return [r for r in inbox_mod.requests(home) if r["manager_pending"] and (
        r["kind"] == "chat" or (r["kind"] == inbox_mod.PROPOSAL and _age_s(r["at"], now) > PROPOSAL_STUCK_S))]


def _manager(home: Path, config: Config, now: datetime, state: dict, book: dict, lanes: list[dict],
             kill: Callable, active: set[str]) -> dict:
    if not config.dispatch_auto_launch:
        return _check("ok", OK_TEXT["manager"])
    mbook = state.setdefault("manager", {})
    churn = managerd_mod.restarts_within(mbook, now, RESTARTS_WINDOW_S)
    if len(churn) > MAX_RESTARTS_PER_HOUR:
        key = "manager-restarts"
        active.add(key)
        _queue(home, book, key, str((mbook.get("session") or {}).get("project") or ""), "",
               f"The manager session was restarted fresh {len(churn)} times in the last hour (at "
               f"{', '.join(t[11:19] for t in churn)} UTC); compacting its conversation is not bringing the context "
               f"down. Last stop: {(mbook.get('session') or {}).get('ended_at')}. Look at the manager's flows "
               "(\"Manager compacted\" rows show the sizes before and after).")
        return _check("failed", f"The manager session was restarted {len(churn)} times in an hour.")
    mine = {mbook.get("lane"), mbook.get("ended")} - {None, ""}
    lane = next((l for l in lanes if l.get("id") in mine), None)
    if lane and not lane.get("ended_at") and managerd_mod.pid_alive(lane.get("pid")):
        done = managerd_mod.result_at(lane)
        if done is not None and (now - done).total_seconds() >= LINGER_S:
            key = f"manager-linger:{lane['id']}"
            active.add(key)
            state_ = _attempt(book, key, now, lambda: kill(lane))
            if state_ != "failed":
                if state_ == "acted":
                    _notice(book, key, "A manager process stayed open after answering; I closed it.", now)
                return _check("healing", "A manager process stayed open after answering; closing it.")
            _queue(home, book, key, str(lane.get("project") or ""), str(lane["id"]),
                   f"The manager process of lane {lane['id']} is still alive {_minutes((now - done).total_seconds())} "
                   f"min after its answer and could not be closed. pid {lane.get('pid')}, pgid {lane.get('pgid')}.")
            return _check("failed", "A manager process stayed open and could not be closed.")
    waiting = _chat_waiting(home, now)
    if not waiting or managerd_mod.attached(home, now) or managerd_mod.running(mbook, lanes, now) \
            or managerd_mod.session_busy(home):
        return _check("ok", OK_TEXT["manager"])
    oldest = min(waiting, key=lambda r: str(r["at"] or ""))
    age = _age_s(oldest["at"], now)
    if age <= MANAGER_STUCK_S and oldest["kind"] == "chat":
        return _check("ok", OK_TEXT["manager"])
    key = f"manager-stuck:{oldest['id']}"
    active.add(key)
    what = "the owner's chat message" if oldest["kind"] == "chat" else "the autopilot proposal"
    evidence = (f"Self-check: {what} {oldest['id']} has waited {int(age)} s with no manager "
                f"turn working on it ({len(waiting)} message(s) waiting; manager state: tries "
                f"{mbook.get('tries')}, last error {mbook.get('error') or 'none'}; "
                f"session: {(mbook.get('session') or {}).get('error') or 'none'}). Answer them first.")

    def restart() -> None:
        for field in ("key", "at", "error"):
            mbook.pop(field, None)
        mbook["tries"] = 0
        mbook["evidence"] = evidence  # in the brief of the next manager turn

    paused = mbook.get("paused")
    if paused and any(q["id"] == paused for q in asks_mod.queue_items(home, only_open=True)):
        result = "failed"  # the pause has its own queue item with the log tail
    else:
        result = _attempt(book, key, now, restart)
    if result != "failed":
        if result == "acted":
            _notice(book, key, f"The manager got stuck for {_minutes(age)} min; I restarted it and answered your "
                               "{n} message(s).", now, [r["id"] for r in waiting])
        return _check("healing", "The manager was stuck; restarting it.")
    _queue(home, book, key, oldest["project"], "", evidence + " Restarting it did not help.")
    return _check("failed", "The manager is not answering and restarting it did not help.")


def _autopilot(config: Config, now: datetime, state: dict, book: dict, home: Path, agents: list[dict],
               projects: list[dict], active: set[str]) -> dict:
    if not autopilot_mod.effective(config, state):
        book.pop("idle", None)
        return _check("ok", "The autopilot is off.")
    if diskdiet_mod.low(state):
        book.pop("idle", None)
        return _check("ok", OK_TEXT["autopilot"])
    open_items = autopilot_mod.sitting(home, state, projects, agents, now)
    late = [c for c in open_items if c["idle_s"] >= AUTOPILOT_IDLE_S]
    waiting = [r for r in autopilot_mod.unanswered(home, state, now) if r["age_s"] >= PROPOSAL_STUCK_S]
    if not late and not waiting:
        book.pop("idle", None)
        return _check("ok", OK_TEXT["autopilot"])
    if waiting and not late:
        n = len(waiting)
        return _check("healing", f"{n} proposal{'s' if n != 1 else ''} of the autopilot "
                                 f"wait{'s' if n == 1 else ''} for the manager.")
    book.setdefault("idle", now_iso(now))
    key = "autopilot-idle"
    active.add(key)
    items = "; ".join(f"{c['project']} step {c['step']}: {c['item'][:80]}" for c in late[:5])
    _queue(home, book, key, late[0]["project"], "",
           f"Self-check: {len(late)} open plan item(s) have sat with nobody on them and no proposal for "
           f"{_minutes(max(c['idle_s'] for c in late))} min: {items}. Decide for each: launch it with a written "
           "brief, combine it with a lane, wait (say why) or drop it.")
    return _check("healing", "Some plan items have nobody on them; the manager is looking into it.")


def _disk(state: dict, now: datetime, active: set[str]) -> dict:
    if not diskdiet_mod.low(state):
        return _check("ok", OK_TEXT["disk"])
    disk = state.get("disk") or {}
    gb = int(disk.get("free_bytes") or 0) / diskdiet_mod.GB
    active.add("disk")
    return _check("failed", f"The disk is almost full ({gb:.1f} GB free); no new work starts until space is freed.")


def _timeouts(home: Path, now: datetime, book: dict, active: set[str]) -> dict:
    """A command class that timed out ``TIMEOUTS_REPEATED`` times within the hour: one manager queue item."""
    worst = _check("ok", OK_TEXT["timeouts"])
    for cls, rows in sorted(timeouts_mod.recent_timeouts(home, TIMEOUTS_WINDOW_S, now).items()):
        if len(rows) < TIMEOUTS_REPEATED:
            continue
        key = f"timeouts:{cls}"
        active.add(key)
        seen = "; ".join(f"{r.get('at')}: {r.get('command') or '?'} (limit {r.get('seconds')} s)" for r in rows[-4:])
        _queue(home, book, key, "tirith", str(rows[-1].get("lane") or ""),
               f"Self-check: commands of class '{cls}' ran out of time {len(rows)} times in the last hour. "
               f"Find out why they hang (a lock, a prompt, a slow test?) or set a fitting limit with "
               f"`tirith timeout set --class {timeouts_mod.base_class(cls)} --seconds N --reason \"…\"`.\n{seen}")
        worst = _check("failed", f"Commands of class '{cls}' keep running out of time ({len(rows)} times in an hour).")
    return worst


def diff_files(repo: Path, lane_id: str) -> list[str]:
    done = timeouts_mod.run(["git", "-C", str(repo), "diff", "--name-only", f"HEAD...lane/{lane_id}"], "git",
                          capture_output=True, text=True)
    return done.stdout.split() if done.returncode == 0 else []


def failing_tests(message: str) -> set[str]:
    return {Path(m).name for m in _TEST_FILE.findall(message or "")}


def unrelated(message: str, files: list[str]) -> bool:
    """True when the failing tests named in ``message`` are all outside the lane's diff: not a test file the lane
    changed and not the test of a module it changed (``test_foo.py`` ~ ``foo.py`` / ``foo.js``). No test named: False."""
    failed = failing_tests(message)
    touched = {Path(f).name for f in files}
    stems = {Path(f).stem for f in files}
    for name in failed:
        base = name.replace(".test.mjs", "") if name.endswith(".test.mjs") else name[len("test_"):-len(".py")]
        if name in touched or base in stems or any(base in s or s in base for s in stems if len(s) > 3):
            return False
    return bool(failed)


def _rerun(home: Path, config: Config, lane_id: str, spawn: Callable) -> None:
    argv = [*launch_mod.tirith_argv(home), "lane", "review", lane_id]
    spawn(argv, home, home / "logs" / f"{lane_id}.review.log")


def _reviews(home: Path, config: Config, now: datetime, book: dict, lanes: list[dict], spawn: Callable,
             kill: Callable, diff: Callable, active: set[str]) -> dict:
    worst = _check("ok", OK_TEXT["reviews"])
    for seen in lanes:
        lane_id = str(seen.get("id") or "")
        lane = read_lane(home, lane_id) if lane_id else None
        review = lane.get("review") if isinstance(lane, dict) else None
        if not isinstance(review, dict):
            continue
        project, status = str(lane.get("project") or ""), review.get("status")
        if status == review_mod.RUNNING:
            for key in (f"review-stuck:{lane_id}", f"review-unrelated:{lane_id}"):
                if key in book["tries"]:
                    active.add(key)  # a re-run is in progress: the episode goes on
        if status == review_mod.RUNNING and _age_s(review.get("at"), now) >= REVIEW_STUCK_S:
            key, mins = f"review-stuck:{lane_id}", _minutes(_age_s(review.get("at"), now))
            active.add(key)

            def act() -> None:
                if review_mod._alive(review.get("pid")):
                    kill({"pid": review.get("pid")})
                else:
                    _rerun(home, config, lane_id, spawn)
            tried = _attempt(book, key, now, act)
            if tried != "failed":
                if tried == "acted":
                    _notice(book, key, f"The review of lane {lane_id} was stuck for {mins} min; I started it again.", now)
                found = _check("healing", f"The review of lane {lane_id} was stuck; running it again.")
            else:
                _queue(home, book, key, project, lane_id,
                       f"Self-check: the review of lane {lane_id} has been running for {mins} min and "
                       f"{MAX_TRIES} restarts did not finish it. Run `tirith lane review {lane_id}` yourself.")
                found = _check("failed", f"The review of lane {lane_id} is stuck and could not be restarted.")
        elif status == review_mod.TESTS_FAILED:
            repo = autoreview_repo(config, lane)
            files = diff(repo, lane_id) if repo else []
            if not unrelated(str(review.get("message") or ""), files):
                continue
            key = f"review-unrelated:{lane_id}"
            active.add(key)
            tried = _attempt(book, key, now, lambda: _rerun(home, config, lane_id, spawn), max_tries=1)
            if tried != "failed":
                if tried == "acted":
                    _notice(book, key, f"The tests of lane {lane_id} failed on something the lane did not touch; "
                                       "I ran them again.", now)
                found = _check("healing", f"Tests failed on code lane {lane_id} did not touch; running them again.")
            else:
                _queue(home, book, key, project, lane_id,
                       f"Self-check: the tests of lane {lane_id} fail twice in "
                       f"{', '.join(sorted(failing_tests(str(review.get('message') or ''))))}, which its diff "
                       f"({len(files)} file(s)) does not touch. Something else broke them; look at main.\n"
                       f"{str(review.get('message') or '')[-800:]}")
                found = _check("failed", f"Tests fail on code lane {lane_id} did not touch.")
        else:
            continue
        if LEVELS.index(found["level"]) > LEVELS.index(worst["level"]):
            worst = found
    return worst


def autoreview_repo(config: Config, lane: dict) -> Path | None:
    from tirith import autoreview as autoreview_mod
    return autoreview_mod.repo_of(config, lane)


# -- the pass ---------------------------------------------------------------------------------


def run(home: Path, config: Config, now: datetime, state: dict, *, lanes: list[dict], agents: list[dict] | None = None,
        projects: list[dict] | None = None, spawn: Callable | None = None, kill: Callable | None = None,
        diff: Callable | None = None) -> bool:
    """One self-check; True when ``state`` (``state["health"]``) changed."""
    home = Path(home)
    book = _book(state)
    spawn = spawn or launch_mod.spawn_detached
    kill = kill or managerd_mod.terminate_group
    active: set[str] = set()
    checks = {"collector": _collector(book, now)}
    checks["manager"] = _manager(home, config, now, state, book, lanes, kill, active)
    checks["autopilot"] = _autopilot(config, now, state, book, home, agents or [], projects or [], active)
    checks["disk"] = _disk(state, now, active)
    checks["reviews"] = _reviews(home, config, now, book, lanes, spawn, kill, diff or diff_files, active)
    checks["timeouts"] = _timeouts(home, now, book, active)
    changed = _announce(home, book, now)
    active |= set(book["notices"])  # an episode lasts until the owner has been told how it ended
    for key in list(book["tries"]):  # an episode that is over starts the next one from zero
        if key not in active:
            del book["tries"][key]
            changed = True
    for key in list(book["queued"]):
        if key not in active:
            try:
                asks_mod.queue_done(home, book["queued"][key], "the problem is gone")
            except asks_mod.AskError:
                pass
            del book["queued"][key]
            changed = True
    levels = [c["level"] for c in checks.values()]
    level = "red" if "failed" in levels else "amber" if "healing" in levels else "green"
    if checks != book.get("checks") or level != book.get("level") or changed or not book.get("at") \
            or _age_s(book["at"], now) >= SAVE_EVERY_S:
        book.update(at=now_iso(now), checks=checks, level=level)
        return True
    return False


def public(book: dict | None) -> dict | None:
    """``state.health``: ``{level: green|amber|red, at, checks: [{id, label, level, text}]}`` (None before the first pass)."""
    if not isinstance(book, dict) or not book.get("checks"):
        return None
    return {"level": book.get("level") or "green", "at": book.get("at"),
            "checks": [{"id": cid, "label": LABELS.get(cid, cid), "level": c["level"], "text": c["text"]}
                       for cid, c in book["checks"].items()]}
