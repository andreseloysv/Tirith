"""The lane scoreboard: which agents, models and projects deliver work that merges, and what it costs.

``tirith lane stats`` reads the ended lanes of the last ``--days`` (lane files plus each lane's log for its
cost) and groups them ``--by`` agent (``alias/model``, the default), model, project or class. Per group:
lanes, done on the first try (plan report ``done``, or ended done without a plan item, and never sent back
by keepgoing), average keepgoing
rounds, stalled lanes, failed lanes, merged lanes, merge trouble (conflict, red suite, sent back), lanes the
repeat rule blocked, total cost and cost per merged lane. ``gaps`` counts the kinds of gaps keepgoing sent
lanes back with, most common first: what briefs should ask for up front. Only reads.
"""

from __future__ import annotations

import re
from collections import Counter
from datetime import datetime, timedelta, timezone
from pathlib import Path

from tirith import agentrun as agentrun_mod
from tirith import plangate as plangate_mod
from tirith.home import parse_iso
from tirith.store import read_lanes

BY = ("agent", "model", "project", "class")
MERGE_TROUBLE = ("conflict", "red-suite", "sent-back")


def gap_kind(gap: str) -> str:
    """``"claim check: said 12 passed…"`` -> ``"claim check"``; branch names are dropped."""
    kind = str(gap).split(":", 1)[0].strip()
    return re.sub(r"^branch \S+ does not merge cleanly with \S+", "branch does not merge cleanly", kind)


def _group(lane: dict, by: str) -> str:
    ran = lane.get("ran") if isinstance(lane.get("ran"), dict) else {}
    model = str(ran.get("model") or "default")
    if by == "model":
        return model
    if by == "project":
        return str(lane.get("project") or "?")
    if by == "class":
        return str(lane.get("class") or "?")
    return f"{lane.get('agent') or 'unnamed'}/{model}"


def ended_lanes(home: Path, days: float, now: datetime | None = None) -> list[dict]:
    """Agent lanes ended within ``days``; the manager and the owner's own sessions are not scored."""
    since = (now or datetime.now(timezone.utc)) - timedelta(days=days)
    out = []
    for lane in read_lanes(home):
        if not isinstance(lane, dict) or not lane.get("ended_at") or lane.get("interactive"):
            continue
        if str(lane.get("id", "")).startswith("manager"):
            continue
        ended = parse_iso(lane.get("ended_at"))
        if ended is not None and ended >= since:
            out.append(lane)
    return out


def score(lanes: list[dict], by: str = "agent", cost=None) -> dict:
    """``{rows: [...], gaps: [[kind, lanes], ...], lanes: n}``; ``cost(lane)`` gives USD or None (the log)."""
    cost = cost or (lambda lane: (agentrun_mod.parse_usage(lane.get("log")) or {}).get("cost_usd"))
    groups: dict[str, dict] = {}
    gaps: Counter = Counter()
    for lane in lanes:
        g = groups.setdefault(_group(lane, by), {"lanes": 0, "first_try": 0, "rounds": 0, "stalled": 0,
                                                  "failed": 0, "merged": 0, "merge_trouble": 0,
                                                  "repeat_blocked": 0, "cost_usd": 0.0, "costed": 0})
        kg = lane.get("keep_going") if isinstance(lane.get("keep_going"), dict) else {}
        rounds = int(kg.get("round") or 0)
        outcome = (lane.get("outcome") or {}).get("status") if isinstance(lane.get("outcome"), dict) else None
        review = (lane.get("review") or {}).get("status") if isinstance(lane.get("review"), dict) else None
        g["lanes"] += 1
        g["rounds"] += rounds
        reported = outcome == "done" or plangate_mod.lane_item(lane) is None  # a lane without a plan item: no report
        g["first_try"] += int(lane.get("result") == "done" and reported and rounds == 0)
        g["stalled"] += int(bool(kg.get("stalled")))
        g["failed"] += int(lane.get("result") != "done")
        g["merged"] += int(review == "merged")
        g["merge_trouble"] += int(review in MERGE_TROUBLE)
        g["repeat_blocked"] += int(bool((lane.get("repeats") or {}).get("blocked")))
        usd = cost(lane)
        if isinstance(usd, (int, float)):
            g["cost_usd"] += float(usd)
            g["costed"] += 1
        gaps.update({gap_kind(m) for m in kg.get("missing") or []})
    rows = []
    for name, g in sorted(groups.items(), key=lambda kv: -kv[1]["lanes"]):
        n = g["lanes"]
        rows.append({"group": name, **{k: v for k, v in g.items() if k != "costed"},
                     "first_try_pct": round(100 * g["first_try"] / n),
                     "avg_rounds": round(g["rounds"] / n, 2),
                     "cost_usd": round(g["cost_usd"], 2) if g["costed"] else None,
                     "usd_per_merged": round(g["cost_usd"] / g["merged"], 2) if g["costed"] and g["merged"] else None})
    return {"lanes": len(lanes), "rows": rows, "gaps": gaps.most_common()}


def render(board: dict, by: str, days: float) -> str:
    if not board["rows"]:
        return f"No ended agent lanes in the last {days:g} days."
    head = (by, "lanes", "1st try", "rounds", "stalled", "failed", "merged", "merge trbl", "loop blk", "cost $",
            "$/merged")
    table = [head] + [(r["group"], r["lanes"], f"{r['first_try_pct']}%", f"{r['avg_rounds']:.2f}", r["stalled"],
                       r["failed"], r["merged"], r["merge_trouble"], r["repeat_blocked"],
                       "-" if r["cost_usd"] is None else f"{r['cost_usd']:.2f}",
                       "-" if r["usd_per_merged"] is None else f"{r['usd_per_merged']:.2f}")
                      for r in board["rows"]]
    widths = [max(len(str(row[i])) for row in table) for i in range(len(head))]
    lines = [f"{board['lanes']} ended agent lanes, last {days:g} days, by {by}:", ""]
    for n, row in enumerate(table):
        lines.append("  ".join(str(c).ljust(w) if i == 0 else str(c).rjust(w) for i, (c, w) in enumerate(zip(row, widths))))
        if n == 0:
            lines.append("  ".join("-" * w for w in widths))
    if board["gaps"]:
        lines += ["", "Gaps lanes were sent back with (lanes):"]
        lines += [f"  {count:>4}  {kind}" for kind, count in board["gaps"]]
    return "\n".join(lines)
