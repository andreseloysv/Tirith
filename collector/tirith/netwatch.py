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


_SECOND_LEVEL = {"co", "com", "net", "org", "gov", "ac", "edu"}
RESOLVE_TTL_S = 600.0
_resolved: dict[str, tuple[float, set[str]]] = {}


def base_domain(name: str) -> str | None:
    """``github.com`` for ``lb-140-82-112-3-iad.github.com``; None for an address or a single label."""
    try:
        ipaddress.ip_address(name)
        return None
    except ValueError:
        pass
    labels = [l for l in str(name or "").lower().rstrip(".").split(".") if l]
    if len(labels) < 2:
        return None
    keep = 3 if len(labels) >= 3 and len(labels[-1]) == 2 and labels[-2] in _SECOND_LEVEL else 2
    return ".".join(labels[-keep:])


def _addresses(domain: str, now: float) -> set[str]:
    hit = _resolved.get(domain)
    if hit and now - hit[0] < RESOLVE_TTL_S:
        return hit[1]
    future = _pool.submit(lambda: {i[4][0] for i in socket.getaddrinfo(domain, None)})
    done, _ = wait([future], timeout=LOOKUP_BUDGET_S)
    try:
        ips = future.result() if done else set()
    except OSError:
        ips = set()
    _resolved[domain] = (now, ips)
    return ips


# domains agents and their tools commonly reach: an address with no reverse name is matched against these
KNOWN_DOMAINS = ("api.anthropic.com", "api.meta.ai", "statsig.anthropic.com", "api.statsig.com", "sentry.io", "api.openai.com",
                 "chatgpt.com", "generativelanguage.googleapis.com", "github.com", "api.github.com",
                 "codeload.github.com", "objects.githubusercontent.com", "raw.githubusercontent.com", "pypi.org",
                 "files.pythonhosted.org", "registry.npmjs.org", "registry.yarnpkg.com", "crates.io",
                 "static.crates.io", "agentsmessenger.com", "http-intake.logs.datadoghq.com", "api.segment.io")


CERT_FILE = "netwatch-certs.json"
_certs: dict[str, dict] = {}
_cert_pending: set[str] = set()


def read_cert(address: str, port: int = 443, run: Callable[[list[str]], str] | None = None) -> dict:
    """The names the server at ``address`` serves, from its TLS certificate (a handshake without a name, like any
    client's first hello; nothing is sent): ``{"domain": "logs.us5.datadoghq.com", "org": "Datadog, Inc."}`` or
    ``{}``. A wildcard name loses its ``*.``; the first DNS name of the certificate wins over its subject."""
    def default(argv: list[str]) -> str:
        try:
            hello = subprocess.run(argv[:4], input="", capture_output=True, text=True, timeout=8)
            return subprocess.run(argv[4:], input=hello.stdout, capture_output=True, text=True, timeout=8).stdout
        except (OSError, subprocess.TimeoutExpired):
            return ""
    text = (run or default)(["openssl", "s_client", "-connect", f"{address}:{port}",
                             "openssl", "x509", "-noout", "-subject", "-ext", "subjectAltName"])
    names = [n.strip()[4:] for n in text.replace(",", "\n").splitlines() if n.strip().startswith("DNS:")]
    subject = next((l for l in text.splitlines() if l.startswith("subject=")), "")
    cn = next((part.split("=", 1)[1].strip() for part in subject[8:].split(",") if part.strip().startswith("CN")), "")
    org = next((part.split("=", 1)[1].strip() for part in subject[8:].split(",") if part.strip().startswith("O")), "")
    if not org and " O=" in subject:
        org = subject.split(" O=", 1)[1].split(",")[0].strip()
    domain = (names[0] if names else cn).lstrip("*.").strip()
    return {"domain": domain, **({"org": org} if org else {})} if "." in domain else {}


def cert_domain(home: Path | None, address: str, port: int = 443, wait_s: float = 0.0) -> dict:
    """``read_cert`` once per address, kept in ``<home>/netwatch-certs.json``; looked up in the background, so a
    first call may return ``{}`` and a later one the names."""
    if not _certs and home is not None:
        _certs.update(read_json(Path(home) / CERT_FILE, {}) or {})
    if address in _certs:
        return _certs[address]
    if address not in _cert_pending:
        _cert_pending.add(address)

        def look() -> None:
            found = read_cert(address, port)
            _certs[address] = found
            _cert_pending.discard(address)
            if home is not None:
                write_json_atomic(Path(home) / CERT_FILE, dict(list(_certs.items())[-SEEN_MAX:]))
        future = _pool.submit(look)
        if wait_s:
            wait([future], timeout=wait_s)
    return _certs.get(address, {})


def known_domain(address: str, resolve: Callable[[str], set[str]] | None = None) -> str | None:
    """The domain of ``KNOWN_DOMAINS`` that resolves to ``address``, else None."""
    resolve = resolve or (lambda d: _addresses(d, time.time()))
    return next((d for d in KNOWN_DOMAINS if address in resolve(d)), None)


def annotate(rows: list[dict], resolve: Callable[[str], set[str]] | None = None, home: Path | None = None,
             certs: Callable[[str], dict] | None = None) -> list[dict]:
    """Connection rows whose domain was not known when written get it now, for the page: a known domain that
    resolves to the address, else the server's certificate (looked up in the background, cached)."""
    certs = certs or (lambda a: cert_domain(home, a))
    for row in rows:
        if row.get("tool") != "connection" or (row.get("domain") and row.get("org") is not None):
            continue
        address = row.get("address") or str((row.get("external") or [""])[0]).split(" ", 1)[-1].rsplit(":", 1)[0]
        if not address:
            continue
        found = row.get("domain") or known_domain(address, resolve)
        cert = {} if found and row.get("domain") else certs(address)
        found = found or cert.get("domain")
        if found:
            row["domain"], row["address"] = found, address
        if cert.get("org"):
            row["org"] = cert["org"]
    return rows


def lane_domains(home: Path, lane: dict) -> list[str]:
    """The domains a lane's own calls named (``external`` of its access rows), newest first."""
    from tirith.store import read_jsonl
    project = str(lane.get("project") or access_mod.LOOSE)
    out = []
    for row in reversed(read_jsonl(Path(home) / access_mod.ACCESS_DIR / f"{project}.ndjson")[-400:]):
        if row.get("lane") != lane.get("id") or row.get("tool") == "connection":
            continue
        for item in row.get("external") or []:
            host = str(item).split(" ", 1)[-1].split(":")[0].strip("[]")
            if "." in host and base_domain(host) and host not in out:
                out.append(host)
    return out[:20]


def domain_of(address: str, reverse: str, domains: list[str], resolve: Callable[[str], set[str]]) -> str | None:
    """The domain visited at ``address``: one the lane named that resolves to it, else the base of the reverse
    name, else None."""
    for domain in [*domains, *KNOWN_DOMAINS]:
        if address in resolve(domain):
            return domain
    if reverse != address and base_domain(reverse):
        return base_domain(reverse)
    return None


BLOCKED_EVERY_S = 3.0  # with blocked destinations the watch looks more often


def _kill(pid: int) -> bool:
    import os
    import signal
    try:
        os.kill(pid, signal.SIGTERM)
        return True
    except OSError:
        return False


def sample(home: Path, lanes: list[dict], now: float | None = None, run: Callable[[list[str]], str] = _run,
           name: Callable[[str], str] = _name, resolve: Callable[[str], set[str]] | None = None,
           kill: Callable[[int], bool] = _kill) -> list[dict]:
    """One pass when ``EVERY_S`` passed: the new connections of the lanes' processes, written to the access log."""
    from tirith import netblock as netblock_mod
    now = time.time() if now is None else now
    block = netblock_mod.rules(home)
    if now - _last["at"] < (BLOCKED_EVERY_S if block else EVERY_S):
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
        hit = netblock_mod.blocked(block, c["host"], _names.get(c["host"]), *(
            [_certs[c["host"]].get("domain")] if c["host"] in _certs else [])) if block else None
        if key in seen and (not hit or c["pid"] in roots):  # a blocked one is stopped each time; the agent's own once
            continue
        seen[key] = now_iso()
        reverse = name(c["host"])
        domain = domain_of(c["host"], reverse, lane_domains(home, lane),
                           resolve or (lambda d: _addresses(d, now)))
        cert = {} if resolve is not None else cert_domain(home, c["host"], c["port"], wait_s=LOOKUP_BUDGET_S)
        domain = domain or cert.get("domain")
        hit = hit or (netblock_mod.blocked(block, domain, reverse) if block else None)
        kind = PORTS.get(c["port"], "tcp")
        runtime = c["pid"] in roots  # the agent's own program (its model API, telemetry), not what its work runs
        project = str(lane.get("project") or access_mod.LOOSE)
        where = domain or c["host"]
        program = "the agent" if runtime else c["program"]
        stopped = bool(hit) and not runtime and kill(c["pid"])
        row = {"ts": now_iso(), "agent": lane.get("agent"), "lane": lane["id"], "runtime": runtime,
               "plan": access_mod.plan_point(home, lane["id"]), "system": "network", "tool": "connection",
               "action": f"{program} connected to {where}:{c['port']}" + (f" ({c['host']})" if domain else ""),
               "external": [f"{kind} {where}:{c['port']}"],
               "domain": domain, "address": c["host"], "reverse": reverse if reverse != c["host"] else None,
               **({"org": cert["org"]} if cert.get("org") else {})}
        if hit:
            row["blocked"] = hit
            row["action"] = (f"stopped {program}: {where}:{c['port']} is blocked (rule {hit})" if stopped else
                             f"{program} reached blocked {where}:{c['port']} (rule {hit}); "
                             + ("the agent program itself was not stopped" if runtime else "it could not be stopped"))
        append_jsonl(home / access_mod.ACCESS_DIR / f"{project}.ndjson", row)
        rows.append(row)
    if rows:
        keep = sorted(seen.items(), key=lambda kv: kv[1])[-SEEN_MAX:]
        write_json_atomic(home / SEEN_FILE, dict(keep))
    return rows
