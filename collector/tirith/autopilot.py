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
import json
import re
from datetime import datetime
from pathlib import Path
from typing import Callable

from tirith import actions as actions_mod
from tirith import coordination
from tirith import flows as flows_mod
from tirith import inbox as inbox_mod
from tirith import loadlimit as loadlimit_mod
from tirith import mergeq as mergeq_mod
from tirith import plangate as plangate_mod
from tirith import projects as projects_mod
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
HOLD_RECHECK_S = 60.0    # a held step is checked again after this long (the project check reads files)
OWNER_STOP_HOLD_S = 6 * 3600.0  # a step whose lane the owner stopped is not proposed again for this long
ANSWER_EXPIRES_S = 2 * 3600.0   # an answered proposal whose items still sit with nobody on them is asked again
ANSWER_QUOTED = 200             # characters of the earlier answer quoted in the new proposal
ANSWER_EARLY_S = 10 * 60.0      # ... or after this long, once what the answer waited for ended (a lane, its review)
# an answer that waits on the owner: asked again, it says so when nothing is open for the owner to answer
WAITS_ON_OWNER = re.compile(r"\b(?:owner|owner's|dueñ[oa])\b", re.I)
# an answer that waits for a free slot: any lane that ends after it may be what it waited for
WAITS_FOR_SLOT = re.compile(r"\bslots?\b|\bbusy\b|\blimit\b|\b\d+\s*/\s*\d+\b", re.I)


def _own(state: dict) -> dict:
    own = state.setdefault("autopilot", {})
    for key in ("owner_items", "proposals", "seen", "waits"):
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


def candidates(projects: list[dict], agents: list[dict], queued: set[str] | None = None, home: Path | None = None
               ) -> tuple[dict[str, dict], set[str]]:
    """``(open, present)``: ``open`` maps item key -> the open agent items nobody works on (the ones to tell the
    manager about; not the ones whose lane is in the merge queue); ``present`` is the keys of every open agent
    item (working or not)."""
    found: dict[str, dict] = {}
    present: set[str] = set()
    picked: dict[str, list[list[str]]] = {}
    for project in projects:
        if projects_mod.is_merge_worktree(project.get("folder")):
            continue  # a merge queue copy (_mergeq-*) repeats the plan of its project: no proposals from it
        road = project.get("roadmap") or {}
        all_items = [item for group in road.get("steps", []) for item in group["items"]]
        by_id = {i.get("id") or roadmap_mod.item_id(i["text"]): i for i in all_items}
        reservations = coordination.reservations(home, project["id"]) if home else []
        for step in road.get("steps", []):
            if step["id"] == plangate_mod.OPS_STEP:
                continue  # step 0 holds Tirith's own work (reviews, plan refresh, manager)
            mine = _on_step(agents, project["id"], step["id"])
            for item in _open_agent(step):
                clean, state, detail = roadmap_mod.split_status(item["text"])
                task_id = item.get("id") or roadmap_mod.item_id(item["text"])
                key = f"{project['id']}|{step['id']}|{task_id}"
                present.add(key)
                # Old lane records without an item remain conservative; known items no longer block a whole step.
                related = [agent for agent in mine if (not agent.get("item") and not agent.get("task_id")) or
                           roadmap_mod.split_status(str(agent.get("item") or ""))[0] == clean or agent.get("task_id") == task_id]
                busy = _busy(related, project["id"], step["id"])
                blocked = bool(item.get("metadata_error")) or any(dependency not in by_id or not by_id[dependency]["done"]
                              for dependency in item.get("depends_on", []))
                reserved = any(row["id"] == task_id or coordination.overlaps(item.get("writes", []), json.loads(row["writes"]))
                               for row in reservations if not by_id.get(row["id"], {}).get("done"))
                for agent in agents:
                    if agent.get("project") != project["id"] or not (
                            agent.get("status") == "working" or _review_status(agent) in (*REVIEW_OPEN, "ready")):
                        continue
                    other = next((i for i in all_items if
                                  (i.get("id") and i["id"] == agent.get("task_id")) or
                                  (agent.get("item") and roadmap_mod.split_status(i["text"])[0] ==
                                   roadmap_mod.split_status(agent["item"])[0])), {})
                    reserved |= coordination.overlaps(item.get("writes", []), other.get("writes", []))
                # Two ready items that write the same files are not launched together: the first in plan order goes.
                writes = item.get("writes", [])
                reserved |= any(coordination.overlaps(writes, taken) for taken in picked.get(project["id"], []))
                if not busy and not blocked and not reserved and not _queue_owned(clean, detail, mine, queued or set()):
                    if writes:
                        picked.setdefault(project["id"], []).append(writes)
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


def proposal_text(cands: list[dict], agents: list[dict], now_line: str = "") -> str:
    """One step's proposal, under ``MAX_TEXT`` characters: its items that need a decision, each with its facts, and
    ``now_line`` (the slots in use, which is no fact of the step: its fingerprint is taken without it)."""
    first = cands[0]
    plan = str(first["plan"] or "docs/TODO.md")
    plan = plan if len(plan) <= 60 else "…" + plan[-59:]
    head = [f"Autopilot: {first['project']} step {first['step']} ({flows_mod.one_line(first['title'], 40)}): "
            f"{len(cands)} item{'s' if len(cands) != 1 else ''} need{'' if len(cands) != 1 else 's'} a decision. "
            f"Plan: {plan}."]
    tail = ["Decide: launch (written brief + plan link), combine with a lane, wait (say why) or drop (plan edit; "
            "`tirith plan done` if on main). Answer here, then close it."]
    shown = cands[:ITEMS_SHOWN]
    room = MAX_TEXT - len(head[0]) - len(tail[0]) - 30 - len(shown) - (len(now_line) + 1 if now_line else 0)
    each = max(50, room // max(len(shown), 1))  # 8 lines of 50 leave room for the head, the slots and the tail
    lines = [flows_mod.one_line(f"- {c['item'][:60]} [{_why_short(c['state'])}] {_facts(c, agents)}", each) for c in shown]
    if len(cands) > len(shown):
        lines.append(f"- … and {len(cands) - len(shown)} more")
    return "\n".join([*head, *lines, *([now_line] if now_line else []), *tail])[:MAX_TEXT]


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


def _item_fp(cand: dict, agents: list[dict]) -> str:
    """A hash of one item's own facts: its text and note, why it is open and the lane facts shown for it."""
    return _step_fp(f"{cand['fp']}|{_why_short(cand['state'])}|{_facts(cand, agents)}")


def _remember_waits(own: dict, reqs: dict[str, dict], now: datetime) -> bool:
    """Remember, per item, that the manager answered the proposal that listed it (the item still sat with nobody
    on it, so the answer was a wait), keyed by the item and a hash of its own facts. Forgets what is ``ANSWER_EXPIRES_S`` old."""
    changed = False
    for rec in own["proposals"].values():
        req, items = reqs.get(rec.get("id")), rec.get("items")
        answers = [a for a in (req or {}).get("answers", []) if a.get("by") == "manager"]
        if not answers or not isinstance(items, dict):
            continue
        for ikey, ifp in items.items():
            row = {"fp": ifp, "at": answers[-1]["at"], "text": _clip(answers[-1].get("text"), ANSWER_QUOTED)}
            if own["waits"].get(ikey) != row:
                own["waits"][ikey] = row
                changed = True
    for ikey in [k for k, w in own["waits"].items()
                 if not (parse_iso(w.get("at")) and (now - parse_iso(w["at"])).total_seconds() < ANSWER_EXPIRES_S)]:
        del own["waits"][ikey]
        changed = True
    return changed


def _unwaited(own: dict, found: dict[str, dict], agents: list[dict], now: datetime) -> dict[str, dict]:
    """``found`` without the items the manager answered with a wait whose own facts did not change since, even when
    other items of their step changed. The wait ends as the step's answer does: ``ANSWER_EXPIRES_S`` old, or earlier
    once what it waited for happened (:func:`_waited_for`)."""
    out = {}
    for key, cand in found.items():
        wait = own["waits"].get(key)
        if wait and wait.get("fp") == _item_fp(cand, agents) and not _waited_for(wait, agents, now):
            at = parse_iso(wait.get("at"))
            if at is not None and (now - at).total_seconds() < ANSWER_EXPIRES_S:
                continue
        out[key] = cand
    return out


def _working(agents: list[dict]) -> list[dict]:
    """The agents that take a slot now: every working lane but the manager's."""
    return [a for a in agents if a.get("status") == "working" and not str(a.get("id", "")).startswith("manager")]


def _now_line(home: Path, config: Config | None, agents: list[dict]) -> str:
    """The slots in use as the proposal is written, so the manager answers on the facts of now, not on what its
    conversation remembers (a lane it saw working may have died with a restart since)."""
    working = sorted(str(a.get("id")) for a in _working(agents))
    used = f"{len(working)} of {loadlimit_mod.current(config, home)[0]}" if config is not None else str(len(working))
    return flows_mod.one_line(f"Now: {used} agent slots in use" + (f" ({', '.join(working)})" if working else ""),
                              120)


def _hold(home: Path, config: Config, project: str, step: str, agents: list[dict], now: datetime) -> str | None:
    """Why the manager could not launch anything for ``project`` step ``step`` now, in plain words; None when it can.
    Decided by code so that no manager turn is spent answering "wait"."""
    working = len(_working(agents))
    allowed, why = loadlimit_mod.current(config, home)
    if working >= allowed:
        return f"the agents running now fill the limit of {allowed}" + (f" ({why})" if why else "")
    proj = next((p for p in config.projects if p.id == project), None)
    if proj is not None and proj.repo:
        from tirith import manifest as manifest_mod
        from tirith.guardwrap import default_words
        failed = [p for p in manifest_mod.check(proj.repo, config.words_for(proj, default_words()))
                  if p.rule != "manifest-missing"]
        if failed:
            return f"tirith check fails for '{project}' (run `tirith check --project {project}`)"
    from tirith.store import read_lanes
    for lane in read_lanes(home):
        stop = lane.get("stop") if isinstance(lane, dict) else None
        at = parse_iso((stop or {}).get("at")) if isinstance(stop, dict) else None
        if (at and stop.get("by") == "owner" and lane.get("project") == project
                and str(lane.get("item_step") or lane.get("step") or "") == step
                and (now - at).total_seconds() < OWNER_STOP_HOLD_S):
            return f"the owner stopped lane {lane['id']} of this step at {stop['at']}"
    return None


def _clip(text: str, limit: int) -> str:
    text = " ".join(str(text or "").split())
    return text if len(text) <= limit else text[:limit - 1] + "…"


def _ended_since(agents: list[dict], at: datetime) -> list[tuple[datetime, str, str]]:
    """``(when, lane id, what happened)`` for every lane (not the manager's or the CEO's) that ended, or whose review
    ended, after ``at``; newest first."""
    done = (mergeq_mod.SENT_BACK_STATUS, "merged", *REVIEW_FAILED)
    found = []
    for agent in agents:
        lane = str(agent.get("id", ""))
        if not lane or lane.startswith(("manager", "ceo-")):
            continue
        ended = parse_iso(agent.get("ended_at"))
        if ended and ended > at:
            found.append((ended, lane, f"lane {lane} ended at {agent['ended_at']}"))
        review = agent.get("review") if isinstance(agent.get("review"), dict) else {}
        closed = parse_iso(review.get("at"))
        if closed and closed > at and review.get("status") in done:
            found.append((closed, lane, f"the review of lane {lane} ended ({review['status']}) at {review['at']}"))
    return sorted(found, key=lambda row: row[0], reverse=True)


def _names(text: str, lane: str) -> bool:
    """Whether the answer ``text`` names ``lane``: by its id, or by its number as the manager writes it (``-28``)."""
    if lane in text:
        return True
    number = re.search(r"-\d+$", lane)
    return bool(number and re.search(rf"(?<![\w-]){re.escape(number.group())}(?!\d)", text))


def _waited_for(last: dict, agents: list[dict], now: datetime) -> str | None:
    """What the answer ``last`` waited for and that happened since, once it is ``ANSWER_EARLY_S`` old: a lane it
    names ended (or its review did), or it waited for a free slot and a lane ended. None otherwise: a wait for
    anything else (the owner, a key) stands until it expires."""
    at, text = parse_iso(last.get("at")), str(last.get("text") or "")
    if at is None or not text or (now - at).total_seconds() < ANSWER_EARLY_S:
        return None
    ended = _ended_since(agents, at)
    named = next((what for _, lane, what in ended if _names(text, lane)), None)
    return named or (ended[0][2] if ended and WAITS_FOR_SLOT.search(text) else None)


def _owner_asked(home: Path, project: str) -> bool:
    """Whether anything of ``project`` is on the owner's list now (``state.json``'s ``waiting_on_you``)."""
    state = read_json(home / "state.json", None)
    rows = state.get("waiting_on_you") if isinstance(state, dict) else None
    return any(isinstance(w, dict) and str(w.get("project") or "") == project for w in rows or [])


def _expired_answer(req: dict | None, now: datetime, agents: list[dict] | None = None) -> dict | None:
    """The manager's last answer to the done proposal ``req`` once it is ``ANSWER_EXPIRES_S`` old, or earlier once
    what it waited for happened (``since`` says what, see :func:`_waited_for`); else None. A proposal closed without an
    answer is no decision: it counts from when it was asked (its ``text`` is empty)."""
    if not req:
        return None
    answers = [a for a in req.get("answers", []) if a.get("by") == "manager"]
    last = answers[-1] if answers else {"at": req.get("at"), "text": ""}
    at = parse_iso(last.get("at"))
    if at is None:
        return None
    if (now - at).total_seconds() >= ANSWER_EXPIRES_S:
        return last
    since = _waited_for(last, agents or [], now)
    return {**last, "since": since} if since else None


def _propose(home: Path, state: dict, projects: list[dict], agents: list[dict], now: datetime,
             config: Config | None = None) -> bool:
    """One open proposal per (project, step) that lists its items nobody works on (after ``DEBOUNCE_S``). When the
    facts change it is closed and replaced; older ones (before this rule) are collapsed into it."""
    own, at, changed = _own(state), now_iso(now), False
    found, present = candidates(projects, agents, queued_lanes(home), home=home)
    reqs = {r["id"]: r for r in inbox_mod.requests(home) if r["kind"] == inbox_mod.PROPOSAL}
    changed = _remember_waits(own, reqs, now) or changed
    all_groups = _step_groups(found)
    groups = _step_groups(_unwaited(own, found, agents, now))
    present_steps = {k.rsplit("|", 1)[0] for k in present}
    by_step: dict[str, list[dict]] = {}
    answered: dict[str, dict] = {}
    for req in inbox_mod.requests(home):
        if req["kind"] == inbox_mod.PROPOSAL and req["status"] != "done":
            by_step.setdefault(_key(req["project"], str(req["step"] or "")), []).append(req)
        elif req["kind"] == inbox_mod.PROPOSAL:
            answered[req["id"]] = req
    for key in [k for k in own["proposals"] if k.count("|") != 1]:
        del own["proposals"][key]  # one per item, from before: collapsed below
        changed = True
    for key in list(own["seen"]):
        if key not in all_groups:  # a step whose items all wait keeps its debounce
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
        earlier = None
        if rec and rec["fp"] == fp:
            earlier = _expired_answer(answered.get(rec["id"]), now, agents)
            if earlier is None:
                continue  # answered (or still open) and not expired: the manager's decision stands
        seen = own["seen"].get(key)
        if not seen or seen["fp"] != fp:
            own["seen"][key] = {"fp": fp, "at": at}
            changed = True
            continue
        first = parse_iso(seen["at"])
        if first is None or (now - first).total_seconds() < DEBOUNCE_S:
            continue
        if config is not None:
            held = own.setdefault("held", {})
            last = held.get(key) if isinstance(held.get(key), dict) else None
            checked = parse_iso((last or {}).get("checked"))
            if last and checked and (now - checked).total_seconds() < HOLD_RECHECK_S:
                continue
            why = _hold(home, config, cands[0]["project"], str(cands[0]["step"]), agents, now)
            if why:
                if not last or last.get("why") != why:
                    changed = True
                held[key] = {"why": why, "checked": at}
                continue
            if held.pop(key, None) is not None:
                changed = True
        text = proposal_text(cands, agents, _now_line(home, config, agents))
        for req in by_step.get(key, []):
            if _open_request(req) and rec and req["id"] == rec["id"]:
                _close(home, req, cands[0]["project"], cands[0]["step"], "its facts changed; replaced by a new one", at)
        if earlier is not None and earlier.get("since"):  # what it waited for happened: no need to sit out the wait
            text += (f"\nAsked again: you answered at {earlier['at']}: \"{_clip(earlier['text'], ANSWER_QUOTED)}\"; "
                     f"since then {earlier['since']}.")
        elif earlier is not None and earlier["text"]:  # a wait without a deadline would park the step forever
            text += (f"\nAsked again: you answered at {earlier['at']} and nobody is on these items since: "
                     f"\"{_clip(earlier['text'], ANSWER_QUOTED)}\"")
        elif earlier is not None:  # closed without a word: the step would never be asked again
            text += (f"\nAsked again: it was asked at {earlier['at']} and closed without an answer; nobody is on "
                     "these items since. Answer what you decide (launch, or wait until when, and why).")
        if earlier is not None and WAITS_ON_OWNER.search(str(earlier.get("text") or "")) \
                and not _owner_asked(home, cands[0]["project"]):  # a wait on an owner who was never asked
            text += ("\nNothing of this project is on the owner's list (no question, no stop): if only the owner "
                     "can decide, ask them now in one line (`tirith wait add`), else decide yourself.")
        row = inbox_mod.add_request(home, inbox_mod.PROPOSAL, cands[0]["project"], cands[0]["step"],
                                    text, at=at, sender="tirith")
        own["proposals"][key] = {"id": row["id"], "fp": fp, "at": at,
                                 "items": {c["key"]: _item_fp(c, agents) for c in cands[:ITEMS_SHOWN]}}
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
    groups = _step_groups(_unwaited(own, found, agents, now))
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
        changed |= _propose(home, state, planned, agents, now, config)
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
    for key, held in (own.get("held") or {}).items():
        if isinstance(held, dict) and key.count("|") == 1:
            project, step = key.split("|")
            counts["skipped"].append({"project": project, "step": step, "why": held.get("why")})
    steps = {(p["id"], st["id"]) for p in planned for st in p["roadmap"]["steps"]}
    working = sum(1 for a in agents if a.get("status") == "working" and (a.get("project"), str(a.get("step") or "")) in steps)
    allowed, why = loadlimit_mod.current(config, home)  # lowered while the machine is loaded or the disk is low
    return {"enabled": effective(config, state), "max_parallel": config.autopilot_max_parallel,
            "limit": allowed, "limit_why": why,
            "working": working, "started": [], "paused_steps": [], **counts}
