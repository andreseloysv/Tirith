"""The citadel brain: Spark model calls behind a small client interface.

Official API shape (Meta Model API docs, https://dev.meta.ai/docs/):
base URL ``https://api.meta.ai/v1``, ``POST /chat/completions`` with an
OpenAI-style ``{model, messages}`` body and ``Authorization: Bearer <key>``.
Model ids look like ``muse-spark-1.3``. Tirith reads the key only from the
environment (``META_API_KEY``, or ``META_SI_API_KEY`` as the owner's shell
names it; see KEY_ENV_VARS) and never logs or stores it.

All model work runs in a background worker thread with a hash-keyed file
cache under ``brain/``, so the collector's 2 second loop never blocks.
Without a key the brain is disabled and callers use documented fallbacks.
"""

from __future__ import annotations

import hashlib
import json
import os
import queue
import re
import threading
import time
import urllib.error
import urllib.request
from pathlib import Path
from datetime import datetime
from typing import Callable, Protocol

from tirith.home import Config, now_iso
from tirith.store import read_json, write_json_atomic


class BrainError(RuntimeError):
    """A model call failed (message never contains the API key)."""


class ChatClient(Protocol):
    """Anything that can answer chat messages with plain text."""

    def chat(self, messages: list[dict], *, max_tokens: int = 512) -> str:
        """Return the assistant text for ``messages``."""
        ...


class MetaModelClient:
    """Spark client over the Meta Model API chat-completions endpoint."""

    def __init__(self, api_key: str, model: str, base_url: str, timeout_s: float = 90.0,
                 reasoning_effort: str | None = "low"):
        self._api_key = api_key
        self.model = model
        self.base_url = base_url.rstrip("/")
        self.timeout_s = timeout_s
        self.reasoning_effort = reasoning_effort

    def chat(self, messages: list[dict], *, max_tokens: int = 4096) -> str:
        # Spark reasons before it answers and the reasoning counts against max_tokens: with a small
        # budget the whole budget goes to reasoning and the answer comes back empty (measured: 510 of
        # 512 tokens). A generous budget plus low effort keeps short jobs fast and complete.
        request_body: dict = {
            "model": self.model,
            "messages": messages,
            "max_tokens": max_tokens,
            "temperature": 0.3,
        }
        if self.reasoning_effort:
            request_body["reasoning_effort"] = self.reasoning_effort
        body = json.dumps(request_body).encode()
        request = urllib.request.Request(
            f"{self.base_url}/chat/completions",
            data=body,
            headers={
                "Authorization": f"Bearer {self._api_key}",
                "Content-Type": "application/json",
            },
            method="POST",
        )
        try:
            with urllib.request.urlopen(request, timeout=self.timeout_s) as response:
                payload = json.loads(response.read().decode())
        except urllib.error.HTTPError as exc:
            raise BrainError(f"model API returned HTTP {exc.code}") from None
        except (urllib.error.URLError, TimeoutError, ValueError, OSError) as exc:
            raise BrainError(f"model API call failed: {type(exc).__name__}") from None
        return _extract_text(payload)


def _extract_text(payload: dict) -> str:
    """Assistant text from a chat-completions response payload."""
    try:
        content = payload["choices"][0]["message"]["content"]
    except (KeyError, IndexError, TypeError):
        raise BrainError("unexpected model API response shape") from None
    if isinstance(content, str):
        text = content.strip()
    elif isinstance(content, list):
        text = "".join(str(p.get("text", "")) for p in content if isinstance(p, dict)).strip()
    else:
        text = ""
    if not text:
        # Typically the reasoning used the whole token budget (finish_reason "length").
        raise BrainError("model API returned no answer text")
    return text


# Environment variables that may hold the Meta Model API key, in order of preference.
KEY_ENV_VARS = ("META_API_KEY", "META_SI_API_KEY")


def api_key_from_env() -> str | None:
    """The first non-empty key among KEY_ENV_VARS, or None."""
    for name in KEY_ENV_VARS:
        if os.environ.get(name):
            return os.environ[name]
    return None


def make_client(config: Config, api_key: str | None = None) -> ChatClient | None:
    """Build a model client from config + env, or None when disabled."""
    if not config.brain_enabled:
        return None
    key = api_key if api_key is not None else api_key_from_env()
    if not key:
        return None
    return MetaModelClient(key, config.brain_model, config.brain_base_url,
                           reasoning_effort=config.brain_reasoning_effort or None)


# Parts of a log or transcript that change without changing what the brain has to judge.
_NOISE = re.compile(
    r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}(?::\d{2}(?:\.\d+)?)?(?:Z|[+-]\d{2}:?\d{2})?"  # ISO timestamps
    r"|\b\d{1,2}:\d{2}(?::\d{2})?(?:\s?[ap]m)?\b"                                   # clock times
    r"|\b\d+(?:\.\d+)?\s?(?:ms|s|sec|secs|m|min|mins|h|hr|hrs|d)\s+ago\b"           # ages
    r"|\b(?:elapsed|took|in|after|for)\s+\d+(?:\.\d+)?\s?(?:ms|s|sec|secs|m|min|mins|h|hr|hrs)\b"  # durations
    r"|\b\d+(?:\.\d+)?\s?%"                                                         # percentages
    r"|\b\d[\d,]*\s+(?:tokens?|bytes|KB|MB|GB)\b",                                  # usage counters
    re.IGNORECASE)

# Kinds whose input is a growing log or chat: only these are normalized.
_NOISY_KINDS = frozenset({"summary", "waiting", "room_summary", "room_owner"})


def stable_text(text: str) -> str:
    """``text`` without timestamps, ages, durations and counters, blank lines dropped."""
    lines = (" ".join(_NOISE.sub("", line).split()) for line in text.splitlines())
    return "\n".join(line for line in lines if line)


def cache_key(kind: str, text: str) -> str:
    """Stable cache file stem for a brain job kind + input text.

    Log-like kinds hash the text without time/counter noise, so an input that only
    changed its times maps to the same cached answer instead of a new paid call.
    """
    if kind in _NOISY_KINDS:
        text = stable_text(text)
    return hashlib.sha256(f"{kind}\n{text}".encode()).hexdigest()[:32]


class BrainCache:
    """Hash-keyed JSON cache files under ``<home>/brain/``."""

    def __init__(self, home: Path):
        self.dir = home / "brain"
        self.dir.mkdir(parents=True, exist_ok=True)

    def get(self, kind: str, text: str) -> dict | None:
        data = read_json(self.dir / f"{cache_key(kind, text)}.json", None)
        return data if isinstance(data, dict) else None

    def put(self, kind: str, text: str, result: dict) -> dict:
        entry = {"kind": kind, "created_at": now_iso(), **result}
        write_json_atomic(self.dir / f"{cache_key(kind, text)}.json", entry)
        return entry


# --- The four brain jobs (pure prompt/parse helpers, easy to test) ---

_SYSTEM = "You serve Tirith, a dashboard for AI coding agents. Be brief and factual."


def job_summary(client: ChatClient, log_text: str) -> str:
    """Job 1: log tail -> one plain sentence of current activity."""
    text = client.chat([
        {"role": "system", "content": _SYSTEM},
        {"role": "user", "content": (
            "Here is the recent log of a coding agent. Reply with ONE plain "
            "sentence saying what the agent is doing now. No preamble.\n\n" + log_text[-12000:]
        )},
    ])
    sentence = " ".join(text.strip().split())
    return sentence[:300] or "Working."


def job_waiting(client: ChatClient, log_text: str) -> list[str]:
    """Job 2: log tail/report -> items that need the owner (may be [])."""
    text = client.chat([
        {"role": "system", "content": _SYSTEM},
        {"role": "user", "content": (
            "Read this coding-agent log tail. List anything that needs the "
            "human owner personally (a decision, approval, credential, manual "
            "step). Reply with ONLY a JSON array of short strings, or [] "
            "when nothing needs the owner.\n\n" + log_text[-12000:]
        )},
    ])
    return _parse_string_list(text)


def job_epithet(client: ChatClient, summary: str, hero_class: str) -> str:
    """Job 4: first summary -> a short fitting hero epithet."""
    text = client.chat([
        {"role": "system", "content": _SYSTEM},
        {"role": "user", "content": (
            f"A {hero_class} hero just did this work: {summary}\n"
            "Reply with ONLY a short heroic epithet of 1-4 words "
            "(for example Stonehand or of the Seventh Gate). No punctuation."
        )},
    ], max_tokens=32)
    epithet = " ".join(text.strip().strip('."').split())[:60]
    return epithet or "the Steady"


def job_architecture(client: ChatClient, tree_text: str) -> dict:
    """Job 3: file tree + key files -> architecture nodes/edges dict."""
    text = client.chat([
        {"role": "system", "content": _SYSTEM},
        {"role": "user", "content": (
            "Map this project's architecture. Reply with ONLY JSON of the form "
            '{"nodes":[{"id":"slug","label":"Short label","kind":"core|service|data|ui|input","files":1}],'
            '"edges":[{"from":"slug","to":"slug","label":"what flows"}]} '
            "with 3-10 nodes. Project files:\n\n" + tree_text[-16000:]
        )},
    ], max_tokens=1200)
    data = _parse_json_object(text)
    nodes, edges = _clean_architecture(data)
    return {"nodes": nodes, "edges": edges}


def job_room_summary(client: ChatClient, transcript: str) -> str:
    """Job 5: recent room messages -> one sentence on what the room discusses now."""
    text = client.chat([
        {"role": "system", "content": _SYSTEM},
        {"role": "user", "content": (
            "Below are the latest messages of a chat room where AI agents talk. Reply "
            "with ONE plain sentence saying what the room is discussing right now. "
            "No preamble.\n\n" + transcript[-12000:]
        )},
    ])
    sentence = " ".join(text.strip().split())
    return sentence[:300] or "Quiet."


def job_room_owner(client: ChatClient, transcript: str, owner_names: list[str]) -> list[dict]:
    """Job 6: room messages -> those that need the human owner to act or answer.

    Returns ``[{"n": <message number>, "reason": <short text>}]`` (may be empty).
    """
    who = ", ".join(owner_names) if owner_names else "the human owner"
    text = client.chat([
        {"role": "system", "content": _SYSTEM},
        {"role": "user", "content": (
            f"Below are the latest messages of a chat room where AI agents talk. The human "
            f"owner is called: {who}. Find messages addressed to the owner or that need the "
            "owner personally (a question for them, a decision, an approval, a credential). "
            "Messages between agents that do not need the owner do not count. Reply with ONLY "
            'a JSON array like [{"n": 12, "reason": "asks the owner to approve the budget"}], '
            "or [] when nothing needs the owner.\n\n" + transcript[-12000:]
        )},
    ])
    data = _try_json(text)
    items: list[dict] = []
    if isinstance(data, list):
        for raw in data:
            if not isinstance(raw, dict):
                continue
            try:
                number = int(raw.get("n"))
            except (TypeError, ValueError):
                continue
            reason = " ".join(str(raw.get("reason") or "").split())[:200]
            items.append({"n": number, "reason": reason or "needs you"})
    return items[:5]


def job_inbox_answer(client: ChatClient, context: str) -> str:
    """Job 7: the owner's question about a plan step + what is known of it -> a short markdown answer."""
    text = client.chat([
        {"role": "system", "content": _SYSTEM},
        {"role": "user", "content": (
            "The owner of a project asks about one step of its plan. Answer in at most 8 lines of "
            "markdown, using only the facts below; say plainly what is not known. No preamble.\n\n"
            + context[-12000:]
        )},
    ])
    return text.strip()[:4000]


def _parse_string_list(text: str) -> list[str]:
    data = _try_json(text)
    if isinstance(data, list):
        return [str(item)[:240] for item in data if str(item).strip()][:5]
    return []


def _parse_json_object(text: str) -> dict:
    data = _try_json(text)
    return data if isinstance(data, dict) else {}


def _try_json(text: str):
    cleaned = text.strip()
    if cleaned.startswith("```"):
        cleaned = cleaned.strip("`").removeprefix("json").strip()
    try:
        return json.loads(cleaned)
    except ValueError:
        start = min([i for i in (cleaned.find("{"), cleaned.find("[")) if i >= 0], default=-1)
        if start >= 0:
            try:
                return json.loads(cleaned[start:])
            except ValueError:
                return None
    return None


def _clean_architecture(data: dict) -> tuple[list[dict], list[dict]]:
    nodes: list[dict] = []
    seen: set[str] = set()
    for raw in data.get("nodes", []) if isinstance(data.get("nodes"), list) else []:
        if not isinstance(raw, dict) or not raw.get("id"):
            continue
        node_id = str(raw["id"])[:40]
        if node_id in seen:
            continue
        seen.add(node_id)
        try:
            files = int(raw.get("files") or 0)
        except (TypeError, ValueError):
            files = 0
        nodes.append({
            "id": node_id,
            "label": str(raw.get("label") or node_id)[:80],
            "kind": str(raw.get("kind") or "service")[:20],
            "files": max(0, files),
        })
    edges: list[dict] = []
    for raw in data.get("edges", []) if isinstance(data.get("edges"), list) else []:
        if not isinstance(raw, dict):
            continue
        src, dst = str(raw.get("from") or ""), str(raw.get("to") or "")
        if src in seen and dst in seen and src != dst:
            edges.append({"from": src, "to": dst, "label": str(raw.get("label") or "")[:60]})
    return nodes[:24], edges[:48]


# Minimum seconds between two calls of the same kind (0 or missing: no limit).
MIN_INTERVAL_S: dict[str, float] = {
    "waiting": 60.0, "summary": 60.0, "room_summary": 60.0, "room_owner": 60.0,
}
# Brain calls allowed per day, all kinds together.
DAILY_CALL_BUDGET = 400


class BrainWorker:
    """Background thread running brain jobs into the cache.

    The collector enqueues (kind, input text, runner) triples only when the
    input hash is not already cached or in flight, so unchanged logs never
    trigger a second model call. Epithet/architecture side effects are
    handled by the runner callbacks supplied by the collector/map command.
    """

    def __init__(self, client: ChatClient | None, cache: BrainCache, *,
                 daily_budget: int = DAILY_CALL_BUDGET,
                 min_interval_s: dict[str, float] | None = None,
                 clock: Callable[[], float] = time.time):
        self.client = client
        self.cache = cache
        self.daily_budget = daily_budget
        self.min_interval_s = MIN_INTERVAL_S if min_interval_s is None else min_interval_s
        self._clock = clock
        self._usage_path = cache.dir / "usage.json"
        self.last_error: str | None = None
        self._queue: queue.Queue = queue.Queue()
        self._inflight: set[str] = set()
        self._lock = threading.Lock()
        self._thread: threading.Thread | None = None

    @property
    def enabled(self) -> bool:
        return self.client is not None

    def start(self) -> None:
        if self._thread is None:
            self._thread = threading.Thread(target=self._run, name="tirith-brain", daemon=True)
            self._thread.start()

    def submit(self, kind: str, text: str, runner: Callable[[ChatClient], dict]) -> bool:
        """Queue a job unless cached/in flight. Returns True if queued."""
        if self.client is None or not text.strip():
            return False
        if kind in _NOISY_KINDS and not stable_text(text):
            return False
        if self.cache.get(kind, text) is not None:
            return False
        key = f"{kind}:{cache_key(kind, text)}"
        with self._lock:
            if key in self._inflight:
                return False
            if not self._allow(kind):
                return False
            self._inflight.add(key)
        self.start()
        self._queue.put((kind, text, key, runner))
        return True

    # -- call accounting: counts per kind per day, never contents ---------------

    def _today(self) -> str:
        return datetime.fromtimestamp(self._clock()).strftime("%Y-%m-%d")

    def _usage(self) -> dict:
        data = read_json(self._usage_path, None)
        if not isinstance(data, dict) or data.get("day") != self._today():
            kept = data.get("last", {}) if isinstance(data, dict) else {}
            data = {"day": self._today(), "calls": {}, "last": kept}
        return data

    def _allow(self, kind: str) -> bool:
        """Apply the per-kind minimum interval and the daily budget; count the call when allowed."""
        usage = self._usage()
        now = self._clock()
        if sum(usage["calls"].values()) >= self.daily_budget:
            return False
        last = usage["last"].get(kind)
        if isinstance(last, (int, float)) and now - last < self.min_interval_s.get(kind, 0.0):
            return False
        usage["calls"][kind] = int(usage["calls"].get(kind, 0)) + 1
        usage["last"][kind] = now
        write_json_atomic(self._usage_path, usage)
        return True

    def stats(self) -> dict:
        """Today's call counts per kind, the budget, and a health note once it is reached."""
        usage = self._usage()
        total = sum(usage["calls"].values())
        reached = total >= self.daily_budget
        return {
            "day": usage["day"], "calls": dict(usage["calls"]), "total": total,
            "daily_budget": self.daily_budget, "budget_reached": reached,
            "health_note": (f"Daily brain budget of {self.daily_budget} calls reached; "
                            "features use their non-brain fallbacks until tomorrow.") if reached else None,
        }

    def drain(self, timeout_s: float = 10.0) -> None:
        """Wait (bounded) for queued jobs; used by tests and ``tirith map``."""
        if self._thread is None:
            return
        try:
            self._queue.join(timeout=timeout_s)  # type: ignore[arg-type]
        except TypeError:
            # queue.join has no timeout before 3.?? fallback: poll briefly.
            import time
            deadline = time.monotonic() + timeout_s
            while self._queue.unfinished_tasks and time.monotonic() < deadline:
                time.sleep(0.05)

    def _run(self) -> None:
        while True:
            kind, text, key, runner = self._queue.get()
            try:
                result = runner(self.client)  # type: ignore[arg-type]
                if isinstance(result, dict):
                    self.cache.put(kind, text, result)
                self.last_error = None
            except Exception as exc:  # keep the worker alive on any failure
                self.last_error = f"{type(exc).__name__}: {exc}"[:240]
            finally:
                with self._lock:
                    self._inflight.discard(key)
                self._queue.task_done()
