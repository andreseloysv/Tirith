"""The plan is the only way to start and to finish an agent (``docs/CONTRACT-roadmap.md``, "The plan gate").

* **Single writer.** Only Tirith writes the project's plan file, on the main checkout, one commit per change
  (``Roadmap: <what> in step N (<who>)``), serialized with a lock. Agents never edit it (the ``roadmap-edit``
  gate in ``tirith.gates``); they ask Tirith: ``tirith plan add`` and ``tirith plan report``.
* **Start = a linked item.** ``lane run`` needs ``--step N --item "<exact open item>"`` and :func:`check_start`
  looks it up in the plan. Tirith creates the items for its own automatic work (:func:`ensure_item`) before it
  starts that work: a review, a plan refresh, the manager, a verification.
* **Finish = an outcome.** ``tirith plan report --lane L --status done|partial|blocked`` records it
  (:func:`report`). A lane that ends without one is ``no-outcome``: not merged, started again once
  (:func:`sync`), then it goes to the manager queue.
* **Done only when verified.** ``done`` marks the item "(en revisión)"; the review ticks it (:func:`review_passed`)
  or reopens it with the reason (:func:`review_failed`).
"""

from __future__ import annotations

import re
import subprocess
import time
from contextlib import contextmanager
from pathlib import Path
from typing import Callable

try:
    import fcntl
except ImportError:  # Windows: the lock is a no-op there (one dispatcher, one writer)
    fcntl = None  # type: ignore[assignment]

from tirith import timeouts as timeouts_mod
from tirith import asks as asks_mod
from tirith import manifest as manifest_mod
from tirith import projects as projects_mod
from tirith import roadmap as roadmap_mod
from tirith.home import Config, load_config, now_iso
from tirith.store import lane_path, read_json, write_json_atomic

OPS_STEP = roadmap_mod.OPS_STEP
OPS_TITLE = "Trabajo automático de Tirith"
MANAGER_ITEM = "Manager de guardia: atender el chat"
CHAT_ITEM = "Chat con el dueño"  # followed by ": <chat id>"; ticked when the chat ends
STANDING_CLASSES = ("steward", "chat")  # lanes that run for as long as the owner wants, on a standing item
STATUSES = ("done", "partial", "blocked")
NO_OUTCOME = "no-outcome"
LOCK_WAIT_S = 30.0
OWNER_REASON_RE = re.compile(r"^(credentials|payment|personal-account|physical-device|owner-decision)\b", re.I)


class PlanGateError(Exception):
    """The plan does not allow it; the message says why and what to do instead."""


def add_hint(project: str, step: str | None) -> str:
    return (f'Add it to the plan first: tirith plan add --project {project} --step {step or "<N>"} '
            '--text "<what you will do>"')


# -- where the plan is -----------------------------------------------------------------------


def repo_of(home: Path, project: str, config: Config | None = None) -> Path | None:
    """The main checkout of a project (registered or automatic)."""
    config = config or load_config(home)
    proj = config.project(project)
    if proj is not None and proj.repo:
        return projects_mod.real(proj.repo)
    auto = next((p for p in projects_mod.load_auto(home)["projects"] if str(p["id"]) == project), None)
    return projects_mod.real(auto["folder"]) if auto else None


def plan_file(repo: Path) -> Path | None:
    return roadmap_mod.find_file(repo, manifest_mod.load(repo).roadmap)


def _read(repo: Path) -> tuple[Path, list[dict]]:
    path = plan_file(repo)
    steps = roadmap_mod.parse(path.read_text(encoding="utf-8")) if path else None
    if path is None or steps is None:
        raise PlanGateError("the project has no plan file (docs/TODO.md). Create one with a `## <n>. <title>` step, "
                            "then add the item to it")
    return path, steps


# -- the single writer -----------------------------------------------------------------------


@contextmanager
def writer_lock(home: Path, project: str):
    """One writer of a project's plan at a time (``<home>/locks/roadmap-<project>.lock``)."""
    folder = Path(home) / "locks"
    folder.mkdir(parents=True, exist_ok=True)
    handle = open(folder / f"roadmap-{re.sub(r'[^A-Za-z0-9_.-]', '_', project)}.lock", "a")
    try:
        if fcntl is not None:
            deadline = time.monotonic() + LOCK_WAIT_S
            while True:
                try:
                    fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    break
                except OSError:
                    if time.monotonic() > deadline:
                        raise PlanGateError("another change to the plan is taking too long; try again")
                    time.sleep(0.05)
        yield
    finally:
        handle.close()  # closing releases the flock


def _git(repo: Path, *args: str) -> subprocess.CompletedProcess:
    return timeouts_mod.run(["git", "-C", str(repo), *args], "git", text=True)


def commit_change(home: Path, repo: Path, project: str, mutate: Callable[[str], str], message: str,
                  path: Path | None = None) -> bool:
    """Apply ``mutate`` to the plan file and commit only that file; False when nothing changed.

    Raises ``PlanGateError`` when the file has uncommitted changes, the edit does not apply or the commit
    is refused (the file is restored then)."""
    path = path or plan_file(repo)
    if path is None:
        raise PlanGateError(f"project '{project}' has no plan file (docs/TODO.md)")
    if not path.is_relative_to(repo):
        raise PlanGateError("the plan file is outside the project folder")
    rel = str(path.relative_to(repo))
    with writer_lock(home, project):
        if _git(repo, "status", "--porcelain", "--", rel).stdout.strip():
            raise PlanGateError(f"{rel} has uncommitted changes, so it was left as it is")
        text = path.read_text(encoding="utf-8")
        try:
            new = mutate(text)
        except roadmap_mod.EditError as err:
            raise PlanGateError(str(err))
        if new == text:
            return False
        path.write_text(new, encoding="utf-8")
        _git(repo, "add", "--", rel)
        made = _git(repo, "commit", "-q", "-m", message, "--", rel)
        if made.returncode != 0 and "nothing to commit" in (made.stdout + made.stderr).lower():
            return False  # the plan already was in the target state: success
        if made.returncode != 0:  # the guard (or git) refused: leave the plan as it was
            _git(repo, "reset", "-q", "--", rel)
            _git(repo, "checkout", "--", rel)
            raise PlanGateError(f"committing the plan failed: {(made.stderr or made.stdout).strip()[:200]}")
    return True


def _message(what: str, step: str, who: str) -> str:
    return f"Roadmap: {what} in step {step} ({who})"


# -- start ------------------------------------------------------------------------------------


def check_start(home: Path, repo: Path, project: str, step: str | None, item: str | None,
                lane_id: str | None = None) -> str:
    """The clean text of the open plan item an agent may start on; raises ``PlanGateError`` otherwise.

    The item is found by its exact text (any status note is ignored) or by its id (``i-xxxxxxxx``) in step
    ``step``. An item another lane is working on, in review or blocked is not startable."""
    step = str(step or "").strip()
    if not step or not str(item or "").strip():
        raise PlanGateError("an agent starts on a plan item: pass --step <N> --item \"<exact text of the open item>\". "
                            + add_hint(project, step))
    path, steps = _read(repo)
    if not any(s["id"] == step for s in steps):
        raise PlanGateError(f"step {step} is not in {path.name}. " + add_hint(project, step))
    found = roadmap_mod.find_item(steps, step, str(item))
    if found is None:
        done = next((i for i in next(s for s in steps if s["id"] == step)["items"] if i["done"]
                     and roadmap_mod.split_status(i["text"])[0] == roadmap_mod.split_status(str(item))[0]), None)
        raise PlanGateError(("that item is already done in the plan. " if done else
                             f"that item is not an open item of step {step}. ") + add_hint(project, step))
    clean, state, detail = roadmap_mod.split_status(found["text"])
    if state == roadmap_mod.STATE_REVIEW:
        raise PlanGateError("that item is in review; it is done when the review passes")
    if state == roadmap_mod.STATE_BLOCKED:
        raise PlanGateError(f"that item is blocked ({detail or 'no reason given'}); unblock it in the plan first")
    if state == roadmap_mod.STATE_PROGRESS and detail and detail != lane_id:
        other = read_json(lane_path(home, detail), None)
        if isinstance(other, dict) and not other.get("ended_at"):
            raise PlanGateError(f"that item is already in progress by lane {detail}")
    return clean


def begin(home: Path, repo: Path, project: str, step: str, item: str, lane_id: str) -> bool:
    """Mark ``item`` "(en curso: <lane>)" in the plan; True when the plan changed."""
    clean = roadmap_mod.split_status(item)[0]
    return commit_change(
        home, repo, project,
        lambda text: roadmap_mod.set_item(text, step, clean, roadmap_mod.with_status(clean, roadmap_mod.STATE_PROGRESS, lane_id)),
        _message("start", step, lane_id))


def release(home: Path, repo: Path, project: str, step: str, item: str, lane_id: str) -> bool:
    """Take the "(en curso)" note off an item again (a standing item, or a lane the owner stopped)."""
    clean = roadmap_mod.split_status(item)[0]
    def mutate(text: str) -> str:
        return roadmap_mod.set_item(text, step, clean, clean)
    return commit_change(home, repo, project, mutate, _message("release", step, lane_id))


def ensure_item(home: Path, repo: Path, project: str, step: str, text: str, lane_id: str | None = None,
                who: str = "tirith", parent: str | None = None, title: str | None = None) -> str:
    """Make sure the open item ``text`` exists in step ``step`` (a sub-item of ``parent`` when given), already
    marked "(en curso: <lane>)" when ``lane_id`` is given so the autopilot never takes it. Returns its clean text.
    A missing step is created with ``title``."""
    clean = roadmap_mod.split_status(text)[0]
    note = roadmap_mod.with_status(clean, roadmap_mod.STATE_PROGRESS, lane_id) if lane_id else clean

    def mutate(body: str) -> str:
        if title:
            body = roadmap_mod.add_step(body, step, title)
        found = roadmap_mod.find_item(roadmap_mod.parse(body), step, clean)
        if found is not None:
            return body
        if parent:
            return roadmap_mod.add_sub(body, step, parent, note)
        return roadmap_mod.edit(body, step, add=note)

    commit_change(home, repo, project, mutate, _message("add", step, who))
    return clean


def add_item(home: Path, repo: Path, project: str, step: str, text: str, who: str, title: str | None = None) -> str:
    """``tirith plan add``: one new open item at the end of step ``step``; returns its clean text."""
    text = roadmap_mod.norm(text)
    if not text:
        raise PlanGateError("the new item is empty")
    return ensure_item(home, repo, project, step, text, None, who, title=title)


def ops_item(home: Path, repo: Path, project: str, text: str, lane_id: str | None, who: str = "tirith",
             parent: tuple[str, str] | None = None) -> tuple[str, str]:
    """An item for Tirith's own automatic work: ``(step, clean text)``. Under ``parent`` ``(step, item)`` it is a
    sub-item of the reviewed item, else it goes in the "Trabajo automático" step."""
    if parent:
        return parent[0], ensure_item(home, repo, project, parent[0], text, lane_id, who, parent=parent[1])
    return OPS_STEP, ensure_item(home, repo, project, OPS_STEP, text, lane_id, who, title=OPS_TITLE)


# -- finish -----------------------------------------------------------------------------------


def lane_item(lane: dict) -> tuple[str, str] | None:
    """``(step, item)`` a lane is linked to, else None."""
    item = str(lane.get("item") or "")
    step = str(lane.get("item_step") or lane.get("step") or "")
    return (step, item) if item and step else None


def apply_outcome(home: Path, repo: Path, project: str, lane: dict) -> None:
    """Write a lane's outcome into the plan: the item's note and the new items (idempotent)."""
    outcome, linked = lane.get("outcome") or {}, lane_item(lane)
    if not outcome or not linked:
        return
    step, item = linked
    clean = roadmap_mod.split_status(item)[0]
    status, reason = outcome["status"], str(outcome.get("reason") or "")
    if status == "blocked":
        if OWNER_REASON_RE.match(reason):  # needs the owner: an owner tag the owner gate understands
            note = f"{clean} (dueño: {re.sub(r'[()]', ' ', reason)})"
        else:
            note = roadmap_mod.with_status(clean, roadmap_mod.STATE_BLOCKED, reason)
    else:
        note = roadmap_mod.with_status(clean, roadmap_mod.STATE_REVIEW)
    adds = [roadmap_mod.norm(a) for a in outcome.get("added") or [] if roadmap_mod.norm(a)]

    def mutate(text: str) -> str:
        text = roadmap_mod.set_item(text, step, clean, note)
        taken = {roadmap_mod.split_status(i["text"])[0] for s in roadmap_mod.parse(text) if s["id"] == step
                 for i in s["items"]}
        for new in adds:
            if roadmap_mod.split_status(new)[0] not in taken:
                text = roadmap_mod.edit(text, step, add=new)
                taken.add(roadmap_mod.split_status(new)[0])
        return text

    commit_change(home, repo, project, mutate, _message(f"report {status}", step, str(lane["id"])))


def report(home: Path, lane_id: str, status: str, adds: list[str] | None = None, reason: str | None = None) -> dict:
    """``tirith plan report``: record the outcome of a lane's item and write it into the plan.

    Returns the lane. Raises ``PlanGateError`` when the report is not valid. The outcome is kept even when
    the plan could not be written now (the dispatcher writes it later, :func:`sync`)."""
    lane = read_json(lane_path(home, lane_id), None)
    if not isinstance(lane, dict):
        raise PlanGateError(f"unknown lane: {lane_id}")
    if status not in STATUSES:
        raise PlanGateError(f"--status must be one of {', '.join(STATUSES)}")
    if lane_item(lane) is None:
        raise PlanGateError(f"lane {lane_id} is not linked to a plan item")
    adds = [roadmap_mod.norm(a) for a in adds or [] if roadmap_mod.norm(a)]
    if status == "blocked" and not roadmap_mod.norm(reason or ""):
        raise PlanGateError('--status blocked needs --reason "<why, and who can unblock it>"')
    if status == "partial" and not adds:
        raise PlanGateError('--status partial needs the rest as new items: --add "<what is left>"')
    lane["outcome"] = {"status": status, "added": adds, "reason": roadmap_mod.norm(reason or "") or None,
                       "at": now_iso(), "applied": False}
    write_json_atomic(lane_path(home, lane_id), lane)
    repo = repo_of(home, str(lane.get("project") or ""))
    if repo is not None:
        try:
            apply_outcome(home, repo, str(lane["project"]), lane)
            lane["outcome"]["applied"] = True
        except PlanGateError as err:
            lane["outcome"]["error"] = str(err)
        write_json_atomic(lane_path(home, lane_id), lane)
    return lane


def standing(lane: dict) -> bool:
    return (str(lane.get("class") or "") in STANDING_CLASSES or bool(lane.get("interactive"))
            or str(lane.get("id") or "").startswith("manager"))


def lane_ended(home: Path, lane: dict) -> str:
    """What an ended lane's result should be: ``no-outcome`` for ``done`` without an outcome on an item.
    Standing lanes (the manager, the owner's chat) give their item back instead."""
    linked = lane_item(lane)
    if linked is None or lane.get("result") not in ("done", "failed"):
        return str(lane.get("result") or "")
    if standing(lane):
        repo = repo_of(home, str(lane.get("project") or ""))
        if repo is not None:
            try:  # the manager's item stays open for the next manager; a chat is over, so its item is done
                if str(lane.get("class")) == "chat" or lane.get("interactive"):
                    clean = roadmap_mod.split_status(linked[1])[0]
                    commit_change(home, repo, str(lane["project"]),
                                  lambda text: roadmap_mod.set_item(text, linked[0], clean, clean, tick=True),
                                  _message("done", linked[0], str(lane["id"])))
                else:
                    release(home, repo, str(lane["project"]), linked[0], linked[1], str(lane["id"]))
            except PlanGateError:
                pass
        return str(lane.get("result"))
    if lane.get("result") == "done" and not lane.get("outcome"):
        return NO_OUTCOME
    return str(lane.get("result"))


# -- the reviews ------------------------------------------------------------------------------


def review_text(clean: str) -> str:
    return f"Revisar: {clean}"


def review_started(home: Path, repo: Path, lane: dict) -> None:
    linked = lane_item(lane)
    if linked is None:
        return
    step, item = linked
    ensure_item(home, repo, str(lane["project"]), step, review_text(roadmap_mod.split_status(item)[0]),
                f"revisión de {lane['id']}", parent=roadmap_mod.split_status(item)[0])


def on_main(repo: Path, lane: dict) -> bool:
    """The claim check: the lane's work is on the project's branch (its branch, if still there, is merged)."""
    branch = f"refs/heads/lane/{lane['id']}"
    if _git(repo, "rev-parse", "--verify", "-q", branch).returncode != 0:
        return True  # merged and removed by the review
    return _git(repo, "merge-base", "--is-ancestor", branch, "HEAD").returncode == 0


def review_passed(home: Path, repo: Path, lane: dict) -> str:
    """The review passed: tick the item (if the claim holds on main) and its review sub-item. Returns a line."""
    linked = lane_item(lane)
    if linked is None:
        return ""
    step, item = linked
    clean, project = roadmap_mod.split_status(item)[0], str(lane["project"])
    if not on_main(repo, lane):
        return review_failed(home, repo, lane, "the lane's work is not on main")
    sub = review_text(clean)

    def mutate(text: str) -> str:
        steps = roadmap_mod.parse(text)
        if roadmap_mod.find_item(steps, step, sub) is not None:
            text = roadmap_mod.set_item(text, step, sub, roadmap_mod.norm(sub), tick=True)
        if roadmap_mod.find_item(roadmap_mod.parse(text), step, clean) is None:
            return text  # already ticked (:func:`tick_merged`)
        return roadmap_mod.set_item(text, step, clean, clean, tick=True)

    commit_change(home, repo, project, mutate, _message("done", step, str(lane["id"])))
    return f"ticked the item of step {step}"


def review_failed(home: Path, repo: Path, lane: dict, reason: str) -> str:
    """The review failed: close the review sub-item with the reason and reopen the item with it."""
    linked = lane_item(lane)
    if linked is None:
        return ""
    step, item = linked
    clean, project = roadmap_mod.split_status(item)[0], str(lane["project"])
    sub = review_text(clean)
    reason = roadmap_mod.norm(reason)[:160]

    def mutate(text: str) -> str:
        steps = roadmap_mod.parse(text)
        if roadmap_mod.find_item(steps, step, sub) is not None:
            text = roadmap_mod.set_item(text, step, sub, f"{sub} — falló: {re.sub(r'[()]', ' ', reason)}", tick=True)
        return roadmap_mod.set_item(text, step, clean, roadmap_mod.with_status(clean, roadmap_mod.STATE_REOPENED, reason))

    commit_change(home, repo, project, mutate, _message("reopen", step, str(lane["id"])))
    return f"reopened the item of step {step}: {reason}"


# -- items whose work is on main --------------------------------------------------------------

DOC_SUFFIXES = (".md", ".txt", ".rst")


def done_note(clean: str, sha: str) -> str:
    return f"{clean} (hecho: {sha})"


def merged_commit(repo: Path, lane: dict) -> str | None:
    """Short sha of the lane's merge commit (or branch tip) when its work is an ancestor of main, else None."""
    branch = f"refs/heads/lane/{lane['id']}"
    if _git(repo, "rev-parse", "--verify", "-q", branch).returncode == 0:
        if _git(repo, "merge-base", "--is-ancestor", branch, "HEAD").returncode != 0:
            return None
        merge = _git(repo, "log", "--merges", "--ancestry-path", "--format=%h", f"{branch}..HEAD").stdout.split()
        if merge:
            return merge[-1]  # the oldest merge after the branch tip is the one that brought it in
        return _git(repo, "rev-parse", "--short", branch).stdout.strip() or None
    found = _git(repo, "log", "--merges", "-1", "--format=%h", "--grep", f"lane/{lane['id']}", "-F").stdout.strip()
    return found or None


def tick_merged(home: Path, config: Config, lanes: list[dict]) -> bool:
    """Lanes that reported ``done`` and merged themselves never get a review, so their item would stay
    "(en revisión)": tick it "(hecho: <sha>)" once the lane's work is on main (the claim check, :func:`on_main`)."""
    changed, plans = False, {}
    for seen in lanes:
        outcome = seen.get("outcome") or {}
        if outcome.get("status") != "done" or not outcome.get("applied") or "item" not in seen:
            continue
        lane = read_json(lane_path(home, str(seen.get("id") or "")), None)
        linked = lane_item(lane) if isinstance(lane, dict) else None
        if linked is None or standing(lane):
            continue
        project = str(lane.get("project") or "")
        repo = repo_of(home, project, config)
        if repo is None:
            continue
        if project not in plans:
            try:
                plans[project] = _read(repo)[1]
            except PlanGateError:
                plans[project] = []
        step, clean = linked[0], roadmap_mod.split_status(linked[1])[0]
        found = roadmap_mod.find_item(plans[project], step, clean)
        if found is None or roadmap_mod.split_status(found["text"])[1] != roadmap_mod.STATE_REVIEW:
            continue
        sha = merged_commit(repo, lane)
        if sha is None:
            continue
        try:
            changed |= commit_change(
                home, repo, project,
                lambda text: roadmap_mod.set_item(text, step, clean, done_note(clean, sha), tick=True),
                _message("done", step, str(lane["id"])))
        except PlanGateError:
            continue
        plans.pop(project, None)
    return changed


def mark_done(home: Path, repo: Path, project: str, step: str, item: str, commit: str) -> str:
    """``tirith plan done``: tick ``item`` because ``commit`` (on main, with code beyond docs) did its work.
    Returns the short sha; raises ``PlanGateError`` when it does not hold."""
    sha = _git(repo, "rev-parse", "--verify", "-q", f"{commit}^{{commit}}").stdout.strip()
    if not sha:
        raise PlanGateError(f"{commit} is not a commit of {project}")
    if _git(repo, "merge-base", "--is-ancestor", sha, "HEAD").returncode != 0:
        raise PlanGateError(f"{commit} is not on main yet")
    files = _git(repo, "diff", "--name-only", f"{sha}^1", sha).stdout.split() if _git(
        repo, "rev-parse", "-q", "--verify", f"{sha}^1").returncode == 0 else _git(
        repo, "show", "--name-only", "--format=", sha).stdout.split()
    code = [f for f in files if not f.startswith("docs/") and not f.lower().endswith(DOC_SUFFIXES)]
    if not code:
        raise PlanGateError(f"{commit} changes nothing outside the docs, so it is not the work of that item")
    short = _git(repo, "rev-parse", "--short", sha).stdout.strip()
    _, steps = _read(repo)
    if not any(s["id"] == step for s in steps):
        raise PlanGateError(f"step {step} is not in the plan")
    clean = roadmap_mod.split_status(item)[0]
    if roadmap_mod.find_item(steps, step, clean) is None:
        raise PlanGateError(f"that item is not an open item of step {step}")
    commit_change(home, repo, project,
                  lambda text: roadmap_mod.set_item(text, step, clean, done_note(clean, short), tick=True),
                  _message("done", step, "manager"))
    return short


# One-time backfill: work merged before ticking existed. (project, step, start of the item, merge commit)
BACKFILL = (
    ("tirith", "14", "El collector vuelve a tardar menos de 3 s por ciclo", "abdf921"),
    ("tirith", "14", "Fusionar nunca es una pregunta para el dueño", "cd1b9c1"),
    ("tirith", "14", "Regla en código", "222849b"),
    ("tirith", "14", "El collector se reinicia solo cuando cambia su código en main", "1df0b8d"),
)
BACKFILL_MARK = "plan-backfill-ticks.done"


def backfill(home: Path, config: Config) -> bool:
    """Once, through the writer: tick the listed items with their merge commits (each open item that starts
    with the listed text). Items that cannot be verified now are tried again next pass."""
    mark = Path(home) / BACKFILL_MARK
    if mark.exists():
        return False
    changed, pending = False, False
    for project, step, start, commit in BACKFILL:
        repo = repo_of(home, project, config)
        if repo is None:
            continue
        try:
            steps = _read(repo)[1]
            targets = [roadmap_mod.split_status(i["text"])[0] for s in steps if s["id"] == step
                       for i in s["items"] if not i["done"] and roadmap_mod.split_status(i["text"])[0].startswith(start)]
            for target in targets:
                mark_done(home, repo, project, step, target, commit)
                changed = True
        except PlanGateError:
            pending = True
    if not pending:
        mark.parent.mkdir(parents=True, exist_ok=True)
        mark.write_text(now_iso(), encoding="utf-8")
    return changed


# -- the dispatcher's pass --------------------------------------------------------------------


def sync(home: Path, config: Config, lanes: list[dict], relauncher: Callable | None = None) -> bool:
    """One pass over ended lanes: write outcomes the plan could not take yet, and deal with ``no-outcome``
    lanes (one more run with a note, then the manager queue). Returns True when something changed."""
    changed = backfill(home, config)
    changed |= tick_merged(home, config, lanes)
    for seen in lanes:
        waiting = (seen.get("outcome") and not seen["outcome"].get("applied")) or (
            seen.get("result") == NO_OUTCOME and not seen.get("no_outcome_handled"))
        if "item" in seen and not waiting:
            continue  # nothing to write for this lane (a row without the field is read from its file)
        lane = read_json(lane_path(home, str(seen.get("id") or "")), None)
        if not isinstance(lane, dict) or lane_item(lane) is None:
            continue
        repo = repo_of(home, str(lane.get("project") or ""), config)
        outcome = lane.get("outcome")
        if outcome and not outcome.get("applied") and repo is not None:
            try:
                apply_outcome(home, repo, str(lane["project"]), lane)
                outcome["applied"] = True
                outcome.pop("error", None)
            except PlanGateError as err:
                if outcome.get("error") == str(err):
                    continue
                outcome["error"] = str(err)
            write_json_atomic(lane_path(home, str(lane["id"])), lane)
            changed = True
        if lane.get("result") == NO_OUTCOME and not lane.get("no_outcome_handled"):
            changed |= _no_outcome(home, lane, repo, relauncher)
    return changed


def _no_outcome(home: Path, lane: dict, repo: Path | None, relauncher: Callable | None) -> bool:
    lane_id, project = str(lane["id"]), str(lane.get("project") or "")
    step, item = lane_item(lane) or ("", "")
    note = ("Your previous run ended without recording its outcome, so its work was not merged. Do the item again "
            "and, before you finish, run `tirith plan report --lane $TIRITH_LANE --status done|partial|blocked`.")
    if repo is not None:
        try:
            commit_change(home, repo, project, lambda text: roadmap_mod.set_item(
                text, step, item, roadmap_mod.with_status(
                    roadmap_mod.split_status(item)[0], roadmap_mod.STATE_REOPENED, f"{lane_id} terminó sin informe")),
                _message("reopen", step, lane_id))
        except PlanGateError:
            pass
    again = None
    if not lane.get("retry_of") and relauncher is not None:
        try:
            again = relauncher(home, lane, str(lane.get("agent") or ""), note)
        except Exception as err:  # the manager queue below says it
            note = f"{note} (the restart failed: {err})"
    lane["no_outcome_handled"] = again or "queue"
    write_json_atomic(lane_path(home, lane_id), lane)
    if again is None:
        asks_mod.queue_add(home, project, lane_id,
                           f"Lane {lane_id} ended without a plan outcome" + (" twice" if lane.get("retry_of") else "")
                           + f": its work was not merged. Item: {item}\nLook at it, then: tirith lane drop {lane_id} "
                           f"or start the item again.", kind="no-outcome", item_id=f"sq-no-outcome-{lane_id}")
    return True
