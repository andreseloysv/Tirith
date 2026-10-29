"""The plan lanes: "Update the plan" (``plan-refresh``) and "Start a new plan" (``plan-new``).

Both inbox requests are handled by the dispatcher with a generated brief and a lane in its own worktree,
like ``tirith launch`` does for a step. The auditor edits only the project's plan file; the planner writes
``docs/plans/<slug>.md`` and registers it in ``tirith.toml``. The automatic review merges the result and
answers the request with the lane's report. See ``docs/CONTRACT-inbox.md`` and ``docs/CONTRACT-roadmap.md``.
"""

from __future__ import annotations

import re
from pathlib import Path

LANE_STEPS = {"plan-refresh": "plan-refresh", "plan-new": "plan-new"}  # request kind -> the lane's step
HERO_CLASS = "loremaster"
PLANS_DIR = "docs/plans"
REQUESTS_SHOWN = 30
LANES_SHOWN = 40
RULES = (
    "Change only the files named below. The review of this lane runs the tests, commits and merges, "
    "so do not commit yourself.",
    "The plan is the only way to start and to finish: never edit the project's plan file (a gate blocks it; "
    "only Tirith writes it). Missing work goes in with `tirith plan add --project <id> --step <N> --text "
    "\"<item>\"`. Before you finish, report the outcome of your item: `tirith plan report --lane $TIRITH_LANE "
    "--status done|partial|blocked [--add \"<new item>\"]... [--reason \"<why>\"]`.",
    "Name agents only by their alias (see `docs/CONTRACT-hooks.md` \"Agent aliases\"), never by product or "
    "model name.",
    "No servers and no open ports.",
    "Do not touch git hooks, `.git/`, or anything outside this worktree; everything outside it is read only.",
    "Do not start sub-agents.",
)


OWNER_GATE_RULES = (
    "- **Needs the owner** lists ONLY what no agent can do: " + ", ".join(
        f"`{key}`" for key in ("credentials", "payment", "personal-account", "physical-device", "owner-decision"))
    + ". Start every bullet with its reason in brackets, e.g. `- [personal-account: Apple ID] Sign in to ...`. "
    "A bullet without a valid reason is not shown to the owner: code moves it to the manager queue as "
    "\"agent can do\" and a verification lane does it.",
    "- A check an agent can do (open a `tirith://` link with `open`, take a screenshot with playwright, read "
    "state or a file) is NEVER an owner check: put it under **Still missing** instead.",
    "- Never list anything about an item marked `(fuera de este plan ...)`; it is dropped and logged.",
)


def slug_for(title: str, taken: set[str]) -> str:
    """A plan file name (without ``.md``) for ``title`` that is not in ``taken``."""
    base = re.sub(r"[^a-z0-9]+", "-", title.lower()).strip("-")[:40].strip("-") or "plan"
    candidate, n = base, 2
    while candidate in taken:
        candidate, n = f"{base}-{n}", n + 1
    return candidate


def taken_slugs(repo: Path) -> set[str]:
    folder = repo / PLANS_DIR
    return {p.stem for p in folder.glob("*.md")} if folder.is_dir() else set()


def lane_title(kind: str, plan: dict | None) -> str:
    return "Update the plan" if kind == "plan-refresh" else f"New plan {(plan or {}).get('title') or ''}".strip()


def _short(text: object, limit: int) -> str:
    return " ".join(str(text or "").split())[:limit]


def refresh_brief(project: str, plan_file: str, plan_text: str, merged: list[dict],
                  requests: list[dict], home: Path, step: str | None = None) -> str:
    """The auditor's brief: the plan, the merged lanes and the inbox, ready to compare with the code.
    With ``step`` it audits only that step's items."""
    out = [f"# Lane: Update the plan of {project}", "", "## Why",
           f"The owner pressed \"Update the plan\" on {project}. Review everything that is done and everything "
           f"that is missing, then bring `{plan_file}` up to date with the truth.", "",
           "## What to build",
           f"- Do NOT edit `{plan_file}` (only Tirith writes it). Compare EVERY item of it, open and done, with "
           "the code in this worktree, the merged lanes, the pipeline results and the inbox below.",
           "- Add the missing items, under the step they belong to, for work that was requested in the inbox "
           "or in chat but is not in the plan: `tirith plan add --project " + project + " --step <N> --text "
           "\"<item>\"` (once per item; `--title \"<title>\"` creates a step that is not there).",
           "- Items that are really done and working are ticked by Tirith when their review passes; list the "
           "ones that are wrong (ticked but not done, or open but already done) in your report.",
           "- Finish with a short summary as your final report, in three parts: **Done since last update**, "
           "**Still missing**, **Needs the owner**. Short bullets, no repetition. Then run `tirith plan report "
           "--lane $TIRITH_LANE --status done`.",
           *OWNER_GATE_RULES, *(
               [f"- Limited to step {step}: compare only the items of step {step}; add items only to "
                "that step."] if step else []), "",
           "## Hard rules", *(f"- {rule}" for rule in RULES), "",
           "## Where to read",
           "- `git log --oneline -60` and the code itself: an item is done only if you can see it in the code.",
           f"- Pipeline results: `{home}/events.jsonl` (read only).",
           f"- The inbox: `{home}/inbox.jsonl` (read only); the recent requests are listed below.", "",
           f"## The plan now (`{plan_file}`)", "", "```markdown", plan_text.rstrip(), "```", "",
           "## Merged lanes of this project (newest last)", ""]
    out += [f"- `{lane['id']}` (step {lane.get('step') or '-'}): {_short(lane.get('feature'), 120)}"
            + (f": {_short(lane.get('report'), 200)}" if lane.get("report") else "")
            for lane in merged[-LANES_SHOWN:]] or ["- none yet"]
    out += ["", "## Recent requests in the inbox (newest last)", ""]
    out += [f"- {req.get('at')} {req['kind']} [{req['status']}]"
            + (f" step {req['step']}" if req["kind"] in ("finish", "ask") else "")
            + f": {_short(req.get('text'), 240) or '(no text)'}"
            for req in requests[-REQUESTS_SHOWN:]] or ["- none"]
    return "\n".join(out) + "\n"


def new_plan_brief(project: str, plan: dict, slug: str, existing: list[str]) -> str:
    """The planner's brief for "Start a new plan"."""
    rel = f"{PLANS_DIR}/{slug}.md"
    out = [f"# Lane: New plan {plan['title']}", "", "## Why",
           f"The owner started a new plan for {project}: **{plan['title']}**.", ""]
    if plan.get("goal"):
        out += ["Goal:", "", plan["goal"], ""]
    if plan.get("deadline"):
        out += [f"Deadline: {plan['deadline']}", ""]
    deadline_line = f", a line `Deadline: {plan['deadline']}`" if plan.get("deadline") else ""
    out += ["## What to build",
            f"- Write `{rel}` in the same format as `docs/TODO.md`: a `# <title>` heading, one intro paragraph "
            f"(the goal){deadline_line}, then numbered steps `## <n>. <title>`, each with `- [ ]` items small "
            "enough for one agent.",
            "- Base the steps on the goal and on the code in this worktree; no filler steps."]
    if plan.get("steps"):
        out += ["- The owner's first steps, in their words. Turn them into steps and items, and add what is "
                "obviously missing around them:", "", "```text", plan["steps"], "```"]
    register = (f"- Register it in `tirith.toml` at the top level: `plans = [...]` must list `{rel}` "
                f"(keep what is already listed: {', '.join(existing) or 'nothing yet'}), and "
                f"`roadmap = \"{rel}\"` makes it the current plan. If the old current plan (`docs/TODO.md` "
                "or the old `roadmap`) is not in `plans`, add it so it stays in the plan switcher.")
    if plan.get("deadline"):
        register += " Set `deadline` to the deadline above."
    out += [register,
            "- Finish with a short report: the steps you wrote and anything the owner must decide.", "",
            "## Hard rules", *(f"- {rule}" for rule in RULES),
            "- Touch only the new plan file and `tirith.toml`; write the plan file completely before you "
            "register it (once registered, the plan file is Tirith's).", "",
            "## Where to read",
            "- `docs/TODO.md` (read only) as the model of the format; `docs/CONTRACT-roadmap.md`.", ""]
    return "\n".join(out)
