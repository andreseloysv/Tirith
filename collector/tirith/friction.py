"""Friction: every time a person was needed or something waited, and the biggest friction on top of the plan.

:func:`record` appends one row to ``<home>/friction.jsonl`` ``{at, project, kind, key, detail, seconds}``:
``kind`` is ``person`` (an ask reached the owner, see ``cli.add_wait``) or ``wait`` (a lane, a stage or the
merge queue waited). Rows with the same ``key`` (the detail without numbers, case and spacing) are the same
friction. :func:`ranked` sorts them by a score (a person needed weighs ``PERSON_SECONDS`` of waiting, plus the
seconds waited); :func:`surface` puts the biggest one as the first open item of the plan's automatic-work step,
once per friction (an item already in the plan, open or done, is not added again).
"""

from __future__ import annotations

import re
from pathlib import Path

from tirith.home import now_iso
from tirith.store import append_jsonl, read_jsonl

FILE = "friction.jsonl"
PERSON, WAIT = "person", "wait"
PERSON_SECONDS = 600  # one time a person was needed counts like ten minutes of waiting
MIN_TIMES = 2         # a friction is surfaced when it happened at least this many times


def _key(detail: str) -> str:
    return re.sub(r"[^a-z]+", " ", str(detail or "").lower()).strip()[:80]


def record(home: Path, project: str, kind: str, detail: str, seconds: float = 0, **extra) -> dict:
    """Log one friction event (a person was needed, or something waited ``seconds``)."""
    kind = kind if kind in (PERSON, WAIT) else WAIT
    detail = " ".join(str(detail or "").split())
    row = {"at": now_iso(), "project": str(project or ""), "kind": kind, "key": _key(detail), "detail": detail,
           "seconds": max(0.0, float(seconds or 0)), **{k: v for k, v in extra.items() if v is not None}}
    append_jsonl(Path(home) / FILE, row)
    return row


def ranked(home: Path, project: str | None = None) -> list[dict]:
    """Frictions of ``project`` (all when ``None``), the biggest first: ``{key, kind, detail, times, seconds, score}``."""
    groups: dict[tuple, dict] = {}
    for row in read_jsonl(Path(home) / FILE):
        if not isinstance(row, dict) or (project and row.get("project") != project):
            continue
        key = (row.get("project"), row.get("kind"), row.get("key"))
        g = groups.setdefault(key, {"project": row.get("project"), "kind": row.get("kind"), "key": row.get("key"),
                                    "detail": row.get("detail"), "times": 0, "seconds": 0.0})
        g["times"] += 1
        g["seconds"] += float(row.get("seconds") or 0)
        g["detail"] = row.get("detail") or g["detail"]
    for g in groups.values():
        g["score"] = g["seconds"] + (g["times"] * PERSON_SECONDS if g["kind"] == PERSON else 0)
    return sorted(groups.values(), key=lambda g: (-g["score"], -g["times"], str(g["key"])))


def item_text(entry: dict) -> str:
    what = "hizo falta una persona" if entry["kind"] == PERSON else "algo esperó"
    return (f"Quitar fricción ({entry['times']} veces, {what}): {entry['detail'][:120]} "
            f"[fricción: {entry['key'][:40]}]")


def surface(home: Path, project: str) -> str | None:
    """Put the biggest friction of ``project`` as the first open item of the automatic-work step.
    Returns the item text, or ``None`` when nothing qualifies or it is already in the plan."""
    from tirith import plangate, roadmap as roadmap_mod
    top = next((g for g in ranked(home, project) if g["times"] >= MIN_TIMES), None)
    repo = plangate.repo_of(home, project)
    if top is None or repo is None:
        return None
    text = item_text(top)
    marker = f"[fricción: {top['key'][:40]}]"

    def mutate(body: str) -> str:
        body = roadmap_mod.add_step(body, plangate.OPS_STEP, plangate.OPS_TITLE)
        if any(marker in i["text"] for s in roadmap_mod.parse(body) for i in s["items"]):
            return body
        return roadmap_mod.edit(body, plangate.OPS_STEP, add=text, top=True)

    changed = plangate.commit_change(home, repo, project, mutate, f"Roadmap: friction on top ({project})")
    return text if changed else None
