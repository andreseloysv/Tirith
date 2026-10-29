"""Environment variables per project (see ``docs/CONTRACT-projenv.md``).

Each project keeps its variables in ``<home>/env/<project>.toml`` (mode 0600): plain ``KEY = "value"``
lines.  Every stage run, lane and Run action of the project gets them on top of the inherited
environment.  The values are shown in full on the Settings page, on purpose.
"""

from __future__ import annotations

import json
import os
import re
import tomllib
from pathlib import Path

from tirith.store import write_text_atomic

ENV_DIR = "env"
KEY_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
PROJECT_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*$")


class EnvError(ValueError):
    """A key or project id that cannot be stored; the text says how to fix it."""


def path_for(home: Path, project: str) -> Path:
    if not PROJECT_RE.match(project or ""):
        raise EnvError(f"bad project id {project!r}. Fix: use letters, digits, '.', '_' and '-'")
    return Path(home) / ENV_DIR / f"{project}.toml"


def check(env: dict) -> dict[str, str]:
    """``env`` with validated keys and string values, else :class:`EnvError`."""
    out: dict[str, str] = {}
    for key, value in env.items():
        if not KEY_RE.match(str(key)):
            raise EnvError(f"bad variable name {key!r}. Fix: use [A-Za-z_][A-Za-z0-9_]*")
        value = "" if value is None else str(value)
        if "\0" in value:
            raise EnvError(f"the value of {key} holds a NUL character. Fix: remove it")
        out[str(key)] = value
    return out


def load(home: Path, project: str) -> dict[str, str]:
    """The project's variables; a missing, broken or badly named file gives ``{}``."""
    try:
        raw = tomllib.loads(path_for(home, project).read_text(encoding="utf-8"))
    except (OSError, EnvError, tomllib.TOMLDecodeError):
        return {}
    return {k: str(v) for k, v in raw.items() if KEY_RE.match(k) and not isinstance(v, (dict, list))}


def save(home: Path, project: str, env: dict) -> dict[str, str]:
    """Replace the project's variables with ``env`` (0600); an empty ``env`` removes the file."""
    env = check(env)
    path = path_for(home, project)
    if not env:
        path.unlink(missing_ok=True)
        return {}
    write_text_atomic(path, "".join(f"{k} = {json.dumps(v)}\n" for k, v in env.items()))
    os.chmod(path, 0o600)
    return env


def set_vars(home: Path, project: str, changes: dict) -> dict[str, str]:
    return save(home, project, {**load(home, project), **check(changes)})


def unset_vars(home: Path, project: str, keys: list[str]) -> dict[str, str]:
    env = load(home, project)
    for key in keys:
        env.pop(key, None)
    return save(home, project, env)


def apply(home: Path, project: str | None, env: dict | None = None) -> dict:
    """``env`` (default: the process environment) with the project's variables on top."""
    out = dict(os.environ if env is None else env)
    if project:
        out.update(load(home, project))
    return out
