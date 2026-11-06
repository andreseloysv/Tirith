"""Network guard for lanes: the agent runs in a macOS sandbox that only lets it reach this machine, and every
outside connection goes through Tirith's proxy, which sees the exact domain and refuses the blocked ones
(``docs/CONTRACT-access-audit.md``).

- **Sandbox** (``sandbox-exec``, no root): a lane's agent process tree may open network connections only to
  ``localhost`` and local sockets. A script that ignores the proxy and dials an address gets "Operation not
  permitted" instead of reaching it.
- **Proxy** (``tirith net serve``, one process for the machine, started by the collector): HTTP ``CONNECT`` and
  plain HTTP on ``127.0.0.1:<port>``. The lane's agent gets ``HTTPS_PROXY`` and friends with its lane id, so each
  connection is the lane's; the proxy checks ``tirith.netblock`` rules (the domain, then its addresses) and
  answers ``403`` for a blocked one, else connects and pipes. Each new ``(lane, host, port, program)`` and every
  refusal is written to the lane's access log (``tool: "proxy"``).

Off with ``[network] sandbox = false`` in ``config.toml``; macOS only. When the proxy is not running a lane starts
without the guard, and ``state.network`` says why.
"""

from __future__ import annotations

import asyncio
import base64
import ctypes
import fcntl
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import time
import tomllib
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from tirith.home import now_iso
from tirith.store import append_jsonl, lane_path, read_json, write_json_atomic

DEFAULT_PORT = 47811
LOCK = "netproxy.lock"
INFO = "netproxy.json"
PROFILE_DIR = "netproxy"
SANDBOXED_ENV = "TIRITH_SANDBOXED"
HEAD_MAX = 65536
CONNECT_TIMEOUT_S = 15.0
PROFILE = """(version 1)
(allow default)
(deny network-outbound)
(allow network-outbound (remote ip "localhost:*"))
(allow network-outbound (remote unix-socket))
"""
PROXY_VARS = ("HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY", "https_proxy", "http_proxy", "all_proxy")
NO_PROXY = "localhost,127.0.0.1,::1"


# -- settings and state ------------------------------------------------------------------------


def settings(home: Path) -> dict:
    """``[network]`` of config.toml: ``sandbox`` (default on) and ``proxy_port``."""
    try:
        raw = tomllib.loads((Path(home) / "config.toml").read_text(encoding="utf-8")).get("network") or {}
    except (OSError, tomllib.TOMLDecodeError):
        raw = {}
    port = raw.get("proxy_port")
    return {"sandbox": raw.get("sandbox", True) is not False,
            "port": port if isinstance(port, int) and 1024 <= port <= 65535 else DEFAULT_PORT}


def available() -> bool:
    return sys.platform == "darwin" and shutil.which("sandbox-exec") is not None


def sandboxed() -> bool:
    """Whether this process already runs in a sandbox (a lane started from inside another lane)."""
    if os.environ.get(SANDBOXED_ENV) == "1":
        return True
    if sys.platform != "darwin":
        return False
    try:
        lib = ctypes.CDLL("/usr/lib/libSystem.dylib")
        lib.sandbox_check.argtypes = [ctypes.c_int, ctypes.c_char_p, ctypes.c_int]
        return lib.sandbox_check(os.getpid(), None, 0) != 0
    except (OSError, AttributeError):
        return False


def running(home: Path) -> int | None:
    """The proxy's pid when it holds its lock, else None (only tries the lock)."""
    path = Path(home) / LOCK
    if not path.exists():
        return None
    with path.open("a+") as handle:
        try:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError:
            info = read_json(Path(home) / INFO, {}) or {}
            return int(info.get("pid") or 0)
        fcntl.flock(handle, fcntl.LOCK_UN)
    return None


def status(home: Path) -> dict:
    """``state.network``: whether lanes run behind the guard, and why not."""
    conf, info = settings(home), read_json(Path(home) / INFO, {}) or {}
    if not conf["sandbox"]:
        return {"guard": "off", "why": "[network] sandbox = false in config.toml"}
    if not available():
        return {"guard": "off", "why": "the sandbox needs macOS (sandbox-exec)"}
    if running(home) is None:
        return {"guard": "off", "why": info.get("error") or "the proxy is not running"}
    return {"guard": "on", "port": info.get("port"), "since": info.get("started_at")}


def ensure(home: Path, spawn=None) -> dict:
    """Start the proxy when the guard is on and it is not running (called by the collector, never from a lane:
    a proxy started inside a sandbox could not reach anything). Returns :func:`status`."""
    conf = settings(home)
    if conf["sandbox"] and available() and running(home) is None and not sandboxed():
        info = read_json(Path(home) / INFO, {}) or {}
        tried = info.get("tried_at")
        if not tried or time.time() - float(tried) > 60:  # a proxy that cannot start is tried once a minute
            write_json_atomic(Path(home) / INFO, {**info, "tried_at": time.time()})
            from tirith import launch as launch_mod
            argv = [*launch_mod.tirith_argv(home), "--home", str(home), "net", "serve"]
            (spawn or launch_mod.spawn_detached)(argv, Path(home), Path(home) / "logs" / "netproxy.log")
    return status(home)


# -- a lane behind the guard -----------------------------------------------------------------------


def profile(home: Path) -> Path:
    path = Path(home) / PROFILE_DIR / "lane.sb"
    if not path.is_file() or path.read_text(encoding="utf-8") != PROFILE:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(PROFILE, encoding="utf-8")
    return path


def lane_env(home: Path, lane_id: str, token: str | None, env: dict) -> dict:
    """``env`` for a lane's agent behind the guard: the proxy (with the lane id) in every proxy variable, and the
    marker that :func:`argv` wraps the command in the sandbox. Unchanged when the guard is off."""
    state = status(home)
    if state["guard"] != "on":
        return env
    url = f"http://{lane_id}:{token or 'x'}@127.0.0.1:{state['port']}"
    out = {**env, **{name: url for name in PROXY_VARS}, "NO_PROXY": NO_PROXY, "no_proxy": NO_PROXY,
           "NODE_USE_ENV_PROXY": "1", SANDBOXED_ENV: "1"}
    return out


def argv(home: Path, command: list[str], env: dict) -> list[str]:
    """The command in the sandbox when :func:`lane_env` put the lane behind the guard (and this process is not in
    one already: a sandbox only narrows, so the lane is confined anyway)."""
    if env.get(SANDBOXED_ENV) != "1" or os.environ.get(SANDBOXED_ENV) == "1" or not available():
        return command
    return ["sandbox-exec", "-f", str(profile(home)), *command]


_SWIFT_LINE = re.compile(r"(?<![\w./-])((?:[\w./-]*/)?swift\s+(?:build|test|run|package))(?=\s|$)(?![^;&|]*--disable-sandbox)")
_SWIFT_SUB = ("build", "test", "run", "package")


LANE_STAGE_ENV = "TIRITH_LANE_STAGE"  # set by run_stage for a stage of a lane (review/merge-queue runs drop TIRITH_LANE)


def nested(env: dict | None) -> bool:
    """Whether a command run with ``env`` may sit inside a sandbox that macOS will not let it nest another in: the lane
    sandbox, the agent's own (``TIRITH_LANE``), or a stage run for a lane by a process started from one."""
    env = env or {}
    return any(env.get(k) or os.environ.get(k) for k in ("TIRITH_LANE", LANE_STAGE_ENV)) \
        or "1" in (env.get(SANDBOXED_ENV), os.environ.get(SANDBOXED_ENV))


def swiftpm(command, env: dict | None = None):
    """``command`` with ``--disable-sandbox`` after ``swift build|test|run|package`` when it runs inside the lane
    sandbox: SwiftPM wraps manifest evaluation in its own ``sandbox-exec``, which macOS refuses to nest
    ("sandbox_apply: Operation not permitted" on Package.swift). The lane is confined by ours already. Only
    those four subcommands change; any other command is returned as it came."""
    if not nested(env):
        return command
    if isinstance(command, str):
        return _SWIFT_LINE.sub(r"\1 --disable-sandbox", command)
    words = [str(w) for w in command]
    if len(words) >= 3 and Path(words[0]).name in ("sh", "bash", "zsh") and re.fullmatch(r"-\w*c", words[1]):
        # `tirith run --stage` hands the manifest line over as ``/bin/sh -c "<line>"``: rewrite the line itself
        return [*command[:2], swiftpm(words[2], env), *command[3:]]
    if len(words) >= 2 and Path(words[0]).name == "swift" and words[1] in _SWIFT_SUB and "--disable-sandbox" not in words:
        return [*command[:2], "--disable-sandbox", *command[2:]]
    return command


def swift_shim(home: Path, env: dict) -> None:
    """Put a ``swift`` wrapper first on ``env["PATH"]`` for a lane stage: ``swiftpm`` only rewrites a literal
    ``swift test`` in the stage command, but a script or ``make`` reaches swift without it and SwiftPM's nested
    ``sandbox-exec`` fails ("zoocore: Invalid manifest"). The wrapper adds ``--disable-sandbox`` to
    build|test|run|package and runs the next ``swift`` on PATH. No-op outside a lane."""
    if not nested(env):
        return
    directory = Path(home) / "shims"
    shim = directory / "swift"
    script = ('#!/bin/sh\n'
              'self="$(cd "$(dirname "$0")" && pwd)"\n'
              "PATH=$(printf %s \"$PATH\" | tr : '\\n' | grep -vxF \"$self\" | paste -sd: -)\n"
              'case "$1" in build|test|run|package)\n'
              '  sub="$1"; shift\n'
              '  case " $* " in *" --disable-sandbox "*) exec swift "$sub" "$@";; esac\n'
              '  exec swift "$sub" --disable-sandbox "$@";;\n'
              'esac\nexec swift "$@"\n')
    try:
        if not shim.exists() or shim.read_text() != script:
            directory.mkdir(parents=True, exist_ok=True)
            shim.write_text(script)
            shim.chmod(0o755)
    except OSError:
        return
    path = env.get("PATH") or os.environ.get("PATH", "")
    if str(directory) not in path.split(os.pathsep):
        env["PATH"] = str(directory) + os.pathsep + path


XCODE_FLAGS = ("-IDEPackageSupportDisableManifestSandbox=YES", "-IDEPackageSupportDisablePluginExecutionSandbox=YES")
XCODE_SWIFT = "OTHER_SWIFT_FLAGS=$(inherited) -disable-sandbox"  # Swift macros (@Observable) need the plugin server unsandboxed
_XCODE_LINE = re.compile(r"(?<![\w./-])(xcodebuild)(?=\s|$)(?![^;&|]*-IDEPackageSupportDisableManifestSandbox)")


def xcodebuild(command, env: dict | None = None):
    """``command`` with the two ``-IDEPackageSupportDisable*Sandbox=YES`` flags after ``xcodebuild`` when it runs
    inside the lane sandbox: Xcode wraps package manifests and plugins in its own ``sandbox-exec``, which macOS
    refuses to nest ("sandbox-exec: Operation not permitted"). Any other command is returned as it came."""
    if not nested(env):
        return command
    if isinstance(command, str):
        extra = "" if "OTHER_SWIFT_FLAGS=" in command else " '" + XCODE_SWIFT + "'"
        return _XCODE_LINE.sub(lambda m: "xcodebuild " + " ".join(XCODE_FLAGS) + extra, command)
    words = [str(w) for w in command]
    if words and Path(words[0]).name == "xcodebuild" and not any(w.startswith(XCODE_FLAGS[0].split("=")[0]) for w in words):
        swift = [] if any(w.startswith("OTHER_SWIFT_FLAGS=") for w in words) else [XCODE_SWIFT]
        return [command[0], *XCODE_FLAGS, *swift, *command[1:]]
    return command


def xcodebuild_shim(home: Path, env: dict) -> None:
    """Put an ``xcodebuild`` wrapper first on ``env["PATH"]`` for a lane stage, so a script or ``make`` that
    reaches xcodebuild gets the flags of :func:`xcodebuild` too. No-op outside a lane."""
    if not nested(env):
        return
    directory = Path(home) / "shims"
    shim = directory / "xcodebuild"
    script = ('#!/bin/sh\n'
              'self="$(cd "$(dirname "$0")" && pwd)"\n'
              "PATH=$(printf %s \"$PATH\" | tr : '\\n' | grep -vxF \"$self\" | paste -sd: -)\n"
              'case " $* " in *" -IDEPackageSupportDisableManifestSandbox"*) exec xcodebuild "$@";; esac\n'
              'case " $* " in *" OTHER_SWIFT_FLAGS="*) exec xcodebuild '
              f'{XCODE_FLAGS[0]} {XCODE_FLAGS[1]} "$@";; esac\n'
              f"exec xcodebuild {XCODE_FLAGS[0]} {XCODE_FLAGS[1]} '{XCODE_SWIFT}' \"$@\"\n")
    try:
        if not shim.exists() or shim.read_text() != script:
            directory.mkdir(parents=True, exist_ok=True)
            shim.write_text(script)
            shim.chmod(0o755)
    except OSError:
        return
    path = env.get("PATH") or os.environ.get("PATH", "")
    if str(directory) not in path.split(os.pathsep):
        env["PATH"] = str(directory) + os.pathsep + path


# -- the proxy -------------------------------------------------------------------------------------


class Proxy:
    """One asyncio server; rules are read again when ``netblock.json`` changes."""

    def __init__(self, home: Path, lookup=None) -> None:
        self.home = Path(home)
        self.seen: set[str] = set()
        self.lanes: dict[str, tuple[float, dict]] = {}
        self.pool = ThreadPoolExecutor(max_workers=4, thread_name_prefix="tirith-netproxy")
        self.lookup = lookup or peer_program
        self.active = 0

    def rules(self) -> list[dict]:
        from tirith import netblock as netblock_mod
        return netblock_mod.rules(self.home)

    def lane(self, lane_id: str | None) -> dict:
        if not lane_id:
            return {}
        hit = self.lanes.get(lane_id)
        if hit and time.time() - hit[0] < 10:
            return hit[1]
        lane = read_json(lane_path(self.home, lane_id), {}) or {}
        self.lanes[lane_id] = (time.time(), lane)
        return lane

    async def verdict(self, host: str, port: int) -> str | None:
        """The rule that blocks ``host`` (as named, then each address it resolves to), else None."""
        from tirith import netblock as netblock_mod
        rules = self.rules()
        if not rules:
            return None
        hit = netblock_mod.blocked(rules, host)
        if hit or not any(netblock_mod._network(r["match"]) for r in rules):
            return hit
        try:
            infos = await asyncio.get_running_loop().getaddrinfo(host, port, type=socket.SOCK_STREAM)
        except OSError:
            return None
        return netblock_mod.blocked(rules, *{i[4][0] for i in infos})

    def record(self, lane_id: str | None, host: str, port: int, peer_port: int, kind: str, rule: str | None,
               what: str, found: tuple | None = None) -> None:
        """One access row for a new (lane, host, port, program) or for any refusal (in the background)."""
        def write() -> None:
            pid, program = found or self.lookup(peer_port)
            lane = self.lane(lane_id)
            runtime = bool(pid) and pid == lane.get("pid")
            who = "the agent" if runtime else (program or "a process")
            key = f"{lane_id}|{host}|{port}|{program}"
            if not rule and key in self.seen:
                return
            self.seen.add(key)
            from tirith import access_audit as access_mod
            project = str(lane.get("project") or access_mod.LOOSE)
            row = {"ts": now_iso(), "agent": lane.get("agent"), "lane": lane_id, "runtime": runtime,
                   "plan": access_mod.plan_point(self.home, lane_id) if lane_id else None, "system": "network",
                   "tool": "proxy", "domain": host,
                   "action": (f"{who} was refused {what} (rule {rule})" if rule else f"{who}: {what}"),
                   "external": [f"{kind} {host}:{port}"]}
            if rule:
                row["blocked"] = rule
            append_jsonl(self.home / access_mod.ACCESS_DIR / f"{project}.ndjson", row)
        self.pool.submit(write)

    async def handle(self, reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
        self.active += 1
        upstream = None
        try:
            head = await asyncio.wait_for(reader.readuntil(b"\r\n\r\n"), 30)
            if len(head) > HEAD_MAX:
                raise ValueError("head too large")
            lines = head.decode("latin-1").split("\r\n")
            method, target, version = (lines[0].split(" ") + ["", "", ""])[:3]
            headers = [l for l in lines[1:] if l]
            lane_id = _lane_of(headers)
            if method.upper() == "CONNECT":
                host, _, port_text = target.rpartition(":")
                host, port, path, kind = host.strip("[]"), int(port_text or 443), "", "https"
            else:
                host, port, path = _split_url(target)
                kind = "http"
            peer_port = (writer.get_extra_info("peername") or ("", 0))[1]
            rule = await self.verdict(host, port)
            what = f"{method.upper()} {host}:{port}" if method.upper() == "CONNECT" else f"{method.upper()} http://{host}{path.split('?')[0]}"
            # a refused client hangs up at once: who it was is looked up while it still waits for the answer
            found = await asyncio.get_running_loop().run_in_executor(self.pool, self.lookup, peer_port) if rule else None
            self.record(lane_id, host, port, peer_port, kind, rule, what, found)
            if rule:
                body = f"Blocked by Tirith: {host} (rule {rule}). The owner blocked it on the Accesos page.\n"
                writer.write((f"HTTP/1.1 403 Forbidden\r\nX-Tirith-Blocked: {rule}\r\nContent-Type: text/plain\r\n"
                              f"Content-Length: {len(body)}\r\nConnection: close\r\n\r\n{body}").encode())
                await writer.drain()
                return
            try:
                up_reader, upstream = await asyncio.wait_for(asyncio.open_connection(host, port), CONNECT_TIMEOUT_S)
            except (OSError, asyncio.TimeoutError) as err:
                writer.write(f"HTTP/1.1 502 Bad Gateway\r\nConnection: close\r\n\r\ncannot reach {host}:{port}: {err}\n".encode())
                await writer.drain()
                return
            if method.upper() == "CONNECT":
                writer.write(b"HTTP/1.1 200 Connection established\r\n\r\n")
                await writer.drain()
            else:
                kept = [h for h in headers if not h.lower().startswith(("proxy-", "connection:"))]
                upstream.write((f"{method} {path or '/'} {version}\r\n" + "".join(h + "\r\n" for h in kept)
                                + "Connection: close\r\n\r\n").encode("latin-1"))
            await asyncio.gather(_pipe(reader, upstream), _pipe(up_reader, writer))
        except (asyncio.IncompleteReadError, asyncio.LimitOverrunError, asyncio.TimeoutError, ValueError,
                ConnectionError, OSError):
            pass
        finally:
            self.active -= 1
            for w in (writer, upstream):
                if w is not None:
                    try:
                        w.close()
                    except Exception:  # noqa: BLE001 - closing a broken pipe
                        pass


async def _pipe(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
    try:
        while True:
            data = await reader.read(65536)
            if not data:
                break
            writer.write(data)
            await writer.drain()
    except (ConnectionError, OSError):
        pass
    finally:
        try:
            writer.write_eof()
        except (OSError, RuntimeError, AttributeError):
            pass


def _lane_of(headers: list[str]) -> str | None:
    for line in headers:
        name, _, value = line.partition(":")
        if name.strip().lower() == "proxy-authorization":
            scheme, _, data = value.strip().partition(" ")
            if scheme.lower() == "basic":
                try:
                    return base64.b64decode(data).decode("utf-8", "replace").split(":", 1)[0] or None
                except ValueError:
                    return None
    return None


def _split_url(target: str) -> tuple[str, int, str]:
    rest = target.split("://", 1)[1] if "://" in target else target
    hostport, _, path = rest.partition("/")
    host, _, port = hostport.rpartition(":") if ":" in hostport and not hostport.endswith("]") else (hostport, "", "")
    return host.strip("[]") or hostport, int(port) if port.isdigit() else 80, "/" + path


def peer_program(peer_port: int) -> tuple[int | None, str | None]:
    """The pid and program on the client side of a loopback connection from ``peer_port`` (lsof)."""
    try:
        out = subprocess.run(["lsof", "-nP", f"-iTCP@127.0.0.1:{peer_port}", "-sTCP:ESTABLISHED", "-Fpc"],
                             capture_output=True, text=True, timeout=5).stdout
    except (OSError, subprocess.TimeoutExpired):
        return None, None
    pid, me = None, os.getpid()
    for line in out.splitlines():
        if line.startswith("p"):
            pid = int(line[1:]) if line[1:].isdigit() else None
        elif line.startswith("c") and pid and pid != me:
            return pid, line[1:]
    return None, None


def serve(home: Path, port: int | None = None) -> int:
    """``tirith net serve``: run the proxy in the foreground, holding its lock (one per machine)."""
    home = Path(home)
    conf = settings(home)
    port = port or conf["port"]
    lock = (home / LOCK).open("a+")
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except OSError:
        print(f"tirith net serve: a proxy is already running (pid {running(home)})", file=sys.stderr)
        return 0
    proxy = Proxy(home)

    async def main() -> None:
        try:
            server = await asyncio.start_server(proxy.handle, "127.0.0.1", port, limit=HEAD_MAX)
        except OSError as err:
            write_json_atomic(home / INFO, {"error": f"cannot listen on 127.0.0.1:{port}: {err}", "tried_at": time.time()})
            raise
        write_json_atomic(home / INFO, {"pid": os.getpid(), "port": port, "started_at": now_iso()})
        print(f"tirith net serve: listening on 127.0.0.1:{port}", flush=True)
        async with server:
            await server.serve_forever()

    try:
        asyncio.run(main())
    except OSError:
        return 1
    except KeyboardInterrupt:
        return 0
    return 0


def describe(home: Path) -> str:
    state = status(home)
    return (f"network guard: on (proxy 127.0.0.1:{state['port']}, since {state.get('since')})" if state["guard"] == "on"
            else f"network guard: off ({state['why']})")


def to_json(home: Path) -> str:
    return json.dumps(status(home))
