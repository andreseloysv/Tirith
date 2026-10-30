"""The continuous collector reloads itself when its own code changed (a merge on main).

Without this a merged change stays invisible until someone restarts the collector by hand. ``CodeWatch`` takes a
cheap fingerprint (path, mtime, size) of ``tirith/**/*.py`` and ``pyproject.toml`` every cycle and says "reload"
once the code differs from what was loaded and has stayed unchanged for ``STABLE_S`` (a merge rewrites many files
over a moment). ``Collector.run`` then returns between cycles, so nothing is cut mid-write; ``tirith collect``
releases the collector lock and re-executes itself in place (same pid, so the desktop app sees nothing).
"""

from __future__ import annotations

import os
import sys
import time
from pathlib import Path
from typing import Callable

STABLE_S = 5.0


class ReloadRequested(Exception):
    """Raised out of the collector loop, between cycles, when the loaded code is stale."""


def code_fingerprint(package_dir: Path) -> tuple:
    files = list(package_dir.rglob("*.py"))
    files.append(package_dir.parent / "pyproject.toml")
    out = []
    for path in sorted(files):
        try:
            stat = path.stat()
        except OSError:
            continue
        out.append((str(path), stat.st_mtime_ns, stat.st_size))
    return tuple(out)


class CodeWatch:
    def __init__(self, package_dir: Path | None = None, stable_s: float = STABLE_S,
                 clock: Callable[[], float] = time.monotonic) -> None:
        self.package_dir = package_dir or Path(__file__).resolve().parent
        self.stable_s = stable_s
        self.clock = clock
        self.loaded = code_fingerprint(self.package_dir)
        self._seen = self.loaded
        self._since = clock()

    def changed(self) -> bool:
        """True once the code differs from the loaded one and has been stable for ``stable_s``."""
        now = self.fingerprint_now()
        if now != self._seen:
            self._seen, self._since = now, self.clock()
        return self._seen != self.loaded and self.clock() - self._since >= self.stable_s

    def fingerprint_now(self) -> tuple:
        return code_fingerprint(self.package_dir)


def reexec_argv() -> list[str]:
    orig = getattr(sys, "orig_argv", None)
    return list(orig) if orig else [sys.executable, *sys.argv]


def reexec(execv: Callable[[str, list[str]], None] = os.execv,
           quiesce: Callable[[], bool] | None = None) -> None:
    """Re-execute in place. ``quiesce`` first stops the background threads (bounded); a thread that outlives it
    is reported, and the exec goes ahead anyway: exec discards threads, a hung collector is the worse outcome."""
    print("tirith collect: code changed; reloading", file=sys.stderr, flush=True)
    if quiesce is not None:
        try:
            clean = quiesce()
        except Exception as exc:  # never let cleanup block the reload
            clean = False
            print(f"tirith collect: quiesce failed ({type(exc).__name__}: {exc})", file=sys.stderr, flush=True)
        if not clean:
            print("tirith collect: background work still running; reloading anyway", file=sys.stderr, flush=True)
    argv = reexec_argv()
    execv(argv[0] if os.path.isabs(argv[0]) else sys.executable, argv)
