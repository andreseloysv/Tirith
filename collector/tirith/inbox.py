"""The owner's inbox: "Finish it" and "What's going on?" requests on plan steps.

The web page cannot run commands, so a request is one line appended to ``<home>/inbox.jsonl``;
the steward watches the file (``tirith inbox watch``) and answers with ``tirith inbox answer``.
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
KINDS = ("finish", "ask")
ANSWERERS = ("steward", "brain")
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
    if not project or not step:
        raise InboxError("a request needs a project and a step")
    if kind == "ask" and not text:
        raise InboxError("a question needs text")
    row = {"id": new_id(), "at": at or now_iso(), "kind": kind, "project": project, "step": step,
           "text": text, "from": "owner", "status": "open"}
    append_jsonl(inbox_path(home), row)
    return row


def find_request(home: Path, request_id: str) -> dict | None:
    return next((r for r in requests(home) if r["id"] == request_id), None)


def add_answer(home: Path, request_id: str, text: str, by: str = "steward", auto: bool = False,
               at: str | None = None, lane: str | None = None, actions: list[dict] | None = None) -> dict:
    if by not in ANSWERERS:
        raise InboxError(f"--by must be one of {', '.join(ANSWERERS)}")
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


def mark_done(home: Path, request_id: str, at: str | None = None) -> dict:
    if find_request(home, request_id) is None:
        raise InboxError(f"no request {request_id}")
    row = {"id": request_id, "at": at or now_iso(), "kind": "done"}
    append_jsonl(inbox_path(home), row)
    return row


# -- reading ----------------------------------------------------------------------------------


def requests(home: Path) -> list[dict]:
    """Every request in file order with ``status`` (open|answered|done) and its ``answers``.

    Answer and done rows for an unknown id are ignored; malformed rows are skipped."""
    out: dict[str, dict] = {}
    for row in read_jsonl(inbox_path(home)):
        rid, kind = str(row.get("id") or ""), row.get("kind")
        if not rid:
            continue
        if kind in KINDS:
            if rid in out:
                continue
            out[rid] = {"id": rid, "at": row.get("at"), "kind": kind, "project": str(row.get("project") or ""),
                        "step": str(row.get("step") if row.get("step") is not None else ""),
                        "text": str(row.get("text") or ""), "from": row.get("from") or "owner",
                        "status": "open", "answers": []}
        elif rid in out and kind == "answer":
            answer = {"at": row.get("at"), "by": row.get("by") or "steward",
                      "text": str(row.get("text") or ""), "auto": bool(row.get("auto")),
                      "lane": str(row["lane"]) if row.get("lane") else None}
            if row.get("actions"):
                answer["actions"] = [dict(a) for a in row["actions"] if isinstance(a, dict)]
            out[rid]["answers"].append(answer)
            if out[rid]["status"] == "open":
                out[rid]["status"] = "answered"
        elif rid in out and kind == "done":
            out[rid]["status"] = "done"
    return list(out.values())


def state_rows(home: Path) -> list[dict]:
    return requests(home)[-STATE_ROWS:]


def link_lanes(rows: list[dict], agents: list[dict]) -> None:
    """Set ``lanes`` on each request: the lane ids named by its answers, then the agents whose
    ``request_id`` is the request."""
    for req in rows:
        ids = [a["lane"] for a in req["answers"] if a.get("lane")]
        ids += [a["id"] for a in agents if a.get("request_id") == req["id"]]
        req["lanes"] = list(dict.fromkeys(ids))


# -- the watcher ------------------------------------------------------------------------------


def watch(home: Path, emit: Callable[[dict], None], *, poll_s: float = 1.0, include_existing: bool = True,
          should_stop: Callable[[], bool] = lambda: False, sleep: Callable[[float], None] = time.sleep) -> None:
    """Call ``emit`` once for each open request, then for each new one; blocks until ``should_stop``.

    A request already answered or done when first seen is not emitted."""
    seen: set[str] = set()
    first = True
    while True:
        for req in requests(home):
            if req["id"] in seen:
                continue
            seen.add(req["id"])
            if (first and not include_existing) or req["status"] != "open":
                continue
            emit(req)
        first = False
        if should_stop():
            return
        sleep(poll_s)


def watch_line(req: dict) -> str:
    return json.dumps({k: req[k] for k in ("id", "at", "kind", "project", "step", "text", "from", "status")})


# -- tirith://inbox links ---------------------------------------------------------------------


def parse_url(url: str) -> dict:
    """``tirith://inbox?kind=ask&project=p&step=2&text=..&token=t`` -> its fields (empty strings if absent)."""
    parts = urlsplit(str(url).strip())
    if parts.scheme.lower() != SCHEME or (parts.netloc or parts.path.strip("/")).lower() != "inbox":
        raise InboxError(f"not a {SCHEME}://inbox link: {url!r}")
    query = {k: v[-1] for k, v in parse_qs(parts.query).items()}
    return {key: query.get(key, "") for key in ("kind", "project", "step", "text", "token")}


def build_url(kind: str, project: str, step: str, text: str, token: str) -> str:
    from urllib.parse import urlencode
    return f"{SCHEME}://inbox?" + urlencode({"kind": kind, "project": project, "step": step,
                                              "text": text, "token": token})


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
