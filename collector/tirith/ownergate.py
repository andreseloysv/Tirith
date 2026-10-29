"""The one gate in front of the owner: nothing reaches the owner unless only the owner can do it.

Every path that raises an owner item (``dispatch.add_owner_item``, ``tirith wait add``, a roadmap
``(dueño: ...)`` tag, the "needs the owner" list of a plan-refresh report) calls ``admit``. It needs a
``why_owner`` from ``asks.WHY_OWNER``. Without one the text is something an agent can do: it becomes a
manager queue item ``agent-can-do`` that the dispatcher turns into a verification lane (``verifylane``).
An item about something marked out of plan is dropped and logged to ``owner_dropped.jsonl``.
See ``docs/CONTRACT-inbox.md`` ("The owner gate").
"""

from __future__ import annotations

import hashlib
import re
from pathlib import Path

from tirith import asks as asks_mod
from tirith import howto as howto_mod
from tirith.home import now_iso
from tirith.store import append_jsonl, read_jsonl

OWNER, MANAGER, DROPPED = "owner", "manager", "dropped"
DROPPED_FILE = "owner_dropped.jsonl"
CONCRETE_KIND = "make-concrete"  # a manager queue item: an owner how-to with placeholders
MOVED_REASON = "moved to the manager queue"
OUT_OVERLAP = 0.6

_OUT_RE = re.compile(r"\((?:fuera de este plan|out of this plan)", re.IGNORECASE)
_OUT_TAIL = re.compile(r"\((?:fuera de este plan|out of this plan).*$", re.IGNORECASE)
_WORD_RE = re.compile(r"[a-z0-9áéíóúñü]{4,}")
_WHY_TAG_RE = re.compile(
    r"^\s*(?:\[\s*(?:why[_-]owner\s*[:=]\s*)?([a-z_ -]+?(?::[^\]]*)?)\s*\]"
    r"|\(\s*why[_-]owner\s*[:=]\s*([a-z_ -]+?(?::[^)]*)?)\s*\))\s*[:,-]?\s*", re.IGNORECASE)


def _words(text: str) -> set[str]:
    return set(_WORD_RE.findall(str(text or "").lower()))


def is_out_of_plan(text: str, out_items: list[str] | tuple[str, ...] = ()) -> bool:
    """The text carries the out-of-plan tag, or mostly repeats the text of an item that does."""
    if _OUT_RE.search(str(text or "")):
        return True
    mine = _words(text)
    return bool(mine) and any(
        len(mine & theirs) >= 2 and len(mine & theirs) / min(len(mine), len(theirs)) >= OUT_OVERLAP
        for theirs in (_words(_OUT_TAIL.sub("", o)) for o in out_items) if theirs)


def _log_dropped(home: Path, item_id: str, project: str | None, text: str, via: str, at: str) -> None:
    append_jsonl(Path(home) / DROPPED_FILE, {"at": at, "id": item_id, "project": project, "via": via,
                                             "text": " ".join(str(text).split())[:asks_mod.QUEUE_TEXT_MAX]})


def agent_can_do(home: Path, project: str | None, text: str) -> dict:
    """Queue ``text`` for the manager as "agent can do: ..." (once per project and text)."""
    clean = " ".join(str(text or "").split())
    key = hashlib.sha1(f"{project or ''}\n{clean.lower()}".encode("utf-8")).hexdigest()[:10]
    return asks_mod.queue_add(home, str(project or ""), "", f"agent can do: {clean}"[:asks_mod.QUEUE_TEXT_MAX],
                              kind=asks_mod.AGENT_CAN_DO_KIND, item_id=f"sq-acd-{key}")


def send_back(home: Path, project: str | None, text: str, problem: str, item_id: str) -> dict:
    """Queue an owner item with a non-concrete how-to for the manager: ``make it concrete: look up ...``."""
    clean = " ".join(str(text or "").split())
    key = hashlib.sha1(f"{project or ''}\n{item_id}\n{problem}".encode("utf-8")).hexdigest()[:10]
    return asks_mod.queue_add(home, str(project or ""), "", f"{problem} (item: {clean})"[:asks_mod.QUEUE_TEXT_MAX],
                              kind=CONCRETE_KIND, item_id="sq-mc-" + key)


def admit(home: Path, item_id: str, project: str | None, text: str, why_owner, *, via: str = "",
          at: str | None = None, out_items: list[str] | tuple[str, ...] = (), out: bool = False,
          howto=None) -> str:
    """``"owner"`` when the item may reach the owner; otherwise it is routed (queued for the manager or
    dropped and logged) and ``"manager"`` / ``"dropped"`` comes back. Callers write the owner item only on
    ``"owner"``. A ``howto`` (dict or JSON) with a placeholder, or a command that cannot run as it is, sends the
    item back to the manager queue (``kind: "make-concrete"``)."""
    at = at or now_iso()
    if out or is_out_of_plan(text, out_items):
        _log_dropped(home, item_id, project, text, via, at)
        return DROPPED
    if asks_mod.why_key(why_owner) is None:
        agent_can_do(home, project, text)
        return MANAGER
    if howto:
        try:
            parsed = howto_mod.parse(howto)
            holder = howto_mod.has_placeholder(text) if parsed and parsed["steps"] else None
            if holder:
                raise howto_mod.HowtoError(f"{howto_mod.MAKE_CONCRETE}the real value of {holder} in the text")
        except howto_mod.HowtoError as error:
            send_back(home, project, text, str(error), item_id)
            return MANAGER
    return OWNER


# -- reports ----------------------------------------------------------------------------------

_OWNER_HEADING = re.compile(
    r"^\s*(?:#{1,6}\s*|\*\*|__)?\s*(?:needs?\s+(?:the\s+)?owner|needs?\s+you|checks?\s+only\s+you\s+can\s+do"
    r"|only\s+you\s+can\s+do|for\s+(?:you|the\s+owner)|owner\s+(?:to[- ]do|actions?|checks?))\b[^\n]*$",
    re.IGNORECASE)
_ANY_HEADING = re.compile(r"^\s*(?:#{1,6}\s+\S|\*\*[^*\n]+\*\*:?\s*$|__[^_\n]+__:?\s*$)")
_BULLET = re.compile(r"^(\s*[-*+]\s+)(.*)$")
_NONE = re.compile(r"(?i)[\s\-*_.]*(none|nothing|n/?a|no)\b.*")
NOTHING = "- nothing: no check is only yours"


def filter_report(home: Path, project: str | None, lane_id: str, report: str,
                  out_items: list[str] | tuple[str, ...] = (), at: str | None = None) -> str:
    """The report with the bullets of its "needs the owner" section passed through ``admit``.

    A bullet stays only when it names a reason: ``- [credentials] Sign in to ...`` or
    ``- [personal-account: Apple ID] ...`` (keys of ``asks.WHY_OWNER``). The others are queued for the
    manager as "agent can do", or dropped when they are about something out of plan; the report says how many."""
    lines = str(report or "").splitlines()
    out, index, moved = [], 0, 0
    while index < len(lines):
        out.append(lines[index])
        index += 1
        if not _OWNER_HEADING.match(out[-1]):
            continue
        section = []
        while index < len(lines) and (_BULLET.match(lines[index]) or not _ANY_HEADING.match(lines[index])):
            section.append(lines[index])
            index += 1
        kept = []
        for row in section:
            bullet = _BULLET.match(row)
            if not bullet or _NONE.fullmatch(bullet.group(2)):
                kept.append(row)
                continue
            body, why, detail = bullet.group(2), None, ""
            tag = _WHY_TAG_RE.match(body)
            if tag:
                why, _, detail = (tag.group(1) or tag.group(2) or "").partition(":")
                body = body[tag.end():]
            item_id = "report-" + hashlib.sha1(f"{lane_id}\n{body}".encode("utf-8")).hexdigest()[:8]
            if admit(home, item_id, project, body, why, via=f"report:{lane_id}", at=at, out_items=out_items) == OWNER:
                kept.append(f"{bullet.group(1)}{body} ({asks_mod.reason_tag(asks_mod.why_key(why), detail.strip() or None)})")
            else:
                moved += 1
        if not any(_BULLET.match(r) for r in kept):
            kept = [NOTHING, *kept]
        out.extend(kept)
    text = "\n".join(out)
    if moved:
        text += f"\n\n_{moved} check(s) an agent can do were handed to the manager, not to you._"
    return text


# -- startup ----------------------------------------------------------------------------------


def migrate(home: Path, at: str | None = None) -> dict:
    """Re-check every open item of ``waits.jsonl`` with the gate: one with a reason (or an old reason key)
    stays and gets its ``why_owner``; the others close and become manager queue items. Safe to repeat."""
    at, path = at or now_iso(), Path(home) / "waits.jsonl"
    latest: dict[str, dict] = {}
    for row in read_jsonl(path):
        if row.get("id"):
            latest[str(row["id"])] = row
    done = {"kept": 0, "moved": 0, "dropped": 0}
    for row in latest.values():
        if row.get("done_at") or asks_mod.why_key(row.get("why_owner")):
            continue
        why = asks_mod.why_key(row.get("reason"))
        if why:
            append_jsonl(path, {**row, "why_owner": why, "reason_tag": asks_mod.reason_tag(why, row.get("why_detail"))})
            done["kept"] += 1
            continue
        verdict = admit(home, str(row["id"]), row.get("project"), str(row.get("text") or ""), None,
                        via="migration", at=at)
        append_jsonl(path, {**row, "done_at": at, "done_reason": MOVED_REASON})
        done["dropped" if verdict == DROPPED else "moved"] += 1
    return done
