"""Local, transactional task ownership and worker mailboxes. No servers or model calls."""

from __future__ import annotations

import json
import hashlib
import os
import sqlite3
import time
import uuid
from collections import Counter
from contextlib import contextmanager
from pathlib import Path, PurePosixPath

from tirith import roadmap
from tirith.store import lane_path, read_json
from tirith.store import write_lane
from tirith.home import now_iso

LEASE_S = 600
MAX_PENDING_MESSAGES = 1000
MAX_ACKNOWLEDGED_MESSAGES = 1000


class CoordinationError(ValueError):
    pass


@contextmanager
def database(home: Path):
    Path(home).mkdir(parents=True, exist_ok=True)
    try:
        db = sqlite3.connect(Path(home) / "coordination.sqlite3", timeout=10)
    except sqlite3.Error as error:
        raise CoordinationError(f"coordination storage unavailable: {error}") from error
    db.row_factory = sqlite3.Row
    try:
        db.executescript("""
            CREATE TABLE IF NOT EXISTS tasks (
                project TEXT NOT NULL, id TEXT NOT NULL, step TEXT NOT NULL,
                text TEXT NOT NULL, done INTEGER NOT NULL, present INTEGER NOT NULL DEFAULT 1,
                needs TEXT NOT NULL, writes TEXT NOT NULL, owner TEXT, attempt TEXT,
                phase TEXT NOT NULL DEFAULT 'ready', expires REAL,
                PRIMARY KEY(project, id));
            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT, project TEXT NOT NULL, sender TEXT NOT NULL,
                recipient TEXT NOT NULL, text TEXT NOT NULL, delivery_key TEXT NOT NULL,
                acknowledged INTEGER NOT NULL DEFAULT 0,
                UNIQUE(project, sender, delivery_key));
        """)
        db.execute("BEGIN IMMEDIATE")
        yield db
        db.commit()
    except sqlite3.Error as error:
        db.rollback()
        raise CoordinationError(f"coordination storage unavailable: {error}") from error
    except BaseException:
        db.rollback()
        raise
    finally:
        db.close()


def write_paths(paths):
    result = []
    for raw in paths:
        path = PurePosixPath(str(raw).replace("\\", "/"))
        if path.is_absolute() or ".." in path.parts or str(path) == "." or any(c in str(path) for c in "*?[]"):
            raise CoordinationError("write paths must be relative files or directories inside the project")
        result.append(str(path).rstrip("/"))
    return sorted(set(result))


def overlaps(left, right):
    return any(a == "." or b == "." or a == b or a.startswith(b + "/") or b.startswith(a + "/")
               for a in left for b in right)


def plan_rows(steps):
    rows = []
    for step in steps:
        for item in step["items"]:
            if item.get("metadata_error"):
                raise CoordinationError(item["metadata_error"])
            rows.append({**item, "id": item.get("id") or roadmap.item_id(item["text"]),
                         "step": str(step["id"]), "depends_on": item.get("depends_on", []),
                         "writes": write_paths(item.get("writes", []))})
    legacy_ids = [row["id"] for row in rows]
    counts = Counter(legacy_ids)
    explicit = [item["id"] for step in steps for item in step["items"] if item.get("id")]
    explicit_set = set(explicit)
    for row in rows:
        # Before permanent IDs existed, identical titles in different steps were legal.
        # Keep those launches distinct without rewriting the owner's entire plan.
        if row["id"] not in explicit_set and counts[row["id"]] > 1:
            row["id"] = "i-" + hashlib.sha256((row["step"] + "\0" + row["id"]).encode()).hexdigest()[:16]
    ids = [row["id"] for row in rows]
    if len(set(explicit)) != len(explicit):
        raise CoordinationError("permanent task IDs must be unique")
    # Legacy plans occasionally repeat an identical checklist item in one or more steps.
    # They remain readable, but claiming ambiguous IDs is refused below.
    by_id = {row["id"]: row for row in sorted(rows, key=lambda row: row["done"], reverse=True)}
    visiting, visited = set(), set()

    def visit(task_id):
        if task_id in visiting:
            raise CoordinationError("task dependencies contain a cycle")
        if task_id in visited:
            return
        visiting.add(task_id)
        for dependency in by_id[task_id]["depends_on"]:
            if dependency not in by_id:
                raise CoordinationError(f"missing prerequisite: {dependency}")
            if ids.count(dependency) != 1:
                raise CoordinationError(f"ambiguous prerequisite: {dependency}")
            visit(dependency)
        visiting.remove(task_id)
        visited.add(task_id)

    for task_id in by_id:
        visit(task_id)
    return rows


def _sync(db, project, rows):
    db.execute("UPDATE tasks SET present=0 WHERE project=?", (project,))
    for row in sorted(rows, key=lambda row: row["done"], reverse=True):
        db.execute("""INSERT INTO tasks(project,id,step,text,done,needs,writes)
            VALUES(?,?,?,?,?,?,?) ON CONFLICT(project,id) DO UPDATE SET
            step=excluded.step,text=excluded.text,done=excluded.done,present=1,
            needs=excluded.needs,writes=excluded.writes""",
                   (project, row["id"], row["step"], row["text"], int(row["done"]),
                    json.dumps(row["depends_on"]), json.dumps(row["writes"])))
    db.execute("UPDATE tasks SET phase='merged',owner=NULL,expires=NULL WHERE project=? AND done=1",
               (project,))


def _alive(home, owner):
    try:
        lane = json.loads(lane_path(home, owner).read_text())
    except FileNotFoundError:
        return False
    except (OSError, ValueError):
        return True  # unreadable state is not proof that the worker is gone
    if not isinstance(lane, dict):
        return True
    if lane.get("ended_at"):
        return False
    for pid in (lane.get("pid"), lane.get("controller_pid")):
        if not isinstance(pid, int) or pid <= 0:
            continue
        try:
            os.kill(pid, 0)
            return True
        except ProcessLookupError:
            continue
        except PermissionError:
            return True
        except OSError:
            return True
    return False


def _recover(db, home, now):
    released = []
    queued = read_json(Path(home) / "mergeq.json", {}).get("queue", [])
    for row in db.execute("SELECT * FROM tasks WHERE owner IS NOT NULL AND expires<=? AND phase IN ('claimed','running','verifying')", (now,)).fetchall():
        if _alive(home, row["owner"]):
            db.execute("UPDATE tasks SET expires=? WHERE project=? AND id=?",
                       (now + LEASE_S, row["project"], row["id"]))
        else:
            if row["phase"] == "verifying" and any(e.get("lane") == row["owner"] and e.get("project") == row["project"] for e in queued):
                continue
            db.execute("UPDATE tasks SET owner=NULL,phase='ready',expires=NULL WHERE project=? AND id=?",
                       (row["project"], row["id"]))
            released.append(row["id"])
            lane = read_json(lane_path(home, row["owner"]), {})
            if lane and not lane.get("ended_at"):
                lane.update(id=row["owner"], ended_at=now_iso(), result="failed", coordination_recovery="expired task lease; worker exited")
                write_lane(home, lane)
    return released


def recover(home, now=None):
    if not (Path(home) / "coordination.sqlite3").exists():
        return []
    with database(home) as db:
        return _recover(db, home, time.time() if now is None else now)


def claim(home, repo, project, step, item, owner, now=None):
    """Read the authoritative plan and reserve a task and its write paths in one transaction."""
    now = time.time() if now is None else now
    from tirith import plangate
    with plangate.writer_lock(home, project), database(home) as db:
        _, steps = plangate._read(repo)
        rows = plan_rows(steps)
        matches = [row for row in rows if not row["done"] and row["step"] == str(step) and
                   (row["id"] == item or roadmap.split_status(row["text"])[0] == roadmap.split_status(item)[0])]
        if len(matches) != 1:
            raise CoordinationError("task is missing or ambiguous in the plan")
        task = matches[0]
        if sum(row["id"] == task["id"] and not row["done"] for row in rows) != 1:
            raise CoordinationError("task ID is ambiguous; give the items permanent IDs")
        _sync(db, project, rows)
        # Normalize aliases through existing symlinks before reserving files.
        for candidate in rows:
            canonical = []
            for path in candidate["writes"]:
                try:
                    canonical.append(str((Path(repo) / path).resolve().relative_to(Path(repo).resolve())))
                except ValueError as error:
                    raise CoordinationError("write path resolves outside the project") from error
            candidate["writes"] = canonical
            db.execute("UPDATE tasks SET writes=? WHERE project=? AND id=?",
                       (json.dumps(canonical), project, candidate["id"]))
        _recover(db, home, now)
        if task["done"]:
            raise CoordinationError("task is already integrated")
        state = roadmap.item_state(task)[0]
        if state in (roadmap.STATE_BLOCKED, roadmap.STATE_REVIEW):
            raise CoordinationError(f"task is {state}")
        blockers = [dependency for dependency in task["depends_on"]
                    if not all(row["done"] for row in rows if row["id"] == dependency)]
        if blockers:
            raise CoordinationError("unfinished prerequisites: " + ", ".join(blockers))
        current = db.execute("SELECT * FROM tasks WHERE project=? AND id=?", (project, task["id"])).fetchone()
        if current["owner"] == owner and current["phase"] == "running":
            raise CoordinationError("this worker is already executing the task")
        if db.execute("SELECT 1 FROM tasks WHERE owner=? AND done=0 AND (project<>? OR id<>?)",
                      (owner, project, task["id"])).fetchone():
            raise CoordinationError("a lane can own only one unfinished task")
        if current["owner"] and current["owner"] != owner:
            lane = read_json(lane_path(home, current["owner"]), {})
            queued = read_json(Path(home) / "mergeq.json", {}).get("queue", [])
            in_queue = any(e.get("lane") == current["owner"] and e.get("project") == project for e in queued)
            if not lane.get("ended_at") or in_queue:
                raise CoordinationError(f"task is already claimed by {current['owner']}")
        for other in db.execute("SELECT * FROM tasks WHERE project=? AND owner IS NOT NULL AND done=0 AND present=1", (project,)):
            if other["id"] != task["id"] and overlaps(task["writes"], json.loads(other["writes"])):
                raise CoordinationError(f"write paths are reserved by task {other['id']}")
        token = current["attempt"] if current["owner"] == owner else uuid.uuid4().hex
        db.execute("UPDATE tasks SET owner=?,attempt=?,phase='claimed',expires=? WHERE project=? AND id=?",
                   (owner, token, now + LEASE_S, project, task["id"]))
        return {"task_id": task["id"], "task_attempt": token}


def update(home, lane, phase=None, release=False, now=None):
    if not lane.get("task_id"):
        return
    with database(home) as db:
        row = db.execute("SELECT * FROM tasks WHERE project=? AND id=?",
                         (lane["project"], lane["task_id"])).fetchone()
        if row is None or row["owner"] != lane["id"] or row["attempt"] != lane.get("task_attempt"):
            raise CoordinationError("this task attempt no longer owns the task; discard the stale report")
        db.execute("UPDATE tasks SET owner=?,phase=?,expires=? WHERE project=? AND id=?",
                   (None if release else row["owner"], phase or row["phase"],
                    None if release else (time.time() if now is None else now) + LEASE_S,
                    lane["project"], lane["task_id"]))


def send(home, project, sender, recipient, text, delivery_key):
    if not text.strip() or len(text) > 16000 or not delivery_key:
        raise CoordinationError("a message needs text (at most 16000 characters) and a delivery key")
    for lane_id in (sender, recipient):
        lane = read_json(lane_path(home, lane_id), {})
        if lane.get("project") != project:
            raise CoordinationError("sender and recipient must be lanes of this project")
    with database(home) as db:
        old = db.execute("SELECT * FROM messages WHERE project=? AND sender=? AND delivery_key=?",
                         (project, sender, delivery_key)).fetchone()
        if old:
            if old["recipient"] != recipient or old["text"] != text:
                raise CoordinationError("delivery key already used for a different message")
            return old["id"]
        count = db.execute("SELECT COUNT(*) FROM messages WHERE project=? AND recipient=? AND acknowledged=0",
                           (project, recipient)).fetchone()[0]
        if count >= MAX_PENDING_MESSAGES:
            raise CoordinationError("recipient inbox is full; acknowledge delivered messages before sending more")
        return db.execute("INSERT INTO messages(project,sender,recipient,text,delivery_key) VALUES(?,?,?,?,?)",
                          (project, sender, recipient, text, delivery_key)).lastrowid


def inbox(home, project, recipient):
    with database(home) as db:
        return [dict(row) for row in db.execute("SELECT id,sender,text FROM messages WHERE project=? AND recipient=? AND acknowledged=0 ORDER BY id", (project, recipient))]


def acknowledge(home, project, recipient, ids):
    with database(home) as db:
        for message_id in ids:
            db.execute("UPDATE messages SET acknowledged=1 WHERE project=? AND recipient=? AND id=?", (project, recipient, int(message_id)))
        db.execute("""DELETE FROM messages WHERE project=? AND recipient=? AND acknowledged=1
            AND id NOT IN (SELECT id FROM messages WHERE project=? AND recipient=? AND acknowledged=1
                           ORDER BY id DESC LIMIT ?)""",
                   (project, recipient, project, recipient, MAX_ACKNOWLEDGED_MESSAGES))


def reservations(home, project):
    if not (Path(home) / "coordination.sqlite3").exists():
        return []
    with database(home) as db:
        return [dict(row) for row in db.execute("SELECT id,owner,writes FROM tasks WHERE project=? AND owner IS NOT NULL AND done=0 AND present=1", (project,))]
