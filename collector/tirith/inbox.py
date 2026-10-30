"""The owner's inbox: "Finish it" and "What's going on?" requests on plan steps, and replies to asks.

The web page cannot run commands, so a request is one line appended to ``<home>/inbox.jsonl``;
the manager watches the file (``tirith inbox watch``) and answers with ``tirith inbox answer``.
The collector folds the rows into ``state.inbox``. See ``docs/CONTRACT-inbox.md``.
"""

from __future__ import annotations

import json
import mimetypes
import re
import secrets
import time
from pathlib import Path
from typing import Callable
from urllib.parse import parse_qs, urlsplit

from tirith import actions as actions_mod
from tirith.home import now_iso
from tirith.store import append_jsonl, read_jsonl

INBOX_FILE = "inbox.jsonl"
PROPOSAL = "autopilot-proposal"  # Tirith's own request to the manager: a plan item nobody works on
KINDS = ("finish", "ask", "setup", "chat", "autopilot", PROPOSAL, "plan-refresh", "plan-new", "plan-edit")
PLAN_KINDS = ("plan-refresh", "plan-new")
PLAN_STEP = "plan"  # the step of a request about the whole plan, not one step
MODELS_ACTIONS = ("models-set", "apply-manager")  # the Settings page; the request's project is "settings"
SETUP_ACTIONS = ("add-stage", "add-rule", "remove-stage", "remove-rule", "run-stage") + MODELS_ACTIONS
SETUP_STEP = "setup"
AUTOPILOT_STEP = "autopilot"  # the step of an ``autopilot`` request (text ``on`` or ``off``)
ANSWERERS = ("manager", "steward", "brain", "tirith")  # "steward" is the old name of "manager"
TEXT_MAX_BYTES = 4096
STATE_ROWS = 100
ATTACH_DIR = "attachments"  # <home>/attachments/<message-id>/<n>.<ext>
ATTACH_MAX = 6
ATTACH_MAX_BYTES = 10 * 1024 * 1024
ATTACH_TYPES = {"image/png": "png", "image/jpeg": "jpg", "image/gif": "gif", "image/webp": "webp"}  # shown as pictures
FILE_TYPE = "application/octet-stream"  # any other file: shown as a chip with its name
_MIME = re.compile(r"^[a-z0-9][a-z0-9.+-]{0,63}/[a-z0-9][a-z0-9.+-]{0,127}$")
AUTO_ANSWER_AFTER_S = 120
SCHEME = "tirith"


class InboxError(Exception):
    """A request that cannot be recorded; the message says why."""


def inbox_path(home: Path) -> Path:
    return Path(home) / INBOX_FILE


def clip(text: str, limit: int = TEXT_MAX_BYTES) -> str:
    """``text`` cut to ``limit`` UTF-8 bytes (never in the middle of a character)."""
    return str(text or "").encode("utf-8")[:limit].decode("utf-8", errors="ignore")


def clean_cid(cid) -> str:
    """A client id as stored: at most 64 characters of ``[A-Za-z0-9_-]``, else ''."""
    cid = str(cid or "").strip()
    return cid if 0 < len(cid) <= 64 and all(c.isalnum() and c.isascii() or c in "_-" for c in cid) else ""


def new_id() -> str:
    return "in-" + secrets.token_hex(4)


# -- attachments ------------------------------------------------------------------------------


def sniff_image(data: bytes) -> tuple[str, int, int] | None:
    """``(mime type, width, height)`` of a PNG, JPEG, GIF or WebP from its bytes (0, 0 if the size is
    not found), else None."""
    import struct
    if data[:8] == b"\x89PNG\r\n\x1a\n":
        return ("image/png", *struct.unpack(">II", data[16:24])) if len(data) >= 24 else ("image/png", 0, 0)
    if data[:6] in (b"GIF87a", b"GIF89a"):
        return ("image/gif", *struct.unpack("<HH", data[6:10])) if len(data) >= 10 else ("image/gif", 0, 0)
    if data[:3] == b"\xff\xd8\xff":
        i = 2
        while i + 9 < len(data):
            if data[i] != 0xFF:
                i += 1
                continue
            marker = data[i + 1]
            if marker in (0xD8, 0x01) or 0xD0 <= marker <= 0xD7:
                i += 2
                continue
            if 0xC0 <= marker <= 0xCF and marker not in (0xC4, 0xC8, 0xCC):
                h, w = struct.unpack(">HH", data[i + 5:i + 9])
                return ("image/jpeg", w, h)
            i += 2 + struct.unpack(">H", data[i + 2:i + 4])[0]
        return ("image/jpeg", 0, 0)
    if data[:4] == b"RIFF" and data[8:12] == b"WEBP":
        chunk = data[12:16]
        if chunk == b"VP8X" and len(data) >= 30:
            return ("image/webp", 1 + int.from_bytes(data[24:27], "little"), 1 + int.from_bytes(data[27:30], "little"))
        if chunk == b"VP8 " and len(data) >= 30:
            w, h = struct.unpack("<HH", data[26:30])
            return ("image/webp", w & 0x3FFF, h & 0x3FFF)
        if chunk == b"VP8L" and len(data) >= 25:
            bits = int.from_bytes(data[21:25], "little")
            return ("image/webp", (bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1)
        return ("image/webp", 0, 0)
    return None


def describe_attachment(path: Path | str) -> dict:
    """``{path, type, size, width, height, name}`` for a file of at most 10 MB (absolute path). The type of
    a PNG, JPEG, GIF or WebP comes from its bytes, of anything else from its name; InboxError otherwise."""
    path = Path(path).expanduser().resolve()
    try:
        size = path.stat().st_size
        if not path.is_file():
            raise InboxError(f"attachment {path.name} is not a file")
        data = path.read_bytes()
    except OSError as error:
        raise InboxError(f"cannot read attachment {path}: {error.strerror or error}") from None
    if size > ATTACH_MAX_BYTES:
        raise InboxError(f"attachment {path.name} is over {ATTACH_MAX_BYTES // (1024 * 1024)} MB")
    if not size:
        raise InboxError(f"attachment {path.name} is empty")
    found = sniff_image(data)
    if found is None:
        guessed = (mimetypes.guess_type(path.name)[0] or "").lower()
        kind = guessed if _MIME.match(guessed) and guessed not in ATTACH_TYPES else FILE_TYPE
        found = (kind, 0, 0)
    return {"path": str(path), "type": found[0], "size": size, "width": found[1], "height": found[2], "name": path.name}


def clean_attachments(raw) -> list[dict]:
    """The ``attachments`` of a row as stored: at most 6 well-formed ``{path, type, size, width, height}``
    (and ``name`` when the row has one). A type outside ``ATTACH_TYPES`` is a plain file."""
    out = []
    for item in raw if isinstance(raw, list) else []:
        if isinstance(item, dict) and str(item.get("path") or "").startswith("/") \
                and _MIME.match(str(item.get("type") or "")):
            num = lambda k: int(item[k]) if isinstance(item.get(k), (int, float)) and item[k] >= 0 else 0
            picture = item["type"] in ATTACH_TYPES
            clean = {"path": str(item["path"]), "type": item["type"], "size": num("size"),
                     "width": num("width") if picture else 0, "height": num("height") if picture else 0}
            name = "".join(c for c in str(item.get("name") or "") if c.isprintable())[:120].strip()
            if name:
                clean["name"] = name
            out.append(clean)
    return out[:ATTACH_MAX]


def attachment_list(paths: list[str] | None) -> list[dict]:
    """Describe the files named with ``--attach`` (at most 6)."""
    paths = list(paths or [])
    if len(paths) > ATTACH_MAX:
        raise InboxError(f"at most {ATTACH_MAX} files per message")
    return [describe_attachment(p) for p in paths]


def attachment_lines(req: dict) -> str:
    """The attachment paths of a request or answer as text for the manager ('' if none)."""
    return "\n".join(f"    image: {a['path']} ({a['type']}, {a['width']}x{a['height']})" if a["type"] in ATTACH_TYPES
                     else f"    file: {a['path']} ({a.get('name') or Path(a['path']).name}, {a['type']}, {a['size']} bytes)"
                     for a in req.get("attachments") or [])


# -- writing ----------------------------------------------------------------------------------


def add_request(home: Path, kind: str, project: str = "", step: str = "", text: str = "",
                at: str | None = None, attachments: list[dict] | None = None, rid: str | None = None,
                cid: str | None = None, sender: str = "owner", via: str = "") -> dict:
    """Append an open request and return its row. A ``cid`` (the page's client id) already on a row makes
    this a retry: nothing is written and that row is returned. ``sender`` is ``tirith`` for its own proposals.
    ``via`` names the private room a message came through (``room:<sealed-room>``); empty for the desktop."""
    if kind not in KINDS:
        raise InboxError(f"kind must be one of {', '.join(KINDS)}, not {kind!r}")
    project, step, text = str(project or "").strip(), str(step if step is not None else "").strip(), clip(text.strip())
    if kind == "setup":
        step = step or SETUP_STEP
        text = json.dumps(parse_setup(text), ensure_ascii=False, separators=(",", ":"))
    if kind == "autopilot":
        project, step, text = project or "all", step or AUTOPILOT_STEP, text.lower()
        if text not in ("on", "off"):
            raise InboxError("an autopilot request needs text 'on' or 'off'")
    if kind in PLAN_KINDS:
        step = step or PLAN_STEP
        if kind == "plan-new":
            text = json.dumps(parse_plan_new(text), ensure_ascii=False, separators=(",", ":"))
    if kind == "plan-edit":
        text = json.dumps(parse_plan_edit(text), ensure_ascii=False, separators=(",", ":"))
    attachments = clean_attachments(attachments)
    if kind == "chat":
        if not text and not attachments:
            raise InboxError("a chat message needs text or an attachment")
    elif not project or not step:
        raise InboxError("a request needs a project and a step")
    if kind == "ask" and not text:
        raise InboxError("a question needs text")
    cid = clean_cid(cid)
    if cid:
        seen = next((r for r in read_jsonl(inbox_path(home)) if r.get("cid") == cid and r.get("kind") in KINDS), None)
        if seen is not None:
            return seen
    row = {"id": rid or new_id(), "at": at or now_iso(), "kind": kind, "project": project, "step": step,
           "text": text, "from": sender, "status": "open"}
    if cid:
        row["cid"] = cid
    if via:
        row["via"] = str(via)[:200]
    if attachments:
        row["attachments"] = attachments
    append_jsonl(inbox_path(home), row)
    return row


def parse_setup(text: str) -> dict:
    """The JSON of a ``setup`` request: ``{"action": "add-stage", ...}`` with a known action."""
    try:
        data = json.loads(text)
    except (TypeError, ValueError):
        data = None
    if not isinstance(data, dict) or data.get("action") not in SETUP_ACTIONS:
        raise InboxError("a setup request needs text like {\"action\": \"add-stage\", ...} with action one of "
                         + ", ".join(SETUP_ACTIONS))
    return {str(k): v for k, v in data.items() if isinstance(v, (str, int, float, bool, list)) or v is None}


def parse_plan_new(text: str) -> dict:
    """The JSON of a ``plan-new`` request: ``{title, goal, deadline, steps}``; only the title is required."""
    try:
        data = json.loads(text)
    except (TypeError, ValueError):
        data = None
    if not isinstance(data, dict) or not str(data.get("title") or "").strip():
        raise InboxError('a plan-new request needs text like {"title": "...", "goal": "...", '
                         '"deadline": "", "steps": ""} with a title')
    return {key: str(data.get(key) or "").strip() for key in ("title", "goal", "deadline", "steps")}


def parse_plan_edit(text: str) -> dict:
    """The JSON of a ``plan-edit`` request: ``{plan, item_text, new_text | delete: true | add}``.

    Exactly one change: ``new_text`` (needs ``item_text``), ``delete`` (needs ``item_text``) or ``add``."""
    try:
        data = json.loads(text)
    except (TypeError, ValueError):
        data = None
    msg = ('a plan-edit request needs text like {"plan": "", "item_text": "...", "new_text": "..."}, '
           '{"item_text": "...", "delete": true} or {"add": "..."}')
    if not isinstance(data, dict):
        raise InboxError(msg)
    out = {"plan": str(data.get("plan") or "").strip(), "item_text": str(data.get("item_text") or "").strip()}
    given = [k for k in ("new_text", "delete", "add") if k in data]
    if given == ["delete"] and data["delete"] is True and out["item_text"]:
        out["delete"] = True
    elif given == ["new_text"] and isinstance(data["new_text"], str) and data["new_text"].strip() and out["item_text"]:
        out["new_text"] = data["new_text"].strip()
    elif given == ["add"] and isinstance(data["add"], str) and data["add"].strip() and not out["item_text"]:
        out["add"] = data["add"].strip()
    else:
        raise InboxError(msg)
    return out


def edit_verb(edit: dict) -> str:
    """``edit`` / ``delete`` / ``add``: the verb of the commit message."""
    return "delete" if edit.get("delete") else "add" if "add" in edit else "edit"


def setup_summary(setup: dict) -> str:
    """One line for a setup request: ``add-stage ci/lint: make lint``."""
    if setup.get("action") == "models-set":
        what = " ".join(f"{k}={setup[k]}" for k in ("agent", "model", "effort") if setup.get(k))
        if isinstance(setup.get("fallback"), list):
            what += " fallback=" + (",".join(map(str, setup["fallback"])) or "none")
        where = "/".join(str(setup[k]) for k in ("project", "step") if setup.get(k))
        return " ".join(f"models {setup.get('role')} {where} {what}".split())
    where = "/".join(str(setup[k]) for k in ("pipeline", "name", "id") if setup.get(k))
    detail = setup.get("run") or setup.get("match") or setup.get("regex") or setup.get("message") or ""
    return " ".join(f"{setup.get('action')} {where}: {detail}".split()).rstrip(": ")


def find_request(home: Path, request_id: str) -> dict | None:
    return next((r for r in requests(home) if r["id"] == request_id), None)


def add_say(home: Path, text: str, project: str = "", at: str | None = None) -> dict:
    """Append a plain message from the manager to the owner (no call to action, nothing to answer)."""
    text = clip(str(text or "").strip())
    if not text:
        raise InboxError("a message needs text")
    row = {"id": new_id(), "at": at or now_iso(), "kind": "say", "project": str(project or "").strip(),
           "text": text, "from": "manager"}
    append_jsonl(inbox_path(home), row)
    return row


def says(home: Path) -> list[dict]:
    """The manager's plain messages to the owner (``say`` rows), oldest first, the last ``STATE_ROWS``."""
    out = [{"id": str(r["id"]), "at": r.get("at"), "project": str(r.get("project") or ""),
            "text": str(r.get("text") or ""), "from": "manager"}
           for r in read_jsonl(inbox_path(home)) if r.get("kind") == "say" and r.get("id") and r.get("text")]
    return out[-STATE_ROWS:]


def add_answer(home: Path, request_id: str, text: str, by: str = "manager", auto: bool = False,
               at: str | None = None, lane: str | None = None, actions: list[dict] | None = None,
               attachments: list[dict] | None = None) -> dict:
    if by not in ANSWERERS:
        raise InboxError(f"--by must be one of {', '.join(ANSWERERS)}")
    by = "manager" if by == "steward" else by
    if find_request(home, request_id) is None:
        raise InboxError(f"no request {request_id}")
    text = clip(text.strip())
    attachments = clean_attachments(attachments)
    if not text:
        raise InboxError("an answer needs text")
    row = {"id": request_id, "at": at or now_iso(), "kind": "answer", "by": by, "text": text, "auto": bool(auto)}
    if lane:
        row["lane"] = str(lane)  # the agent (lane id) that took the request
    if attachments:
        row["attachments"] = attachments
    if actions:
        row["actions"] = actions  # commands for the owner's "Run" buttons (tirith.actions)
    append_jsonl(inbox_path(home), row)
    return row


def add_reply(home: Path, wait: str, project: str = "", value: str = "", text: str = "",
              at: str | None = None) -> dict:
    """Append the owner's answer to an ask (``waits.jsonl``): one option ``value`` and/or free ``text``."""
    wait, project, value, text = str(wait or "").strip(), str(project or "").strip(), \
        str(value or "").strip()[:200], clip(str(text or "").strip())
    if not wait:
        raise InboxError("a reply needs the id of the ask (--wait)")
    if not value and not text:
        raise InboxError("a reply needs a value or a text")
    row = {"id": new_id(), "at": at or now_iso(), "kind": "reply", "wait": wait, "project": project,
           "value": value, "text": text, "from": "owner", "status": "open"}
    append_jsonl(inbox_path(home), row)
    return row


def mark_done(home: Path, request_id: str, at: str | None = None) -> dict:
    if find_request(home, request_id) is None and find_reply(home, request_id) is None \
            and not any(r.get("wait") == request_id for r in action_results(home)):
        raise InboxError(f"no request {request_id}")
    row = {"id": request_id, "at": at or now_iso(), "kind": "done"}
    append_jsonl(inbox_path(home), row)
    return row


# -- reading ----------------------------------------------------------------------------------


def requests(home: Path) -> list[dict]:
    """Every request in file order with ``status`` (open|answered|done) and its ``answers``.

    Answer and done rows for an unknown id are ignored; malformed rows are skipped."""
    out: dict[str, dict] = {}
    action_owner: dict[str, str] = {}  # action id -> request id, from the answers that declared it
    for row in read_jsonl(inbox_path(home)):
        rid, kind = str(row.get("id") or ""), row.get("kind")
        if kind == "action-run" and row.get("status") in ("passed", "failed"):
            owner = action_owner.get(str(row.get("action_id") or ""))
            if owner in out:  # the owner ran one of the manager's actions: the manager owes an answer
                out[owner]["action_runs"].append({"action_id": row.get("action_id"), "status": row["status"],
                                                  "exit_code": row.get("exit_code"), "at": row.get("at")})
            continue
        if not rid:
            continue
        if kind in KINDS:
            if rid in out:
                continue
            out[rid] = {"id": rid, "at": row.get("at"), "kind": kind, "project": str(row.get("project") or ""),
                        "step": str(row.get("step") if row.get("step") is not None else ""),
                        "text": str(row.get("text") or ""), "from": row.get("from") or "owner",
                        "status": "open", "answers": [], "manager_pending": True, "action_runs": []}
            if clean_cid(row.get("cid")):
                out[rid]["cid"] = clean_cid(row["cid"])
            if row.get("via"):
                out[rid]["via"] = str(row["via"])[:200]
            if clean_attachments(row.get("attachments")):
                out[rid]["attachments"] = clean_attachments(row["attachments"])
            if kind == "setup":
                try:
                    out[rid]["setup"] = parse_setup(out[rid]["text"])
                except InboxError:
                    del out[rid]
            elif kind == "plan-new":
                try:
                    out[rid]["plan"] = parse_plan_new(out[rid]["text"])
                except InboxError:
                    del out[rid]
            elif kind == "plan-edit":
                try:
                    out[rid]["edit"] = parse_plan_edit(out[rid]["text"])
                except InboxError:
                    del out[rid]
        elif rid in out and kind == "answer":
            answer = {"at": row.get("at"), "by": "manager" if row.get("by") in (None, "", "steward") else row["by"],
                      "text": str(row.get("text") or ""), "auto": bool(row.get("auto")),
                      "lane": str(row["lane"]) if row.get("lane") else None}
            if clean_attachments(row.get("attachments")):
                answer["attachments"] = clean_attachments(row["attachments"])
            if row.get("actions"):
                answer["actions"] = [dict(a) for a in row["actions"] if isinstance(a, dict)]
                for act in answer["actions"]:
                    if act.get("id"):
                        action_owner[str(act["id"])] = rid
            out[rid]["answers"].append(answer)
            if out[rid]["status"] == "open":
                out[rid]["status"] = "answered"
        elif rid in out and kind == "done":
            out[rid]["status"] = "done"
    for req in out.values():
        req["manager_pending"] = manager_pending(req)
        req["steward_pending"] = req["manager_pending"]  # old key, same value
    return list(out.values())


def action_results(home: Path) -> list[dict]:
    """Every finished run (``passed`` or ``failed``) of an action on a wait or a request answer, oldest first:
    ``{kind: "action-result", wait|request, project, action_id, label, status, exit_code, at}``."""
    owners = {}
    for action in actions_mod.list_actions(home):
        kind, _, owner = str(action.get("source") or "").partition(":")
        if kind in ("wait", "inbox") and owner:
            owners[action["id"]] = ("wait" if kind == "wait" else "request", owner, action)
    out = []
    for row in actions_mod.run_rows(home):
        found = owners.get(str(row.get("action_id") or ""))
        if found is None or row.get("status") not in ("passed", "failed"):
            continue
        field, owner, action = found
        out.append({"kind": "action-result", field: owner, "project": action.get("project") or "",
                    "action_id": action["id"], "label": action.get("label") or "", "status": row["status"],
                    "exit_code": row.get("exit_code"), "at": row.get("at")})
    return out


def pending_action_results(home: Path) -> list[dict]:
    """Failed runs on waits the manager still owes a look: the action's latest run failed, the wait is
    still open, and no ``inbox done <wait>`` came after it. Passed runs are informational, never pending."""
    closed: dict[str, str] = {}  # wait id -> when the manager closed its results
    for row in read_jsonl(inbox_path(home)):
        if row.get("kind") == "done" and row.get("id"):
            closed[str(row["id"])] = str(row.get("at") or "")
    waits: dict[str, dict] = {}
    for row in read_jsonl(Path(home) / "waits.jsonl"):
        if row.get("id"):
            waits[str(row["id"])] = row
    latest: dict[str, dict] = {}
    for res in action_results(home):
        latest[res["action_id"]] = res
    return [r for r in latest.values()
            if r["status"] == "failed" and r.get("wait") in waits and not waits[r["wait"]].get("done_at")
            and closed.get(r["wait"], "") < str(r["at"] or "")]


def pending_results_text(results: list[dict]) -> str:
    lines = []
    for r in results:
        lines.append("- %s (%s): the owner ran \"%s\" and it failed (exit %s)\n"
                     "    tirith inbox done %s   (or tirith wait done %s) once you have dealt with it"
                     % (r["wait"], r["project"], r["label"], r["exit_code"], r["wait"], r["wait"]))
    return "\n".join(lines)


def manager_pending(req: dict) -> bool:
    """True while the manager owes this request something.

    A ``finish`` or a plan request is Tirith's own job (the dispatcher answers it), so any answer
    settles it; every other kind stays pending until the manager itself answers, whatever the brain said."""
    if req["status"] == "done" or req["kind"] == "autopilot":  # the dispatcher's own switch
        return False
    runs = req.get("action_runs") or []
    if runs:
        last_manager = max((str(a["at"] or "") for a in req["answers"] if a["by"] == "manager"), default="")
        if max(str(r["at"] or "") for r in runs) > last_manager:
            return True  # the owner ran an action the manager gave: the manager must answer the result
    if req["kind"] in ("finish", "plan-edit") or req["kind"] in PLAN_KINDS:
        return not req["answers"]
    return not any(a["by"] == "manager" for a in req["answers"])


def state_rows(home: Path) -> list[dict]:
    return requests(home)[-STATE_ROWS:]


def link_lanes(rows: list[dict], agents: list[dict]) -> None:
    """Set ``lanes`` on each request: the lane ids named by its answers, then the agents whose
    ``request_id`` is the request."""
    for req in rows:
        ids = [a["lane"] for a in req["answers"] if a.get("lane")]
        ids += [a["id"] for a in agents if a.get("request_id") == req["id"]]
        req["lanes"] = list(dict.fromkeys(ids))


def replies(home: Path) -> list[dict]:
    """The owner's replies to asks in file order; ``status`` is ``done`` once the manager closed one."""
    out: dict[str, dict] = {}
    for row in read_jsonl(inbox_path(home)):
        rid, kind = str(row.get("id") or ""), row.get("kind")
        if not rid:
            continue
        if kind == "reply" and row.get("wait") and rid not in out:
            out[rid] = {"id": rid, "at": row.get("at"), "kind": "reply", "wait": str(row["wait"]),
                        "project": str(row.get("project") or ""), "value": str(row.get("value") or ""),
                        "text": str(row.get("text") or ""), "from": row.get("from") or "owner", "status": "open"}
        elif kind == "done" and rid in out:
            out[rid]["status"] = "done"
    return list(out.values())


def find_reply(home: Path, reply_id: str) -> dict | None:
    return next((r for r in replies(home) if r["id"] == reply_id), None)


# -- the watcher ------------------------------------------------------------------------------


def watch(home: Path, emit: Callable[[dict], None], *, poll_s: float = 1.0, include_existing: bool = True,
          should_stop: Callable[[], bool] = lambda: False, sleep: Callable[[float], None] = time.sleep) -> None:
    """Call ``emit`` once for each open request, then for each new one; blocks until ``should_stop``.

    A request the manager owes nothing on (done, or answered by the manager) when first seen is not
    emitted. The owner's replies to asks (``kind: "reply"``) are emitted the same way until the manager
    closes them with ``inbox done``."""
    seen: set[str] = set()
    first = True
    while True:
        for rep in replies(home):
            if rep["id"] in seen:
                continue
            seen.add(rep["id"])
            if not (first and not include_existing) and rep["status"] == "open":
                emit(rep)
        for req in requests(home):
            runs = req.get("action_runs") or []
            key = f'{req["id"]}|{runs[-1]["at"] if runs else ""}'  # a new action result makes it new again
            if key in seen:
                continue
            seen.add(key)
            if (first and not include_existing) or not req["manager_pending"]:
                continue
            emit(req)
        owed = pending_action_results(home)
        for res in action_results(home):
            key = "result|%s|%s" % (res["action_id"], res["at"])
            if key in seen:
                continue
            seen.add(key)
            if first and (not include_existing or res not in owed):
                continue  # history: on start only a failure the manager still owes comes up
            emit(res)
        first = False
        if should_stop():
            return
        sleep(poll_s)


def watch_line(req: dict) -> str:
    if req["kind"] == "action-result":
        return json.dumps({k: v for k, v in req.items() if v is not None or k == "exit_code"})
    keys = ("id", "at", "kind", "project", "wait", "value", "text", "from", "status") if req["kind"] == "reply" \
        else ("id", "at", "kind", "project", "step", "text", "from", "status")
    row = {k: req[k] for k in keys}
    if req.get("attachments"):
        row["attachments"] = req["attachments"]  # absolute paths of the images the owner attached
    if req.get("action_runs"):
        row["action_run"] = req["action_runs"][-1]  # what the owner just ran, and how it went
    return json.dumps(row)


# -- tirith://inbox links ---------------------------------------------------------------------


def parse_url(url: str) -> dict:
    """``tirith://inbox?kind=ask&project=p&step=2&text=..&token=t`` -> its fields (empty strings if absent)."""
    parts = urlsplit(str(url).strip())
    if parts.scheme.lower() != SCHEME or (parts.netloc or parts.path.strip("/")).lower() != "inbox":
        raise InboxError(f"not a {SCHEME}://inbox link: {url!r}")
    query = {k: v[-1] for k, v in parse_qs(parts.query).items()}
    return {key: query.get(key, "") for key in ("kind", "project", "step", "text", "token", "wait", "value", "cid")}


def build_url(kind: str, project: str, step: str, text: str, token: str, cid: str = "") -> str:
    from urllib.parse import urlencode
    query = {"kind": kind, "project": project, "step": step, "text": text, "token": token}
    if cid:
        query["cid"] = cid
    return f"{SCHEME}://inbox?" + urlencode(query)


def build_reply_url(wait: str, project: str, value: str, text: str, token: str) -> str:
    from urllib.parse import urlencode
    return f"{SCHEME}://inbox?" + urlencode({"kind": "reply", "wait": wait, "project": project,
                                              "value": value, "text": text, "token": token})


# -- the brain's automatic answer -------------------------------------------------------------


def answer_prompt(req: dict, project: dict | None, agents: list[dict], flows: list[dict]) -> str:
    """The question plus everything known about the step, as plain text for the brain."""
    step = next((s for s in ((project or {}).get("roadmap") or {}).get("steps") or []
                 if str(s.get("id")) == req["step"]), None)
    lines = [f"Project: {(project or {}).get('name') or req['project']}",
             f"Step {req['step']}: {step['title'] if step else '(not in the plan)'}"]
    if step:
        for item in step.get("items") or []:
            lines.append(f"- [{'x' if item.get('done') else ' '}] {item.get('text')}")
    on_step = [a for a in agents if a.get("project") == req["project"] and str(a.get("step") or "") == req["step"]]
    lines.append("")
    lines.append("Agents on this step:" if on_step else "No agent is on this step.")
    for a in on_step:
        tests = "; ".join(f"{t.get('command')} -> {t.get('result')}" for t in (a.get("tests") or [])[-3:])
        files = ", ".join(f.get("path", "") for f in (a.get("files") or [])[:8])
        lines += [f"* {a.get('name')} ({a.get('status')}): goal: {a.get('goal') or a.get('feature') or '-'}",
                  f"  now: {a.get('summary') or '-'}", f"  tests: {tests or '-'}", f"  files: {files or '-'}"]
    recent = [f for f in flows if f.get("project") == req["project"]][-8:]
    if recent:
        lines += ["", "Recent flows of the project:"]
        lines += [f"- {f.get('from_name')} -> {f.get('to_name')} ({f.get('kind')}): {f.get('text')}" for f in recent]
    lines += ["", "The owner asks:", req["text"] or "What is going on with this step?"]
    return "\n".join(lines)
