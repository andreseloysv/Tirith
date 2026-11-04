"""One continuous collector per Tirith home, enforced by an advisory lock instead of a process search.

``tirith collect`` holds an exclusive non-blocking ``flock`` on ``<TIRITH_HOME>/collector.lock`` for its whole
life and writes its pid into the file. The lock dies with the process, so a crash never leaves a stale
"running" answer. The desktop app asks the same question by trying to take the lock. See
``docs/CONTRACT.md`` ("Collector lock").
"""

from __future__ import annotations

import contextlib
import fcntl
import os
import subprocess
import sys
from pathlib import Path

LOCK_NAME = "collector.lock"
LAUNCHD_LABEL = "com.tirith.collector"
LAUNCHD_PLIST = Path("launchd") / f"{LAUNCHD_LABEL}.plist"  # under the home: a managed Mac may lock ~/Library/LaunchAgents


class CollectorRunning(Exception):
    """Another collector holds the lock; ``pid`` is its pid when the file says so."""

    def __init__(self, pid: int | None) -> None:
        super().__init__(f"a collector is already running, pid {pid if pid else '?'}")
        self.pid = pid


def _read_pid(handle) -> int | None:
    handle.seek(0)
    text = handle.read().strip()
    return int(text) if text.isdigit() else None


@contextlib.contextmanager
def hold_collector_lock(home: Path):
    """Hold the collector lock for the ``with`` body; raise :class:`CollectorRunning` if it is taken."""
    home.mkdir(parents=True, exist_ok=True)
    with (home / LOCK_NAME).open("a+") as handle:
        try:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError:
            raise CollectorRunning(_read_pid(handle)) from None
        handle.seek(0)
        handle.truncate()
        handle.write(f"{os.getpid()}\n")
        handle.flush()
        yield


def running_pid(home: Path) -> int | None:
    """The pid of the collector holding the lock, 0 when it holds it without a pid in the file, None when free.
    Only tries the lock (released at once): a cheap check before the expensive start-up."""
    path = home / LOCK_NAME
    if not path.exists():
        return None
    with path.open("a+") as handle:
        try:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError:
            return _read_pid(handle) or 0
        fcntl.flock(handle, fcntl.LOCK_UN)
    return None


def ensure_launchd(home: Path, run=subprocess.run) -> str:
    """Register the owner's launchd fallback (``<home>/launchd/com.tirith.collector.plist``) when it is not loaded.

    A job loaded from outside ``~/Library/LaunchAgents`` lasts until logout, so every collector start loads it
    again: after a restart, opening the app once brings back the collector that keeps Tirith running when the app
    is closed. Returns ``"loaded"``, ``"present"`` or ``"none"`` (no file, not macOS, or launchctl refused)."""
    plist = home / LAUNCHD_PLIST
    if sys.platform != "darwin" or not plist.is_file():
        return "none"
    domain = f"gui/{os.getuid()}"
    try:
        if run(["launchctl", "print", f"{domain}/{LAUNCHD_LABEL}"], capture_output=True, timeout=10).returncode == 0:
            return "present"
        done = run(["launchctl", "bootstrap", domain, str(plist)], capture_output=True, timeout=10)
    except (OSError, subprocess.TimeoutExpired):
        return "none"
    return "loaded" if done.returncode == 0 else "none"
