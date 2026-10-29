"""The owner's inbox: "Finish it" and "What's going on?" requests on plan steps, and replies to asks.

The web page cannot run commands, so a request is one line appended to ``<home>/inbox.jsonl``;
the manager watches the file (``tirith inbox watch``) and answers with ``tirith inbox answer``.
The collector folds the rows into ``state.inbox``. See ``docs/CONTRACT-inbox.md``.
"""

from __future__ import annotations

import json
import secrets
import time
from pathlib import Path
from typing import Callable
from urllib.parse import parse_qs, urlsplit

from tirith.home import now_iso
from tirith.store import append_jsonl, read_jsonl

INBOX_FILE = "inbox.jsonl"
KINDS = ("finish", "ask", "setup")
SETUP_ACTIONS = ("add-stage", "add-rule", "remove-stage", "remove-rule", "run-stage")
SETUP_STEP = "setup"
ANSWERERS = ("manager", "steward", "brain", "tirith")  # "steward" is the old name of "manager"
TEXT_MAX_BYTES = 4096
STATE_ROWS = 100
AUTO_ANSWER_AFTER_S = 120
SCHEME = "tirith"


class InboxError(Exception):
    """A request that cannot be recorded; the message says why."""


def inbox_path(home: Path) -> Path:
    return Path(home) / INBOX_FILE


def clip(text: str, limit: int = TEXT_MAX_BYTES) -> str:
    """``text`` cut to ``limit`` UTF-8 bytes (never in the middle of a character)."""
    return str(text or "").encode("utf-8")[:limit].decode("utf-8", errors="ignore")


def new_id() -> str:
    return "in-" + secrets.token_hex(4)


# -- writing ----------------------------------------------------------------------------------


def add_request(home: Path, kind: str, project: str, step: str, text: str = "",
                at: str | None = None) -> dict:
    """Append an open request and return its row."""
    if kind not in KINDS:
        raise InboxError(f"kind must be one of {', '.join(KINDS)}, not {kind!r}")
    project, step, text = str(project or "").strip(), str(step if step is not None else "").strip(), clip(text.strip())
    if kind == "setup":
        step = step or SETUP_STEP
        text = json.dumps(parse_setup(text), ensure_ascii=False, separators=(",", ":"))
    if not project or not step:
        raise InboxError("a request needs a project and a step")
    if kind == "ask" and not text:
        raise InboxError("a question needs text")
    row = {"id": new_id(), "at": at or now_iso(), "kind": kind, "project": project, "step": step,
           "text": text, "from": "owner", "status": "open"}
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


def setup_summary(setup: dict) -> str:
    """One line for a setup request: ``add-stage ci/lint: make lint``."""
    where = "/".join(str(setup[k]) for k in ("pipeline", "name", "id") if setup.get(k))
    detail = setup.get("run") or setup.get("match") or setup.get("regex") or setup.get("message") or ""
    return " ".join(f"{setup.get('action')} {where}: {detail}".split()).rstrip(": ")


def find_request(home: Path, request_id: str) -> dict | None:
    return next((r for r in requests(home) if r["id"] == request_id), None)


def add_answer(home: Path, request_id: str, text: str, by: str = "manager", auto: bool = False,
               at: str | None = None, lane: str | None = None, actions: list[dict] | None = None) -> dict:
    if by not in ANSWERERS:
        raise InboxError(f"--by must be one of {', '.join(ANSWERERS)}")
    by = "manager" if by == "steward" else by
    if find_request(home, request_id) is None:
        raise InboxError(f"no request {request_id}")
    text = clip(text.strip())
    if not text:
        raise InboxError("an answer needs text")
    row = {"id": request_id, "at": at or now_iso(), "kind": "answer", "by": by, "text": text, "auto": bool(auto)}
    if lane:
        row["lane"] = str(lane)  # the agent (lane id) that took the request
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
    if find_request(home, request_id) is None and find_reply(home, request_id) is None:
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
            if kind == "setup":
                try:
                    out[rid]["setup"] = parse_setup(out[rid]["text"])
                except InboxError:
                    del out[rid]
        elif rid in out and kind == "answer":
            answer = {"at": row.get("at"), "by": "manager" if row.get("by") in (None, "", "steward") else row["by"],
                      "text": str(row.get("text") or ""), "auto": bool(row.get("auto")),
                      "lane": str(row["lane"]) if row.get("lane") else None}
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


def manager_pending(req: dict) -> bool:
    """True while the manager owes this request something.

    A ``finish`` is Tirith's own job (the dispatcher answers it), so any answer settles it; every
    other kind stays pending until the manager itself answers, whatever the brain said."""
    if req["status"] == "done":
        return False
    runs = req.get("action_runs") or []
    if runs:
        last_manager = max((str(a["at"] or "") for a in req["answers"] if a["by"] == "manager"), default="")
        if max(str(r["at"] or "") for r in runs) > last_manager:
            return True  # the owner ran an action the manager gave: the manager must answer the result
    if req["kind"] == "finish":
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
        first = False
        if should_stop():
            return
        sleep(poll_s)


def watch_line(req: dict) -> str:
    keys = ("id", "at", "kind", "project", "wait", "value", "text", "from", "status") if req["kind"] == "reply" \
        else ("id", "at", "kind", "project", "step", "text", "from", "status")
    row = {k: req[k] for k in keys}
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
    return {key: query.get(key, "") for key in ("kind", "project", "step", "text", "token", "wait", "value")}


def build_url(kind: str, project: str, step: str, text: str, token: str) -> str:
    from urllib.parse import urlencode
    return f"{SCHEME}://inbox?" + urlencode({"kind": kind, "project": project, "step": step,
                                              "text": text, "token": token})


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
