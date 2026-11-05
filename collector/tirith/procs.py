"""A snapshot of the process table without running ``ps``.

The test sandbox denies executing ``ps``, which silently broke tree kills, the idle-CPU probe and the zombie check.
On macOS this reads libproc in-process, on Linux /proc; ``ps`` is only the last fallback.
"""
from __future__ import annotations

import ctypes
import os
import subprocess
import sys
from dataclasses import dataclass


@dataclass(frozen=True)
class Proc:
    pid: int
    ppid: int
    pgid: int
    cpu: float | None  # CPU seconds used so far (None when unknown)
    zombie: bool = False


class _BsdInfo(ctypes.Structure):
    _fields_ = [("flags", ctypes.c_uint32), ("status", ctypes.c_uint32), ("xstatus", ctypes.c_uint32),
                ("pid", ctypes.c_uint32), ("ppid", ctypes.c_uint32), ("uid", ctypes.c_uint32),
                ("gid", ctypes.c_uint32), ("ruid", ctypes.c_uint32), ("rgid", ctypes.c_uint32),
                ("svuid", ctypes.c_uint32), ("svgid", ctypes.c_uint32), ("rfu1", ctypes.c_uint32),
                ("comm", ctypes.c_char * 16), ("name", ctypes.c_char * 32), ("nfiles", ctypes.c_uint32),
                ("pgid", ctypes.c_uint32), ("pjobc", ctypes.c_uint32), ("tdev", ctypes.c_uint32),
                ("tpgid", ctypes.c_uint32), ("nice", ctypes.c_int32), ("start_s", ctypes.c_uint64),
                ("start_us", ctypes.c_uint64)]


class _TaskInfo(ctypes.Structure):
    _fields_ = [("virtual", ctypes.c_uint64), ("resident", ctypes.c_uint64), ("user", ctypes.c_uint64),
                ("system", ctypes.c_uint64), ("user_t", ctypes.c_uint64), ("system_t", ctypes.c_uint64),
                ("policy", ctypes.c_int32), ("faults", ctypes.c_int32), ("pageins", ctypes.c_int32),
                ("cow", ctypes.c_int32), ("msgs_sent", ctypes.c_int32), ("msgs_recv", ctypes.c_int32),
                ("syscalls_mach", ctypes.c_int32), ("syscalls_unix", ctypes.c_int32),
                ("csw", ctypes.c_int32), ("threads", ctypes.c_int32), ("running", ctypes.c_int32)]


class _Timebase(ctypes.Structure):
    _fields_ = [("numer", ctypes.c_uint32), ("denom", ctypes.c_uint32)]


def _darwin() -> list[Proc]:
    lib = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
    base = _Timebase()
    ctypes.CDLL("/usr/lib/libSystem.dylib").mach_timebase_info(ctypes.byref(base))
    scale = (base.numer or 1) / (base.denom or 1) / 1e9
    count = lib.proc_listallpids(None, 0)
    if count <= 0:
        raise OSError("proc_listallpids failed")
    pids = (ctypes.c_int * (count + 64))()
    count = lib.proc_listallpids(pids, ctypes.sizeof(pids))
    found = []
    for pid in list(pids)[:max(count, 0)]:
        info, task = _BsdInfo(), _TaskInfo()
        if lib.proc_pidinfo(pid, 3, 0, ctypes.byref(info), ctypes.sizeof(info)) != ctypes.sizeof(info):
            continue
        got = lib.proc_pidinfo(pid, 4, 0, ctypes.byref(task), ctypes.sizeof(task))
        cpu = (task.user + task.system) * scale if got == ctypes.sizeof(task) else None
        found.append(Proc(pid, info.ppid, info.pgid, cpu, info.status == 5))
    return found


def _linux() -> list[Proc]:
    ticks = os.sysconf("SC_CLK_TCK")
    found = []
    for name in os.listdir("/proc"):
        if not name.isdigit():
            continue
        try:
            with open(f"/proc/{name}/stat") as fh:
                rest = fh.read().rpartition(")")[2].split()
        except OSError:
            continue
        found.append(Proc(int(name), int(rest[1]), int(rest[2]), (int(rest[11]) + int(rest[12])) / ticks,
                          rest[0] == "Z"))
    return found


def _ps() -> list[Proc]:
    out = subprocess.run(["ps", "-A", "-o", "pid=,ppid=,pgid=,stat="], capture_output=True, text=True,
                         timeout=5).stdout
    found = []
    for line in out.splitlines():
        f = line.split()
        if len(f) == 4 and all(x.isdigit() for x in f[:3]):
            found.append(Proc(int(f[0]), int(f[1]), int(f[2]), None, f[3].startswith("Z")))
    return found


def snapshot() -> list[Proc]:
    """Every process visible to us; empty when no method works."""
    methods = [_darwin] if sys.platform == "darwin" else [_linux]
    for method in (*methods, _ps):
        try:
            found = method()
        except (OSError, subprocess.SubprocessError, AttributeError, ValueError, IndexError):
            continue
        if found:
            return found
    return []


class _VnodePath(ctypes.Structure):
    _fields_ = [("info", ctypes.c_char * 152), ("path", ctypes.c_char * 1024)]


class _VnodePathInfo(ctypes.Structure):
    _fields_ = [("cdir", _VnodePath), ("rdir", _VnodePath)]


def locations(pid: int) -> tuple[str, str]:
    """``(exe, cwd)`` of a process; ``""`` for what cannot be read (gone, or owned by someone else)."""
    exe = cwd = ""
    if sys.platform == "darwin":
        try:
            lib = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
            buf = ctypes.create_string_buffer(4096)
            if lib.proc_pidpath(pid, buf, ctypes.sizeof(buf)) > 0:
                exe = buf.value.decode("utf-8", "replace")
            info = _VnodePathInfo()
            if lib.proc_pidinfo(pid, 9, 0, ctypes.byref(info), ctypes.sizeof(info)) == ctypes.sizeof(info):
                cwd = info.cdir.path.decode("utf-8", "replace")
        except (OSError, AttributeError):
            pass
        return exe, cwd
    for name, kind in (("exe", "exe"), ("cwd", "cwd")):
        try:
            value = os.readlink(f"/proc/{pid}/{name}")
        except OSError:
            value = ""
        if kind == "exe":
            exe = value
        else:
            cwd = value
    return exe, cwd
