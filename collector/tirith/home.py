"""TIRITH_HOME layout, config loading and small shared helpers.

Everything Tirith shares between the CLI and the collector lives as files
under one home directory (``~/.tirith`` by default, ``TIRITH_HOME`` wins).
This module owns that layout plus ``config.toml`` parsing so no other module
hard-codes a path.
"""

from __future__ import annotations

import os
import tomllib
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path

CLASSES = (
    "steward", "stonewright", "loremaster", "warden",
    "swiftrider", "herald", "glasswright", "archivist",
)

DEFAULT_MODEL = "muse-spark-1.3"
DEFAULT_BASE_URL = "https://api.meta.ai/v1"
DEFAULT_STEWARD_NAME = "Aldric of the Tower"
DEFAULT_STEWARD_TITLE = "The Steward"


def tirith_home(explicit: str | Path | None = None) -> Path:
    """Resolve the Tirith home: explicit arg, env, then ``~/.tirith``."""
    if explicit:
        return Path(explicit).expanduser()
    env = os.environ.get("TIRITH_HOME")
    if env:
        return Path(env).expanduser()
    return Path.home() / ".tirith"


def now_iso(now: datetime | None = None) -> str:
    """Current UTC time as ISO-8601 with a ``Z`` suffix."""
    dt = now or datetime.now(timezone.utc)
    return dt.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def parse_iso(value: str | None) -> datetime | None:
    """Parse an ISO-8601 timestamp written by :func:`now_iso` (or similar)."""
    if not value:
        return None
    try:
        dt = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt.astimezone(timezone.utc)


@dataclass
class ProjectConfig:
    """One project row from ``config.toml``."""

    id: str
    name: str
    repo: Path | None = None
    color: str = "#888888"
    guard_words: list[str] | None = None
    architecture: dict | None = None


@dataclass
class RoomConfig:
    """One ``[[rooms]]`` row: an agentsmessenger.com room Tirith watches."""

    id: str
    project: str | None = None
    # room participant name -> Tirith lane id (the hero that speaks under that name)
    aliases: dict[str, str] = field(default_factory=dict)
    title: str | None = None


@dataclass
class Config:
    """Parsed ``config.toml`` (missing file means empty config)."""

    projects: list[ProjectConfig] = field(default_factory=list)
    brain_model: str = DEFAULT_MODEL
    brain_base_url: str = DEFAULT_BASE_URL
    brain_enabled: bool = True
    brain_reasoning_effort: str = "low"
    steward_name: str = DEFAULT_STEWARD_NAME
    steward_title: str = DEFAULT_STEWARD_TITLE
    guard_words: list[str] | None = None
    rooms: list[RoomConfig] = field(default_factory=list)
    owner_names: list[str] = field(default_factory=list)
    rooms_interval_s: float = 15.0
    agents: dict[str, dict] = field(default_factory=dict)  # alias -> {command, settings}
    gates_enabled: bool = True  # [gates] enabled = false is the only off switch

    def room(self, room_id: str) -> RoomConfig | None:
        for room in self.rooms:
            if room.id == room_id:
                return room
        return None

    def project(self, project_id: str) -> ProjectConfig | None:
        for proj in self.projects:
            if proj.id == project_id:
                return proj
        return None

    def words_for(self, proj: ProjectConfig | None, default_words: list[str]) -> list[str]:
        """Guard words for a project: per-project, then global, then default."""
        if proj is not None and proj.guard_words:
            return list(proj.guard_words)
        if self.guard_words:
            return list(self.guard_words)
        return list(default_words)


def _project_from(table: dict, fallback_id: str | None = None) -> ProjectConfig | None:
    pid = table.get("id") or fallback_id
    if not pid:
        return None
    repo = table.get("repo") or table.get("folder")
    words = table.get("guard_words")
    return ProjectConfig(
        id=str(pid),
        name=str(table.get("name") or pid),
        repo=Path(str(repo)).expanduser() if repo else None,
        color=str(table.get("color") or "#888888"),
        guard_words=[str(w) for w in words] if words else None,
        architecture=table.get("architecture") if isinstance(table.get("architecture"), dict) else None,
    )


def load_config(home: Path) -> Config:
    """Read ``<home>/config.toml``; a missing or broken file gives defaults.

    Projects may be written either as ``[[projects]]`` tables or as a
    ``[projects.<id>]`` table per project; both forms are accepted.
    """
    path = home / "config.toml"
    if not path.exists():
        return Config()
    try:
        raw = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError):
        return Config()
    cfg = Config()
    projects_raw = raw.get("projects")
    if isinstance(projects_raw, list):
        for item in projects_raw:
            if isinstance(item, dict):
                proj = _project_from(item)
                if proj:
                    cfg.projects.append(proj)
    elif isinstance(projects_raw, dict):
        for pid, item in projects_raw.items():
            if isinstance(item, dict):
                proj = _project_from(item, fallback_id=str(pid))
                if proj:
                    cfg.projects.append(proj)
    brain = raw.get("brain") if isinstance(raw.get("brain"), dict) else {}
    if brain.get("model"):
        cfg.brain_model = str(brain["model"])
    if brain.get("base_url"):
        cfg.brain_base_url = str(brain["base_url"]).rstrip("/")
    if "enabled" in brain:
        cfg.brain_enabled = bool(brain["enabled"])
    if "reasoning_effort" in brain:
        cfg.brain_reasoning_effort = str(brain["reasoning_effort"] or "")
    steward = raw.get("steward") if isinstance(raw.get("steward"), dict) else {}
    if steward.get("name"):
        cfg.steward_name = str(steward["name"])
    if steward.get("title"):
        cfg.steward_title = str(steward["title"])
    agents_raw = raw.get("agents") if isinstance(raw.get("agents"), dict) else {}
    cfg.agents = {str(alias): dict(table) for alias, table in agents_raw.items()
                  if isinstance(table, dict)}
    gates = raw.get("gates") if isinstance(raw.get("gates"), dict) else {}
    cfg.gates_enabled = gates.get("enabled", True) is not False
    rooms_raw = raw.get("rooms")
    if isinstance(rooms_raw, list):
        for item in rooms_raw:
            if isinstance(item, dict) and item.get("id"):
                aliases = item.get("aliases")
                cfg.rooms.append(RoomConfig(
                    id=str(item["id"]),
                    project=str(item["project"]) if item.get("project") else None,
                    aliases={str(k): str(v) for k, v in aliases.items()}
                    if isinstance(aliases, dict) else {},
                    title=str(item["title"]) if item.get("title") else None,
                ))
    owner = raw.get("owner") if isinstance(raw.get("owner"), dict) else {}
    if isinstance(owner.get("names"), list):
        cfg.owner_names = [str(n) for n in owner["names"] if str(n).strip()]
    messenger = raw.get("messenger") if isinstance(raw.get("messenger"), dict) else {}
    try:
        cfg.rooms_interval_s = max(10.0, float(messenger.get("interval_s", 15.0)))
    except (TypeError, ValueError):
        pass
    words = raw.get("guard_words")
    if isinstance(words, list) and words:
        cfg.guard_words = [str(w) for w in words]
    return cfg


def ensure_home(home: Path) -> Path:
    """Create the home layout (idempotent) and return the home path."""
    for sub in ("lanes", "logs", "brain"):
        (home / sub).mkdir(parents=True, exist_ok=True)
    return home
