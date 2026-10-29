"""Remote work as pipelines and agents (see ``docs/CONTRACT-github.md``).

For projects whose git remote is GitHub, GitHub Actions workflow runs become pipeline runs
(jobs are the stages) and open pull requests by the configured agent accounts become agents.
Everything goes through the REST API with ``urllib``; without a token nothing is read and
nothing is reported.  Results are cached for ``MIN_INTERVAL_S`` and requests are ETag-aware.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Callable, Protocol

from tirith.home import Config, parse_iso

API = "https://api.github.com"
MIN_INTERVAL_S = 60.0
RUNS_PER_PROJECT = 5
PRS_PER_PROJECT = 10
TIMEOUT_S = 10.0
TOKEN_ENVS = ("GITHUB_TOKEN", "GH_TOKEN")

_REMOTE_RE = re.compile(
    r"^(?:https?://(?:[^@/]+@)?github\.com/|git@github\.com:|ssh://git@github\.com/)"
    r"([\w.-]+)/([\w.-]+?)(?:\.git)?/?$")


class Transport(Protocol):
    def get(self, url: str, token: str, etag: str | None) -> tuple[int, object, str | None]:
        """``(status, parsed json, etag)``; status 304 means "unchanged"; raises OSError."""


class HttpTransport:
    def get(self, url: str, token: str, etag: str | None) -> tuple[int, object, str | None]:
        headers = {"Accept": "application/vnd.github+json", "Authorization": f"Bearer {token}",
                   "X-GitHub-Api-Version": "2022-11-28", "User-Agent": "tirith-collector"}
        if etag:
            headers["If-None-Match"] = etag
        request = urllib.request.Request(url, headers=headers)
        try:
            with urllib.request.urlopen(request, timeout=TIMEOUT_S) as resp:
                return resp.status, json.loads(resp.read().decode("utf-8")), resp.headers.get("ETag")
        except urllib.error.HTTPError as exc:
            if exc.code == 304:
                return 304, None, etag
            raise OSError(f"GitHub {exc.code} for {url}") from exc
        except (urllib.error.URLError, ValueError) as exc:
            raise OSError(str(exc)) from exc


def parse_remote(url: str) -> str | None:
    """``owner/repo`` for a GitHub remote URL, else None."""
    match = _REMOTE_RE.match((url or "").strip())
    return f"{match.group(1)}/{match.group(2)}" if match else None


def remote_slug(folder: Path | str | None) -> str | None:
    if not folder or not Path(folder).is_dir():
        return None
    try:
        out = subprocess.run(["git", "-C", str(folder), "remote", "get-url", "origin"],
                             capture_output=True, text=True, timeout=10)
    except (OSError, subprocess.SubprocessError):
        return None
    return parse_remote(out.stdout) if out.returncode == 0 else None


def token_for(config: Config, env: dict | None = None) -> str | None:
    env = os.environ if env is None else env
    names = list(TOKEN_ENVS)
    if config.github_token_env:
        names.append(config.github_token_env)
    for name in names:
        value = (env.get(name) or "").strip()
        if value:
            return value
    return None


# -- mapping --------------------------------------------------------------------------------


def _status(item: dict) -> str:
    """pending | running | passed | failed | skipped for a run, job or check run."""
    state = item.get("status")
    if state == "completed":
        conclusion = item.get("conclusion")
        if conclusion in ("success", "neutral"):
            return "passed"
        if conclusion == "skipped":
            return "skipped"
        return "failed"
    return "running" if state == "in_progress" else "pending"


def _seconds(start, end) -> int | None:
    a, b = parse_iso(start), parse_iso(end)
    return max(0, int((b - a).total_seconds())) if a and b else None


_STEP_MARK = {"passed": "✓", "failed": "✗", "skipped": "-", "running": "…", "pending": "·"}


def map_job(job: dict) -> dict:
    status = _status(job)
    ended = job.get("completed_at") if job.get("status") == "completed" else None
    rows = [(_status(s), s.get("name")) for s in job.get("steps") or [] if isinstance(s, dict)]
    while status == "failed" and rows and rows[-1][0] == "skipped":
        rows.pop()  # the failing step stays last: the Pipelines card shows the last line
    steps = [f"{_STEP_MARK[state]} {name}" for state, name in rows]
    return {"name": str(job.get("name") or "job"), "status": status,
            "started_at": job.get("started_at"), "ended_at": ended,
            "duration_s": _seconds(job.get("started_at"), ended),
            "log_tail": steps[-20:], "html_url": job.get("html_url")}


def map_run(slug: str, project: str, run: dict, jobs: list[dict]) -> dict:
    stages = [map_job(j) for j in jobs if isinstance(j, dict)]
    status = _status(run)
    if stages:
        if any(s["status"] == "failed" for s in stages):
            status = "failed"
        elif status == "passed" and not all(s["status"] in ("passed", "skipped") for s in stages):
            status = "running"
    name = str(run.get("name") or run.get("display_title") or "workflow")
    started = run.get("run_started_at") or run.get("created_at")
    finished = status in ("passed", "failed")
    return {
        "run": f"github:{slug}:{name}#{run.get('run_number') or run.get('id')}",
        "project": project, "pipeline": name, "session": None, "status": status,
        "started_at": started, "ended_at": run.get("updated_at") if finished else None,
        "stages": stages, "source": "github", "html_url": run.get("html_url"),
    }


_CHANGE = {"added": "added", "removed": "deleted", "renamed": "renamed", "modified": "modified",
           "changed": "modified"}


def map_pr(slug: str, project: str, pr: dict, checks: list[dict], files: list[dict]) -> dict:
    states = [_status(c) for c in checks if isinstance(c, dict)]
    if any(s == "failed" for s in states):
        status = "failed"
    elif pr.get("draft") or not states or any(s in ("running", "pending") for s in states):
        status = "working"
    else:
        status = "done"
    rows = [{"path": str(f.get("filename") or ""), "change": _CHANGE.get(f.get("status"), "modified"),
             "insertions": int(f.get("additions") or 0), "deletions": int(f.get("deletions") or 0)}
            for f in files if isinstance(f, dict) and f.get("filename")][:100]
    login = str((pr.get("user") or {}).get("login") or "cloud agent")
    number = pr.get("number")
    started = pr.get("created_at")
    ended = pr.get("updated_at") if status != "working" else None
    passed = sum(1 for s in states if s == "passed")
    summary = f"Pull request #{number} by {login}: " + (
        f"{passed} of {len(states)} checks passed" if states else "no checks yet")
    elapsed = _seconds(started, ended or pr.get("updated_at")) or 0
    return {
        "id": f"gh-{slug.replace('/', '-')}-pr{number}", "project": project, "session": None,
        "name": login.removesuffix("[bot]"), "epithet": f"PR #{number}", "class": "",
        "rank": "recruit", "renown": 0, "feature": str(pr.get("title") or f"Pull request #{number}"),
        "status": status, "started_at": started, "ended_at": ended, "elapsed_s": elapsed,
        "summary": summary, "log_tail": [], "log_entries": [],
        "files_changed": len(rows), "insertions": sum(r["insertions"] for r in rows),
        "deletions": sum(r["deletions"] for r in rows), "report": None, "request": None,
        "parent": "manager", "step": None, "goal": None, "active": status == "working",
        "last_activity_at": pr.get("updated_at"), "files": rows, "tools": [], "tests": [],
        "events": [], "source": "github", "html_url": pr.get("html_url"),
    }


# -- the reader -----------------------------------------------------------------------------


class GithubReader:
    """Polls GitHub at most once per ``MIN_INTERVAL_S`` and serves the cached mapping."""

    def __init__(self, transport: Transport | None = None,
                 clock: Callable[[], float] = time.monotonic,
                 slug_of: Callable[[Path | str | None], str | None] = remote_slug):
        self.transport = transport or HttpTransport()
        self.clock = clock
        self.slug_of = slug_of
        self._etags: dict[str, tuple[str | None, object]] = {}
        self._slugs: dict[str, str | None] = {}
        self._last = -MIN_INTERVAL_S
        self._key: tuple | None = None
        self._result: tuple[list[dict], list[dict]] = ([], [])
        self.last_error: str | None = None

    def _get(self, url: str, token: str):
        etag, body = self._etags.get(url, (None, None))
        status, data, new_etag = self.transport.get(url, token, etag)
        if status == 304:
            return body
        self._etags[url] = (new_etag, data)
        return data

    def _slug(self, folder) -> str | None:
        key = str(folder)
        if key not in self._slugs:
            self._slugs[key] = self.slug_of(folder)
        return self._slugs[key]

    def poll(self, config: Config, entries, env: dict | None = None) -> tuple[list[dict], list[dict]]:
        """``(pipeline runs, agents)``; ([], []) without a token or a GitHub project."""
        token = token_for(config, env)
        if not token:
            return [], []
        targets = [(e.id, slug) for e in entries if e.folder and (slug := self._slug(e.folder))]
        if not targets:
            return [], []
        logins = tuple(sorted(l.lower() for l in config.github_agent_logins))
        key = (tuple(targets), logins)
        now = self.clock()
        if key == self._key and now - self._last < MIN_INTERVAL_S:
            return self._result
        runs: list[dict] = []
        agents: list[dict] = []
        errors: list[str] = []
        for project, slug in targets:
            try:
                runs += self._runs(token, project, slug)
                if logins:
                    agents += self._agents(token, project, slug, logins)
            except OSError as exc:
                errors.append(str(exc))
        self.last_error = errors[0] if errors else None
        self._key, self._last, self._result = key, now, (runs, agents)
        return self._result

    def _runs(self, token: str, project: str, slug: str) -> list[dict]:
        data = self._get(f"{API}/repos/{slug}/actions/runs?per_page={RUNS_PER_PROJECT}", token)
        out = []
        for run in (data or {}).get("workflow_runs") or []:
            if not isinstance(run, dict) or not run.get("id"):
                continue
            jobs = self._get(f"{API}/repos/{slug}/actions/runs/{run['id']}/jobs?per_page=30", token)
            out.append(map_run(slug, project, run, (jobs or {}).get("jobs") or []))
        return out

    def _agents(self, token: str, project: str, slug: str, logins: tuple) -> list[dict]:
        pulls = self._get(f"{API}/repos/{slug}/pulls?state=open&per_page=30", token) or []
        mine = [p for p in pulls if isinstance(p, dict)
                and str((p.get("user") or {}).get("login") or "").lower() in logins]
        out = []
        for pr in mine[:PRS_PER_PROJECT]:
            sha = (pr.get("head") or {}).get("sha")
            checks = self._get(f"{API}/repos/{slug}/commits/{sha}/check-runs?per_page=50",
                               token) if sha else {}
            files = self._get(f"{API}/repos/{slug}/pulls/{pr.get('number')}/files?per_page=100",
                              token)
            out.append(map_pr(slug, project, pr, (checks or {}).get("check_runs") or [],
                              files or []))
        return out
