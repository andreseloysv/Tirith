"""Tirith sees the owner's shell environment (see ``docs/CONTRACT-agents.md``, "The owner's environment").

The desktop app starts from Finder with launchd's bare environment, so keys the owner exports in their shell
(``GITHUB_TOKEN``, ``META_SI_API_KEY``, ...) would be missing for the brain and for every lane. At collector start
:func:`load` asks the owner's login shell for its environment once and adds the variables that are *missing* from
``os.environ``; what is already set is never replaced. Values are never written anywhere: only the outcome
(:func:`status`, no values, no names) is kept for the health check.
"""

from __future__ import annotations

import os
import re
from pathlib import Path

from tirith import timeouts as timeouts_mod

TIMEOUT_S = 60.0                  # a login shell that takes longer is skipped
MARK = "\n<<tirith-owner-env>>\n"  # printed before ``env -0``: rc files may print anything before it
SKIP = frozenset({"_", "PWD", "OLDPWD", "SHLVL", "TERM_SESSION_ID"})
KEY_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")

_status: dict = {"state": "pending", "added": 0, "reason": ""}


def status() -> dict:
    """``{state: pending|ok|failed, added: N, reason: plain words}``; never a name or a value."""
    return dict(_status)


def _set(state: str, added: int = 0, reason: str = "") -> dict:
    _status.update(state=state, added=added, reason=reason)
    return status()


def parse(raw: bytes) -> dict[str, str]:
    """The ``KEY=VALUE`` pairs of an ``env -0`` dump that follows :data:`MARK` (anything before it is noise)."""
    head, found, body = raw.partition(MARK.encode())
    if not found:
        return {}
    out: dict[str, str] = {}
    for entry in body.split(b"\0"):
        key, eq, value = entry.partition(b"=")
        name = key.decode("utf-8", "replace")
        if eq and KEY_RE.match(name) and name not in SKIP:
            out[name] = value.decode("utf-8", "replace")
    return out


def load(home: Path | None = None, environ: dict | None = None) -> dict:
    """Merge the login shell's missing variables into ``environ`` (default ``os.environ``); returns :func:`status`."""
    environ = os.environ if environ is None else environ
    shell = environ.get("SHELL") or "/bin/zsh"
    argv = [shell, "-l", "-i", "-c", f"printf '{MARK}'; exec env -0"]
    done = timeouts_mod.run(argv, "generic", env=dict(environ), home=home, timeout=TIMEOUT_S,
                            stdin=-3)  # subprocess.DEVNULL
    if done.timed_out:
        return _set("failed", reason=f"the login shell took more than {int(TIMEOUT_S)} s")
    if done.returncode != 0:
        return _set("failed", reason="the login shell could not be started or failed")
    found = parse(done.stdout or b"")
    if not found:
        return _set("failed", reason="the login shell printed no environment")
    added = 0
    for name, value in found.items():
        if name not in environ:
            environ[name] = value
            added += 1
    return _set("ok", added)
