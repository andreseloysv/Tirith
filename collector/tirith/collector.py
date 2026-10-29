"""The collector: merge CLI inputs with live facts into ``state.json``.

One cycle reads config, lanes, events, notes, waits and the manager file,
enriches them with process/log/git facts and cached brain results, verifies
every project's commit guard, updates hero renown, and writes ``state.json``
atomically. Brain jobs are only enqueued here; the worker fills the cache
for later cycles, so a cycle never waits on the network.
"""

from __future__ import annotations

import dataclasses
import hashlib
import json
import os
import re
import subprocess
import sys
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path

from tirith import timeouts as timeouts_mod
from tirith import activity as activity_mod
from tirith import asks as asks_mod
from tirith import howto as howto_mod
from tirith import ownergate as ownergate_mod
from tirith import brain as brain_mod
from tirith.guardwrap import default_words, verify_guard
from tirith.home import Config, ProjectConfig, ensure_home, load_config, now_iso, parse_iso
from tirith import agentrun as agentrun_mod
from tirith import keepgoing as keepgoing_mod
from tirith import flows as flows_mod
from tirith import github as github_mod
from tirith import manifest as manifest_mod
from tirith import setup as setup_mod
from tirith import roadmap as roadmap_mod
from tirith import hooks as hooks_mod
from tirith import autopilot as autopilot_mod
from tirith import dispatch as dispatch_mod
from tirith import selfcheck as selfcheck_mod
from tirith import livesteps as livesteps_mod
from tirith import managerd as managerd_mod
from tirith import actions as actions_mod
from tirith import inbox as inbox_mod
from tirith import launch as launch_mod
from tirith import projenv as projenv_mod
from tirith import fallback as fallback_mod
from tirith import filerefs as filerefs_mod
from tirith import models as models_mod
from tirith import review as review_mod
from tirith.logview import entries_from_events, log_entries
from tirith import projects as projects_mod
from tirith.rooms import RoomsService
from tirith.names import (
    RENOWN_LANE_DONE, RENOWN_LANE_FAILED, RENOWN_STAGE_PASSED,
    apply_renown, hero_epithet, hero_name, rank_for,
)
from tirith.store import (
    agent_detail_js, agent_file_name, append_jsonl, read_json, read_jsonl, read_lanes, state_js, tail_lines, write_json_atomic, write_lane, write_text_atomic,
)

STALLED_AFTER_S = 600
ANSWERED_KEPT = 10
GUARD_SNOOZE = timedelta(days=7)
GUARD_MARKER = "FORBIDDEN_WORDS_GUARD"
DROP_ENDED_AFTER = timedelta(hours=24)
MAX_PIPELINE_RUNS = 20
STAGE_TIMEOUT_S = 3600  # a started stage with no end event is "interrupted" after this long (or when its pid is gone)
GIT_HOOK_FLOWS_FIRST = 20
LOG_TAIL_LINES = 20
SUMMARY_LOG_LINES = 200


def pid_alive(pid) -> bool:
    """True when ``pid`` looks like a live process (signal 0 succeeds)."""
    try:
        pid_int = int(pid)
    except (TypeError, ValueError):
        return False
    if pid_int <= 0:
        return False
    try:
        os.kill(pid_int, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    except OSError:
        return False
    return True


def group_alive(pgid) -> bool:
    """True when a process group still has a member (``killpg`` signal 0)."""
    try:
        group = int(pgid)
    except (TypeError, ValueError):
        return False
    if group <= 1 or not hasattr(os, "killpg"):
        return False
    try:
        os.killpg(group, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    except OSError:
        return False
    return True


DEAD_LANE_GRACE_S = 30.0  # a lane that just started (or is just ending by itself) is left alone


def close_dead_lanes(home: Path, lanes: list[dict], now: datetime) -> list[str]:
    """Close a lane whose pid and process group are gone but that has no ``ended_at``
    (``result: "stopped"``), so it stops showing as working. Returns the ids closed."""
    closed = []
    for lane in lanes:
        if lane.get("ended_at") or lane.get("pid") is None:
            continue
        if pid_alive(lane["pid"]) or group_alive(lane.get("pgid")):
            continue
        started = parse_iso(lane.get("started_at"))
        if started and (now - started).total_seconds() < DEAD_LANE_GRACE_S:
            continue
        lane["ended_at"] = now_iso(now)
        lane["result"] = "stopped"
        lane.pop("token", None)
        if not lane.get("report"):
            lane["report"] = "The lane's process is gone; closed as stopped."
        write_lane(home, lane)
        closed.append(str(lane["id"]))
    return closed


def git_head(workspace: str | Path | None) -> str | None:
    """Current HEAD commit of a workspace, or None outside a git repo."""
    if not workspace:
        return None
    try:
        out = timeouts_mod.run(["git", "-C", str(workspace), "rev-parse", "HEAD"], "git", text=True)
    except (OSError, subprocess.SubprocessError):
        return None
    return out.stdout.strip() or None if out.returncode == 0 else None


_SHORTSTAT_FILES = re.compile(r"(\d+) files? changed")
_SHORTSTAT_INS = re.compile(r"(\d+) insertions?\(\+\)")
_SHORTSTAT_DEL = re.compile(r"(\d+) deletions?\(-\)")


def parse_shortstat(text: str) -> tuple[int, int, int]:
    """Parse ``git diff --shortstat`` output into (files, +lines, -lines)."""
    def grab(pattern: re.Pattern) -> int:
        match = pattern.search(text or "")
        return int(match.group(1)) if match else 0
    return grab(_SHORTSTAT_FILES), grab(_SHORTSTAT_INS), grab(_SHORTSTAT_DEL)


def git_shortstat(workspace: str | Path | None, start_commit: str | None) -> tuple[int, int, int]:
    """Diff size of a workspace against the lane's start commit."""
    if not workspace or not start_commit:
        return (0, 0, 0)
    try:
        out = timeouts_mod.run(["git", "-C", str(workspace), "diff", "--shortstat", str(start_commit)],
                               "git", text=True)
    except (OSError, subprocess.SubprocessError):
        return (0, 0, 0)
    if out.returncode != 0:
        return (0, 0, 0)
    return parse_shortstat(out.stdout)


def log_mtime(path: str | Path | None) -> float | None:
    if not path:
        return None
    try:
        return Path(path).stat().st_mtime
    except OSError:
        return None


def lane_status(lane: dict, now: datetime) -> str:
    """working/done/failed/stalled for one lane registration."""
    if lane.get("ended_at"):
        return "done" if lane.get("result") == "done" else "failed"
    pid = lane.get("pid")
    if pid is not None and not pid_alive(pid):
        return "failed"
    last_activity = log_mtime(lane.get("log"))
    if last_activity is None:
        started = parse_iso(lane.get("started_at"))
        last_activity = started.timestamp() if started else now.timestamp()
    if now.timestamp() - last_activity > STALLED_AFTER_S:
        return "stalled"
    return "working"


REPORT_MAX = 16 * 1024


def _word_cut(text: str, limit: int) -> str:
    """``text`` cut at a word boundary with an ellipsis, never mid-word."""
    if len(text) <= limit:
        return text
    cut = text[:limit - 1]
    return (cut.rsplit(" ", 1)[0] if " " in cut else cut).rstrip() + "…"


def _bounded_report(report):
    """The full lane report for the page to render, at most 16 KB, cut at a line boundary."""
    if not report:
        return report
    text = str(report)
    if len(text) <= REPORT_MAX:
        return text
    return text[:REPORT_MAX].rsplit("\n", 1)[0].rstrip() + "\n\n…"


# -- state.json size: full detail only where it is looked at -------------------------------------------

FULL_DETAIL_WINDOW = timedelta(hours=2)   # ended agents keep full detail this long
STATE_AGENTS_MAX = 60
STATE_AGENTS_BYTES = 480 * 1024   # past this the oldest ended agents drop to summaries (working ones never do)
STATE_LOG_ENTRIES = 30
STATE_EVENTS = 12            # the drawer shows 12; the detail file has the rest
STATE_LOG_TAIL = 5          # raw lines: only the page without structured entries reads them
STATE_TEXT_MAX = 160         # longest string of a log entry or event kept in the state
STATE_SYSTEM_TEXT_MAX = 80   # and of the system noise (heartbeats, init)
STATE_FILES = 10
STATE_TESTS = 5
STATE_REPORT_MAX = 4 * 1024
SLIM_REPORT_MAX = 300
TIMING_EVERY = 60  # cycles between per-phase timing lines in the collector log
EVENT_STATE_KEYS = ("ts", "agent", "event", "tool", "summary", "source")   # what the drawer reads of a live event
DETAIL_KEYS = ("log_entries", "log_tail", "events", "files", "tests", "report")


def _first_paragraph(text, limit: int = SLIM_REPORT_MAX):
    if not text:
        return text
    return _word_cut(str(text).strip().split("\n\n", 1)[0].strip(), limit)


def _detail_hash(detail: dict) -> str:
    return hashlib.sha1(json.dumps(detail, sort_keys=True, separators=(",", ":")).encode("utf-8")).hexdigest()[:12]


def _clip_texts(item, limit: int = STATE_TEXT_MAX):
    """``item`` (a log entry or event) with every long string cut; (copy, whether anything was cut)."""
    cut = False

    def walk(value):
        nonlocal cut
        if isinstance(value, str) and len(value) > limit:
            cut = True
            return value[:limit - 1] + "…"
        if isinstance(value, dict):
            return {k: walk(v) for k, v in value.items()}
        if isinstance(value, list):
            return [walk(v) for v in value]
        return value
    return walk(item), cut


def _slim_row(row: dict, full: bool) -> dict:
    """The state row of one agent: all of it for ``full``, else the summary fields only. Sets
    ``report_clipped`` when the report was cut and ``detail_more`` when the detail file has more."""
    out = dict(row)
    report = str(row.get("report") or "")
    more = False
    if full:
        for key, cap in (("log_entries", STATE_LOG_ENTRIES), ("events", STATE_EVENTS),
                         ("files", STATE_FILES), ("tests", STATE_TESTS)):
            items = row.get(key) or []
            if len(items) > cap:
                items = items[-cap:]
                more = True
            if key == "events":
                items = [{k: e[k] for k in EVENT_STATE_KEYS if k in e} for e in items]
            if key == "log_entries":
                items = [{**e, "fields": None} if len(json.dumps(e.get("fields"))) > 120 else e for e in items]
            if key in ("log_entries", "events"):
                clipped = [_clip_texts(i, STATE_SYSTEM_TEXT_MAX if i.get("kind") == "system" else STATE_TEXT_MAX)
                           for i in items]
                items = [c for c, _ in clipped]
                more = more or any(flag for _, flag in clipped)
            out[key] = items
        tail = row.get("log_tail") or []
        if len(tail) > STATE_LOG_TAIL:
            out["log_tail"] = [str(t)[:STATE_TEXT_MAX] for t in tail[-STATE_LOG_TAIL:]]
            more = True
        if len(report) > STATE_REPORT_MAX:
            out["report"] = report[:STATE_REPORT_MAX].rsplit("\n", 1)[0].rstrip() + "\n\n…"
            more = True
    else:
        more = bool(row.get("log_entries") or row.get("events") or row.get("files") or row.get("tests")
                    or row.get("log_tail") or len(report) > SLIM_REPORT_MAX)
        for key in ("log_entries", "log_tail", "events", "files", "tests"):
            out[key] = []
        out["report"] = _first_paragraph(report) or row.get("report")
    out["report_clipped"] = bool(more and report and out.get("report") != row.get("report"))
    out["detail_more"] = more
    return out


def fallback_summary(status: str, log_tail: list[str], feature: str, now: datetime,
                     lane: dict) -> str:
    """No-brain summary: a plain sentence about the status, never raw log output."""
    if status == "stalled":
        mtime = log_mtime(lane.get("log"))
        silent_s = int(now.timestamp() - mtime) if mtime else STALLED_AFTER_S
        return f"Waiting: no log output for {max(1, silent_s // 60)} minutes."
    if status == "done":
        return f"Finished: {feature}." if feature else "Finished."
    if status == "failed":
        return f"Failed: {feature}." if feature else "Failed."
    return f"Working on {feature}." if feature else "Working."


def _first_paragraph(text: str) -> str:
    for block in re.split(r"\n\s*\n", str(text or "").strip()):
        if block.strip():
            return block.strip()
    return ""


def say_summary(log_view: list[dict], report) -> str | None:
    """The "now doing" line: the agent's newest own words (a timed ``say`` entry, never tool output or a bare log line);
    for an ended lane the first paragraph of its report."""
    if report:
        para = _first_paragraph(report)
        if para:
            return _word_cut(para, 300)
    for entry in log_view:  # newest first
        if entry.get("kind") != "say" or not entry.get("ts") or \
                str(entry.get("text") or "").startswith("Used "):
            continue
        text = "\n".join(p for p in (entry.get("text"), entry.get("detail")) if p).strip()
        if text:
            return _word_cut(text, 300)
    return None


def collect_tree(repo: Path | None, limit: int = 400) -> tuple[str, int]:
    """A compact file listing of a repo for the architecture job."""
    if repo is None or not Path(repo).is_dir():
        return "", 0
    skip = {".git", "node_modules", ".venv", "__pycache__", ".build", "DerivedData"}
    lines: list[str] = []
    for root, dirs, files in os.walk(repo):
        dirs[:] = sorted(d for d in dirs if d not in skip and not d.startswith("."))
        for name in sorted(files):
            rel = str((Path(root) / name).relative_to(repo))
            lines.append(rel)
            if len(lines) >= limit:
                return "\n".join(lines), len(lines)
    key_bits: list[str] = []
    for key in ("README.md", "pyproject.toml", "package.json", "Cargo.toml"):
        candidate = Path(repo) / key
        if candidate.is_file():
            try:
                key_bits.append(f"--- {key} ---\n" + candidate.read_text(
                    encoding="utf-8", errors="replace")[:2000])
            except OSError:
                pass
    text = "\n".join(lines)
    if key_bits:
        text += "\n\n" + "\n\n".join(key_bits)
    return text, len(lines)


def heuristic_architecture(repo: Path | None) -> dict:
    """Offline architecture from top-level folders (used by ``tirith map``)."""
    nodes: list[dict] = []
    if repo is not None and Path(repo).is_dir():
        skip = {".git", "node_modules", ".venv", "__pycache__"}
        for entry in sorted(Path(repo).iterdir(), key=lambda p: p.name):
            if entry.name in skip or entry.name.startswith("."):
                continue
            if entry.is_dir():
                count = sum(1 for _ in entry.rglob("*") if _.is_file())
                slug = re.sub(r"[^a-z0-9_-]+", "-", entry.name.lower()).strip("-") or "root"
                nodes.append({"id": slug, "label": entry.name, "kind": "service",
                              "files": count})
        if not nodes:
            count = sum(1 for p in Path(repo).iterdir() if p.is_file())
            nodes.append({"id": "root", "label": Path(repo).name or "Project",
                          "kind": "core", "files": count})
    edges = [{"from": nodes[i]["id"], "to": nodes[i + 1]["id"], "label": ""}
             for i in range(len(nodes) - 1)]
    return {"nodes": nodes[:24], "edges": edges[:48],
            "generated_at": now_iso(), "source": "manual" if nodes else "none"}


def normalize_architecture(raw: dict | None, source_fallback: str = "none") -> dict:
    """Coerce any stored/config architecture into the contract shape."""
    if not isinstance(raw, dict):
        return {"nodes": [], "edges": [], "generated_at": None, "source": "none"}
    nodes: list[dict] = []
    for item in raw.get("nodes") or []:
        if isinstance(item, dict) and item.get("id"):
            try:
                files = int(item.get("files") or 0)
            except (TypeError, ValueError):
                files = 0
            nodes.append({"id": str(item["id"]), "label": str(item.get("label") or item["id"]),
                          "kind": str(item.get("kind") or "service"), "files": max(0, files)})
    edges: list[dict] = []
    for item in raw.get("edges") or []:
        if isinstance(item, dict) and item.get("from") and item.get("to"):
            edges.append({"from": str(item["from"]), "to": str(item["to"]),
                          "label": str(item.get("label") or "")})
    source = str(raw.get("source") or source_fallback)
    if source not in ("muse", "manual", "none"):
        source = source_fallback
    return {"nodes": nodes, "edges": edges,
            "generated_at": raw.get("generated_at"), "source": source}


def _looks_like_path(value: str | None) -> bool:
    return bool(value) and ("/" in value or value.startswith("~"))  # type: ignore[operator]


_AUTO_CLIENT = object()


def _stage_alive(stage: dict, now: datetime) -> bool:
    """A started stage without an end event still runs: its process exists and the stage timeout has not passed."""
    started = parse_iso(stage.get("started_at"))
    limit = stage.get("_timeout") or STAGE_TIMEOUT_S
    if started and (now - started).total_seconds() > limit:
        return False
    pid = stage.get("_pid")
    if pid:
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            return False
        except OSError:
            pass
    return True


def backoff_s(failures: int) -> float:
    """Extra wait after ``failures`` cycles in a row failed: 0, 2, 4, ... up to 30 seconds."""
    return min(30.0, 2.0 ** failures) if failures else 0.0


class Collector:
    """Owns one home directory and builds ``state.json`` cycle by cycle."""

    def __init__(self, home: Path, client=_AUTO_CLIENT, config: Config | None = None,
                 rooms_transport=None, github_transport=None):
        self.home = ensure_home(Path(home))
        self._fixed_config = config
        self.config = config or load_config(self.home)
        self.cache = brain_mod.BrainCache(self.home)
        resolved = brain_mod.make_client(self.config) if client is _AUTO_CLIENT else client
        self.worker = brain_mod.BrainWorker(resolved, self.cache)
        self._agents_dirty = False
        self._brain_pending: set[str] = set()  # lanes whose brain answer has not landed yet
        self._owner_items_checked = False
        self.rooms =RoomsService(self.home, rooms_transport)
        self._one_shot = False
        self.activity = activity_mod.ActivityTracker(self.home, warm=True)
        self.github = github_mod.GithubReader(github_transport)
        self._usage_cache: dict = {}
        self._detail_hashes: dict[str, str] = {}
        self._live_steps: dict[str, list] = {}  # lane id -> live transcript steps of this cycle
        self._cycles = 0
        self._tick_at = time.monotonic()
        self._phase_s: dict[str, float] = {}
        self._phase_total: dict[str, float] = {}
        self._inbox_tried: dict[str, float] = {}  # request id -> monotonic time of the last brain try

    def _tick(self, name: str | None) -> None:
        """Charge the time since the last tick to phase ``name`` (``None`` only restarts the clock)."""
        at = time.monotonic()
        if name is None:
            self._phase_s = {}
        else:
            self._phase_s[name] = self._phase_s.get(name, 0.0) + at - self._tick_at
        self._tick_at = at

    # -- identities / renown -------------------------------------------------

    def _usage(self, lane: dict, ended: bool) -> dict | None:
        """Tokens and cost from the lane's log; re-parsed only when the log changed."""
        log = lane.get("log")
        try:
            st = Path(log).stat() if log else None
        except OSError:
            st = None
        if st is None:
            return None
        key = (st.st_mtime_ns, st.st_size)
        cached = self._usage_cache.get(str(log))
        if cached and cached[0] == key:
            return cached[1]
        usage = agentrun_mod.parse_usage(log)
        self._usage_cache[str(log)] = (key, usage)
        return usage

    def _diff(self, lane: dict) -> dict | None:
        diff = lane.get("diff")
        if not isinstance(diff, dict) or not diff.get("path"):
            return None
        path = Path(str(diff["path"]))
        return {"path": str(path), "size": int(diff.get("size") or 0),
                "script": f"diffs/{path.stem}.js"}

    def _load_agents(self) -> dict:
        data = read_json(self.home / "agents.json", {})
        return data if isinstance(data, dict) else {}

    def _save_agents(self, agents: dict) -> None:
        write_json_atomic(self.home / "agents.json", agents)

    def _identity(self, agents: dict, lane: dict) -> tuple[dict, bool]:
        agent_id = str(lane["id"])
        entry = agents.get(agent_id)
        changed = False
        if not isinstance(entry, dict):
            entry = {
                "name": hero_name(agent_id),
                "epithet": hero_epithet(agent_id, str(lane.get("class") or ""),
                                        str(lane.get("feature") or "")),
                "epithet_source": "tables",
                "class": str(lane.get("class") or "stonewright"),
                "renown": 0,
                "scored_lanes": [],
                "scored_stages": [],
            }
            agents[agent_id] = entry
            changed = True
        if lane.get("class") and entry.get("class") != lane["class"]:
            entry["class"] = str(lane["class"])
            changed = True
        return entry, changed

    def _score_lanes(self, agents: dict, lanes: list[dict], events: list[dict]) -> bool:
        changed = False
        for lane in lanes:
            entry, dirty = self._identity(agents, lane)
            changed = changed or dirty
            if not lane.get("ended_at"):
                continue
            key = f"{lane['id']}@{lane.get('started_at')}"
            scored = entry.setdefault("scored_lanes", [])
            if key in scored:
                continue
            delta = RENOWN_LANE_DONE if lane.get("result") == "done" else RENOWN_LANE_FAILED
            entry["renown"] = apply_renown(int(entry.get("renown") or 0), delta)
            scored.append(key)
            changed = True
        for event in events:
            agent_id = event.get("agent")
            if not agent_id or event.get("event") != "end" or event.get("status") != "passed":
                continue
            entry = agents.get(str(agent_id))
            if not isinstance(entry, dict):
                continue
            key = f"{event.get('run')}:{event.get('stage')}"
            scored = entry.setdefault("scored_stages", [])
            if key in scored:
                continue
            entry["renown"] = apply_renown(int(entry.get("renown") or 0), RENOWN_STAGE_PASSED)
            scored.append(key)
            changed = True
        return changed

    # -- brain plumbing ------------------------------------------------------

    def _summary_for(self, lane: dict, status: str, log_tail: list[str],
                     now: datetime, identity: dict) -> str:
        log_text = "\n".join(tail_lines(lane.get("log"), SUMMARY_LOG_LINES))
        if self.worker.enabled and log_text.strip():
            cached = self.cache.get("summary", log_text)
            if cached and cached.get("summary"):
                summary = str(cached["summary"])
                self._maybe_epithet(lane, summary, identity)
                return summary
            self.worker.submit("summary", log_text,
                               lambda c, t=log_text: {"summary": brain_mod.job_summary(c, t)})
        return fallback_summary(status, log_tail, str(lane.get("feature") or ""), now, lane)

    def _maybe_epithet(self, lane: dict, summary: str, identity: dict) -> None:
        if identity.get("epithet_source") == "brain" or not self.worker.enabled:
            return
        cached = self.cache.get("epithet", summary)
        if cached and cached.get("epithet"):
            identity["epithet"] = str(cached["epithet"])
            identity["epithet_source"] = "brain"
            self._agents_dirty = True
            return
        hero_class = str(identity.get("class") or lane.get("class") or "")
        self.worker.submit(
            "epithet", summary,
            lambda c, s=summary, h=hero_class: {"epithet": brain_mod.job_epithet(c, s, h)})

    def _brain_waits(self, lane: dict) -> list[dict]:
        """What a working agent is waiting on; ended lanes report through their own waits."""
        if not self.worker.enabled or lane.get("ended_at"):
            return []
        bits = tail_lines(lane.get("log"), SUMMARY_LOG_LINES)
        if lane.get("report"):
            bits.append(str(lane["report"]))
        log_text = "\n".join(bits)
        if not log_text.strip():
            return []
        cached = self.cache.get("waiting", log_text)
        if cached is None:
            self.worker.submit("waiting", log_text,
                               lambda c, t=log_text: {"items": brain_mod.job_waiting(c, t)})
            self._brain_pending.add(str(lane.get("id")))
            return []
        since = cached.get("created_at") or lane.get("started_at")
        waits = []
        for index, item in enumerate(cached.get("items") or []):
            # a guess about a working agent: it goes to the manager's queue, never to the owner
            waits.append({
                "id": f"brain-{brain_mod.cache_key('waiting', log_text)[:8]}-{index}",
                "project": str(lane.get("project") or ""), "lane": str(lane.get("id")),
                "text": str(item), "since": since, "link": None, "source": "brain",
            })
        return waits

    def architecture_for(self, proj: ProjectConfig) -> dict:
        """Stored brain/manual map, else config map, else empty/none.

        With the brain on, a missing map is generated in the background,
        and a stored brain map is refreshed when the file tree has changed
        a lot (file count moved by more than 20%); the old map stays on
        screen until the new one lands in the cache.
        """
        target = self.home / "brain" / f"architecture-{proj.id}.json"
        stored = read_json(target, None)
        if self.worker.enabled and proj.repo is not None:
            tree_text, count = collect_tree(proj.repo)
            previous = stored.get("_file_count") if isinstance(stored, dict) else None
            big_change = previous is None or (
                isinstance(previous, int) and previous >= 0
                and abs(count - previous) > max(2, previous * 0.2))
            if tree_text.strip() and big_change:
                cached = self.cache.get("architecture", tree_text)
                if cached and cached.get("nodes") is not None:
                    arch = {"nodes": cached.get("nodes") or [],
                            "edges": cached.get("edges") or [],
                            "generated_at": cached.get("created_at"),
                            "source": "muse", "_file_count": count}
                    write_json_atomic(target, arch)
                    return normalize_architecture(arch, "muse")
                self.worker.submit(
                    "architecture", tree_text,
                    lambda c, t=tree_text: brain_mod.job_architecture(c, t))
        if isinstance(stored, dict) and (stored.get("nodes") or stored.get("source")):
            return normalize_architecture(stored, "manual")
        if proj.architecture:
            return normalize_architecture(proj.architecture, "manual")
        return {"nodes": [], "edges": [], "generated_at": None, "source": "none"}

    # -- pipelines -----------------------------------------------------------

    def _pipelines(self, events: list[dict], resolve=None, entries=()) -> list[dict]:
        declarations = read_json(self.home / "pipelines.json", {})
        if not isinstance(declarations, dict):
            declarations = {}
        declared: dict[tuple[str, str], list[str]] = {}
        for key, value in declarations.items():
            if isinstance(value, dict) and isinstance(value.get("stages"), list):
                project, _, pipeline = str(key).partition("/")
                declared[(project, pipeline)] = [str(s) for s in value["stages"]]
        manifests = [(e.id, manifest_mod.load(e.folder)) for e in entries if e.folder]
        for pid, loaded in manifests:
            for pipe in loaded.pipelines:
                declared.setdefault((pid, pipe.id), [s.name for s in pipe.stages])
        runs: dict[str, dict] = {}
        for event in events:
            run_id = str(event.get("run") or "")
            if not run_id:
                continue
            run = runs.setdefault(run_id, {
                "run": run_id, "project": str(event.get("project") or ""),
                "pipeline": str(event.get("pipeline") or ""), "_stages": {}, "_order": [],
                "session": None, "_cwd": None, "lane": None, "worktree": None, "branch": None,
            })
            for tag in ("lane", "worktree", "branch"):
                if not run[tag] and event.get(tag):
                    run[tag] = str(event[tag])
            if not run["session"] and event.get("session"):
                run["session"] = str(event["session"])
            if not run["_cwd"] and event.get("cwd"):
                run["_cwd"] = str(event["cwd"])
            stage_name = str(event.get("stage") or "")
            if not stage_name:
                continue
            stage = run["_stages"].get(stage_name)
            if stage is None:
                stage = {"name": stage_name, "status": "pending", "started_at": None,
                         "ended_at": None, "duration_s": None, "log_tail": [], "_log": None}
                run["_stages"][stage_name] = stage
                run["_order"].append(stage_name)
            if event.get("log"):
                stage["_log"] = event["log"]
            if event.get("event") == "start":
                stage["status"] = "running"
                stage["started_at"] = event.get("at")
                stage["_pid"] = event.get("pid") if isinstance(event.get("pid"), int) else None
                stage["_timeout"] = event.get("timeout") if isinstance(event.get("timeout"), (int, float)) else None
            elif event.get("event") == "end":
                stage["status"] = str(event.get("status") or "failed")
                stage["ended_at"] = event.get("at")
                start, end = parse_iso(stage["started_at"]), parse_iso(stage["ended_at"])
                if start and end:
                    stage["duration_s"] = max(0, int((end - start).total_seconds()))
        for pid, loaded in manifests:
            for pipe in loaded.pipelines:
                if not any(r["project"] == pid and r["pipeline"] == pipe.id for r in runs.values()):
                    runs[f"{pid}/{pipe.id}#0"] = {
                        "run": f"{pipe.id}#0", "project": pid, "pipeline": pipe.id,
                        "_stages": {}, "_order": [], "session": None, "_cwd": None}
        output: list[dict] = []
        for run in runs.values():
            order = declared.get((run["project"], run["pipeline"])) \
                or declared.get(("*", run["pipeline"])) \
                or (list(manifest_mod.GIT_EVENTS) if run["pipeline"] == manifest_mod.GIT_HOOKS_PIPELINE else [])
            names = list(order) + [n for n in run["_order"] if n not in order]
            stages: list[dict] = []
            failed_seen = False
            independent = run["pipeline"] == manifest_mod.GIT_HOOKS_PIPELINE  # hooks do not follow each other
            for name in names:
                stage = run["_stages"].get(name)
                if stage is None:
                    stage = {"name": name, "status": "skipped" if failed_seen and not independent else "pending",
                             "started_at": None, "ended_at": None, "duration_s": None,
                             "log_tail": [], "_log": None}
                elif stage["status"] == "failed":
                    failed_seen = True
                stages.append(stage)
            now = datetime.now(timezone.utc)
            for stage in stages:
                if stage["status"] == "running" and not _stage_alive(stage, now):
                    stage["status"], stage["reason"] = "failed", "interrupted"
                stage["log_tail"] = tail_lines(stage.pop("_log"), LOG_TAIL_LINES)
                stage.pop("_pid", None)
                stage.pop("_timeout", None)
            if run["pipeline"] != manifest_mod.GIT_HOOKS_PIPELINE and any(s["started_at"] for s in stages) \
                    and not any(s["status"] == "running" for s in stages):
                for stage in stages:  # a finished run: what never started was skipped, not pending
                    if stage["status"] == "pending":
                        stage["status"] = "skipped"
            starts = [s["started_at"] for s in stages if s["started_at"]]
            ends = [s["ended_at"] for s in stages if s["ended_at"]]
            ran = [s for s in stages if s["status"] not in ("pending", "skipped")]
            if any(s["status"] == "failed" for s in stages):
                status = "failed"
            elif any(s["status"] == "running" for s in stages):
                status = "running"
            elif ran and all(s["status"] == "passed" for s in ran):
                status = "passed"
            else:
                status = "pending"
            project = resolve(run["project"], run["_cwd"]) if resolve else run["project"]
            output.append({
                "run": run["run"], "project": project, "pipeline": run["pipeline"],
                "session": run["session"], "lane": run.get("lane"), "worktree": run.get("worktree"),
                "branch": run.get("branch"), "status": status,
                "started_at": min(starts) if starts else None,
                "ended_at": (max(ends) if ends else None) if status != "running" else None,
                "stages": stages,
            })
        output.sort(key=lambda r: str(r["started_at"] or ""), reverse=True)
        return output[:MAX_PIPELINE_RUNS]

    def _git_hook_events(self, resolve) -> list[dict]:
        """Git hook runs (``kind: "git-hook"``) as pipeline events of the project's ``git-hooks`` run."""
        book = read_json(self.home / "pipelines.json", {})
        book = book if isinstance(book, dict) else {}
        out: list[dict] = []
        for row in hooks_mod.read_hook_rows(self.home / "events.jsonl", kind="git-hook"):
            hook, repo = str(row.get("hook") or ""), str(row.get("repo") or "")
            if hook not in manifest_mod.GIT_EVENTS or not row.get("at"):
                continue
            project = resolve(None, repo) or Path(repo).name
            entry = book.get(f"{project}/{manifest_mod.GIT_HOOKS_PIPELINE}")
            run = str(entry["current_run"]) if isinstance(entry, dict) and entry.get("current_run") \
                else f"{manifest_mod.GIT_HOOKS_PIPELINE}#1"
            base = {"at": row["at"], "run": run, "project": project, "cwd": repo, "stage": hook,
                    "pipeline": manifest_mod.GIT_HOOKS_PIPELINE, "reason": str(row.get("reason") or "")}
            status = "failed" if row.get("result") == "blocked" else "passed"
            out.append({**base, "event": "start", "status": "running"})
            out.append({**base, "event": "end", "status": status})
        return out

    def _flow_git_hooks(self, git_events: list[dict]) -> None:
        """One flow guard -> pipeline run per git hook run, the first time it is seen."""
        keyed = [(f"{e['at']}|{e['stage']}|{e['cwd']}", e) for e in git_events if e["event"] == "end"]
        seen = flows_mod.seen_set(self.home, "git_hooks")
        fresh = [(k, e) for k, e in keyed if k not in seen]
        if not seen:  # first sight of an old history: only the latest runs become flows
            fresh = fresh[-GIT_HOOK_FLOWS_FIRST:]
        for _, event in fresh:
            blocked = event["status"] == "failed"
            text = f"{event['stage']} {'blocked' if blocked else 'allowed'}" \
                + (f": {event['reason']}" if blocked and event["reason"] else "")
            flows_mod.append_flow(
                self.home, "guard", f"pipeline:{event['run']}", "alert" if blocked else "message",
                text, session=None, project=event["project"])
        if keyed and (fresh or not seen):
            flows_mod.remember(self.home, "git_hooks", seen | {k for k, _ in keyed})

    def _flow_pipeline_ends(self, pipelines: list[dict]) -> None:
        """One ``report`` flow per pipeline run the first time it is seen finished."""
        seen = flows_mod.seen_set(self.home, "pipeline_runs")
        fresh = {p["run"] for p in pipelines if p["status"] in ("passed", "failed")} - seen
        for run in pipelines:
            if run["run"] in fresh:
                stages = ", ".join(f"{s['name']} {s['status']}" for s in run["stages"]
                                   if s["status"] in ("passed", "failed"))
                flows_mod.append_flow(
                    self.home, f"pipeline:{run['run']}", "manager", "report",
                    f"{run['pipeline']} {run['status']}" + (f": {stages}" if stages else ""),
                    session=run.get("session"), project=run.get("project"))
        if fresh:
            flows_mod.remember(self.home, "pipeline_runs", seen | fresh)

    def _flow_decisions(self, lanes: list[dict]) -> None:
        """One ``decision`` flow per ``Decision:`` line of a lane report, the first time it is seen."""
        seen = flows_mod.seen_set(self.home, "decisions")
        fresh: set[str] = set()
        known = {(f.get("from"), f.get("text")) for f in read_jsonl(self.home / "flows.jsonl")
                 if f.get("kind") == "decision"}
        for lane in lanes:
            for text, why in flows_mod.extract_decisions(lane.get("report")):
                key = f"{lane['id']}:{hashlib.sha1(text.encode('utf-8')).hexdigest()[:12]}"
                if key in seen or key in fresh:
                    continue
                fresh.add(key)
                if (f"agent:{lane['id']}", text) in known:  # already recorded with `tirith decide`
                    continue
                flows_mod.append_flow(
                    self.home, f"agent:{lane['id']}", "manager", "decision", text, why=why,
                    at=lane.get("ended_at"), session=lane.get("session") or None,
                    project=str(lane.get("project") or "") or None)
        if fresh:
            flows_mod.remember(self.home, "decisions", seen | fresh)

    # -- guard + waits ---------------------------------------------------------

    def _guard_state(self) -> dict:
        data = read_json(self.home / "guard-state.json", {})
        return data if isinstance(data, dict) else {}

    def _manual_waits(self, guard_ok: dict[str, bool] | None = None) -> tuple[list[dict], list[dict]]:
        """``(open, answered)`` asks from ``waits.jsonl``; an owner reply in the inbox answers one.

        A reminder to install the commit guard disappears once the project's guard is intact."""
        by_id: dict[str, dict] = {}
        for row in read_jsonl(self.home / "waits.jsonl"):
            wait_id = str(row.get("id") or "")
            if wait_id:
                by_id[wait_id] = row
        replied: dict[str, dict] = {}
        for reply in inbox_mod.replies(self.home):
            replied.setdefault(reply["wait"], reply)  # the first reply is the answer
        waits, answered = [], []
        for row in by_id.values():
            options = asks_mod.clean_options(row.get("options"))
            command = row.get("command") or None
            text = str(row.get("text") or "")
            wait = {
                "id": str(row["id"]), "project": row.get("project"),
                "text": text, "since": row.get("at"),
                "link": row.get("link"), "source": "manual",
                **({"options": row["options"]} if row.get("options") else {}),
                "session": row.get("session") or None,
                "kind": row.get("kind") if row.get("kind") in asks_mod.ASK_KINDS
                else asks_mod.default_kind(options, command),
                "options": options, "command": command, "status": "open",
                "actions": actions_mod.dedupe_actions(
                    [dict(a) for a in row.get("actions") or [] if isinstance(a, dict)]),
                **({"why_owner": row["why_owner"], "why_detail": row.get("why_detail"),
                    "reason_tag": row.get("reason_tag") or asks_mod.reason_tag(row["why_owner"], row.get("why_detail"))}
                   if asks_mod.why_key(row.get("why_owner")) else {}),
                **({"howto": row["howto"]} if isinstance(row.get("howto"), dict) else {}),
            }
            reply = replied.get(wait["id"])
            actions_mod.attach_last_runs(self.home, [wait])
            if reply is not None:
                label = next((o["label"] for o in options if o["value"] == reply["value"]), reply["value"])
                answered.append({**wait, "status": "answered", "answer": {
                    "at": reply["at"], "value": reply["value"], "label": label, "text": reply["text"]}})
            elif row.get("done_at"):
                if row.get("done_reason") == actions_mod.PASSED_REASON:
                    answered.append(self._passed_wait(wait, str(row["done_at"])))
                elif row.get("done_reason") == howto_mod.DONE_REASON:
                    answered.append({**wait, "status": "answered", "answer": {
                        "at": row["done_at"], "value": howto_mod.DONE_REASON, "label": "✓ Done: the check passed",
                        "text": ""}})
                continue
            elif actions_mod.all_passed(wait["actions"]):
                # every Run action passed: the item closes by itself (a rule in code, not a habit)
                done_at = max(str(a["last_run"]["at"] or "") for a in wait["actions"])
                append_jsonl(self.home / "waits.jsonl", {
                    **row, "done_at": done_at, "done_reason": actions_mod.PASSED_REASON})
                answered.append(self._passed_wait(wait, done_at))
            elif (guard_ok or {}).get(str(row.get("project") or "")) and not options \
                    and re.search(r"commit guard", text, re.IGNORECASE):
                continue
            else:
                waits.append(wait)
        waits.sort(key=lambda w: str(w["since"] or ""))
        answered.sort(key=lambda w: str(w["answer"]["at"] or ""), reverse=True)
        return waits, answered[:ANSWERED_KEPT]

    @staticmethod
    def _passed_wait(wait: dict, at: str) -> dict:
        """An item closed because its actions passed: shown in "Answered" as ``✓ <label>``."""
        label = ", ".join(a["label"] for a in wait["actions"])
        return {**wait, "status": "answered", "answer": {
            "at": at, "value": actions_mod.PASSED_REASON, "label": "✓ " + label, "text": ""}}

    def _guard_replies(self, now: datetime) -> dict[str, tuple[str, datetime | None]]:
        """Project -> (``snooze`` | ``never``, snoozed until) from the owner's latest reply to its guard item."""
        out: dict[str, tuple[str, datetime | None]] = {}
        for reply in inbox_mod.replies(self.home):
            if not reply["wait"].startswith("guard-") or reply["value"] not in ("snooze", "never"):
                continue
            at = parse_iso(reply["at"])
            out[reply["wait"][len("guard-"):]] = (
                reply["value"], at + GUARD_SNOOZE if at is not None else None)
        return out

    def _gates_actions(self, proj_row: dict, machine) -> list[dict]:
        """Run buttons for the Setup banner: one stored action per missing install step."""
        return [actions_mod.register_generated(
                    self.home, proj_row["id"],
                    "ac-gates-" + re.sub(r"[^a-z0-9]+", "-", command.lower()).strip("-") + "-" + proj_row["id"],
                    label, command, proj_row.get("folder"))
                for label, command in machine.steps]

    def _guard_wait(self, proj: ProjectConfig, problems: list[str], since: str) -> dict:
        """The owner item for a guard that is not intact: what is wrong and the buttons that fix it.

        A repo that has its own hook gets "add the guard and keep the hook" (``install --chain``); anything
        else gets ``tirith guard install``. Both can wait 7 days or be switched off for the project."""
        chain = any(f"without the {GUARD_MARKER} marker" in p for p in problems)
        cwd = str(proj.repo) if proj.repo else None
        if chain:
            label = f"Add the word guard and keep {proj.name}'s hook"
            action = actions_mod.register_generated(
                self.home, proj.id, f"ac-guard-chain-{proj.id}", label,
                f"tirith guard install {proj.id} --chain", cwd)
        else:
            action = actions_mod.register_generated(
                self.home, proj.id, f"ac-guard-install-{proj.id}", f"Install the word guard in {proj.name}",
                f"tirith guard install {proj.id}", cwd)
        return {
            "id": f"guard-{proj.id}", "project": proj.id,
            "text": f"Commit guard needs attention in {proj.name}: {problems[0]}",
            "since": since, "link": None, "source": "guard", "kind": "action",
            "why_owner": "owner-decision", "why_detail": "choice: install the guard, not now or never",
            "reason_tag": asks_mod.reason_tag("owner-decision", "install the guard, not now or never"),
            "options": [{"label": "Not now", "value": "snooze"},
                        {"label": "Never for this project", "value": "never"}],
            "actions": [action],
        }

    def _settle_generated(self, waits: list[dict]) -> tuple[list[dict], list[dict]]:
        """``(open, answered)`` for the items Tirith raises itself: an owner reply answers one; "let the
        manager handle it" does not exist."""
        replied: dict[str, dict] = {}
        for reply in inbox_mod.replies(self.home):
            replied.setdefault(reply["wait"], reply)
        open_items, answered = [], []
        for wait in waits:
            reply = replied.get(wait["id"])
            if reply is None:
                open_items.append(wait)
                continue
            label = next((o["label"] for o in wait.get("options") or [] if o["value"] == reply["value"]),
                         reply["value"])
            answered.append({**wait, "status": "answered", "answer": {
                "at": reply["at"], "value": reply["value"], "label": label, "text": reply["text"]}})
        return open_items, answered

    def _enforce_cta(self, waiting: list[dict]) -> list[dict]:
        """The rule: nothing reaches the owner unless ``asks.is_owner_item`` (a one-click option or a Run
        action, and an allow-listed source). Any other item goes to the manager's queue (once)."""
        shown, seen = [], {}
        for wait in waiting:
            if asks_mod.is_owner_item(wait):
                key = (str(wait.get("project") or ""), " ".join(str(wait.get("text") or "").split()).lower())
                if key in seen:  # the same text for the same project is one item
                    first = seen[key]
                    first["actions"] = actions_mod.dedupe_actions(
                        (first.get("actions") or []) + (wait.get("actions") or []))
                    continue
                seen[key] = wait
                wait["actions"] = actions_mod.dedupe_actions(wait.get("actions") or [])
                shown.append(wait)
            else:
                kind = asks_mod.NO_CTA_KIND if not asks_mod.has_call_to_action(wait) else asks_mod.NOT_OWNER_KIND
                asks_mod.queue_add(self.home, str(wait.get("project") or ""), str(wait.get("lane") or ""),
                                   wait["text"], kind=kind, item_id="sq-cta-" + re.sub(r"[^\w.-]", "-", wait["id"]))
        return shown

    @staticmethod
    def _violation(row: dict) -> dict:
        message = str(row.get("message") or "")
        blocked, _, instead = message.partition(". Do this instead: ")
        prefix = f"tirith: {row.get('rule') or ''}: "
        return {"ts": row.get("ts"), "rule": str(row.get("rule") or ""), "agent": row.get("agent"),
                "lane": row.get("lane"), "project": row.get("project"), "message": message,
                "blocked": blocked[len(prefix):] if blocked.startswith(prefix) else blocked,
                "instead": instead}

    @staticmethod
    def _rules_state(projects: list[dict], machine, rows: list[dict], now: datetime) -> list[dict]:
        """Every active rule once (custom rules once per project) with what it blocked in 24 hours."""
        found: dict[tuple[str, str | None], dict] = {}
        base = setup_mod._rules(manifest_mod.Manifest(path=Path(manifest_mod.FILENAME)), machine, False)
        for rule in base:
            found[(rule["id"], None)] = dict(rule)
        for proj in projects:
            for rule in (proj.get("setup") or {}).get("rules") or []:
                if rule["source"] == "built-in":
                    found[(rule["id"], None)]["active"] = found[(rule["id"], None)]["active"] or rule["active"]
                else:
                    found[(rule["id"], proj["id"])] = dict(rule, project=proj["id"])
        out = []
        for (rid, pid), rule in found.items():
            if not rule["active"]:
                continue
            hits = [r for r in rows if r.get("rule") == rid and (pid is None or r.get("project") == pid)
                    and (parse_iso(r.get("ts")) or now - timedelta(days=9)) >= now - timedelta(hours=24)]
            last = max(hits, key=lambda r: str(r.get("ts")), default=None)
            out.append({"id": rid, "description": rule["description"], "source": rule["source"],
                        "project": pid, "count_24h": len(hits), "last_at": last.get("ts") if last else None,
                        "last_agent": (last.get("lane") or last.get("agent")) if last else None})
        return out

    @staticmethod
    def _violation_waits(violations: list[dict], now: datetime) -> list[dict]:
        """One item per violation of the last hour, so the owner sees what slipped."""
        waits = []
        for index, row in enumerate(violations):
            when = parse_iso(row.get("ts"))
            if when is None or now - when > timedelta(hours=1):
                continue
            who = row.get("lane") or row.get("agent") or "an agent"
            waits.append({"id": f"violation-{row['ts']}-{index}", "project": row.get("project"),
                          "text": f"Rule {row['rule']} blocked {who}: {row['message']}",
                          "since": row["ts"], "link": None, "source": "violation",
                          "options": [dict(o) for o in asks_mod.GOT_IT_OPTIONS]})
        return waits

    # -- main cycle ------------------------------------------------------------

    def build_state(self, now: datetime | None = None) -> dict:
        """Build (but do not write) one ``state.json`` payload."""
        now = (now or datetime.now(timezone.utc)).astimezone(timezone.utc)
        if self._fixed_config is None:
            self.config = load_config(self.home)
        if not self._owner_items_checked:  # once at startup: every open owner item passes the gate again
            self._owner_items_checked = True
            ownergate_mod.migrate(self.home, now_iso(now))
        self._tick(None)
        howto_mod.check_open(self.home, now)  # an owner item whose done_check passes closes by itself
        lanes = read_lanes(self.home)
        close_dead_lanes(self.home, lanes, now)
        events = [e for e in read_jsonl(self.home / "events.jsonl") if e.get("kind") not in ("hook", "violation", "git-hook")]
        hook_rows = hooks_mod.read_hook_rows(self.home / "events.jsonl")
        _, loose_events = hooks_mod.attach(hook_rows, [str(l["id"]) for l in lanes])
        rows_by_lane: dict[str, list[dict]] = {}
        for row in hook_rows:
            rows_by_lane.setdefault(str(row.get("lane")), []).append(row)
        # Projects are folders: derive auto projects for folders no project covers yet.
        room_folders = [r.project for r in self.config.rooms if _looks_like_path(r.project)]
        folders = ([lane.get("workspace") for lane in lanes]
                   + [event.get("cwd") for event in events] + room_folders)
        entries = projects_mod.discover(self.home, self.config, [f for f in folders if f])
        entry_by_id = {e.id: e for e in entries}

        def resolve(project_id, folder) -> str:
            """Explicit known project id wins; else the longest-prefix folder match."""
            pid = str(project_id or "")
            if pid in entry_by_id:
                return pid
            found = projects_mod.match(folder, entries)
            return found.id if found else pid

        config = self.config
        if room_folders:
            config = dataclasses.replace(config, rooms=[
                dataclasses.replace(r, project=resolve(None, r.project))
                if _looks_like_path(r.project) else r for r in config.rooms])
        agents = self._load_agents()
        self._agents_dirty = False
        self._brain_pending = set()
        scored =self._score_lanes(agents, lanes, events)

        cutoff = now - DROP_ENDED_AFTER
        agent_rows: list[dict] = []
        brain_waits: list[dict] = []
        manager_turns = (dispatch_mod.load_state(self.home).get("manager") or {}).get("turns") or {}
        self._live_steps = {}
        self.activity.prefetch([l for l in lanes if not (parse_iso(l.get("ended_at")) or now) < cutoff])
        for lane in lanes:
            ended = parse_iso(lane.get("ended_at"))
            if ended and ended < cutoff:
                continue
            identity = agents[str(lane["id"])]
            lane_project = resolve(lane.get("project"), lane.get("workspace"))
            status = lane_status(lane, now)
            log_tail = tail_lines(lane.get("log"), LOG_TAIL_LINES)
            started = parse_iso(lane.get("started_at"))
            end_ref = ended or now
            elapsed = max(0, int((end_ref - started).total_seconds())) if started else 0
            act = self.activity.lane(lane, status, rows_by_lane.get(str(lane["id"]), []), now)
            files, insertions, deletions = act.pop("totals")
            log_view = log_entries(lane.get("log"), lane.get("workspace")) \
                or entries_from_events(act["events"], lane.get("workspace"))
            renown = int(identity.get("renown") or 0)
            usage = self._usage(lane, ended is not None)
            live_steps, live_sum = livesteps_mod.steps_for(
                lane.get("log"), lane.get("workspace"), lane.get("ended_at") or lane.get("started_at"), now)
            self._live_steps[str(lane["id"])] = live_steps
            # Summary first: it may upgrade the epithet on the identity.
            spoken = say_summary(log_view, None if status in ("working", "stalled") else lane.get("report"))
            summary = spoken or self._summary_for(lane, status, log_tail, now, identity)
            agent_rows.append({
                "id": str(lane["id"]), "project": lane_project,
                "session": lane.get("session") or None,
                "name": str(identity.get("name") or hero_name(str(lane["id"]))),
                "epithet": str(identity.get("epithet") or ""),
                "class": str(identity.get("class") or lane.get("class") or ""),
                "rank": rank_for(renown), "renown": renown,
                "feature": str(lane.get("feature") or ""), "status": status,
                "started_at": lane.get("started_at"), "ended_at": lane.get("ended_at"),
                "elapsed_s": elapsed,
                "summary": summary,
                "log_tail": log_tail,
                "log_entries": log_view,
                "files_changed": files, "insertions": insertions, "deletions": deletions,
                "report": _bounded_report(lane.get("report")),
                "usage": usage,
                "ran": models_mod.ran_with(lane, usage),
                "agent": lane.get("agent") or None,
                "fallback_from": lane.get("fallback_from") or None,
                "keep_going": keepgoing_mod.round_label(lane),
                "diff": self._diff(lane),
                "request": lane.get("request") if isinstance(lane.get("request"), dict) else None,
                "parent": "manager" if lane.get("parent") in (None, "", "steward") else str(lane["parent"]),
                "step": str(lane["step"]) if lane.get("step") else None,
                "item": str(lane["item"]) if lane.get("item") else None,
                "outcome": (lane.get("outcome") or {}).get("status") or None,
                "request_id": str(lane["request_id"]) if lane.get("request_id") else None,
                "turn_requests": list(manager_turns.get(str(lane["id"])) or []),
                "live": {**live_sum, "elapsed_s": elapsed} if live_steps else None,
                "review": review_mod.public(lane, (entry_by_id[lane_project].folder if lane_project in entry_by_id else None)),
                **act,
            })
            for wait in self._brain_waits({**lane, "project": lane_project}):
                brain_waits.append({**wait, "session": lane.get("session") or None})
        by_id = {row["id"]: row for row in agent_rows}
        for row in agent_rows:  # children inherit the request of the nearest ancestor that has one
            parent, seen = row, set()
            while parent and not parent.get("request") and parent["id"] not in seen:
                seen.add(parent["id"])
                parent = by_id.get(parent.get("parent"))
            if parent and parent is not row:
                row["request"] = parent["request"]
        if scored or self._agents_dirty:
            self._save_agents(agents)

        words_default = default_words()
        guard_since = self._guard_state()
        guard_dirty = False
        projects: list[dict] = []
        guard_waits: list[dict] = []
        known_ids = set()
        guard_replies = self._guard_replies(now)
        for proj in self.config.projects:
            known_ids.add(proj.id)
            if guard_replies.get(proj.id, ("", None))[0] == "never" and proj.guard:
                projects_mod.set_project_guard(self.home, proj.id, False)
                proj = dataclasses.replace(proj, guard=False)
            problems = [] if not proj.guard else verify_guard(
                proj.repo, self.config.words_for(proj, words_default)) \
                if proj.repo is not None else ["no repo path configured"]
            guard_ok = not problems
            if guard_ok:
                if guard_since.pop(proj.id, None) is not None:
                    guard_dirty = True
            else:
                if proj.id not in guard_since:
                    guard_since[proj.id] = now_iso(now)
                    guard_dirty = True
                    flows_mod.append_flow(
                        self.home, "guard", "owner", "alert",
                        f"Commit guard needs attention in {proj.name}: {problems[0]}",
                        session=None, project=proj.id)
                snoozed = guard_replies.get(proj.id)
                if not (snoozed and snoozed[0] == "snooze" and snoozed[1] and now < snoozed[1]):
                    guard_waits.append(self._guard_wait(proj, problems, guard_since[proj.id]))
            folder = entry_by_id[proj.id].folder if proj.id in entry_by_id else None
            projects.append({
                "id": proj.id, "name": proj.name, "color": proj.color,
                "folder": str(folder) if folder else None, "auto": False,
                "architecture": self.architecture_for(proj),
                "guard": {"ok": guard_ok, "problems": problems, **({"disabled": True} if not proj.guard else {})},
                "roadmap": roadmap_mod.build(folder, proj.id, agent_rows, now.astimezone().date()),
                "plans": roadmap_mod.build_plans(folder, proj.id, agent_rows),
            })
        for entry in entries:
            if entry.id in known_ids:
                continue
            known_ids.add(entry.id)
            projects.append({
                "id": entry.id, "name": entry.name, "color": entry.color,
                "folder": str(entry.folder) if entry.folder else None, "auto": True,
                "architecture": self.architecture_for(ProjectConfig(id=entry.id, name=entry.name)),
                "guard": {"ok": False, "problems": ["auto-discovered folder: no commit guard configured"]},
                "roadmap": roadmap_mod.build(entry.folder, entry.id, agent_rows, now.astimezone().date()),
                "plans": roadmap_mod.build_plans(entry.folder, entry.id, agent_rows),
            })
        self._tick("lanes")
        git_runs = self._git_hook_events(resolve)
        local_runs = self._pipelines(sorted(events + git_runs, key=lambda e: str(e.get("at") or ""))
                                     if git_runs else events, resolve, entries)
        self._flow_git_hooks(git_runs)
        remote_runs, remote_agents = self.github.poll(self.config, entries)
        runs = sorted(local_runs + remote_runs, key=lambda r: str(r["started_at"] or ""),
                      reverse=True)
        agent_rows += remote_agents
        self._tick("pipelines")
        for pid in [row["project"] for row in agent_rows] + [r["project"] for r in runs]:
            if pid and pid not in known_ids:
                known_ids.add(pid)
                stub = ProjectConfig(id=pid, name=pid)
                projects.append({
                    "id": pid, "name": pid, "color": "#888888", "folder": None, "auto": False,
                    "architecture": self.architecture_for(stub),
                    "guard": {"ok": False, "problems": ["project not defined in config.toml"]},
                    "roadmap": None, "plans": [],
                })
        if guard_dirty:
            write_json_atomic(self.home / "guard-state.json", guard_since)
        machine = setup_mod.machine_state(self.config, protection=hooks_mod.protect(self.home, self.config, now),
                                          home=self.home)
        setup_requests = [r for r in inbox_mod.requests(self.home) if r["kind"] == "setup"]
        for proj_row in projects:
            proj_row["env"] = projenv_mod.load(self.home, proj_row["id"])  # shown in full on Settings
            guard = proj_row.get("guard") or {}
            proj_row["setup"] = setup_mod.build(
                Path(proj_row["folder"]) if proj_row.get("folder") else None, proj_row["id"], machine, runs,
                setup_requests, bool(guard.get("ok")), list(guard.get("problems") or []), [],
                self._gates_actions(proj_row, machine))

        orchestrator = read_json(self.home / "orchestrator.json", {})
        if not isinstance(orchestrator, dict):
            orchestrator = {}
        raw_notes = [{"at": n.get("at"), "project": resolve(n.get("project"), None) or n.get("project"),
                      "text": str(n.get("text") or ""), "session": n.get("session") or None}
                     for n in read_jsonl(self.home / "notes.jsonl")]
        notes = raw_notes[-20:]
        self._flow_pipeline_ends(local_runs)
        events_pipelines = runs
        self._flow_decisions(lanes)
        self.rooms.ensure_polling(config, background=not self._one_shot)
        rooms_state, room_waits = self.rooms.build(
            config, agent_rows, self.worker, self.cache, now)
        violation_rows = hooks_mod.read_violation_rows(self.home / "events.jsonl")
        violations = [self._violation(row) for row in violation_rows][-hooks_mod.VIOLATIONS_KEPT:]
        rules_state = self._rules_state(projects, machine, violation_rows, now)
        manual_waits, answered_waits = self._manual_waits(
            {p["id"]: p["guard"]["ok"] for p in projects if p.get("guard")})
        # violations are not the owner's turn (the blocked agent was already told what to do instead);
        # they are shown in the Rules section only
        asks_mod.sync_agent_waiting(self.home, brain_waits, self._brain_pending)  # manager only
        generated, generated_answered = self._settle_generated(room_waits)
        answered_waits = sorted(answered_waits + generated_answered,
                                key=lambda w: str(w["answer"]["at"] or ""), reverse=True)[:ANSWERED_KEPT]
        waiting = self._enforce_cta(manual_waits + generated + guard_waits)
        for wait in waiting + answered_waits:
            wait.setdefault("session", None)
            if wait.get("project"):
                wait["project"] = resolve(wait["project"], None)
        endpoint_projects: dict[str, str] = {}
        for row in agent_rows:
            endpoint_projects[f"agent:{row['id']}"] = row["project"]
        for run in runs:
            endpoint_projects[f"pipeline:{run['run']}"] = run["project"]
        for room in rooms_state:
            if room.get("project"):
                endpoint_projects[f"room:{room['id']}"] = str(room["project"])
        all_flows = flows_mod.flows_state(
            self.home, {a["id"]: f"{a['name']} {a['epithet']}".strip() for a in agent_rows},
            projects=endpoint_projects, limit=None)
        self._fill_counts(projects, agent_rows, runs, all_flows, now)
        self._tick("flows")
        inbox_rows = self._inbox(projects, agent_rows, all_flows, now, lanes)
        self._tick("inbox")
        sessions = self._sessions(lanes, events, all_flows, raw_notes, waiting, resolve)
        for sess in sessions:
            sess["usage"] = agentrun_mod.sum_usage(
                [a.get("usage") for a in agent_rows if a.get("session") == sess["id"]])
        for proj in projects:
            proj["usage"] = agentrun_mod.sum_usage(
                [a.get("usage") for a in agent_rows if a.get("project") == proj["id"]])
        manager = {
            "name": self.config.manager_name, "title": self.config.manager_title,
            "activity": orchestrator.get("activity"),
            "since": orchestrator.get("since"), "notes": notes,
            "presence": managerd_mod.public(self.home, dispatch_mod.load_state(self.home), lanes, now),
            "live": self._manager_live(),
            "turns": self._manager_turns(agent_rows, lanes, now),
        }
        manager_queue = asks_mod.queue_items(self.home, only_open=True)[-asks_mod.QUEUE_STATE_ROWS:]
        says = inbox_mod.says(self.home)
        return {
            "version": 1,
            "generated_at": now_iso(now),
            "collector": {"warming": self.activity.warming, "pending": self.activity.pending()},
            "brain": {"enabled": self.worker.enabled,
                      "model": self.config.brain_model,
                      "last_error": self.worker.last_error},
            "manager": manager,
            "steward": manager,  # old key, same value
            "waiting_on_you": waiting,
            "answered_waits": answered_waits,
            "manager_queue": manager_queue,
            "steward_queue": manager_queue,  # old key, same value
            "projects": projects,
            "agents": agent_rows,
            "pipelines": events_pipelines,
            "rooms": rooms_state,
            "flows": all_flows[-flows_mod.STATE_FLOWS:],
            "sessions": sessions,
            "loose_events": loose_events,
            "violations": violations,
            "rules": rules_state,
            "inbox": inbox_rows,
            "says": says,
            "file_refs": filerefs_mod.build(
                filerefs_mod.chat_texts(inbox_rows, says, waiting + answered_waits), projects),
            "disk": dispatch_mod.load_state(self.home).get("disk"),
            "health": selfcheck_mod.public(dispatch_mod.load_state(self.home).get("health")),
            "timeouts": timeouts_mod.table(self.home, self.config),
            "autopilot": autopilot_mod.public(self.home, self.config, agent_rows, projects, now),
            "launch": {"token": launch_mod.launch_token(self.home),
                       "available": launch_mod.handler_installed()},
            "models": models_mod.state(self.config, [
                inbox_mod.setup_summary(r["setup"]) for r in setup_requests
                if r["project"] == "settings" and r["status"] == "open" and r.get("setup")],
                fallback_mod.status(self.config, read_lanes(self.home), now)),
        }

    def _manager_turns(self, agent_rows: list[dict], lanes: list[dict], now: datetime) -> list[dict]:
        """``state.manager.turns``: one row per turn of the persistent manager session, shaped like an agent row
        so the chat draws the same live block (``turn_requests``, ``live``), plus ``lanes``: the sub-agent
        lanes the manager started during the turn."""
        dstate = dispatch_mod.load_state(self.home)
        session = (dstate.get("manager") or {}).get("session") or {}
        mine = session.get("lane")
        rows = []
        for view in managerd_mod.turn_views(self.home, dstate)[-managerd_mod.TURNS_KEPT:]:
            ended = view.get("ended_at")
            start = parse_iso(view["started_at"])
            end = parse_iso(ended) if ended else now
            kids = [a["id"] for a in agent_rows
                    if a.get("parent") not in (None, "", "manager") and a["parent"].startswith(managerd_mod.LANE_PREFIX)
                    and (start is None or (parse_iso(a["started_at"]) or now) >= start)
                    and (parse_iso(a["started_at"]) or now) <= end]
            elapsed = max(0, int((end - start).total_seconds())) if start else 0
            summary = view.get("summary") or {}
            row = {"id": view["id"], "name": "Manager", "project": session.get("project"),
                   "status": "working" if not ended else ("failed" if view.get("error") else "done"),
                   "started_at": view["started_at"], "ended_at": ended, "elapsed_s": elapsed,
                   "turn_requests": view.get("requests") or [], "lanes": kids, "parent": "manager",
                   "interrupted": bool(view.get("interrupted"))}
            self._live_steps[view["id"]] = view.get("steps") or []
            ref = self._write_steps(row)
            row["live"] = {**{k: summary.get(k) for k in ("steps", "done", "error", "cost_usd", "duration_s")},
                           "elapsed_s": elapsed, "detail": ref}
            rows.append(row)
        return rows

    def _manager_live(self) -> dict | None:
        """``{heartbeat}`` from ``manager.json`` (the live manager session's last sign of life), or None
        when the file is missing or has no readable time. The chat shows "typing" only if it moved."""
        data = read_json(self.home / "manager.json", None)
        if not isinstance(data, dict):
            return None
        for key in ("heartbeat", "at", "updated_at"):
            beat = parse_iso(data[key]) if isinstance(data.get(key), str) else None
            if beat is not None:
                return {"heartbeat": now_iso(beat)}
        return None

    def _inbox(self, projects: list[dict], agents: list[dict], flows: list[dict],
               now: datetime, lanes: list[dict] | None = None) -> list[dict]:
        """``state.inbox``; the dispatcher handles ``finish``, an unanswered question gets one brain answer
        (at once when the dispatcher is on, else after ``AUTO_ANSWER_AFTER_S``)."""
        rows = dispatch_mod.run(self.home, self.config, now, agents=agents, lanes=lanes or [],
                                status_of=lane_status, projects=projects)
        wait_s = 0 if self.config.dispatch_enabled else inbox_mod.AUTO_ANSWER_AFTER_S
        if self.worker.enabled:
            by_project = {p["id"]: p for p in projects}
            retry_after = 300.0
            for req in rows:
                if req["kind"] != "ask" or req["status"] != "open":
                    continue
                asked = parse_iso(req["at"])
                if asked is None or (now - asked).total_seconds() < wait_s:
                    continue
                cached = self.cache.get("inbox-answer", req["id"])
                if cached is not None:
                    if cached.get("text"):
                        row = inbox_mod.add_answer(self.home, req["id"], cached["text"], by="brain",
                                                   auto=True, at=now_iso(now))
                        req["answers"].append({"at": row["at"], "by": "brain", "text": row["text"], "auto": True,
                                               "lane": None})
                        req["status"] = "answered"
                        if self.config.dispatch_enabled:
                            dispatch_mod.note_brain_answer(self.home, req, row["at"])
                    continue
                last = self._inbox_tried.get(req["id"])
                if last is not None and time.monotonic() - last < retry_after:
                    continue
                context = inbox_mod.answer_prompt(req, by_project.get(req["project"]), agents, flows)
                if self.worker.submit("inbox-answer", req["id"],
                                      lambda c, t=context: {"text": brain_mod.job_inbox_answer(c, t)}):
                    self._inbox_tried[req["id"]] = time.monotonic()
        inbox_mod.link_lanes(rows, agents)
        rows = rows[-inbox_mod.STATE_ROWS:]
        actions_mod.attach_last_runs(self.home, [a for req in rows for a in req["answers"]])
        return rows

    @staticmethod
    def _fill_counts(projects: list[dict], agents: list[dict], runs: list[dict],
                     flows: list[dict], now: datetime) -> None:
        cutoff = now - timedelta(hours=24)
        for proj in projects:
            pid = proj["id"]
            mine = [a for a in agents if a["project"] == pid]
            proj["counts"] = {
                "agents_working": sum(1 for a in mine if a["status"] == "working"),
                "agents_total": len(mine),
                "pipelines_running": sum(1 for r in runs
                                         if r["project"] == pid and r["status"] == "running"),
                "flows_24h": sum(1 for f in flows if f["project"] == pid
                                 and (parse_iso(f["at"]) or cutoff) >= cutoff),
            }

    def _sessions(self, lanes: list[dict], events: list[dict], flows: list[dict],
                  notes: list[dict], waits: list[dict], resolve) -> list[dict]:
        """``state.sessions[]``: every known session with its project ids and item counts."""
        table: dict[str, dict] = {}
        for row in projects_mod.read_sessions(self.home):
            table[str(row["id"])] = {
                "id": str(row["id"]), "title": str(row.get("title") or row["id"]),
                "started_at": row.get("started_at"), "ended_at": row.get("ended_at"),
                "projects": set(), "counts": {"agents": 0, "pipelines": 0, "flows": 0, "notes": 0}}

        def touch(session, kind, project, at, run=None, seen=None):
            if not session:
                return
            entry = table.get(str(session))
            if entry is None:  # used (env var) but never started: still list it
                entry = table[str(session)] = {
                    "id": str(session), "title": str(session), "started_at": at,
                    "ended_at": None, "projects": set(),
                    "counts": {"agents": 0, "pipelines": 0, "flows": 0, "notes": 0}}
            if kind:
                if seen is not None:
                    if (session, kind, seen) in done:
                        kind = None
                    else:
                        done.add((session, kind, seen))
                if kind:
                    entry["counts"][kind] += 1
            if project:
                entry["projects"].add(project)
            if at and (not entry["started_at"] or str(at) < str(entry["started_at"])):
                entry["started_at"] = at

        done: set = set()
        for lane in lanes:
            touch(lane.get("session"), "agents",
                  resolve(lane.get("project"), lane.get("workspace")), lane.get("started_at"))
        for event in events:
            run_id = str(event.get("run") or "")
            touch(event.get("session"), "pipelines",
                  resolve(event.get("project"), event.get("cwd")), event.get("at"), seen=run_id)
        for flow in flows:
            touch(flow.get("session"), "flows", flow.get("project"), flow.get("at"))
        for note in notes:
            touch(note.get("session"), "notes", note.get("project"), note.get("at"))
        for wait in waits:
            touch(wait.get("session"), None, wait.get("project"), wait.get("since"))
        out = []
        for entry in table.values():
            entry["projects"] = sorted(entry["projects"])
            out.append(entry)
        out.sort(key=lambda e: (str(e["started_at"] or ""), e["id"]), reverse=True)
        return out

    def _write_detail(self, row: dict) -> dict:
        """Write ``agents/<id>.js`` when the agent's full detail changed; ``{path, hash}`` for the state row."""
        detail = {"id": row["id"], **{k: row.get(k) for k in DETAIL_KEYS}}
        digest = _detail_hash(detail)
        path = self.home / "agents" / agent_file_name(row["id"])
        if self._detail_hashes.get(row["id"]) != digest or not path.exists():
            write_text_atomic(path, agent_detail_js(detail))
            self._detail_hashes[row["id"]] = digest
        return {"path": str(path), "hash": digest}

    def _write_steps(self, row: dict) -> dict | None:
        """Write ``agents/<id>.steps.js`` (the live transcript) when it changed; ``{path, hash}`` or None."""
        steps = self._live_steps.get(row["id"])
        if not steps:
            return None
        sid = f"{row['id']}.steps"
        digest = _detail_hash({"id": sid, "steps": steps})
        path = self.home / "agents" / agent_file_name(sid)
        if self._detail_hashes.get(sid) != digest or not path.exists():
            write_text_atomic(path, agent_detail_js({"id": sid, "steps": steps}))
            self._detail_hashes[sid] = digest
        return {"path": str(path), "hash": digest}

    def slim_state(self, state: dict, now: datetime) -> dict:
        """``state`` as written to ``state.json``: the agents trimmed to full detail for working and
        recently ended ones, the summary fields for the rest, at most ``STATE_AGENTS_MAX`` of them.
        Writes ``agents/<id>.js`` (the full detail) for every agent whose detail changed."""
        rows = state.get("agents") or []
        cutoff = now - FULL_DETAIL_WINDOW

        def full(row: dict) -> bool:
            ended = parse_iso(row.get("ended_at"))
            return ended is None or ended >= cutoff

        order = sorted(range(len(rows)), key=lambda i: (
            0 if full(rows[i]) else 1,
            -(parse_iso(rows[i].get("ended_at")) or now).timestamp(), i))
        keep = sorted(order[:STATE_AGENTS_MAX])
        slim: dict[int, dict] = {}
        steps_ref_of: dict[int, dict | None] = {}
        for i, row in enumerate(rows):  # every agent gets its detail file, listed or not
            ref = self._write_detail(row)
            steps_ref = steps_ref_of[i] = self._write_steps(row) if full(row) else None   # old ended agents: no transcript file
            if i in keep:
                slim[i] = {**_slim_row(row, full(row)), "detail": ref}
                if steps_ref and slim[i].get("live"):
                    slim[i]["live"] = {**slim[i]["live"], "detail": steps_ref}
        sizes = {i: len(json.dumps(r, separators=(",", ":"))) for i, r in slim.items()}
        for i in order[::-1]:  # past the byte budget the oldest full ended agents become summaries
            if sum(sizes.values()) <= STATE_AGENTS_BYTES:
                break
            if i in slim and rows[i].get("ended_at") and full(rows[i]):
                slim[i] = {**_slim_row(rows[i], False), "detail": slim[i]["detail"]}
                if slim[i].get("live"):
                    slim[i]["live"] = {**slim[i]["live"], "detail": steps_ref_of.get(i)}
                sizes[i] = len(json.dumps(slim[i], separators=(",", ":")))
        out = [slim[i] for i in keep]
        return {**state, "agents": out}

    def cycle(self, now: datetime | None = None) -> dict:
        """One collector cycle: build state and write it atomically."""
        now = (now or datetime.now(timezone.utc)).astimezone(timezone.utc)
        started = time.monotonic()
        self._tick(None)
        built = self.build_state(now)
        self._tick("sessions")  # what is left of build_state after the inbox
        state = self.slim_state(built, now)
        self._tick("slim")
        write_json_atomic(self.home / "state.json", state, compact=True)
        # The same JSON as a script, so a page opened from file:// can load it without CORS.
        write_text_atomic(self.home / "state.js", state_js(state, compact=True))
        self._tick("write")
        self._cycles += 1
        for name, spent in self._phase_s.items():
            self._phase_total[name] = self._phase_total.get(name, 0.0) + spent
        self._phase_total["cycle"] = self._phase_total.get("cycle", 0.0) + time.monotonic() - started
        if self._cycles % TIMING_EVERY == 0:
            line = " ".join(f"{k}={v / TIMING_EVERY:.2f}s" for k, v in self._phase_total.items())
            print(f"tirith collect: {TIMING_EVERY} cycles, mean per cycle: {line}", file=sys.stderr, flush=True)
            self._phase_total = {}
        return state

    def run(self, interval: float = 2.0, once: bool = False, reload_check=None) -> dict:
        """Loop forever (or once) writing state every ``interval`` seconds.

        ``reload_check`` (the continuous mode of ``tirith collect``) is asked between cycles; when it says the
        code changed the loop ends with ``ReloadRequested`` so the caller can re-execute."""
        self._one_shot = once
        if once and self.config.rooms:
            # One-shot mode has no background poller to wait for: read the rooms now,
            # then give the brain a bounded moment for the room jobs and cycle again.
            self.rooms.poll_now(self.config)
            state = self.cycle()
            if self.rooms.pending_jobs:
                self.worker.drain(90.0)
                state = self.cycle()
            return state
        if once:
            return self.cycle()
        state: dict = {}
        failures = 0
        try:
            state = self.cycle()
        except Exception as exc:
            failures = 1
            print(f"tirith collect: cycle failed ({type(exc).__name__}: {exc}); "
                  f"retrying (failure {failures})", file=sys.stderr, flush=True)
        while True:
            time.sleep(max(0.2, interval) + backoff_s(failures))
            if reload_check is not None and reload_check():
                from tirith.selfreload import ReloadRequested
                raise ReloadRequested()
            try:
                state = self.cycle()
                failures = 0
            except Exception as exc:  # one bad cycle (disk full, bad file) must not end the collector
                failures += 1
                print(f"tirith collect: cycle failed ({type(exc).__name__}: {exc}); "
                      f"retrying (failure {failures})", file=sys.stderr, flush=True)
        return state  # pragma: no cover
