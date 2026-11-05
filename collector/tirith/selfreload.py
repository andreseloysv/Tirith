"""The continuous collector reloads itself when its own code changed (a merge on main).

Without this a merged change stays invisible until someone restarts the collector by hand. ``CodeWatch`` takes a
cheap fingerprint (path, mtime, size) of ``tirith/**/*.py`` and ``pyproject.toml`` every cycle and says "reload"
once the code differs from what was loaded and has stayed unchanged for ``STABLE_S`` (a merge rewrites many files
over a moment). ``Collector.run`` then returns between cycles, so nothing is cut mid-write; ``tirith collect``
releases the collector lock and re-executes itself in place (same pid, so the desktop app sees nothing).
"""

from __future__ import annotations

import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Callable

STABLE_S = 5.0
PREFLIGHT_S = 30.0


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
                 clock: Callable[[], float] = time.monotonic,
                 verify: Callable[[], bool] | None = None) -> None:
        self.verify = verify
        self.package_dir = package_dir or Path(__file__).resolve().parent
        self.stable_s = stable_s
        self.clock = clock
        self.loaded = code_fingerprint(self.package_dir)
        self._seen = self.loaded
        self._since = clock()

    def changed(self) -> bool:
        """True once the code differs from the loaded one and has been stable for ``stable_s``."""
        try:
            now = self.fingerprint_now()
        except Exception:  # a watcher fault must never end the loop
            return False
        if now != self._seen:
            self._seen, self._since = now, self.clock()
        if self._seen == self.loaded or self.clock() - self._since < self.stable_s:
            return False
        if self.verify is not None:
            try:
                ok = bool(self.verify())
            except Exception:
                ok = False
            if not ok:  # broken new code: keep running the old, retry only after the next change
                print("tirith collect: changed code failed its preflight; keeping the running code",
                      file=sys.stderr, flush=True)
                self.loaded = self._seen
                return False
        return True

    def fingerprint_now(self) -> tuple:
        return code_fingerprint(self.package_dir)


def reexec_argv() -> list[str]:
    orig = getattr(sys, "orig_argv", None)
    return list(orig) if orig else [sys.executable, *sys.argv]


def preflight(timeout: float = PREFLIGHT_S) -> bool:
    """Can the new code at least import? Checked in a child with a hard timeout, so a broken or hanging
    import never takes the running collector down with it."""
    try:
        done = subprocess.run([sys.executable, "-c", "import tirith.cli"], capture_output=True,
                              timeout=timeout, cwd=str(Path(__file__).resolve().parent.parent))
    except (OSError, subprocess.SubprocessError):
        return False
    return done.returncode == 0


def reexec(execv: Callable[[str, list[str]], None] = os.execv,
           quiesce: Callable[[], bool] | None = None) -> bool:
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
    try:
        execv(argv[0] if os.path.isabs(argv[0]) else sys.executable, argv)
    except Exception as exc:  # exec failed: the old code is still here, keep running it
        print(f"tirith collect: reload failed ({type(exc).__name__}: {exc}); keeping the running code",
              file=sys.stderr, flush=True)
        return False
    return True
