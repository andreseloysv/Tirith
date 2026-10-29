"""Verification lanes: what an agent can do, done by an agent instead of asked of the owner.

The owner gate (``tirith.ownergate``) queues a check without a valid ``why_owner`` as a manager queue item
``agent-can-do``. The dispatcher turns each into one lane with the brief below. It reports what it saw; it
never asks the owner for anything. See ``docs/CONTRACT-inbox.md`` ("The owner gate").
"""

from __future__ import annotations

from pathlib import Path

from tirith import asks as asks_mod
from tirith import planlane as planlane_mod
from tirith.home import Config
from tirith.store import read_json, write_json_atomic

LANE_STEP = "verify"
HERO_CLASS = "loremaster"
STATE_FILE = "verify-state.json"
PREFIX = "agent can do: "
MAX_PER_CYCLE = 1


def lane_title(text: str) -> str:
    return "Verify " + " ".join(text.removeprefix(PREFIX).split())[:48]


def brief(project: str, text: str) -> str:
    """The verification lane's brief for one check."""
    check = text.removeprefix(PREFIX).strip()
    out = [f"# Lane: {lane_title(text)}", "", "## Why",
           f"A check was about to be put on the owner's list for {project}. An agent can do it, so you do it.", "",
           "## What to build",
           f"- Do this check yourself and report what you saw: {check}",
           "- A `tirith://` link: open it with `open \"tirith://...\"` (macOS) and read the result.",
           "- Something to look at on screen: take a screenshot with playwright and describe it (save it "
           "under this worktree's `scratch/`, do not commit it).",
           "- State or a count: read the file or run the command that has it, and quote the numbers.",
           "- Finish with a short report: **Checked** (what you did and saw), **Result** (pass or fail, one "
           "line), **Needs the owner** (almost always `- nothing`; a bullet is allowed only with its reason "
           "in brackets, `[credentials]`, `[payment]`, `[personal-account]`, `[physical-device]` or "
           "`[owner-decision]`, and only for something you really could not do).", "",
           "## Hard rules", *(f"- {rule}" for rule in planlane_mod.RULES),
           "- Do not change any file that is tracked by git; this lane only looks and reports.", ""]
    return "\n".join(out)


def _state(home: Path) -> dict:
    state = read_json(Path(home) / STATE_FILE, None)
    return state if isinstance(state, dict) else {"started": {}}


def pending(home: Path) -> list[dict]:
    """Open ``agent-can-do`` queue items with a project and no verification lane yet."""
    started = _state(home)["started"]
    return [i for i in asks_mod.queue_items(home, only_open=True)
            if i["kind"] == asks_mod.AGENT_CAN_DO_KIND and i["project"] and i["id"] not in started]


def run(home: Path, config: Config, launcher, *, room: bool = True) -> int:
    """Start a verification lane for the oldest pending items (at most ``MAX_PER_CYCLE`` per pass).

    ``launcher(home, project, text) -> lane id`` raises ``launch.LaunchError`` when it must not start; the
    item then stays in the manager queue for the manager. Returns how many lanes started."""
    from tirith import launch as launch_mod
    if not room or not config.dispatch_auto_launch:
        return 0
    state, started = _state(home), 0
    for item in pending(home)[:MAX_PER_CYCLE]:
        try:
            lane_id = launcher(home, item["project"], item["text"])
        except launch_mod.LaunchError as err:
            state["started"][item["id"]] = f"refused: {err}"[:200]
            continue
        except Exception as err:  # never let one check stop the cycle
            state["started"][item["id"]] = f"error: {type(err).__name__}: {err}"[:200]
            continue
        state["started"][item["id"]] = lane_id
        asks_mod.queue_done(home, item["id"], f"started verification lane {lane_id}")
        started += 1
    if state != _state(home):
        write_json_atomic(Path(home) / STATE_FILE, state)
    return started
