"""Adaptive parallelism: how many lanes and stage runs may go at once, from CPU load and free disk.

The limit is the configured maximum, lowered while the load average (per core) is high or the disk is
nearly full, and never below 1 (work always moves). Stage runs wait for a slot (a lock file per slot)."""

from __future__ import annotations

import os
import time
from contextlib import contextmanager
from pathlib import Path
from typing import Callable, Iterator

try:
    import fcntl
except ImportError:  # Windows: no slots, the limit is advisory
    fcntl = None  # type: ignore[assignment]

HIGH_LOAD = 1.0  # load per core at which the limit starts to drop
GB = 1024 ** 3


def cores() -> int:
    return os.cpu_count() or 1


def load_average() -> float:
    try:
        return os.getloadavg()[0]
    except (OSError, AttributeError):
        return 0.0


def limit(maximum: int, *, cpus: int | None = None, load: float | None = None,
          free_bytes: int | None = None, min_free_bytes: int = 0) -> tuple[int, str]:
    """``(allowed, why)``: ``maximum`` lowered by load per core and by low disk; ``why`` is empty at full speed."""
    maximum = max(int(maximum), 1)
    cpus = max(cpus or cores(), 1)
    load = load_average() if load is None else load
    allowed, why = maximum, ""
    per_core = load / cpus
    if per_core > HIGH_LOAD:
        idle = max(cpus - load, 0)  # cores that are not busy
        allowed = max(min(maximum, int(idle // 2) or 1), 1)
        if allowed < maximum:
            why = f"load {load:.1f} on {cpus} cores"
    if free_bytes is not None and min_free_bytes > 0:
        if free_bytes < min_free_bytes:
            allowed, why = 1, f"disk {free_bytes / GB:.1f} GB free, below {min_free_bytes / GB:.1f} GB"
        elif free_bytes < 2 * min_free_bytes and allowed > 1:
            allowed, why = max(allowed // 2, 1), f"disk {free_bytes / GB:.1f} GB free, close to the limit"
    return allowed, why


def current(config, home: Path, maximum: int | None = None,
            free: Callable[[Path], int] | None = None) -> tuple[int, str]:
    """The limit now for ``config`` (its disk guard and autopilot maximum)."""
    from tirith import diskdiet as diskdiet_mod
    free = free or diskdiet_mod.free_bytes
    try:
        smallest = min(free(f) for f in diskdiet_mod.lanes_volumes(home, config))
    except (OSError, ValueError):
        smallest = None
    top = config.autopilot_max_parallel if maximum is None else maximum
    return limit(top, free_bytes=smallest, min_free_bytes=int(config.disk_min_free_gb * GB))


@contextmanager
def slot(home: Path, maximum: int, *, wait: float = 600.0, poll: float = 1.0) -> Iterator[int | None]:
    """Hold one of ``maximum`` stage slots; waits (up to ``wait`` seconds, then runs anyway) while all are taken."""
    if fcntl is None or maximum < 1:
        yield None
        return
    folder = home / "slots"
    folder.mkdir(parents=True, exist_ok=True)
    deadline = time.monotonic() + wait
    handle, number = None, None
    while handle is None:
        for n in range(maximum):
            fh = open(folder / f"{n}.lock", "a")
            try:
                fcntl.flock(fh, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except OSError:
                fh.close()
                continue
            handle, number = fh, n
            break
        if handle is None:
            if time.monotonic() >= deadline:
                break
            time.sleep(poll)
    try:
        yield number
    finally:
        if handle is not None:
            fcntl.flock(handle, fcntl.LOCK_UN)
            handle.close()
