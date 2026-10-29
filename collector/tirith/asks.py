"""Asks: what the manager needs from the owner, each with a clear call to action.

An ask is a row of ``<home>/waits.jsonl`` with ``options`` (one-click answers), an optional
``command`` to copy and a ``kind``. The owner's answer is a ``reply`` row in ``inbox.jsonl``
(see ``inbox.add_reply``); the collector marks the ask answered. What only the manager must
see (a lane report with an "open" section) goes to ``<home>/manager_queue.jsonl`` instead.
See ``docs/CONTRACT-inbox.md``.
"""

from __future__ import annotations

import hashlib
import re
import secrets
from pathlib import Path

from tirith.home import now_iso
from tirith.store import append_jsonl, read_jsonl

ASK_KINDS = ("decision", "action", "approval", "info")
MAX_OPTIONS = 6
LABEL_MAX = 200  # an option label is a whole sentence at times; the page wraps it (the reply value keeps 200 too)
QUEUE_FILE = "manager_queue.jsonl"
OLD_QUEUE_FILE = "steward_queue.jsonl"  # before the rename; read while the new file does not exist
QUEUE_TEXT_MAX = 1500
QUEUE_STATE_ROWS = 50

NO_CTA_KIND = "no-call-to-action"
NOT_OWNER_KIND = "not-owner-item"  # an item with a call to action but a source the owner is not shown
AGENT_WAITING_KIND = "agent-waiting"  # what the brain guesses a working agent waits on
AUTO_CLOSED = "no longer waiting"
# options of the owner items Tirith itself generates (violations, brain-detected waits, room waits).
# There is no "let the manager handle it": the manager's work is the default, never an option.
GOT_IT_OPTIONS = [{"label": "Got it", "value": "ack"}]

# Why an item may be on the owner's list at all: only what an agent cannot do (``why_owner``).
# Anything else, or no reason, is not an owner item (see ``tirith.ownergate``).
WHY_OWNER = {
    "credentials": "credentials",  # passwords, keys, store/developer accounts
    "payment": "payment",
    "personal-account": "personal account",  # a login only the owner can do
    "physical-device": "physical device",
    "owner-decision": "decision",  # a product choice with options
}
LEGACY_WHY = {"account": "personal-account", "device": "physical-device", "legal": "owner-decision"}
AGENT_CAN_DO_KIND = "agent-can-do"  # a manager queue item the dispatcher turns into a verification lane


def why_key(why) -> str | None:
    """The key of ``WHY_OWNER`` for ``why`` (an old key is mapped), else ``None``."""
    key = str(why or "").strip().lower().replace("_", "-").replace(" ", "-")
    key = LEGACY_WHY.get(key, key)
    return key if key in WHY_OWNER else None


def reason_tag(why: str, detail: str | None = None) -> str:
    """``"Needs your Apple account"`` for a key of ``WHY_OWNER`` and an optional detail."""
    detail = re.sub(r"^your\s+", "", " ".join(str(detail or "").split()), flags=re.IGNORECASE)[:60]
    if why in ("credentials", "personal-account", "physical-device"):
        return f"Needs your {detail or WHY_OWNER[why]}"
    return f"Needs your {WHY_OWNER[why]}" + (f": {detail}" if detail else "")


_BUTTON_CLAIM = re.compile(
    r"\(?\b(?:(?:bot[oó]n|button|click|clic|pulsa|press|haz\s+clic\s+en)\s+[\"“'`]?(?P<a>run|open|done|abrir|ejecutar)\b"
    r"|(?P<b>run|open|done)\s+button)[^()]*?\)?", re.IGNORECASE)
_BUTTON_KIND = {"run": "run", "ejecutar": "run", "open": "open", "abrir": "open", "done": "done"}


def button_claims(text: str) -> set[str]:
    """The buttons (``run`` / ``open`` / ``done``) a text promises ("botón Run", "click Run", "Run button")."""
    return {_BUTTON_KIND[(m.group("a") or m.group("b")).lower()] for m in _BUTTON_CLAIM.finditer(str(text or ""))}


def missing_buttons(text: str, options=None, actions=None, link: str | None = None) -> set[str]:
    """Buttons the text promises that the item does not have."""
    labels = {str(o.get("label") or "").strip().lower() for o in options or [] if isinstance(o, dict)}
    present = {"run": any(isinstance(a, dict) and a.get("command") for a in actions or [])
               or any(isinstance(o, dict) and o.get("command") for o in options or []),
               "open": bool(link), "done": "done" in labels}
    return {kind for kind in button_claims(text) if not present[kind]}


def drop_button_claims(text: str) -> str:
    """The text without the phrases that promise a button ("(dueño: botón Run)" and the like)."""
    out = _BUTTON_CLAIM.sub("", str(text or ""))
    out = re.sub(r"\(\s*(?:due[ñn]o|owner)?\s*:?\s*\)", "", out, flags=re.IGNORECASE)
    out = re.sub(r"\s+([.,;:])", r"\1", re.sub(r"\s+", " ", out))
    return out.strip(" :;,-")

LANE_OPTIONS = [{"label": "Retry", "value": "retry"}, {"label": "Show the log", "value": "log"},
                {"label": "Drop it", "value": "drop"}]

_OPEN_HEADING = re.compile(
    r"^\s*(?:#{1,6}\s*|\*\*|__)?\s*(?:open(?:\s+(?:questions?|items?|points?|issues?))?"
    r"|needs?\s+(?:the\s+)?owner|needs?\s+you|left\s+(?:for|to)\s+(?:you|the\s+owner)"
    r"|for\s+(?:you|the\s+owner)|owner\s+(?:to[- ]do|actions?))\b[^\n]*$", re.IGNORECASE)
_ANY_HEADING = re.compile(r"^\s*(?:#{1,6}\s+\S|\*\*[^*\n]+\*\*:?\s*$|__[^_\n]+__:?\s*$)")


class AskError(Exception):
    """An ask that cannot be recorded; the message says why."""


def cut_label(label: str) -> str:
    """``label`` in at most ``LABEL_MAX`` characters: a longer one ends at a word with "…"."""
    label = " ".join(str(label).split())
    if len(label) <= LABEL_MAX:
        return label
    head = label[:LABEL_MAX - 1]
    if " " in head:
        head = head.rsplit(" ", 1)[0]
    return head.rstrip(" ,;:.-") + "…"


def parse_option(spec: str) -> dict:
    """``"Label|value"`` -> ``{"label", "value"}`` (the value defaults to the label)."""
    label, sep, value = str(spec).partition("|")
    label, value = cut_label(label), value.strip()
    if not label:
        raise AskError(f"an option needs a label: {spec!r}")
    return {"label": label, "value": value if sep and value else label}


def clean_options(options) -> list[dict]:
    out = []
    for item in options or []:
        if isinstance(item, str):
            item = parse_option(item)
        if isinstance(item, dict) and str(item.get("label") or "").strip():
            label = cut_label(item["label"])
            option = {"label": label, "value": str(item.get("value") or label)}
            if item.get("command"):  # an option that runs a command (dispatcher "Fix it" / "Try again")
                option["command"] = str(item["command"])
            out.append(option)
    return out[:MAX_OPTIONS]


def has_call_to_action(item: dict) -> bool:
    """The rule: an item for the owner carries at least one one-click option or one Run action."""
    return bool(clean_options(item.get("options"))) or any(
        isinstance(a, dict) and a.get("command") for a in item.get("actions") or [])


# Sources the owner may be shown. Anything else a generator raises (a brain guess about a working
# agent, a future source) goes to the manager's queue.
OWNER_SOURCES = ("manual", "guard", "setup", "plan", "lane", "room")


def is_owner_item(item: dict) -> bool:
    """The collector's last gate: a one-click option or Run action, an allow-listed source AND a ``why_owner``."""
    return has_call_to_action(item) and str(item.get("source") or "") in OWNER_SOURCES \
        and why_key(item.get("why_owner")) is not None


def default_kind(options, command) -> str:
    return "decision" if options else "action" if command else "info"


def open_section(report: str | None) -> str:
    """The "open / needs the owner / left for you" part of a lane report, else ``""``.

    It runs from that heading to the next heading; a one-line ``Open: ...`` counts too."""
    lines = str(report or "").splitlines()
    for index, line in enumerate(lines):
        if not _OPEN_HEADING.match(line):
            continue
        head = re.sub(r"^\s*(?:#{1,6}\s*|\*\*|__)?", "", line)
        inline = re.sub(r"^[^:\n]*?(?:\*\*|__)?:\s*(?:\*\*|__)?", "", head, count=1).strip() if ":" in head else ""
        body = [inline] if inline and not re.fullmatch(r"[*_\s]*", inline) else []
        for later in lines[index + 1:]:
            if body and not later.strip() and inline:
                break  # a one-line "Open: ..." ends at the blank line
            if _ANY_HEADING.match(later) and not later.lstrip().startswith(("- ", "* ")):
                break
            body.append(later.rstrip())
        text = "\n".join(body).strip()
        if text and not re.fullmatch(r"(?i)[\s\-*_.]*(none|nothing|n/?a|no)[\s\-*_.]*", text):
            return text[:QUEUE_TEXT_MAX]
    return ""


# -- the manager's own queue ------------------------------------------------------------------


def queue_path(home: Path) -> Path:
    new, old = Path(home) / QUEUE_FILE, Path(home) / OLD_QUEUE_FILE
    return old if old.exists() and not new.exists() else new


def queue_add(home: Path, project: str, lane: str, text: str, kind: str = "review",
              item_id: str | None = None) -> dict:
    """Add a manager queue item; with ``item_id`` an id that is already queued is not added twice."""
    if item_id:
        for row in queue_items(home):
            if row["id"] == item_id:
                return row
    row = {"id": item_id or "sq-" + secrets.token_hex(4), "at": now_iso(), "kind": kind, "project": project,
           "lane": lane, "text": text, "to": "manager", "status": "open"}
    append_jsonl(queue_path(home), row)
    return row


def sync_agent_waiting(home: Path, found: list[dict], keep_lanes: set[str]) -> None:
    """Manager queue items ``agent-waiting`` for what the brain thinks working agents wait on.

    ``found``: ``{project, lane, text}`` rows. One open item per lane + text (the id is a hash of both).
    An open item whose text is no longer in ``found`` is closed (``result: AUTO_CLOSED``), unless its lane
    is in ``keep_lanes`` (the brain has not answered for it yet). A text the brain says again after it
    was closed automatically opens a new item; one the manager closed stays closed."""
    current = set()
    for row in found:
        text = " ".join(str(row["text"]).split())
        base = "sq-aw-" + hashlib.sha1(f"{row['lane']}\n{text.lower()}".encode("utf-8")).hexdigest()[:10]
        current.add(base)
        same = [r for r in queue_items(home) if r["id"] == base or r["id"].startswith(base + "-")]
        if any(r["status"] == "open" for r in same):
            continue
        if same and not all(r.get("result") == AUTO_CLOSED for r in same):
            continue
        queue_add(home, str(row["project"] or ""), str(row["lane"]), text, kind=AGENT_WAITING_KIND,
                  item_id=base if not same else f"{base}-{len(same) + 1}")
    for item in queue_items(home, only_open=True):
        if item["kind"] == AGENT_WAITING_KIND and item["lane"] not in keep_lanes \
                and not any(item["id"] == c or item["id"].startswith(c + "-") for c in current):
            queue_done(home, item["id"], AUTO_CLOSED)


def queue_done(home: Path, item_id: str, result: str | None = None) -> None:
    if not any(r["id"] == item_id for r in queue_items(home)):
        raise AskError(f"no manager queue item {item_id}")
    row = {"id": item_id, "at": now_iso(), "done": True}
    if result:
        row["result"] = str(result)[:QUEUE_TEXT_MAX]
    append_jsonl(queue_path(home), row)


def queue_items(home: Path, only_open: bool = False) -> list[dict]:
    out: dict[str, dict] = {}
    for row in read_jsonl(queue_path(home)):
        rid = str(row.get("id") or "")
        if not rid:
            continue
        if row.get("done"):
            if rid in out:
                out[rid]["status"] = "done"
                if row.get("result"):
                    out[rid]["result"] = str(row["result"])
        elif rid not in out:
            out[rid] = {"id": rid, "at": row.get("at"), "kind": row.get("kind") or "review",
                        "project": row.get("project"), "lane": row.get("lane"),
                        "text": str(row.get("text") or ""), "to": "manager", "status": "open"}
    rows = list(out.values())
    return [r for r in rows if r["status"] == "open"] if only_open else rows
