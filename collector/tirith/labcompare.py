"""``tirith lab compare``: on-demand roles vs fixed roles over the same scenarios.

Input: two folders of ``tirith lab run`` reports (``<scenario>.json``). Only scenarios present in both
are compared. Per side: total cost, total seconds (time to main), first-try success (ok, no duplicate
items, no owner interruptions) and items to main. The recommendation sets promotion thresholds from
the on-demand promotions that paid off.
"""

from __future__ import annotations

import json
from pathlib import Path


def load_reports(folder: Path) -> dict[str, dict]:
    out = {}
    for f in sorted(Path(folder).glob("*.json")):
        try:
            r = json.loads(f.read_text(encoding="utf-8"))
        except ValueError:
            continue
        if isinstance(r, dict) and "scenario" in r:
            out[r["scenario"]] = r
    return out


def first_try(r: dict) -> bool:
    return bool(r.get("ok")) and not r.get("duplicate_items") and not r.get("owner_interruptions")


def _side(reports: list[dict]) -> dict:
    n = len(reports)
    ft = sum(first_try(r) for r in reports)
    return {"scenarios": n,
            "cost_usd": round(sum(r.get("cost_usd", 0) for r in reports), 6),
            "seconds": round(sum(r.get("seconds", 0) for r in reports), 3),
            "items_to_main": sum(r.get("items_to_main", 0) for r in reports),
            "first_try_success": ft,
            "first_try_rate": round(ft / n, 3) if n else 0.0,
            "promotions": sum(len(r.get("promotions") or []) for r in reports)}


def recommend(on: dict[str, dict], fx: dict[str, dict]) -> dict:
    """Thresholds for promoting an on-demand role: only when it pays off against fixed roles."""
    paid = [r for name, r in on.items()
            if r.get("promotions") and first_try(r) >= first_try(fx[name])]
    ratios = [r["cost_usd"] / fx[r["scenario"]]["cost_usd"] for r in paid if fx[r["scenario"]].get("cost_usd")]
    return {
        "promote_min_items": min((r["items_to_main"] for r in paid), default=None),
        "max_cost_ratio": round(max(ratios), 3) if ratios else None,
        "promotions_that_paid_off": len(paid),
        "promotions_total": sum(1 for r in on.values() if r.get("promotions")),
    }


def compare(on_demand: dict[str, dict], fixed: dict[str, dict]) -> dict:
    names = sorted(set(on_demand) & set(fixed))
    on = {n: on_demand[n] for n in names}
    fx = {n: fixed[n] for n in names}
    a, b = _side(list(on.values())), _side(list(fx.values()))
    per = [{"scenario": n,
            "on_demand": {"cost_usd": on[n].get("cost_usd", 0), "seconds": on[n].get("seconds", 0),
                          "first_try": first_try(on[n])},
            "fixed": {"cost_usd": fx[n].get("cost_usd", 0), "seconds": fx[n].get("seconds", 0),
                      "first_try": first_try(fx[n])}} for n in names]
    return {"scenarios": names, "on_demand": a, "fixed": b, "per_scenario": per,
            "delta": {"cost_usd": round(a["cost_usd"] - b["cost_usd"], 6),
                      "seconds": round(a["seconds"] - b["seconds"], 3),
                      "first_try_rate": round(a["first_try_rate"] - b["first_try_rate"], 3)},
            "skipped": sorted(set(on_demand) ^ set(fixed)),
            "recommendation": recommend(on, fx)}


def markdown(c: dict) -> str:
    a, b, d, rec = c["on_demand"], c["fixed"], c["delta"], c["recommendation"]
    rows = [("cost (USD)", f"{a['cost_usd']:.4f}", f"{b['cost_usd']:.4f}", f"{d['cost_usd']:+.4f}"),
            ("time to main (s)", f"{a['seconds']:.0f}", f"{b['seconds']:.0f}", f"{d['seconds']:+.0f}"),
            ("first-try success", f"{a['first_try_success']}/{a['scenarios']}",
             f"{b['first_try_success']}/{b['scenarios']}", f"{d['first_try_rate']:+.0%}"),
            ("items to main", str(a["items_to_main"]), str(b["items_to_main"]), "")]
    t = "| metric | on-demand | fixed | delta |\n|---|---|---|---|\n"
    t += "\n".join("| " + " | ".join(r) + " |" for r in rows)
    items = rec["promote_min_items"]
    ratio = rec["max_cost_ratio"]
    return (f"# Lab compare: on-demand vs fixed roles ({len(c['scenarios'])} scenarios)\n\n{t}\n\n"
            f"## Promotion thresholds\n"
            f"- promote when a request has at least: {items if items is not None else 'n/a (no promotion paid off)'} items\n"
            f"- keep cost within: {f'{ratio}x' if ratio is not None else 'n/a'} of the fixed-role cost\n"
            f"- promotions that paid off: {rec['promotions_that_paid_off']}/{rec['promotions_total']}\n")


def write(c: dict, out: Path) -> tuple[Path, Path]:
    out.mkdir(parents=True, exist_ok=True)
    j, m = out / "compare.json", out / "compare.md"
    j.write_text(json.dumps(c, indent=2) + "\n", encoding="utf-8")
    m.write_text(markdown(c), encoding="utf-8")
    return j, m
