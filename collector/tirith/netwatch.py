"""Network watch: the connections the agents' processes really open (``docs/CONTRACT-access-audit.md``).

A hook sees the command an agent runs, not what a script does inside it: a Python file that calls an API or a
Node process that opens a websocket. Every ``EVERY_S`` the collector lists the established TCP connections of the
machine (``lsof``), keeps those of a process under a lane (its agent, a command it runs, a test stage), and writes
each new ``(lane, host, port, program)`` once to the lane's access log as ``tool: "connection"``,
``system: "network"`` with ``external: ["<kind> <host>:<port>"]``. Loopback is skipped. Host names come from a
reverse lookup with a short budget, else the address stays. Read only: it never blocks or changes a connection.
"""

from __future__ import annotations

import ipaddress
import socket
import subprocess
import time
from concurrent.futures import ThreadPoolExecutor, wait
from pathlib import Path
from typing import Callable

from tirith import access_audit as access_mod
from tirith.home import now_iso
from tirith.store import append_jsonl, read_json, write_json_atomic

EVERY_S = 15.0
SEEN_FILE = "netwatch.json"
SEEN_MAX = 2000
LOOKUP_BUDGET_S = 1.0
_last = {"at": 0.0}
_names: dict[str, str] = {}
_pool = ThreadPoolExecutor(max_workers=4, thread_name_prefix="tirith-netwatch")
PORTS = {443: "https", 80: "http", 22: "ssh", 9418: "git"}


def _run(argv: list[str]) -> str:
    try:
        return subprocess.run(argv, capture_output=True, text=True, timeout=10).stdout
    except (OSError, subprocess.TimeoutExpired):
        return ""


def connections(run: Callable[[list[str]], str] = _run) -> list[dict]:
    """Established TCP connections: ``{pid, program, host, port}`` (lsof ``-F`` fields, no name lookups)."""
    out, pid, program = [], None, ""
    for line in run(["lsof", "-nP", "-iTCP", "-sTCP:ESTABLISHED", "-FpcnT"]).splitlines():
        tag, value = line[:1], line[1:]
        if tag == "p":
            pid = int(value) if value.isdigit() else None
        elif tag == "c":
            program = value
        elif tag == "n" and "->" in value and pid is not None:
            remote = value.split("->", 1)[1]
            host, _, port = remote.rpartition(":")
            host = host.strip("[]")
            if port.isdigit():
                out.append({"pid": pid, "program": program, "host": host, "port": int(port)})
    return out


def parents(run: Callable[[list[str]], str] = _run) -> dict[int, int]:
    table = {}
    for line in run(["ps", "-A", "-o", "pid=,ppid="]).splitlines():
        parts = line.split()
        if len(parts) == 2 and parts[0].isdigit() and parts[1].isdigit():
            table[int(parts[0])] = int(parts[1])
    return table


def _local(host: str) -> bool:
    try:
        return ipaddress.ip_address(host).is_loopback
    except ValueError:
        return host in ("localhost",)


def _lane_of(pid: int, ppid: dict[int, int], roots: dict[int, str]) -> str | None:
    seen = 0
    while pid and seen < 64:
        if pid in roots:
            return roots[pid]
        pid, seen = ppid.get(pid, 0), seen + 1
    return None


def _name(host: str) -> str:
    if host in _names:
        return _names[host]
    future = _pool.submit(lambda: socket.gethostbyaddr(host)[0])
    done, _ = wait([future], timeout=LOOKUP_BUDGET_S)
    try:
        _names[host] = future.result() if done else host
    except OSError:
        _names[host] = host
    return _names[host]


def sample(home: Path, lanes: list[dict], now: float | None = None, run: Callable[[list[str]], str] = _run,
           name: Callable[[str], str] = _name) -> list[dict]:
    """One pass when ``EVERY_S`` passed: the new connections of the lanes' processes, written to the access log."""
    now = time.time() if now is None else now
    if now - _last["at"] < EVERY_S:
        return []
    _last["at"] = now
    roots = {}
    for lane in lanes:
        if not isinstance(lane, dict) or lane.get("ended_at"):
            continue
        for key in ("pid", "controller_pid"):
            if isinstance(lane.get(key), int):
                roots[lane[key]] = lane
    if not roots:
        return []
    conns = [c for c in connections(run) if not _local(c["host"])]
    if not conns:
        return []
    ppid, home = parents(run), Path(home)
    seen = read_json(home / SEEN_FILE, {}) or {}
    rows = []
    for c in conns:
        lane = _lane_of(c["pid"], ppid, {pid: l for pid, l in roots.items()})
        if lane is None:
            continue
        key = f"{lane['id']}|{c['host']}|{c['port']}|{c['program']}"
        if key in seen:
            continue
        seen[key] = now_iso()
        host = name(c["host"])
        kind = PORTS.get(c["port"], "tcp")
        project = str(lane.get("project") or access_mod.LOOSE)
        row = {"ts": now_iso(), "agent": lane.get("agent"), "lane": lane["id"],
               "plan": access_mod.plan_point(home, lane["id"]), "system": "network", "tool": "connection",
               "action": f"{c['program']} connected to {host}:{c['port']}",
               "external": [f"{kind} {host}:{c['port']}"]}
        append_jsonl(home / access_mod.ACCESS_DIR / f"{project}.ndjson", row)
        rows.append(row)
    if rows:
        keep = sorted(seen.items(), key=lambda kv: kv[1])[-SEEN_MAX:]
        write_json_atomic(home / SEEN_FILE, dict(keep))
    return rows
