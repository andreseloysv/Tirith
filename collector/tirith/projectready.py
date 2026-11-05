"""``tirith project add`` leaves a project ready in one go: a plan, a minimal ``tirith.toml`` and the guard
waiting for the owner's one click. See ``docs/CONTRACT-roadmap.md`` ("Project ready").

Every part is skipped when it already exists, so adding a project twice changes nothing.
"""

from __future__ import annotations

import hashlib
import shlex
from pathlib import Path

from tirith import actions as actions_mod
from tirith import dispatch as dispatch_mod
from tirith import manifest as manifest_mod
from tirith import plangate as plangate_mod
from tirith import roadmap as roadmap_mod
from tirith.home import now_iso

PLAN_STEP, PLAN_TITLE = "1", "Primeros pasos"
MINIMAL_TOML = ('# Pipelines and rules of this project. Format: docs/CONTRACT-manifest.md\n'
                'deploy = "none"\n'
                'roadmap = "docs/TODO.md"\n'
                '\n'
                '[roles]\n'
                'ceo = "on-demand"\n'
                'manager = "on-demand"\n')


def guard_item_id(project: str) -> str:
    return f"guard-install-{project}"


def _is_git(folder: Path) -> bool:
    return (folder / ".git").exists()


def ensure_plan(home: Path, folder: Path, project: str) -> bool:
    """Create ``docs/TODO.md`` with step 1 when the project has no plan. True when it was created."""
    if plangate_mod.plan_file(folder) is not None:
        return False
    if _is_git(folder):
        try:
            plangate_mod.commit_change(home, folder, project,
                                       lambda text: roadmap_mod.add_step(text, PLAN_STEP, PLAN_TITLE),
                                       f"plan: create the plan of {project}", create=True)
            return True
        except plangate_mod.PlanGateError:
            pass  # the commit was refused: keep the file in the folder, uncommitted
    path = folder / "docs" / "TODO.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(roadmap_mod.add_step("# TODO\n", PLAN_STEP, PLAN_TITLE), encoding="utf-8")
    return True


def ensure_manifest(folder: Path) -> bool:
    """Write a minimal ``tirith.toml`` unless there is one. True when it was written."""
    path = folder / manifest_mod.FILENAME
    if path.exists():
        return False
    path.write_text(MINIMAL_TOML, encoding="utf-8")
    return True


def ask_guard(home: Path, folder: Path, project: str, name: str) -> bool:
    """Put the guard install on the owner's list as one Run click. Nothing when the folder is no git repo."""
    if not _is_git(folder):
        return False
    verdict = dispatch_mod.add_owner_item(
        home, guard_item_id(project), project,
        f"Install the commit guard in {name}.", [],
        now_iso(),
        actions=[actions_mod.register_generated(
            home, project, "ac-" + hashlib.sha1(guard_item_id(project).encode()).hexdigest()[:8],
            "Install the guard", f"tirith guard install {shlex.quote(project)}", str(folder))],
        why_owner="owner-decision", why_detail="only you may install git hooks")
    return verdict == "owner"


def make_ready(home: Path, project: str, folder: str | Path, name: str) -> dict:
    """Do the three things; returns what was done (``plan``, ``manifest``, ``guard``: booleans)."""
    root = Path(folder)
    return {"plan": ensure_plan(home, root, project), "manifest": ensure_manifest(root),
            "guard": ask_guard(home, root, project, name)}
