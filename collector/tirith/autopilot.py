"""The autopilot: Tirith watches every plan and tells the manager what is ready; it never starts an agent.

Every collector cycle ``dispatch.run`` calls :func:`run`. For each open agent item of a plan that nobody works
on (no agent working on its step and no review running or waiting to be merged), once it has sat for
``DEBOUNCE_S``, it posts ONE ``autopilot-proposal`` request to the manager (from ``tirith``): the project,
step, item, why it is open, the linked lane, its report, whether its branch is merged, the claims result and
the last attempts. The manager decides like it does for an owner message: launch with a written brief, combine
with a lane, wait, or drop the item through a plan edit, and answers on the proposal. A new proposal follows
only when the item's text or state changes. Nothing here calls the launcher, so one brain decides.
It also keeps one owner item for every open owner-only roadmap item (closed when the item is ticked) and
handles the owner's ``autopilot`` on/off requests (kept in ``<home>/autopilot.json``, so they survive any
restart). State lives in ``dispatch.json`` under ``autopilot``. See ``docs/CONTRACT-roadmap.md`` and
``docs/CONTRACT-inbox.md``.
"""

from __future__ import annotations

import hashlib
import re
from datetime import datetime
from pathlib import Path
from typing import Callable

from tirith import actions as actions_mod
from tirith import flows as flows_mod
from tirith import inbox as inbox_mod
from tirith import loadlimit as loadlimit_mod
from tirith import plangate as plangate_mod
from tirith import roadmap as roadmap_mod
from tirith.home import Config, now_iso, parse_iso
from tirith.store import read_json, write_json_atomic

SUMMARY_MAX = 400
# a report saying the work is already in the code; checked before CANNOT (a done report may say "can't")
ALREADY = re.compile(r"already (?:built|done|exists?|implemented|merged|in the (?:committed )?code|ticked)"
                     r"|nothing (?:is )?left|changed no files|no files (?:were )?changed", re.I)
# a report saying the lane could not do an item
CANNOT = re.compile(r"\bcan(?:'|’)?t\b|\bcannot\b|couldn(?:'|’)?t|could not|needs? the owner|\bblocked\b", re.I)
DEBOUNCE_S = 30.0        # an open item nobody works on is reported after this long (a normal handoff is not)
SWITCH_FILE = "autopilot.json"
ATTEMPTS_SHOWN = 3
REVIEW_OPEN = ("pending", "running")
REVIEW_FAILED = ("tests-failed", "claims-failed", "merge-failed")
REVIEW_BUSY = (*REVIEW_OPEN, "ready", *REVIEW_FAILED)
INTERRUPTED = "interrupted"  # in the message of a review whose process died (``review.public``)


def _own(state: dict) -> dict:
    own = state.setdefault("autopilot", {})
    for key in ("owner_items", "proposals", "seen"):
        if not isinstance(own.get(key), dict):
            own[key] = {}
    return own


def effective(config: Config, state: dict) -> bool:
    """On unless ``[autopilot] enabled = false``; the owner's on/off request wins over the file."""
    on = _own(state).get("on")
    return bool(config.autopilot_enabled) if on is None else bool(on)


def _key(project: str, step: str) -> str:
    return f"{project}|{step}"


# -- the owner's switch -----------------------------------------------------------------------


def _sync_switch(home: Path, state: dict) -> bool:
    """The switch file is the truth: it survives a lost or stale ``dispatch.json``. An older ``on`` in the state
    (before the file existed) is written out once."""
    own, row = _own(state), read_json(Path(home) / SWITCH_FILE, {})
    saved = row.get("on") if isinstance(row, dict) else None
    if isinstance(saved, bool):
        if own.get("on") != saved:
            own["on"] = saved
            return True
    elif isinstance(own.get("on"), bool):
        write_json_atomic(Path(home) / SWITCH_FILE, {"on": own["on"], "at": now_iso()})
    return False


def _switch(home: Path, state: dict, now: datetime) -> bool:
    own, changed = _own(state), False
    for req in inbox_mod.requests(home):
        if req["kind"] != "autopilot" or req["status"] == "done":
            continue
        on = req["text"].strip().lower() == "on"
        own["on"] = on
        write_json_atomic(Path(home) / SWITCH_FILE, {"on": on, "at": now_iso(now)})
        flows_mod.append_flow(home, "owner", "tirith", "request", f"Autopilot {'on' if on else 'off'}",
                              ref=req["id"], session=None, project=None, at=now_iso(now))
        inbox_mod.mark_done(home, req["id"], now_iso(now))
        changed = True
    return changed


# -- owner items ------------------------------------------------------------------------------


def _owner_item(home: Path, project: dict, step: dict, item: dict, at: str) -> None:
    from tirith.dispatch import add_owner_item  # late: dispatch imports this module
    parts = roadmap_mod.owner_parts(item["text"])
    actions = options = link = None
    if parts["command"]:
        action_id = "ac-" + hashlib.sha1(item["owner_item"].encode()).hexdigest()[:8]
        actions = [actions_mod.register_generated(home, project["id"], action_id, parts["label"],
                                                  parts["command"], project.get("folder"))]
        options = []
    elif parts["url"]:
        link, options = parts["url"], [{"label": "Done", "value": "done"}]
    else:
        options = [{"label": "Done", "value": "done"}, {"label": "Not now", "value": "later"}]
    add_owner_item(home, item["owner_item"], project["id"], parts["sentence"], options, at, actions, link=link,
                   why_owner=parts["why_owner"], why_detail=parts["why_detail"])


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



# -- proposals --------------------------------------------------------------------------------


def _steps(projects: list[dict]) -> dict[str, dict]:
    return {_key(p["id"], s["id"]): s for p in projects for s in (p.get("roadmap") or {}).get("steps", [])}


def _agent_open(step: dict) -> list[str]:
    """Clean texts (no status note) of the items an agent can take now: open or reopened, not in progress."""
    return [roadmap_mod.split_status(i["text"])[0] for i in step["items"] if not i["done"]
            and i.get("who") == "agent" and i.get("state", "open") in (roadmap_mod.STATE_OPEN, roadmap_mod.STATE_REOPENED)]


def _open_agent(step: dict) -> list[dict]:
    """The open items of the agents (whatever their note says: open, in progress, in review, blocked, reopened)."""
    return [i for i in step["items"] if not i["done"] and i.get("who") == "agent"]


def _agents_by_id(agents: list[dict]) -> dict[str, dict]:
    return {str(a["id"]): a for a in agents}


def _review_status(agent: dict) -> str | None:
    """The review status of a lane; a review whose process died counts as running (the self-check runs it again)."""
    review = agent.get("review") or {}
    status = review.get("status")
    if status == "merge-failed" and INTERRUPTED in str(review.get("message") or ""):
        return "running"
    return status


def _on_step(agents: list[dict], project: str, step: str) -> list[dict]:
    return [a for a in agents if a.get("project") == project and str(a.get("step") or "") == step]


def _busy(agents: list[dict], project: str, step: str) -> str | None:
    """Plain words for why nobody needs to be told about this step now: an agent works on it, or Tirith is
    checking or merging the work of an earlier one. None when nobody is on it."""
    mine = _on_step(agents, project, step)
    if any(a.get("status") == "working" for a in mine):
        return "an agent is already on it"
    states = {_review_status(a) for a in mine}
    if states & set(REVIEW_OPEN):
        return "the work of an earlier agent on it is being checked by Tirith"
    if "ready" in states:
        return "the work of an earlier agent on it is checked and Tirith is merging it"
    return None


def _why_not(own: dict, agents: list[dict], project: str, step: dict, now: datetime) -> str | None:
    """Plain words for why the manager is not told about this step now; None when it can be."""
    if not _open_agent(step):
        return "only the owner's items are left"
    return _busy(agents, project, step["id"])


def _fingerprint(step: str, raw: str) -> str:
    return hashlib.sha1(f"{step}|{raw}".encode()).hexdigest()[:12]


def _why_open(clean: str, state: str, detail: str | None) -> str:
    if state == roadmap_mod.STATE_REVIEW:
        return "it is marked in review but no check is running and nobody works on it" \
               + (f" (linked: {detail})" if detail else "")
    if state == roadmap_mod.STATE_PROGRESS:
        return "it is marked in progress but its lane is not working any more" + (f" ({detail})" if detail else "")
    if state == roadmap_mod.STATE_BLOCKED:
        return "it is marked blocked" + (f" ({detail})" if detail else "") + " and nobody works on it"
    if state == roadmap_mod.STATE_REOPENED:
        return "it was reopened" + (f" ({detail})" if detail else "") + " and nobody works on it"
    return "it is open and nobody works on it"


def candidates(projects: list[dict], agents: list[dict]) -> tuple[dict[str, dict], set[str]]:
    """``(open, present)``: ``open`` maps item key -> the open agent items nobody works on (the ones to tell the
    manager about); ``present`` is the keys of every open agent item (working or not)."""
    found: dict[str, dict] = {}
    present: set[str] = set()
    for project in projects:
        road = project.get("roadmap") or {}
        for step in road.get("steps", []):
            if step["id"] == plangate_mod.OPS_STEP:
                continue  # step 0 holds Tirith's own work (reviews, plan refresh, manager)
            busy = _busy(agents, project["id"], step["id"])
            for item in _open_agent(step):
                clean, state, detail = roadmap_mod.split_status(item["text"])
                key = f"{project['id']}|{step['id']}|{roadmap_mod.item_id(item['text'])}"
                present.add(key)
                if busy is None:
                    found[key] = {"key": key, "project": project["id"], "step": step["id"], "title": step.get("title") or "",
                                  "item": clean, "state": state, "detail": detail, "raw": item["text"],
                                  "fp": _fingerprint(step["id"], item["text"]), "plan": road.get("file")}
    return found, present


def _lane_line(agent: dict) -> str:
    review = agent.get("review") or {}
    bits = [str(agent.get("status") or "?")]
    if agent.get("outcome"):
        bits.append(f"outcome {agent['outcome']}")
    if review.get("status") == "merged":
        bits.append("branch merged")
    elif review.get("status"):
        bits.append(f"review {review['status']}")
        if review.get("message"):
            bits.append(flows_mod.one_line(str(review["message"]), 120))
    else:
        bits.append("no review")
    if review.get("claims"):
        bits.append("claims: " + flows_mod.one_line(str(review["claims"]), 120))
    return f"- lane {agent['id']} ({', '.join(bits)}): {flows_mod.one_line(report_summary(agent.get('report')), 220)}"


def _facts(cand: dict, agents: list[dict]) -> list[str]:
    """Lines about the lanes that worked on the item's step: the lane linked in its note, then the last attempts."""
    mine = [a for a in _on_step(agents, cand["project"], cand["step"]) if a.get("status") != "working"]
    same = [a for a in mine if a.get("item") and roadmap_mod.norm(a["item"]) == cand["item"]]
    mine = sorted(same or mine, key=lambda a: str(a.get("ended_at") or a.get("started_at") or ""))[-ATTEMPTS_SHOWN:]
    linked = next((a for a in agents if cand["detail"] and str(a.get("id")) == cand["detail"].split()[-1]), None)
    if linked and linked not in mine:
        mine.append(linked)
    return [_lane_line(a) for a in mine] or ["- no earlier lane worked on it"]


def proposal_text(cand: dict, agents: list[dict]) -> str:
    return "\n".join([
        f"Autopilot: {cand['project']} step {cand['step']} — {cand['item']}",
        f"Why now: {_why_open(cand['item'], cand['state'], cand['detail'])}.",
        f"Plan: {cand['plan'] or 'docs/TODO.md'}, step {cand['step']} ({cand['title']}).",
        "Last attempts:", *_facts(cand, agents),
        "Decide: launch (with a written brief and the plan link), combine with an existing lane, wait (say why), "
        "or drop (plan edit through Tirith; `tirith plan done` when the work is already on main). "
        "Answer here, then close it.",
    ])


def proposal_line(text: str) -> str:
    """The first line of a proposal: what the chat shows."""
    return text.split("\n", 1)[0]


def _open_request(req: dict | None) -> bool:
    return req is not None and req["status"] != "done"


def _propose(home: Path, state: dict, projects: list[dict], agents: list[dict], now: datetime) -> bool:
    """Post one proposal per open item nobody works on (after ``DEBOUNCE_S``), at most one per item state."""
    own, at, changed = _own(state), now_iso(now), False
    found, present = candidates(projects, agents)
    reqs = {r["id"]: r for r in inbox_mod.requests(home)}
    for key, rec in list(own["proposals"].items()):
        cand = found.get(key)
        if key in present and (cand is None or cand["fp"] == rec["fp"]):
            continue  # the item is still there in the same state (or somebody works on it): the manager decides
        req = reqs.get(rec["id"])
        if _open_request(req) and not any(a["by"] == "manager" for a in req["answers"]):
            inbox_mod.mark_done(home, rec["id"], at)  # Tirith's own proposal closes quietly
        if key not in present:
            del own["proposals"][key]
            changed = True
    for key in list(own["seen"]):
        if key not in found:
            del own["seen"][key]
            changed = True
    for key, cand in found.items():
        rec = own["proposals"].get(key)
        if rec and rec["fp"] == cand["fp"]:
            continue
        seen = own["seen"].get(key)
        if not seen or seen["fp"] != cand["fp"]:
            own["seen"][key] = {"fp": cand["fp"], "at": at}
            changed = True
            continue
        first = parse_iso(seen["at"])
        if first is None or (now - first).total_seconds() < DEBOUNCE_S:
            continue
        row = inbox_mod.add_request(home, inbox_mod.PROPOSAL, cand["project"], cand["step"],
                                    proposal_text(cand, agents), at=at, sender="tirith")
        own["proposals"][key] = {"id": row["id"], "fp": cand["fp"], "at": at}
        flows_mod.append_flow(home, "tirith", "manager", "request",
                              f"Autopilot proposes {cand['project']} step {cand['step']}: {flows_mod.one_line(cand['item'], 100)}",
                              ref=row["id"], session=None, project=cand["project"], at=at)
        changed = True
    return changed


def sitting(home: Path, state: dict, projects: list[dict], agents: list[dict], now: datetime) -> list[dict]:
    """Open items nobody works on that the manager was not told about yet, each with ``idle_s`` (how long)."""
    own = _own(state)
    found, _ = candidates([p for p in projects if p.get("roadmap")], agents)
    out = []
    for key, cand in found.items():
        rec = own["proposals"].get(key)
        seen = parse_iso((own["seen"].get(key) or {}).get("at"))
        if not (rec and rec["fp"] == cand["fp"]):
            out.append({**cand, "idle_s": (now - seen).total_seconds() if seen else 0.0})
    return out


def unanswered(home: Path, state: dict, now: datetime) -> list[dict]:
    """Proposals the manager has not answered yet, each with ``age_s``."""
    out = []
    for req in inbox_mod.requests(home):
        if req["kind"] == inbox_mod.PROPOSAL and req["manager_pending"]:
            asked = parse_iso(req["at"])
            out.append({**req, "age_s": (now - asked).total_seconds() if asked else 0.0})
    return out


# -- what a lane report says (the manager's brief and the plan updater use these) -------------------


def report_kind(report: str | None) -> str:
    """``done`` (already built), ``cannot`` (the lane could not do it) or ``stuck`` (anything else)."""
    text = str(report or "")
    if ALREADY.search(text):
        return "done"
    return "cannot" if CANNOT.search(text) else "stuck"


def report_summary(report: str | None) -> str:
    """The first paragraph of a lane report on one line, for a queue item."""
    paragraphs = [" ".join(p.split()) for p in str(report or "").split("\n\n") if p.strip()]
    text = paragraphs[0] if paragraphs else "The lane ended without a report."
    return text if len(text) <= SUMMARY_MAX else text[:SUMMARY_MAX - 1].rsplit(" ", 1)[0] + "…"


# -- the cycle --------------------------------------------------------------------------------


def run(home: Path, config: Config, now: datetime, state: dict, *, agents: list[dict], projects: list[dict],
        launcher: Callable | None = None, launch: bool = True) -> bool:
    """One autopilot pass; True when ``state`` changed. It never starts an agent (``launcher`` is ignored).
    ``launch`` False (disk almost full): no new proposal, the manager could not start anything anyway."""
    changed = _sync_switch(home, state)
    changed |= _switch(home, state, now)
    planned = [p for p in projects if p.get("roadmap")]
    changed |= _owner_items(home, state, planned, now_iso(now))
    if launch and effective(config, state):
        changed |= _propose(home, state, planned, agents, now)
    return changed


def _counts(own: dict, requests: list[dict], projects: list[dict], agents: list[dict]) -> dict:
    found, present = candidates(projects, agents)
    live = {rec["id"] for rec in own["proposals"].values()
            if any(r["id"] == rec["id"] and r["status"] != "done" for r in requests)}
    proposed = sum(1 for key, rec in own["proposals"].items() if rec["id"] in live and key in present)
    working = reviewing = 0
    skipped = []
    for p in projects:
        for st in (p.get("roadmap") or {}).get("steps", []):
            if st["id"] == plangate_mod.OPS_STEP or not _open_agent(st):
                continue
            why = _busy(agents, p["id"], st["id"])
            if why:
                skipped.append({"project": p["id"], "step": st["id"], "why": why})
                if why == "an agent is already on it":
                    working += 1
                else:
                    reviewing += 1
    return {"proposed": proposed, "queued": sum(1 for key in found if key not in own["proposals"]
                                               or own["proposals"][key]["fp"] != found[key]["fp"]),
            "steps_working": working, "steps_in_review": reviewing, "skipped": skipped,
            "open_items": len(present)}


def public(home: Path, config: Config, agents: list[dict] | None = None, projects: list[dict] | None = None,
           now: datetime | None = None) -> dict:
    """``state.autopilot``: the switch and, given the agents and projects, the header counts in plain terms:
    ``working`` (agents working on a plan step), ``steps_working`` / ``steps_in_review`` (steps with an agent or
    with its work being checked), ``proposed`` (open items the manager has been told about), ``queued`` (open
    items nobody works on that are about to be), ``open_items``, and ``skipped`` (steps with open items it does
    not report because somebody is on them, each with the reason in plain words)."""
    from tirith.dispatch import load_state
    state = load_state(home)
    _sync_switch(home, state)
    own = _own(state)
    agents, planned = agents or [], [p for p in projects or [] if p.get("roadmap")]
    counts = _counts(own, inbox_mod.requests(home), planned, agents)
    from tirith import diskdiet as diskdiet_mod
    if diskdiet_mod.low(state):
        counts["skipped"] += [{"project": p, "step": s, "why": "it waits for free disk space"}
                              for p, s in {tuple(k.split("|")[:2]) for k in candidates(planned, agents)[0]}]
    steps = {(p["id"], st["id"]) for p in planned for st in p["roadmap"]["steps"]}
    working = sum(1 for a in agents if a.get("status") == "working" and (a.get("project"), str(a.get("step") or "")) in steps)
    allowed, why = loadlimit_mod.current(config, home)  # lowered while the machine is loaded or the disk is low
    return {"enabled": effective(config, state), "max_parallel": config.autopilot_max_parallel,
            "limit": allowed, "limit_why": why,
            "working": working, "started": [], "paused_steps": [], **counts}
