"""Adaptive parallelism: how many lanes and stage runs may go at once, from CPU load and free disk.

The limit is the configured maximum, lowered while the CPU left over by Tirith's own processes (lanes, CI stages,
reviews and everything they spawn, measured over at least ``TIRITH_SAMPLE_S`` seconds) is low or the disk is nearly
full, and never below 1 (work always moves). Other apps' CPU does not count. Without process stats it falls back to
the 1-minute load average. Stage runs wait for a slot (a lock file per slot)."""

from __future__ import annotations

import os
import re
import subprocess
import sys
import time
from contextlib import contextmanager
from pathlib import Path
from typing import Callable, Iterator

try:
    import fcntl
except ImportError:  # Windows: no slots, the limit is advisory
    fcntl = None  # type: ignore[assignment]

SUITE_WAIT_S = 3 * 3600.0  # a full suite waits this long for the machine's one full-suite slot
HIGH_LOAD = 1.0  # load per core at which the limit starts to drop (fallback when no CPU stats)
LOW_IDLE_PCT = 25.0  # free CPU (idle %) under which the limit starts to drop
TIRITH_SAMPLE_S = 2.0  # least window over which Tirith's own CPU is measured
IDLE_SAMPLE_S = 0.1  # window between the two host CPU counter readings
GB = 1024 ** 3


def cores() -> int:
    return os.cpu_count() or 1


def load_average() -> float:
    try:
        return os.getloadavg()[0]
    except (OSError, AttributeError):
        return 0.0


def _cpu_ticks() -> tuple[float, float] | None:
    """``(idle, total)`` CPU ticks of the whole host since boot, or None when the platform has no counters."""
    try:
        if sys.platform.startswith("linux"):
            parts = Path("/proc/stat").read_text(encoding="utf-8").splitlines()[0].split()
            values = [float(v) for v in parts[1:9]]  # user nice system idle iowait irq softirq steal
            return values[3] + values[4], sum(values)
        if sys.platform == "darwin":
            import ctypes
            lib = ctypes.CDLL("/usr/lib/libSystem.B.dylib")
            lib.mach_host_self.restype = ctypes.c_uint
            count = ctypes.c_uint(4)
            info = (ctypes.c_uint * 4)()  # user system idle nice
            # HOST_CPU_LOAD_INFO = 3
            if lib.host_statistics(lib.mach_host_self(), 3, info, ctypes.byref(count)) != 0:
                return None
            return float(info[2]), float(sum(info))
    except Exception:  # noqa: BLE001 - any failure means "no stats", never a crash
        return None
    return None


def cpu_idle(sample: float = IDLE_SAMPLE_S) -> float | None:
    """Real free CPU of the host as an idle percentage (0-100) over a short window, or None if unavailable."""
    first = _cpu_ticks()
    if first is None:
        return None
    time.sleep(sample)
    second = _cpu_ticks()
    if second is None:
        return None
    total = second[1] - first[1]
    if total <= 0:
        return None
    return min(max((second[0] - first[0]) / total * 100.0, 0.0), 100.0)


def _parse_cputime(text: str) -> float:
    """``[DD-][hh:]mm:ss[.cc]`` as seconds (the ``ps`` cputime column); 0 when unreadable."""
    try:
        days, _, rest = text.rpartition("-")
        seconds = 0.0
        for part in rest.split(":"):
            seconds = seconds * 60 + float(part)
        return seconds + (int(days) * 86400 if days else 0)
    except ValueError:
        return 0.0


def tirith_cpu_seconds() -> dict[int, float] | None:
    """CPU seconds used so far by each of Tirith's own processes (any process whose command line mentions tirith,
    plus all their descendants: agents, stage runners, test runs), or None when ``ps`` is unavailable."""
    try:
        out = subprocess.run(["ps", "-Ao", "pid=,ppid=,cputime=,args="], capture_output=True, text=True,
                             timeout=10, check=True).stdout
    except (OSError, subprocess.SubprocessError):
        return None
    rows: dict[int, tuple[int, float, str]] = {}
    for line in out.splitlines():
        m = re.match(r"\s*(\d+)\s+(\d+)\s+(\S+)\s*(.*)", line)
        if m and "ps -Ao" not in m[4]:
            rows[int(m[1])] = (int(m[2]), _parse_cputime(m[3]), m[4])
    own = {pid for pid, (_, _, args) in rows.items() if "tirith" in args.lower()}
    own.add(os.getpid())
    grew = True
    while grew:  # descendants of Tirith's processes are Tirith's too
        grew = False
        for pid, (parent, _, _) in rows.items():
            if pid not in own and parent in own:
                own.add(pid)
                grew = True
    return {pid: rows[pid][1] for pid in own if pid in rows}


_history: list[tuple[float, dict[int, float]]] = []  # recent (monotonic time, per-process CPU seconds) readings
_HISTORY_KEEP_S = 30.0


def tirith_busy_cores(sample: float = TIRITH_SAMPLE_S, *, reader: Callable[[], dict[int, float] | None] | None = None,
                      clock: Callable[[], float] = time.monotonic,
                      sleep: Callable[[float], None] = time.sleep) -> float | None:
    """Cores kept busy by Tirith's own processes, averaged over at least ``sample`` seconds: the rate between a
    reading at least that old (kept from earlier calls, so a long-running caller never waits) and a fresh one.
    None when the platform cannot tell."""
    reader = reader or tirith_cpu_seconds
    now = reader()
    if now is None:
        return None
    t_now = clock()
    old = [(t, snap) for t, snap in _history if t_now - t >= sample]
    if old:
        t0, base = old[-1]  # the newest reading that is old enough
    else:
        t0, base = _history[0] if _history else (t_now, now)
        if sample - (t_now - t0) > 0:
            sleep(sample - (t_now - t0))
            now = reader()
            if now is None:
                return None
            t_now = clock()
    _history.append((t_now, now))
    _history[:] = [(t, snap) for t, snap in _history if t_now - t <= _HISTORY_KEEP_S]
    if t_now - t0 <= 0:
        return None
    # a process that exited between the readings is missed; a new one counts all its CPU
    return sum(max(cpu - base.get(pid, 0.0), 0.0) for pid, cpu in now.items()) / (t_now - t0)


def tirith_idle(sample: float = TIRITH_SAMPLE_S, *, cpus: int | None = None) -> float | None:
    """Percentage of the machine's CPU left after Tirith's own processes (100 - their share), or None if unknown."""
    busy = tirith_busy_cores(sample)
    if busy is None:
        return None
    return min(max(100.0 - busy / max(cpus or cores(), 1) * 100.0, 0.0), 100.0)


def limit(maximum: int, *, cpus: int | None = None, load: float | None = None, idle: float | None = None,
          free_bytes: int | None = None, min_free_bytes: int = 0) -> tuple[int, str]:
    """``(allowed, why)``: ``maximum`` lowered by low free CPU and by low disk; ``why`` is empty at full speed.
    ``idle`` is the CPU percentage left by Tirith's own processes (measured when neither ``idle`` nor ``load`` is
    given); the 1-minute ``load`` average is only the fallback when the processes cannot be read."""
    maximum = max(int(maximum), 1)
    cpus = max(cpus or cores(), 1)
    if idle is None and load is None:
        idle = tirith_idle(cpus=cpus)
    allowed, why = maximum, ""
    if idle is not None:
        free = cpus * idle / 100.0  # cores that are not busy
        if idle < LOW_IDLE_PCT:
            allowed = max(min(maximum, int(free // 2) or 1), 1)
            if allowed < maximum:
                why = f"CPU {idle:.0f}% left by Tirith on {cpus} cores"
    else:
        load = load_average() if load is None else load
        if load / cpus > HIGH_LOAD:
            free = max(cpus - load, 0)
            allowed = max(min(maximum, int(free // 2) or 1), 1)
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


@contextmanager
def full_suite(home: Path, *, wait: float = SUITE_WAIT_S, poll: float = 1.0) -> Iterator[bool]:
    """Hold the machine's one full-suite lock (at most one full suite at a time, whatever the stage slots allow);
    waits up to ``wait`` seconds, then runs anyway. Yields whether the lock is held. Take it before a stage slot."""
    if fcntl is None:
        yield False
        return
    home.mkdir(parents=True, exist_ok=True)
    handle = open(home / "full-suite.lock", "a")
    held = False
    deadline = time.monotonic() + wait
    while True:
        try:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
            held = True
            break
        except OSError:
            if time.monotonic() >= deadline:
                break
            time.sleep(poll)
    try:
        yield held
    finally:
        if held:
            fcntl.flock(handle, fcntl.LOCK_UN)
        handle.close()
