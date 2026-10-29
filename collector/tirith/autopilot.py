"""The autopilot: Tirith drives every plan to the end by itself, by code.

Every collector cycle ``dispatch.run`` calls :func:`run`. For each project with a roadmap it starts one lane
(like ``tirith launch``, the brief lists only the step's agent items) on every ``ready`` step, oldest step
first, never two on the same step and at most ``[autopilot] max_parallel`` autopilot lanes running. A step
whose last two autopilot lanes ended without ticking anything is paused and becomes a manager queue item.
It also keeps one owner item for every open owner-only roadmap item (closed when the item is ticked) and
handles the owner's ``autopilot`` on/off requests. State lives in ``dispatch.json`` under ``autopilot``.
See ``docs/CONTRACT-roadmap.md`` and ``docs/CONTRACT-inbox.md``.
"""

from __future__ import annotations

import hashlib
import re
from datetime import datetime
from pathlib import Path
from typing import Callable

from tirith import actions as actions_mod
from tirith import asks as asks_mod
from tirith import flows as flows_mod
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith.home import Config, now_iso, parse_iso

STUCK_AFTER = 2          # lanes in a row that ended without ticking anything
GRACE_S = 180.0          # a lane that has not shown up as an agent yet still counts as running
RETRY_REFUSED_S = 600.0
KEPT_LANES = 200
STARTED_SHOWN = 50
REVIEW_BUSY = ("pending", "running", "ready", "tests-failed", "merge-failed")
REVIEW_OPEN = ("pending", "running")
_URL = re.compile(r"https?://[^\s)>\]`]+")


def _own(state: dict) -> dict:
    own = state.setdefault("autopilot", {})
    for key in ("lanes", "stuck", "paused", "owner_items", "refused"):
        if not isinstance(own.get(key), dict):
            own[key] = {}
    return own


def effective(config: Config, state: dict) -> bool:
    """On unless ``[autopilot] enabled = false``; the owner's on/off request wins over the file."""
    on = _own(state).get("on")
    return bool(config.autopilot_enabled) if on is None else bool(on)


def _key(project: str, step: str) -> str:
    return f"{project}|{step}"


def _step_name(project: str, step: str) -> str:
    return f"{project} step {step}"


# -- the owner's switch -----------------------------------------------------------------------


def _switch(home: Path, state: dict, now: datetime) -> bool:
    own, changed = _own(state), False
    for req in inbox_mod.requests(home):
        if req["kind"] != "autopilot" or req["status"] == "done":
            continue
        on = req["text"].strip().lower() == "on"
        own["on"] = on
        flows_mod.append_flow(home, "owner", "tirith", "request", f"Autopilot {'on' if on else 'off'}",
                              ref=req["id"], session=None, project=None, at=now_iso(now))
        inbox_mod.mark_done(home, req["id"], now_iso(now))
        changed = True
    return changed


# -- owner items ------------------------------------------------------------------------------


def _owner_item(home: Path, project: dict, step: dict, item: dict, at: str) -> None:
    from tirith.dispatch import add_owner_item  # late: dispatch imports this module
    text = item["text"]
    shown = f"{project['name']} step {step['id']} ({step['title']}): {text}"
    command = actions_mod.find_owner_command(text)
    url = _URL.search(text)
    actions = options = link = None
    if command:
        action_id = "ac-" + hashlib.sha1(item["owner_item"].encode()).hexdigest()[:8]
        actions = [actions_mod.register_generated(home, project["id"], action_id, f"Run {command}"[:60],
                                                  command, project.get("folder"))]
        options = []
    elif url:
        link, options = url.group(0).rstrip(".,;"), [{"label": "Done", "value": "done"}]
    else:
        options = [{"label": "Done", "value": "done"}, {"label": "Not now", "value": "later"}]
    add_owner_item(home, item["owner_item"], project["id"], shown, options, at, actions, link=link)


def _owner_items(home: Path, state: dict, projects: list[dict], at: str) -> bool:
    """One owner item per open owner-only item; closed when the item is ticked or gone."""
    from tirith.dispatch import close_owner_item
    own, changed, seen = _own(state), False, set()
    for project in projects:
        for step in (project.get("roadmap") or {}).get("steps", []):
            for item in step["items"]:
                if item.get("who") != "owner" or item["done"] or not item.get("owner_item"):
                    continue
                seen.add(item["owner_item"])
                if item["owner_item"] not in own["owner_items"]:
                    _owner_item(home, project, step, item, at)
                    own["owner_items"][item["owner_item"]] = _key(project["id"], step["id"])
                    changed = True
    for item_id in list(own["owner_items"]):
        if item_id not in seen:
            close_owner_item(home, item_id, at)
            del own["owner_items"][item_id]
            changed = True
    return changed


# -- lanes ------------------------------------------------------------------------------------


def _steps(projects: list[dict]) -> dict[str, dict]:
    return {_key(p["id"], s["id"]): s for p in projects for s in (p.get("roadmap") or {}).get("steps", [])}


def _agent_open(step: dict) -> list[str]:
    return [i["text"] for i in step["items"] if not i["done"] and i.get("who") == "agent"]


def _agents_by_id(agents: list[dict]) -> dict[str, dict]:
    return {str(a["id"]): a for a in agents}


def _running(rec: dict, agent: dict | None, now: datetime) -> bool:
    if agent is not None:
        return agent.get("status") == "working"
    started = parse_iso(rec.get("at"))
    return started is not None and (now - started).total_seconds() < GRACE_S


def _settle(home: Path, state: dict, projects: list[dict], agents: list[dict], now: datetime) -> bool:
    """Count each ended autopilot lane once: progress resets the step's counter, none adds to it."""
    own, steps, by_id, changed = _own(state), _steps(projects), _agents_by_id(agents), False
    for lane_id, rec in own["lanes"].items():
        if rec.get("settled"):
            continue
        agent = by_id.get(lane_id)
        if _running(rec, agent, now):
            continue
        review = (agent or {}).get("review") or {}
        if review.get("status") in REVIEW_OPEN:
            continue  # its work is not merged yet; the plan is ticked on the merge
        key = _key(rec["project"], rec["step"])
        step = steps.get(key)
        open_now = set(_agent_open(step)) if step else set()
        progressed = review.get("status") in ("ready", "merged") or any(t not in open_now for t in rec["items"])
        rec["settled"] = True
        changed = True
        if progressed:
            own["stuck"].pop(key, None)
            continue
        own["stuck"][key] = own["stuck"].get(key, 0) + 1
        if own["stuck"][key] >= STUCK_AFTER and key not in own["paused"]:
            _pause(home, own, key, lane_id, rec, sorted(open_now), now)
    return changed


def _pause(home: Path, own: dict, key: str, lane_id: str, rec: dict, open_now: list[str], now: datetime) -> None:
    at = now_iso(now)
    own["paused"][key] = {"at": at, "items": open_now}
    name = _step_name(rec["project"], rec["step"])
    text = (f"Autopilot paused: {name} is not converging. The last {STUCK_AFTER} lanes ended without "
            "ticking anything. Read their reports (`tirith lane review <lane>`), then fix the brief or the "
            "items; the autopilot resumes by itself when the open items change.")
    asks_mod.queue_add(home, rec["project"], lane_id, text, kind="autopilot",
                       item_id=f"sq-autopilot-{rec['project']}-{rec['step']}")
    flows_mod.append_flow(home, "tirith", "owner", "alert", f"{name} is not converging; autopilot paused there",
                          session=None, project=rec["project"], at=at)


def _unpause(home: Path, state: dict, steps: dict[str, dict]) -> bool:
    """A paused step resumes when its open agent items are not the ones it stopped on."""
    own, changed = _own(state), False
    for key, row in list(own["paused"].items()):
        step = steps.get(key)
        if step is None or sorted(_agent_open(step)) != row["items"]:
            del own["paused"][key]
            own["stuck"].pop(key, None)
            project, _, step_id = key.partition("|")
            try:
                asks_mod.queue_done(home, f"sq-autopilot-{project}-{step_id}", "the open items changed")
            except asks_mod.AskError:
                pass
            changed = True
    return changed


def _blocked_by_review(agents: list[dict], project: str, step: str) -> bool:
    return any(a.get("project") == project and str(a.get("step") or "") == step
               and (a.get("review") or {}).get("status") in REVIEW_BUSY for a in agents)


def _launch(home: Path, state: dict, project: dict, step: dict, launcher: Callable, now: datetime) -> bool | None:
    """True: a lane started. False: refused (recorded). None: refused recently, nothing was tried."""
    from tirith.dispatch import close_owner_item, add_owner_item, refusal_fix, _short
    own, at = _own(state), now_iso(now)
    pid, sid = project["id"], step["id"]
    key, name = _key(pid, sid), _step_name(pid, sid)
    refused = own["refused"].get(key)
    if refused:
        asked = parse_iso(refused["at"])
        if asked and (now - asked).total_seconds() < RETRY_REFUSED_S:
            return None
    try:
        lane_id = launcher(home, pid, sid, None)
    except launch_mod.LaunchError as err:
        reason = str(err)
    except Exception as err:  # never let one step stop the cycle
        reason = f"{type(err).__name__}: {err}"
    else:
        if own["refused"].pop(key, None):
            close_owner_item(home, f"autopilot-{pid}-{sid}", at)
        own["lanes"][lane_id] = {"project": pid, "step": sid, "at": at, "items": _agent_open(step), "settled": False}
        for old in list(own["lanes"])[:-KEPT_LANES]:
            del own["lanes"][old]
        flows_mod.append_flow(home, "tirith", f"agent:{lane_id}", "brief", f"Autopilot started {name}",
                              session=None, project=pid, at=at)
        return True
    first = not refused or refused["reason"] != reason
    own["refused"][key] = {"at": at, "reason": reason}
    if first:
        fix = refusal_fix(pid, sid, reason)
        flows_mod.append_flow(home, "tirith", "owner", "alert", f"Autopilot could not start {name}: {_short(reason)}",
                              session=None, project=pid, at=at)
        add_owner_item(home, f"autopilot-{pid}-{sid}", pid, f"Autopilot could not start an agent on {name}: {_short(reason)}",
                       [{"label": "Fix it", "command": fix},
                        {"label": "Try again", "command": f"tirith launch --project {pid} --step {sid}"}], at)
    return False


# -- the cycle --------------------------------------------------------------------------------


def run(home: Path, config: Config, now: datetime, state: dict, *, agents: list[dict], projects: list[dict],
        launcher: Callable) -> bool:
    """One autopilot pass; True when ``state`` changed."""
    own = _own(state)
    changed = _switch(home, state, now)
    planned = [p for p in projects if p.get("roadmap")]
    changed |= _owner_items(home, state, planned, now_iso(now))
    changed |= _settle(home, state, planned, agents, now)
    changed |= _unpause(home, state, _steps(planned))
    if not effective(config, state):
        return changed
    by_id = _agents_by_id(agents)
    running = sum(1 for lane_id, rec in own["lanes"].items()
                  if not rec.get("settled") and _running(rec, by_id.get(lane_id), now))
    for project in planned:
        for step in project["roadmap"]["steps"]:
            if running >= config.autopilot_max_parallel:
                return changed
            key = _key(project["id"], step["id"])
            if step["status"] != "ready" or key in own["paused"] or _blocked_by_review(agents, project["id"], step["id"]):
                continue
            if any(r["project"] == project["id"] and r["step"] == step["id"] and not r.get("settled")
                   and _running(r, by_id.get(lane_id), now) for lane_id, r in own["lanes"].items()):
                continue
            launched = _launch(home, state, project, step, launcher, now)
            if launched is not None:
                changed = True
            if launched:
                running += 1
    return changed


def public(home: Path, config: Config) -> dict:
    """``state.autopilot``: the switch, the limit and the lanes the autopilot started (newest last)."""
    from tirith.dispatch import load_state
    state = load_state(home)
    own = _own(state)
    started = [{"lane": lane_id, "project": rec["project"], "step": rec["step"], "at": rec["at"]}
               for lane_id, rec in own["lanes"].items()]
    return {"enabled": effective(config, state), "max_parallel": config.autopilot_max_parallel,
            "started": started[-STARTED_SHOWN:],
            "paused_steps": [{"project": k.partition("|")[0], "step": k.partition("|")[2], "at": v["at"]}
                             for k, v in own["paused"].items()]}
