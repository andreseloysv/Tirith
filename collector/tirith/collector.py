"""The collector: merge CLI inputs with live facts into ``state.json``.

One cycle reads config, lanes, events, notes, waits and the steward file,
enriches them with process/log/git facts and cached brain results, verifies
every project's commit guard, updates hero renown, and writes ``state.json``
atomically. Brain jobs are only enqueued here; the worker fills the cache
for later cycles, so a cycle never waits on the network.
"""

from __future__ import annotations

import dataclasses
import os
import re
import subprocess
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path

from tirith import activity as activity_mod
from tirith import brain as brain_mod
from tirith.guardwrap import default_words, verify_guard
from tirith.home import Config, ProjectConfig, ensure_home, load_config, now_iso, parse_iso
from tirith import flows as flows_mod
from tirith import manifest as manifest_mod
from tirith import hooks as hooks_mod
from tirith.logview import entries_from_events, log_entries
from tirith import projects as projects_mod
from tirith.rooms import RoomsService
from tirith.names import (
    RENOWN_LANE_DONE, RENOWN_LANE_FAILED, RENOWN_STAGE_PASSED,
    apply_renown, hero_epithet, hero_name, rank_for,
)
from tirith.store import (
    read_json, read_jsonl, read_lanes, state_js, tail_lines, write_json_atomic, write_text_atomic,
)

STALLED_AFTER_S = 600
DROP_ENDED_AFTER = timedelta(hours=24)
MAX_PIPELINE_RUNS = 20
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


def git_head(workspace: str | Path | None) -> str | None:
    """Current HEAD commit of a workspace, or None outside a git repo."""
    if not workspace:
        return None
    try:
        out = subprocess.run(
            ["git", "-C", str(workspace), "rev-parse", "HEAD"],
            capture_output=True, text=True, timeout=10,
        )
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
        out = subprocess.run(
            ["git", "-C", str(workspace), "diff", "--shortstat", str(start_commit)],
            capture_output=True, text=True, timeout=15,
        )
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


def fallback_summary(status: str, log_tail: list[str], feature: str, now: datetime,
                     lane: dict) -> str:
    """No-brain summary: last non-empty log line (documented fallback)."""
    if status == "stalled":
        mtime = log_mtime(lane.get("log"))
        silent_s = int(now.timestamp() - mtime) if mtime else STALLED_AFTER_S
        return f"Waiting: no log output for {max(1, silent_s // 60)} minutes."
    for line in reversed(log_tail):
        if line.strip():
            return line.strip()[:300]
    if status == "done":
        return f"Finished: {feature}." if feature else "Finished."
    if status == "failed":
        return f"Failed: {feature}." if feature else "Failed."
    return f"Working on {feature}." if feature else "Working."


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


class Collector:
    """Owns one home directory and builds ``state.json`` cycle by cycle."""

    def __init__(self, home: Path, client=_AUTO_CLIENT, config: Config | None = None,
                 rooms_transport=None):
        self.home = ensure_home(Path(home))
        self._fixed_config = config
        self.config = config or load_config(self.home)
        self.cache = brain_mod.BrainCache(self.home)
        resolved = brain_mod.make_client(self.config) if client is _AUTO_CLIENT else client
        self.worker = brain_mod.BrainWorker(resolved, self.cache)
        self._agents_dirty = False
        self.rooms = RoomsService(self.home, rooms_transport)
        self._one_shot = False
        self.activity = activity_mod.ActivityTracker()

    # -- identities / renown -------------------------------------------------

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
        if not self.worker.enabled:
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
            return []
        since = cached.get("created_at") or lane.get("started_at")
        waits = []
        for index, item in enumerate(cached.get("items") or []):
            waits.append({
                "id": f"brain-{brain_mod.cache_key('waiting', log_text)[:8]}-{index}",
                "project": str(lane.get("project") or ""),
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
                "session": None, "_cwd": None,
            })
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
                or declared.get(("*", run["pipeline"])) or []
            names = list(order) + [n for n in run["_order"] if n not in order]
            stages: list[dict] = []
            failed_seen = False
            for name in names:
                stage = run["_stages"].get(name)
                if stage is None:
                    stage = {"name": name, "status": "skipped" if failed_seen else "pending",
                             "started_at": None, "ended_at": None, "duration_s": None,
                             "log_tail": [], "_log": None}
                elif stage["status"] == "failed":
                    failed_seen = True
                stages.append(stage)
            for stage in stages:
                stage["log_tail"] = tail_lines(stage.pop("_log"), LOG_TAIL_LINES)
            starts = [s["started_at"] for s in stages if s["started_at"]]
            ends = [s["ended_at"] for s in stages if s["ended_at"]]
            if any(s["status"] == "failed" for s in stages):
                status = "failed"
            elif stages and all(s["status"] == "passed" for s in stages):
                status = "passed"
            elif not starts:
                status = "pending"
            else:
                status = "running"
            project = resolve(run["project"], run["_cwd"]) if resolve else run["project"]
            output.append({
                "run": run["run"], "project": project, "pipeline": run["pipeline"],
                "session": run["session"], "status": status,
                "started_at": min(starts) if starts else None,
                "ended_at": (max(ends) if ends else None) if status != "running" else None,
                "stages": stages,
            })
        output.sort(key=lambda r: str(r["started_at"] or ""), reverse=True)
        return output[:MAX_PIPELINE_RUNS]

    def _flow_pipeline_ends(self, pipelines: list[dict]) -> None:
        """One ``report`` flow per pipeline run the first time it is seen finished."""
        seen = flows_mod.seen_set(self.home, "pipeline_runs")
        fresh = {p["run"] for p in pipelines if p["status"] in ("passed", "failed")} - seen
        for run in pipelines:
            if run["run"] in fresh:
                stages = ", ".join(f"{s['name']} {s['status']}" for s in run["stages"]
                                   if s["status"] in ("passed", "failed"))
                flows_mod.append_flow(
                    self.home, f"pipeline:{run['run']}", "steward", "report",
                    f"{run['pipeline']} {run['status']}" + (f": {stages}" if stages else ""),
                    session=run.get("session"), project=run.get("project"))
        if fresh:
            flows_mod.remember(self.home, "pipeline_runs", seen | fresh)

    # -- guard + waits ---------------------------------------------------------

    def _guard_state(self) -> dict:
        data = read_json(self.home / "guard-state.json", {})
        return data if isinstance(data, dict) else {}

    def _manual_waits(self) -> list[dict]:
        by_id: dict[str, dict] = {}
        for row in read_jsonl(self.home / "waits.jsonl"):
            wait_id = str(row.get("id") or "")
            if wait_id:
                by_id[wait_id] = row
        waits = []
        for row in by_id.values():
            if row.get("done_at"):
                continue
            waits.append({
                "id": str(row["id"]), "project": row.get("project"),
                "text": str(row.get("text") or ""), "since": row.get("at"),
                "link": row.get("link"), "source": "manual",
                "session": row.get("session") or None,
            })
        waits.sort(key=lambda w: str(w["since"] or ""))
        return waits

    # -- main cycle ------------------------------------------------------------

    def build_state(self, now: datetime | None = None) -> dict:
        """Build (but do not write) one ``state.json`` payload."""
        now = (now or datetime.now(timezone.utc)).astimezone(timezone.utc)
        if self._fixed_config is None:
            self.config = load_config(self.home)
        lanes = read_lanes(self.home)
        events = [e for e in read_jsonl(self.home / "events.jsonl") if e.get("kind") != "hook"]
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
        scored = self._score_lanes(agents, lanes, events)

        cutoff = now - DROP_ENDED_AFTER
        agent_rows: list[dict] = []
        brain_waits: list[dict] = []
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
            # Summary first: it may upgrade the epithet on the identity.
            summary = self._summary_for(lane, status, log_tail, now, identity)
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
                "report": lane.get("report"),
                "parent": str(lane.get("parent") or "steward"),
                **act,
            })
            for wait in self._brain_waits({**lane, "project": lane_project}):
                brain_waits.append({**wait, "session": lane.get("session") or None})
        if scored or self._agents_dirty:
            self._save_agents(agents)

        words_default = default_words()
        guard_since = self._guard_state()
        guard_dirty = False
        projects: list[dict] = []
        guard_waits: list[dict] = []
        known_ids = set()
        for proj in self.config.projects:
            known_ids.add(proj.id)
            problems = verify_guard(proj.repo, self.config.words_for(proj, words_default)) \
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
                guard_waits.append({
                    "id": f"guard-{proj.id}", "project": proj.id,
                    "text": f"Commit guard needs attention in {proj.name}: {problems[0]}",
                    "since": guard_since[proj.id], "link": None, "source": "guard",
                })
            folder = entry_by_id[proj.id].folder if proj.id in entry_by_id else None
            projects.append({
                "id": proj.id, "name": proj.name, "color": proj.color,
                "folder": str(folder) if folder else None, "auto": False,
                "architecture": self.architecture_for(proj),
                "guard": {"ok": guard_ok, "problems": problems},
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
            })
        runs = self._pipelines(events, resolve, entries)
        for pid in [row["project"] for row in agent_rows] + [r["project"] for r in runs]:
            if pid and pid not in known_ids:
                known_ids.add(pid)
                stub = ProjectConfig(id=pid, name=pid)
                projects.append({
                    "id": pid, "name": pid, "color": "#888888", "folder": None, "auto": False,
                    "architecture": self.architecture_for(stub),
                    "guard": {"ok": False, "problems": ["project not defined in config.toml"]},
                })
        if guard_dirty:
            write_json_atomic(self.home / "guard-state.json", guard_since)

        orchestrator = read_json(self.home / "orchestrator.json", {})
        if not isinstance(orchestrator, dict):
            orchestrator = {}
        raw_notes = [{"at": n.get("at"), "project": resolve(n.get("project"), None) or n.get("project"),
                      "text": str(n.get("text") or ""), "session": n.get("session") or None}
                     for n in read_jsonl(self.home / "notes.jsonl")]
        notes = raw_notes[-20:]
        self._flow_pipeline_ends(events_pipelines := runs)
        self.rooms.ensure_polling(config, background=not self._one_shot)
        rooms_state, room_waits = self.rooms.build(
            config, agent_rows, self.worker, self.cache, now)
        waiting = self._manual_waits() + brain_waits + room_waits + guard_waits
        for wait in waiting:
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
        sessions = self._sessions(lanes, events, all_flows, raw_notes, waiting, resolve)
        return {
            "version": 1,
            "generated_at": now_iso(now),
            "brain": {"enabled": self.worker.enabled,
                      "model": self.config.brain_model,
                      "last_error": self.worker.last_error},
            "steward": {
                "name": self.config.steward_name, "title": self.config.steward_title,
                "activity": orchestrator.get("activity"),
                "since": orchestrator.get("since"), "notes": notes,
            },
            "waiting_on_you": waiting,
            "projects": projects,
            "agents": agent_rows,
            "pipelines": events_pipelines,
            "rooms": rooms_state,
            "flows": all_flows[-flows_mod.STATE_FLOWS:],
            "sessions": sessions,
            "loose_events": loose_events,
        }

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

    def cycle(self, now: datetime | None = None) -> dict:
        """One collector cycle: build state and write it atomically."""
        state = self.build_state(now)
        write_json_atomic(self.home / "state.json", state)
        # The same JSON as a script, so a page opened from file:// can load it without CORS.
        write_text_atomic(self.home / "state.js", state_js(state))
        return state

    def run(self, interval: float = 2.0, once: bool = False) -> dict:
        """Loop forever (or once) writing state every ``interval`` seconds."""
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
        state = self.cycle()
        if once:
            return state
        while True:
            time.sleep(max(0.2, interval))
            state = self.cycle()
        return state  # pragma: no cover
