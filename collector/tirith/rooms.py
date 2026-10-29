"""agentsmessenger.com rooms: polling, local cache, config editing and the ``rooms`` state.

Agents (Tirith heroes, the owner's other agents, visitors) talk in public rooms on
https://agentsmessenger.com. This module reads those rooms (HTTP GET only) and keeps
the last messages of each in ``<home>/rooms/<room>.json``; ``build_rooms_state`` merges
that cache with hero identities and the brain's cached jobs into the ``rooms`` section
of ``state.json`` (see ``docs/CONTRACT-rooms.md``).

Writing to a room happens only through ``tirith room say`` (``Transport.post``); the
poller and the collector never POST. The only host contacted is agentsmessenger.com.

Polling never blocks the collector's 2 s loop: a daemon thread polls each room at a
polite interval (at least 10 s, backing off exponentially on errors) and the collector
only reads the cache files.
"""

from __future__ import annotations

import json
import re
import threading
import time
import urllib.error
import urllib.request
from datetime import datetime
from pathlib import Path
from typing import Callable, Protocol

from tirith import asks as asks_mod
from tirith import brain as brain_mod
from tirith.home import Config, RoomConfig, now_iso, parse_iso
from tirith.store import read_json, write_json_atomic

HOST = "agentsmessenger.com"
BASE_URL = f"https://{HOST}/r/"
MIN_INTERVAL_S = 10.0
DEFAULT_INTERVAL_S = 15.0
MAX_BACKOFF_S = 300.0
KEPT_MESSAGES = 100          # on disk, per room
STATE_MESSAGES = 40          # in state.json, per room
TRANSCRIPT_MESSAGES = 30     # sent to the brain
MAX_PAGES = 5                # follow-up pages in one poll
MAX_TEXT = 1200              # characters of a message kept in state.json
MAX_SAY = 2000
ROOM_ID_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$")
NAME_BAD_RE = re.compile(r"[^A-Za-z0-9._-]")


class RoomError(RuntimeError):
    """A room request failed (message never contains secrets)."""


def valid_room_id(room_id: str) -> bool:
    return bool(ROOM_ID_RE.match(room_id or ""))


def room_url(room_id: str) -> str:
    return BASE_URL + room_id


def room_title(room_id: str) -> str:
    """A readable title from a room id: ``meta-hackathon-ideas`` -> ``Meta Hackathon Ideas``."""
    words = [w for w in re.split(r"[-_.]+", room_id) if w]
    if not words:
        return room_id
    if len(room_id) > 40 and not any(c in room_id for c in "-_."):
        return room_id[:12] + "..."
    return " ".join(w[:1].upper() + w[1:] for w in words)


def clean_name(name: str) -> str:
    """The service's rule: letters, digits . _ - (max 40); anything else becomes ``-``."""
    return NAME_BAD_RE.sub("-", name.strip())[:40]


# -- transport -------------------------------------------------------------------


class Transport(Protocol):
    """How rooms are read and written (a fake one is used in tests)."""

    def get(self, room_id: str, after: int | None) -> dict:
        """The room's JSON document (``messages`` strictly after ``after``)."""
        ...

    def post(self, room_id: str, payload: dict) -> dict:
        """POST one message; returns the service's JSON answer."""
        ...


class HttpTransport:
    """The real thing: plain HTTPS to agentsmessenger.com, standard library only."""

    def __init__(self, timeout_s: float = 20.0, max_bytes: int = 6_000_000):
        self.timeout_s = timeout_s
        self.max_bytes = max_bytes

    def _open(self, request: urllib.request.Request) -> dict:
        request.add_header("User-Agent", "tirith-collector/1 (read-only room watcher)")
        request.add_header("Accept", "application/json")
        try:
            with urllib.request.urlopen(request, timeout=self.timeout_s) as response:
                body = response.read(self.max_bytes + 1)
        except urllib.error.HTTPError as exc:
            raise RoomError(f"agentsmessenger returned HTTP {exc.code}") from None
        except (urllib.error.URLError, TimeoutError, OSError) as exc:
            raise RoomError(f"agentsmessenger unreachable: {type(exc).__name__}") from None
        if len(body) > self.max_bytes:
            raise RoomError("agentsmessenger answer too large")
        try:
            data = json.loads(body.decode("utf-8", errors="replace"))
        except ValueError:
            raise RoomError("agentsmessenger answer is not JSON") from None
        if not isinstance(data, dict):
            raise RoomError("agentsmessenger answer has an unexpected shape")
        return data

    def get(self, room_id: str, after: int | None) -> dict:
        if not valid_room_id(room_id):
            raise RoomError(f"invalid room id: {room_id!r}")
        query = f"format=json&t={int(time.time() * 1000)}"
        if after:
            query += f"&after={int(after)}"
        return self._open(urllib.request.Request(f"{BASE_URL}{room_id}?{query}", method="GET"))

    def post(self, room_id: str, payload: dict) -> dict:
        if not valid_room_id(room_id):
            raise RoomError(f"invalid room id: {room_id!r}")
        request = urllib.request.Request(
            f"{BASE_URL}{room_id}", data=json.dumps(payload).encode("utf-8"),
            headers={"Content-Type": "application/json"}, method="POST")
        return self._open(request)


# -- normalising service messages ----------------------------------------------------


def _signed_kind(raw: dict) -> str:
    """``self`` (own key), ``platform`` (the platform vouches for the name) or ``none``."""
    signer = str(raw.get("signed_by") or "").lower()
    if not signer:
        return "none"
    return "platform" if signer == "platform" else "self"


def _agreed_names(raw) -> list[str]:
    names: list[str] = []
    if not isinstance(raw, list):
        return names
    for item in raw:
        if isinstance(item, dict):
            name = item.get("from") or item.get("name") or item.get("by")
        else:
            name = item
        if name and str(name) not in names:
            names.append(str(name)[:60])
    return names


def normalize_message(raw) -> dict | None:
    """One service message in Tirith's stored shape (None when it has no number)."""
    if not isinstance(raw, dict):
        return None
    try:
        number = int(raw.get("n"))
    except (TypeError, ValueError):
        return None
    refs: list[int] = []
    for item in raw.get("re") or []:
        try:
            refs.append(int(item))
        except (TypeError, ValueError):
            continue
    return {
        "n": number,
        "at": raw.get("at") if isinstance(raw.get("at"), str) else None,
        "from": str(raw.get("from") or "unknown")[:60],
        "text": str(raw.get("text") or "")[:4000],
        "re": refs,
        "agreed": _agreed_names(raw.get("agreed")),
        "signed": _signed_kind(raw),
        "kind": str(raw.get("kind") or "message")[:20],
        "truncated": bool(raw.get("truncated")),
    }


def merge_messages(existing: list[dict], fetched: list[dict]) -> list[dict]:
    """Merge by message number (new data wins), oldest first, last KEPT_MESSAGES."""
    by_number = {m["n"]: m for m in existing if isinstance(m, dict) and "n" in m}
    for message in fetched:
        by_number[message["n"]] = message
    ordered = [by_number[n] for n in sorted(by_number)]
    return ordered[-KEPT_MESSAGES:]


# -- cache files -------------------------------------------------------------------


def room_cache_path(home: Path, room_id: str) -> Path:
    return home / "rooms" / f"{room_id}.json"


def read_room_cache(home: Path, room_id: str) -> dict:
    data = read_json(room_cache_path(home, room_id), {})
    return data if isinstance(data, dict) else {}


def _empty_cache(room_id: str) -> dict:
    return {"id": room_id, "after": 0, "count": 0, "messages": [], "here": [],
            "polled_at": None, "error": None, "error_at": None}


class RoomPoller:
    """Incremental, polite reader of rooms into the cache files.

    ``poll_room`` does one fetch (plus follow-up pages when the room is long);
    ``poll_due`` applies the schedule and the back-off; ``start`` runs ``poll_due``
    on a daemon thread. ``clock`` is injectable so tests need no sleeping.
    """

    def __init__(self, home: Path, transport: Transport | None = None,
                 interval_s: float = DEFAULT_INTERVAL_S,
                 clock: Callable[[], float] = time.monotonic):
        self.home = Path(home)
        self.transport: Transport = transport or HttpTransport()
        self.interval_s = max(MIN_INTERVAL_S, float(interval_s))
        self._clock = clock
        self._schedule: dict[str, dict] = {}
        self._wanted: list[str] = []
        self._lock = threading.Lock()
        self._poll_lock = threading.Lock()
        self._stop = threading.Event()
        self._thread: threading.Thread | None = None

    # -- one poll ----------------------------------------------------------------

    def poll_room(self, room_id: str) -> bool:
        """Fetch new messages and update the cache. True on success; never raises."""
        with self._poll_lock:
            return self._poll_room(room_id)

    def _poll_room(self, room_id: str) -> bool:
        cache = read_room_cache(self.home, room_id) or _empty_cache(room_id)
        cache["id"] = room_id
        messages = [m for m in cache.get("messages") or [] if isinstance(m, dict)]
        after = max([int(m["n"]) for m in messages if "n" in m], default=0)
        try:
            data = self.transport.get(room_id, after or None)
            count = data.get("count")
            if isinstance(count, int) and count < after:
                # The room was reset or replaced: start over from message 1.
                messages, after = [], 0
                data = self.transport.get(room_id, None)
            fetched = [m for m in map(normalize_message, data.get("messages") or []) if m]
            pages = 0
            while pages < MAX_PAGES and fetched and isinstance(data.get("count"), int) \
                    and max(m["n"] for m in fetched) < data["count"]:
                pages += 1
                more = self.transport.get(room_id, max(m["n"] for m in fetched))
                extra = [m for m in map(normalize_message, more.get("messages") or []) if m]
                if not extra:
                    break
                fetched.extend(extra)
                data = {**data, "count": more.get("count", data.get("count"))}
            messages = merge_messages(messages, fetched)
            cache["messages"] = messages
            cache["after"] = max([m["n"] for m in messages], default=0)
            cache["count"] = int(data["count"]) if isinstance(data.get("count"), int) \
                else len(messages)
            here = []
            for item in data.get("here") or []:
                if isinstance(item, dict) and item.get("name"):
                    here.append({"name": str(item["name"])[:60],
                                 "waiting": bool(item.get("waiting")),
                                 "read_at": item.get("read_at")
                                 if isinstance(item.get("read_at"), str) else None})
            cache["here"] = here
            cache["polled_at"] = now_iso()
            cache["error"] = None
            ok = True
        except RoomError as exc:
            cache["error"] = str(exc)[:200]
            cache["error_at"] = now_iso()
            ok = False
        except Exception as exc:  # a broken transport must not kill the poller
            cache["error"] = f"{type(exc).__name__}"[:200]
            cache["error_at"] = now_iso()
            ok = False
        write_json_atomic(room_cache_path(self.home, room_id), cache)
        return ok

    # -- schedule ----------------------------------------------------------------

    def next_delay(self, fails: int) -> float:
        """Seconds until the next poll: the interval, doubled per consecutive failure."""
        if fails <= 0:
            return self.interval_s
        return min(MAX_BACKOFF_S, self.interval_s * (2 ** fails))

    def poll_due(self, room_ids: list[str]) -> list[str]:
        """Poll every room whose time has come; returns the ids polled."""
        polled: list[str] = []
        now = self._clock()
        for room_id in room_ids:
            entry = self._schedule.setdefault(room_id, {"due": 0.0, "fails": 0})
            if entry["due"] > now:
                continue
            ok = self.poll_room(room_id)
            entry["fails"] = 0 if ok else entry["fails"] + 1
            entry["due"] = self._clock() + self.next_delay(entry["fails"])
            polled.append(room_id)
        for stale in [r for r in self._schedule if r not in room_ids]:
            del self._schedule[stale]
        return polled

    def poll_all(self, room_ids: list[str]) -> None:
        """Poll right now, ignoring the schedule (``tirith collect --once``)."""
        for room_id in room_ids:
            ok = self.poll_room(room_id)
            entry = self._schedule.setdefault(room_id, {"due": 0.0, "fails": 0})
            entry["fails"] = 0 if ok else entry["fails"] + 1
            entry["due"] = self._clock() + self.next_delay(entry["fails"])

    # -- thread ------------------------------------------------------------------

    def set_rooms(self, room_ids: list[str], interval_s: float | None = None) -> None:
        with self._lock:
            self._wanted = list(room_ids)
        if interval_s is not None:
            self.interval_s = max(MIN_INTERVAL_S, float(interval_s))

    def start(self) -> None:
        if self._thread is not None:
            return
        self._thread = threading.Thread(target=self._run, name="tirith-rooms", daemon=True)
        self._thread.start()

    def stop(self, timeout_s: float = 0.0) -> bool:
        """Ask the poll thread to end; with ``timeout_s`` wait (bounded) for it. True when it is gone."""
        self._stop.set()
        thread = self._thread
        if thread is None:
            return True
        if timeout_s > 0:
            thread.join(timeout_s)
        return not thread.is_alive()

    def _run(self) -> None:
        while not self._stop.is_set():
            with self._lock:
                wanted = list(self._wanted)
            try:
                self.poll_due(wanted)
            except Exception:
                pass
            self._stop.wait(1.0)


# -- config.toml editing ---------------------------------------------------------------


def _toml_str(value: str) -> str:
    return json.dumps(value)


def _room_block(room_id: str, project: str | None, aliases: dict[str, str]) -> str:
    lines = ["[[rooms]]", f"id = {_toml_str(room_id)}"]
    if project:
        lines.append(f"project = {_toml_str(project)}")
    if aliases:
        inner = ", ".join(f"{_toml_str(k)} = {_toml_str(v)}" for k, v in aliases.items())
        lines.append(f"aliases = {{ {inner} }}")
    return "\n".join(lines) + "\n"


def add_room_to_config(home: Path, room_id: str, project: str | None = None,
                       aliases: dict[str, str] | None = None) -> bool:
    """Append a ``[[rooms]]`` row to config.toml. False when the room is already there."""
    from tirith.home import load_config
    if load_config(home).room(room_id) is not None:
        return False
    path = home / "config.toml"
    existing = path.read_text(encoding="utf-8") if path.exists() else ""
    if existing and not existing.endswith("\n"):
        existing += "\n"
    separator = "\n" if existing.strip() else ""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(existing + separator + _room_block(room_id, project, aliases or {}),
                    encoding="utf-8")
    return True


_ID_LINE_RE = re.compile(r"""^\s*id\s*=\s*["']([^"']+)["']""")


def remove_room_from_config(home: Path, room_id: str) -> bool:
    """Drop the ``[[rooms]]`` row with this id (and its sub-tables). True if one was removed."""
    path = home / "config.toml"
    if not path.exists():
        return False
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    out: list[str] = []
    removed = False
    index = 0
    while index < len(lines):
        if lines[index].strip() == "[[rooms]]":
            end = index + 1
            while end < len(lines) and not (
                    lines[end].strip().startswith("[")
                    and not lines[end].strip().startswith("[rooms.")):
                end += 1
            block = lines[index:end]
            found = next((m.group(1) for m in map(_ID_LINE_RE.match, block) if m), None)
            if found == room_id:
                removed = True
                # Also drop one blank line that separated the block from the previous text.
                if out and not out[-1].strip():
                    out.pop()
                index = end
                continue
            out.extend(block)
            index = end
            continue
        out.append(lines[index])
        index += 1
    if removed:
        path.write_text("".join(out), encoding="utf-8")
    return removed


# -- building the ``rooms`` state ---------------------------------------------------------


def _hero_index(agents: list[dict]) -> dict[str, dict]:
    """lower-case handle -> agent row, for matching speaker names to heroes."""
    index: dict[str, dict] = {}
    for agent in agents:
        handles = {str(agent.get("id") or ""), str(agent.get("name") or ""),
                   f"{agent.get('name', '')} {agent.get('epithet', '')}".strip()}
        for handle in handles:
            if handle:
                index.setdefault(handle.lower(), agent)
                index.setdefault(clean_name(handle).lower(), agent)
    return index


def _transcript(messages: list[dict]) -> str:
    lines = []
    for message in messages[-TRANSCRIPT_MESSAGES:]:
        reply = f" (reply to #{', #'.join(str(n) for n in message['re'])})" if message["re"] else ""
        text = " ".join(message["text"].split())[:400]
        lines.append(f"#{message['n']} {message['from']}{reply}: {text}")
    return "\n".join(lines)


def _mentions(text: str, names: list[str]) -> bool:
    lowered = text.lower()
    for name in names:
        pattern = r"(?<![A-Za-z0-9_])@?" + re.escape(name.lower()) + r"(?![A-Za-z0-9_])"
        if re.search(pattern, lowered):
            return True
    return False


class RoomsService:
    """The collector's handle on rooms: owns the poller and builds the state section."""

    def __init__(self, home: Path, transport: Transport | None = None):
        self.home = Path(home)
        self.transport = transport
        self.poller = RoomPoller(self.home, transport)
        self.pending_jobs = False

    def ensure_polling(self, config: Config, background: bool = True) -> None:
        """Tell the poller which rooms to watch; start its thread when there are any."""
        ids = [r.id for r in config.rooms if valid_room_id(r.id)]
        self.poller.set_rooms(ids, config.rooms_interval_s)
        if ids and background:
            self.poller.start()

    def stop(self, timeout_s: float = 5.0) -> bool:
        return self.poller.stop(timeout_s)

    def poll_now(self, config: Config) -> None:
        self.poller.interval_s = max(MIN_INTERVAL_S, config.rooms_interval_s)
        self.poller.poll_all([r.id for r in config.rooms if valid_room_id(r.id)])

    def build(self, config: Config, agents: list[dict], worker, cache,
              now: datetime) -> tuple[list[dict], list[dict]]:
        """Return ``(rooms, waits)``: the ``rooms`` section and brain "needs you" waits."""
        self.pending_jobs = False
        rooms: list[dict] = []
        waits: list[dict] = []
        heroes = _hero_index(agents)
        owner_lower = {n.lower() for n in config.owner_names}
        for room_cfg in config.rooms:
            if not valid_room_id(room_cfg.id):
                continue
            room, room_waits = self._build_room(
                room_cfg, read_room_cache(self.home, room_cfg.id), config, heroes,
                owner_lower, worker, cache, now)
            rooms.append(room)
            waits.extend(room_waits)
        return rooms, waits

    def _build_room(self, room_cfg: RoomConfig, data: dict, config: Config,
                    heroes: dict[str, dict], owner_lower: set[str], worker, cache,
                    now: datetime) -> tuple[dict, list[dict]]:
        messages = [m for m in data.get("messages") or [] if isinstance(m, dict) and "n" in m]
        known = {m["n"] for m in messages}
        authors = {m["n"]: m["from"] for m in messages}
        aliases = {k.lower(): v for k, v in room_cfg.aliases.items()}

        def classify(name: str) -> dict:
            lowered = name.lower()
            if lowered in owner_lower:
                return {"kind": "owner", "hero_id": None, "hero_name": None}
            lane_id = aliases.get(lowered)
            agent = heroes.get(str(lane_id).lower()) if lane_id else None
            if agent is None:
                agent = heroes.get(lowered) or heroes.get(clean_name(name).lower())
            if agent is not None:
                return {"kind": "tirith-agent", "hero_id": str(agent["id"]),
                        "hero_name": str(agent.get("name") or "")}
            if lane_id:
                return {"kind": "tirith-agent", "hero_id": str(lane_id), "hero_name": None}
            return {"kind": "external", "hero_id": None, "hero_name": None}

        participants: dict[str, dict] = {}
        for message in messages:
            entry = participants.setdefault(message["from"], {
                "name": message["from"], **classify(message["from"]),
                "last_seen": message["at"], "messages": 0, "present": False})
            entry["messages"] += 1
            if (message["at"] or "") >= (entry["last_seen"] or ""):
                entry["last_seen"] = message["at"]
        for here in data.get("here") or []:
            name = str(here.get("name") or "")
            if not name:
                continue
            entry = participants.setdefault(name, {
                "name": name, **classify(name), "last_seen": here.get("read_at"),
                "messages": 0, "present": True})
            entry["present"] = True
            if (here.get("read_at") or "") > (entry["last_seen"] or ""):
                entry["last_seen"] = here.get("read_at")
        ordered = sorted(participants.values(),
                         key=lambda p: (-int(p["messages"]), p["name"].lower()))

        edges: dict[tuple[str, str], int] = {}
        for message in messages:
            for target in message["re"]:
                if target in known and authors[target] != message["from"]:
                    key = (message["from"], authors[target])
                    edges[key] = edges.get(key, 0) + 1
        reply_edges = [{"from": a, "to": b, "count": c}
                       for (a, b), c in sorted(edges.items(), key=lambda kv: -kv[1])]

        last_hour = last_day = 0
        for message in messages:
            at = parse_iso(message["at"])
            if at is None:
                continue
            age = (now - at).total_seconds()
            last_day += 1 if age <= 86400 else 0
            last_hour += 1 if age <= 3600 else 0

        transcript = _transcript(messages)
        summary, summary_source = self._summary(messages, transcript, worker, cache)
        needs, needs_source = self._needs_owner(
            messages, transcript, config.owner_names, worker, cache)
        last = messages[-1] if messages else None
        state_messages = []
        for message in messages[-STATE_MESSAGES:]:
            state_messages.append({
                "n": message["n"], "at": message["at"], "from": message["from"],
                "text": message["text"][:MAX_TEXT],
                "re": [n for n in message["re"] if n in known],
                "agreed": message["agreed"], "signed": message["signed"],
                "kind": message["kind"], "truncated": message["truncated"],
            })
        url = room_url(room_cfg.id)
        waits = []
        for item in needs:
            waits.append({
                "id": f"room-{room_cfg.id}-{item['n']}",
                "project": room_cfg.project,
                "text": f"{room_title(room_cfg.id)}: {item['from']} (#{item['n']}) {item['reason']}",
                "since": item["at"], "link": url, "source": "room",
                "options": [dict(o) for o in asks_mod.GOT_IT_OPTIONS],
            })
        room = {
            "id": room_cfg.id, "url": url,
            "title": room_cfg.title or room_title(room_cfg.id),
            "project": room_cfg.project,
            "summary": summary, "summary_source": summary_source,
            "needs_owner": [{"n": i["n"], "from": i["from"], "reason": i["reason"],
                             "source": needs_source} for i in needs],
            "message_count": int(data.get("count") or len(messages)),
            "last_message_at": last["at"] if last else None,
            "activity_per_hour": last_hour, "messages_last_day": last_day,
            "polled_at": data.get("polled_at"), "error": data.get("error"),
            "participants": ordered, "reply_edges": reply_edges,
            "messages": state_messages,
        }
        return room, waits

    def _summary(self, messages: list[dict], transcript: str, worker, cache) -> tuple[str | None, str]:
        if not messages:
            return None, "none"
        if worker.enabled and transcript.strip():
            cached = cache.get("room_summary", transcript)
            if cached and cached.get("summary"):
                return str(cached["summary"]), "muse"
            if worker.submit("room_summary", transcript,
                             lambda c, t=transcript: {"summary": brain_mod.job_room_summary(c, t)}):
                self.pending_jobs = True
        last = messages[-1]
        snippet = " ".join(last["text"].split())[:200]
        return f"Latest from {last['from']}: {snippet}", "fallback"

    def _needs_owner(self, messages: list[dict], transcript: str, owner_names: list[str],
                     worker, cache) -> tuple[list[dict], str]:
        by_n = {m["n"]: m for m in messages}
        recent = {m["n"] for m in messages[-TRANSCRIPT_MESSAGES:]}
        found: list[dict] = []
        if worker.enabled and transcript.strip():
            key = transcript + "\nOWNER:" + ",".join(owner_names)
            cached = cache.get("room_owner", key)
            if cached is not None:
                for item in cached.get("items") or []:
                    message = by_n.get(item.get("n"))
                    if message is not None and item.get("n") in recent \
                            and message["from"].lower() not in {o.lower() for o in owner_names}:
                        found.append({"n": message["n"], "from": message["from"],
                                      "at": message["at"], "reason": str(item.get("reason") or "")})
                return found, "brain"
            if worker.submit("room_owner", key, lambda c, t=transcript, o=list(owner_names): {
                    "items": brain_mod.job_room_owner(c, t, o)}):
                self.pending_jobs = True
            return [], "brain"
        # No brain: a message that names the owner, not answered by the owner afterwards.
        if owner_names:
            lowered = {o.lower() for o in owner_names}
            for index, message in enumerate(messages[-10:]):
                if message["from"].lower() in lowered:
                    continue
                answered = any(later["from"].lower() in lowered
                               for later in messages[-10:][index + 1:])
                if not answered and _mentions(message["text"], owner_names):
                    found.append({"n": message["n"], "from": message["from"],
                                  "at": message["at"], "reason": "mentions you"})
        return found, "room"
