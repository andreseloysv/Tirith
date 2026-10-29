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
from pathlib import Path

LOCK_NAME = "collector.lock"


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
