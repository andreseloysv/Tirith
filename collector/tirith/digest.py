"""Digest: a visual summary of what reached main, with an opinion button that becomes plan items.

The rows come from the merge queue's history (``mergeq.view``): one card per merged lane. ``render``
writes a static page (no server, no port) to ``<home>/digest/<project>.html``; its opinion button is the
``tirith digest feedback`` line to run (copied from the page, or the ``tirith://`` link the app handles).
``feedback`` turns each non-empty line of the owner's opinion into an open item of the plan through
``plangate.add_item``. See ``docs/CONTRACT-inbox.md``.
"""

from __future__ import annotations

import html
import re
import shlex
from pathlib import Path
from urllib.parse import quote

from tirith import mergeq
from tirith import plangate
from tirith import roadmap as roadmap_mod
from tirith.home import now_iso

MAX_CARDS = 20
GOOD = ("merged",)
ITEM_MAX = 200


def build(home: Path, project: str | None = None) -> dict:
    """``{"project", "at", "cards": [{lane, at, summary, commit, files}], "count"}``, newest first."""
    rows = mergeq.load(Path(home))["history"]
    cards = []
    for row in reversed(rows):
        if row.get("result") not in GOOD or (project and row.get("project") != project):
            continue
        message = " ".join(str(row.get("message") or "").split())
        cards.append({"lane": row.get("lane"), "project": row.get("project"), "at": row.get("at"),
                      "summary": message, "commit": str(row.get("commit") or "")[:10],
                      "train": bool(row.get("train")), "skipped": row.get("skipped") or [],
                      "flaky": row.get("flaky") or []})
        if len(cards) >= MAX_CARDS:
            break
    return {"project": project, "at": now_iso(), "count": len(cards), "cards": cards}


NEXT_MAX = 5


def while_away(home: Path, project: str, since: str, repo: Path | None = None) -> dict:
    """What happened since ``since`` (ISO time): ``{"entered", "failed", "next"}``.

    ``entered`` are the merged lanes, ``failed`` the other merge-queue results, ``next`` the open plan
    items of the first step that still has any (at most ``NEXT_MAX``).
    """
    entered, failed = [], []
    for row in mergeq.load(Path(home))["history"]:
        if row.get("project") != project or str(row.get("at") or "") <= since:
            continue
        line = {"lane": row.get("lane"), "at": row.get("at"), "result": row.get("result"),
                "summary": " ".join(str(row.get("message") or "").split())}
        (entered if row.get("result") in GOOD else failed).append(line)
    upcoming: list[str] = []
    if repo is not None:
        try:
            _, steps = plangate._read(repo)
        except plangate.PlanGateError:
            steps = []
        for step in steps:
            if str(step["id"]) == roadmap_mod.OPS_STEP:
                continue
            upcoming = [i["text"] for i in step["items"] if not i["done"]][:NEXT_MAX]
            if upcoming:
                break
    return {"project": project, "since": since, "entered": entered, "failed": failed, "next": upcoming}


def render_away(data: dict) -> str:
    """The summary as plain text: what came in, what failed, what is next."""
    def block(title: str, rows: list[str]) -> list[str]:
        return [f"{title} ({len(rows)}):"] + ([f"  - {r}" for r in rows] or ["  - nothing"])
    entered = [f"{r['lane']}: {r['summary'] or 'merged'}" for r in data["entered"]]
    failed = [f"{r['lane']}: {r['result']} {r['summary']}".strip() for r in data["failed"]]
    return "\n".join([f"While you were away since {data['since']}:"] + block("Came in", entered)
                      + block("Failed", failed) + block("Next", list(data["next"])))


def feedback_command(project: str) -> str:
    return f"tirith digest feedback --project {shlex.quote(project)} --text \"<your opinion>\""


def render(data: dict) -> str:
    """One self-contained HTML page: a card per merged lane and the opinion button."""
    project = str(data.get("project") or "")
    cards = "".join(
        f"<article class=card><h3>{html.escape(str(c['lane']))}</h3>"
        f"<p>{html.escape(c['summary'] or 'merged')}</p>"
        f"<small>{html.escape(str(c['at'] or ''))} · {html.escape(c['commit'])}"
        f"{' · train' if c['train'] else ''}</small></article>" for c in data["cards"]
    ) or "<p>Nothing reached main yet.</p>"
    link = "tirith://inbox?kind=digest-feedback&project=" + quote(project) + "&text="
    return (
        "<!doctype html><meta charset=utf-8><title>Digest</title>"
        "<style>body{font:15px system-ui;max-width:720px;margin:2em auto}.card{border:1px solid #ccc;"
        "border-radius:8px;padding:.6em 1em;margin:.6em 0}.btn{display:inline-block;padding:.5em 1em;"
        "background:#2563eb;color:#fff;border-radius:6px;text-decoration:none}</style>"
        f"<h1>What reached main{(' · ' + html.escape(project)) if project else ''}</h1>"
        f"{cards}<p><a class=btn href=\"{html.escape(link)}\">Give your opinion</a></p>"
        f"<p>Or run: <code>{html.escape(feedback_command(project))}</code></p>")


def write(home: Path, project: str | None = None) -> Path:
    out = Path(home) / "digest" / f"{project or 'all'}.html"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(render(build(home, project)), encoding="utf-8")
    return out


def opinion_items(text: str) -> list[str]:
    """One plan item per non-empty line of the opinion (bullets stripped, cut to ``ITEM_MAX``)."""
    out = []
    for line in str(text or "").splitlines():
        line = re.sub(r"^\s*(?:[-*+]|\d+[.)])\s+", "", line).strip()
        if line:
            out.append(line[:ITEM_MAX])
    return out


def default_step(repo: Path) -> str:
    """The step to hold opinions: the last real step of the plan with an open item, else the last step."""
    _, steps = plangate._read(repo)
    real = [s for s in steps if str(s["id"]) != roadmap_mod.OPS_STEP]
    for step in reversed(real):
        if any(not i.get("done") for i in step["items"]):
            return str(step["id"])
    if not real:
        raise plangate.PlanGateError("the plan has no step to hold the opinion")
    return str(real[-1]["id"])


def feedback(home: Path, repo: Path, project: str, text: str, who: str, step: str | None = None) -> list[str]:
    """The opinion as open plan items; returns their clean texts."""
    items = opinion_items(text)
    if not items:
        raise plangate.PlanGateError("the opinion is empty")
    step = step or default_step(repo)
    return [plangate.add_item(home, repo, project, step, item, who) for item in items]
