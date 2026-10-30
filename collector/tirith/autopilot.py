"""The autopilot: Tirith watches every plan and tells the manager what is ready; it never starts an agent.

Every collector cycle ``dispatch.run`` calls :func:`run`. For each open agent item of a plan that nobody works
on (no agent working on its step and no review running or waiting to be merged), once it has sat for
``DEBOUNCE_S``, it is listed in the ONE open ``autopilot-proposal`` request of its step (from ``tirith``, under
``MAX_TEXT`` characters): each item with why it is open and the facts of its lane (report, merged or not, claims
result). An item whose lane waits, runs or is being merged in the merge queue is the queue's, not listed; it
comes back only when the queue sends it back. When the facts of a step change its proposal is closed and replaced
(never a second open one), and the first pass collapses older ones. The manager decides like it does for an owner
message: launch with a written brief, combine with a lane, wait, or drop the item through a plan edit, and answers
on the proposal. Closing one never posts a chat line (a flow records it). Nothing here calls the launcher.
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
from tirith import mergeq as mergeq_mod
from tirith import plangate as plangate_mod
from tirith import roadmap as roadmap_mod
from tirith.home import Config, now_iso, parse_iso
from tirith.store import read_json, write_json_atomic

SUMMARY_MAX = 400
MAX_TEXT = 800           # characters of one step's proposal
ITEMS_SHOWN = 8
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


def queued_lanes(home: Path) -> set[str]:
    """Lanes the merge queue owns now: waiting, running or being merged. It sends one back when it cannot merge it."""
    return {str(e["lane"]) for e in mergeq_mod.load(home)["queue"]}


def _queue_owned(item: dict, detail: str | None, mine: list[dict], queued: set[str]) -> bool:
    linked = detail.split()[-1] if detail else ""
    if linked and linked in queued:
        return True
    return any(str(a["id"]) in queued and a.get("item") and roadmap_mod.norm(a["item"]) == item for a in mine)


def candidates(projects: list[dict], agents: list[dict], queued: set[str] | None = None
               ) -> tuple[dict[str, dict], set[str]]:
    """``(open, present)``: ``open`` maps item key -> the open agent items nobody works on (the ones to tell the
    manager about; not the ones whose lane is in the merge queue); ``present`` is the keys of every open agent
    item (working or not)."""
    found: dict[str, dict] = {}
    present: set[str] = set()
    for project in projects:
        road = project.get("roadmap") or {}
        for step in road.get("steps", []):
            if step["id"] == plangate_mod.OPS_STEP:
                continue  # step 0 holds Tirith's own work (reviews, plan refresh, manager)
            busy = _busy(agents, project["id"], step["id"])
            mine = _on_step(agents, project["id"], step["id"])
            for item in _open_agent(step):
                clean, state, detail = roadmap_mod.split_status(item["text"])
                key = f"{project['id']}|{step['id']}|{roadmap_mod.item_id(item['text'])}"
                present.add(key)
                if busy is None and not _queue_owned(clean, detail, mine, queued or set()):
                    found[key] = {"key": key, "project": project["id"], "step": step["id"], "title": step.get("title") or "",
                                  "item": clean, "state": state, "detail": detail, "raw": item["text"],
                                  "fp": _fingerprint(step["id"], item["text"]), "plan": road.get("file")}
    return found, present


def _lane_line(agent: dict, limit: int = 110) -> str:
    review = agent.get("review") or {}
    bits = [str(agent.get("status") or "?")]
    if agent.get("outcome"):
        bits.append(f"outcome {agent['outcome']}")
    if review.get("status") == "merged":
        bits.append("branch merged")
    elif review.get("status") == mergeq_mod.SENT_BACK_STATUS:
        bits.append("sent back by the merge queue" + (": " + flows_mod.one_line(str(review["message"]), 70)
                                                      if review.get("message") else ""))
    elif review.get("status"):
        bits.append(f"review {review['status']}")
        if review.get("message"):
            bits.append(flows_mod.one_line(str(review["message"]), 50))
    else:
        bits.append("no review")
    if review.get("claims"):
        bits.append("claims: " + flows_mod.one_line(str(review["claims"]), 30))
    return flows_mod.one_line(f"lane {agent['id']} ({', '.join(bits)}): {report_summary(agent.get('report'))}", limit)


def _facts(cand: dict, agents: list[dict]) -> str:
    """The lane linked in the item's note, else the last lane that worked on it (or on its step), on one line."""
    mine = [a for a in _on_step(agents, cand["project"], cand["step"]) if a.get("status") != "working"]
    same = [a for a in mine if a.get("item") and roadmap_mod.norm(a["item"]) == cand["item"]]
    mine = sorted(same or mine, key=lambda a: str(a.get("ended_at") or a.get("started_at") or ""))
    linked = next((a for a in agents if cand["detail"] and str(a.get("id")) == cand["detail"].split()[-1]), None)
    last = linked or (mine[-1] if mine else None)
    return _lane_line(last) if last else "no earlier lane"


def _why_short(state: str) -> str:
    return {roadmap_mod.STATE_REVIEW: "marked in review, no check running", roadmap_mod.STATE_PROGRESS:
            "marked in progress, lane gone", roadmap_mod.STATE_BLOCKED: "marked blocked",
            roadmap_mod.STATE_REOPENED: "reopened"}.get(state, "open, nobody on it")


def proposal_text(cands: list[dict], agents: list[dict]) -> str:
    """One step's proposal, under ``MAX_TEXT`` characters: its items that need a decision, each with its facts."""
    first = cands[0]
    plan = str(first["plan"] or "docs/TODO.md")
    plan = plan if len(plan) <= 60 else "…" + plan[-59:]
    head = [f"Autopilot: {first['project']} step {first['step']} ({flows_mod.one_line(first['title'], 40)}): "
            f"{len(cands)} item{'s' if len(cands) != 1 else ''} need{'' if len(cands) != 1 else 's'} a decision. "
            f"Plan: {plan}."]
    tail = ["Decide: launch (written brief + plan link), combine with a lane, wait (say why) or drop (plan edit; "
            "`tirith plan done` if on main). Answer here, then close it."]
    shown = cands[:ITEMS_SHOWN]
    room = MAX_TEXT - len(head[0]) - len(tail[0]) - 30 - len(shown)  # the "more" line and the newlines
    each = max(60, room // max(len(shown), 1))
    lines = [flows_mod.one_line(f"- {c['item'][:60]} [{_why_short(c['state'])}] {_facts(c, agents)}", each) for c in shown]
    if len(cands) > len(shown):
        lines.append(f"- … and {len(cands) - len(shown)} more")
    return "\n".join([*head, *lines, *tail])[:MAX_TEXT]


def proposal_line(text: str) -> str:
    """The first line of a proposal: what the chat shows."""
    return text.split("\n", 1)[0]


def _open_request(req: dict | None) -> bool:
    return req is not None and req["status"] != "done"


def _close(home: Path, req: dict, project: str, step: str, why: str, at: str) -> None:
    """Close an open proposal of Tirith's own without a word in the chat (a flow says why)."""
    inbox_mod.mark_done(home, req["id"], at)
    flows_mod.append_flow(home, "tirith", "manager", "request",
                          f"Autopilot closed its proposal on {project} step {step}: {why}",
                          ref=req["id"], session=None, project=project, at=at)


def _step_groups(found: dict[str, dict]) -> dict[str, list[dict]]:
    groups: dict[str, list[dict]] = {}
    for cand in found.values():
        groups.setdefault(_key(cand["project"], cand["step"]), []).append(cand)
    return groups


def _step_fp(text: str) -> str:
    return hashlib.sha1(text.encode()).hexdigest()[:12]


def _propose(home: Path, state: dict, projects: list[dict], agents: list[dict], now: datetime) -> bool:
    """One open proposal per (project, step) that lists its items nobody works on (after ``DEBOUNCE_S``). When the
    facts change it is closed and replaced; older ones (before this rule) are collapsed into it."""
    own, at, changed = _own(state), now_iso(now), False
    found, present = candidates(projects, agents, queued_lanes(home))
    groups = _step_groups(found)
    present_steps = {k.rsplit("|", 1)[0] for k in present}
    by_step: dict[str, list[dict]] = {}
    for req in inbox_mod.requests(home):
        if req["kind"] == inbox_mod.PROPOSAL and req["status"] != "done":
            by_step.setdefault(_key(req["project"], str(req["step"] or "")), []).append(req)
    for key in [k for k in own["proposals"] if k.count("|") != 1]:
        del own["proposals"][key]  # one per item, from before: collapsed below
        changed = True
    for key in list(own["seen"]):
        if key not in groups:
            del own["seen"][key]
            changed = True
    for key in set(by_step) | set(own["proposals"]):
        project, _, step = key.partition("|")
        rec, opens = own["proposals"].get(key), by_step.get(key, [])
        keep = next((r for r in opens if rec and r["id"] == rec["id"]), None) or (opens[-1] if opens else None)
        if keep is not None and rec is None:
            own["proposals"][key] = rec = {"id": keep["id"], "fp": "", "at": keep["at"]}  # adopt the newest old one
            changed = True
        for req in opens:
            if req is not keep or key not in groups:
                why = "only one proposal stays open per step" if key in groups else \
                    ("it is done or gone from the plan" if key not in present_steps else
                     "the merge queue or an agent has it now")
                _close(home, req, project, step, why, at)
                changed = True
        unseen = keep is not None and not any(a["by"] == "manager" for a in keep["answers"])
        if key not in groups and rec and (key not in present_steps or unseen):
            del own["proposals"][key]  # gone, or closed before the manager saw it: it comes back with new facts
            changed = True
    for key, cands in groups.items():
        text = proposal_text(cands, agents)
        fp = _step_fp(text)
        rec = own["proposals"].get(key)
        if rec and rec["fp"] == fp:
            continue
        seen = own["seen"].get(key)
        if not seen or seen["fp"] != fp:
            own["seen"][key] = {"fp": fp, "at": at}
            changed = True
            continue
        first = parse_iso(seen["at"])
        if first is None or (now - first).total_seconds() < DEBOUNCE_S:
            continue
        for req in by_step.get(key, []):
            if _open_request(req) and rec and req["id"] == rec["id"]:
                _close(home, req, cands[0]["project"], cands[0]["step"], "its facts changed; replaced by a new one", at)
        row = inbox_mod.add_request(home, inbox_mod.PROPOSAL, cands[0]["project"], cands[0]["step"],
                                    text, at=at, sender="tirith")
        own["proposals"][key] = {"id": row["id"], "fp": fp, "at": at}
        flows_mod.append_flow(home, "tirith", "manager", "request",
                              f"Autopilot proposes {cands[0]['project']} step {cands[0]['step']}: "
                              f"{len(cands)} item{'s' if len(cands) != 1 else ''}",
                              ref=row["id"], session=None, project=cands[0]["project"], at=at)
        changed = True
    return changed


def open_per_step(home: Path) -> dict[str, int]:
    """Open proposals of the autopilot per ``project|step``."""
    counts: dict[str, int] = {}
    for req in inbox_mod.requests(home):
        if req["kind"] == inbox_mod.PROPOSAL and req["status"] != "done":
            key = _key(req["project"], str(req["step"] or ""))
            counts[key] = counts.get(key, 0) + 1
    return counts


def sitting(home: Path, state: dict, projects: list[dict], agents: list[dict], now: datetime) -> list[dict]:
    """Open items nobody works on that the manager was not told about yet, each with ``idle_s`` (how long)."""
    own = _own(state)
    found, _ = candidates([p for p in projects if p.get("roadmap")], agents, queued_lanes(home))
    groups = _step_groups(found)
    out = []
    for key, cands in groups.items():
        rec = own["proposals"].get(key)
        if rec and rec["fp"] == _step_fp(proposal_text(cands, agents)):
            continue
        seen = parse_iso((own["seen"].get(key) or {}).get("at"))
        out.extend({**c, "idle_s": (now - seen).total_seconds() if seen else 0.0} for c in cands)
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


def _counts(own: dict, requests: list[dict], projects: list[dict], agents: list[dict], queued: set[str]) -> dict:
    found, present = candidates(projects, agents, queued)
    groups = _step_groups(found)
    live = {rec["id"] for rec in own["proposals"].values()
            if any(r["id"] == rec["id"] and r["status"] != "done" for r in requests)}
    proposed = sum(len(c) for key, c in groups.items() if (own["proposals"].get(key) or {}).get("id") in live)
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
    waiting = sum(len(c) for key, c in groups.items()
                  if (own["proposals"].get(key) or {}).get("fp") != _step_fp(proposal_text(c, agents)))
    return {"proposed": proposed, "queued": waiting,
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
    queued = queued_lanes(home)
    counts = _counts(own, inbox_mod.requests(home), planned, agents, queued)
    from tirith import diskdiet as diskdiet_mod
    if diskdiet_mod.low(state):
        counts["skipped"] += [{"project": p, "step": s, "why": "it waits for free disk space"}
                              for p, s in {tuple(k.split("|")[:2]) for k in candidates(planned, agents, queued)[0]}]
    steps = {(p["id"], st["id"]) for p in planned for st in p["roadmap"]["steps"]}
    working = sum(1 for a in agents if a.get("status") == "working" and (a.get("project"), str(a.get("step") or "")) in steps)
    allowed, why = loadlimit_mod.current(config, home)  # lowered while the machine is loaded or the disk is low
    return {"enabled": effective(config, state), "max_parallel": config.autopilot_max_parallel,
            "limit": allowed, "limit_why": why,
            "working": working, "started": [], "paused_steps": [], **counts}
