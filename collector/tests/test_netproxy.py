"""The network guard: Tirith's proxy, the lane environment and the sandbox (tirith.netproxy)."""

from __future__ import annotations

import asyncio
import base64
import json
import os
import shutil
import subprocess
import sys
import threading
from pathlib import Path

import pytest

from tirith import netblock, netproxy
from tirith.store import write_lane


def _serve(home: Path):
    """The proxy on an ephemeral port in a thread, plus a local echo server standing in for the outside."""
    loop = asyncio.new_event_loop()
    proxy = netproxy.Proxy(home, lookup=lambda port: (4242, "curl"))
    ready = threading.Event()
    ports = {}

    async def echo(reader, writer):
        data = await reader.read(100)
        writer.write(b"echo:" + data)
        await writer.drain()
        writer.close()

    async def main():
        ps = await asyncio.start_server(proxy.handle, "127.0.0.1", 0)
        es = await asyncio.start_server(echo, "127.0.0.1", 0)
        ports["proxy"], ports["echo"] = ps.sockets[0].getsockname()[1], es.sockets[0].getsockname()[1]
        ready.set()
        await asyncio.Event().wait()

    threading.Thread(target=lambda: loop.run_until_complete(main()), daemon=True).start()
    ready.wait(5)
    return ports, proxy


def _connect(port: int, target: str, lane: str | None = "lane-a") -> tuple[str, object]:
    import socket
    s = socket.create_connection(("127.0.0.1", port), timeout=5)
    auth = f"Proxy-Authorization: Basic {base64.b64encode(f'{lane}:tok'.encode()).decode()}\r\n" if lane else ""
    s.sendall(f"CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n{auth}\r\n".encode())
    return s.recv(4096).decode("latin-1"), s


def _rows(home: Path, project: str = "demo") -> list[dict]:
    import time
    path = home / "access" / f"{project}.ndjson"
    for _ in range(50):
        if path.exists():
            break
        time.sleep(0.05)
    time.sleep(0.2)
    return [json.loads(l) for l in path.read_text().splitlines()] if path.exists() else []


def test_the_proxy_pipes_an_allowed_destination_and_logs_it_for_the_lane(home: Path) -> None:
    write_lane(home, {"id": "lane-a", "project": "demo", "agent": "anthropic", "pid": 4242})
    ports, _ = _serve(home)
    head, sock = _connect(ports["proxy"], f"127.0.0.1:{ports['echo']}")
    assert head.startswith("HTTP/1.1 200")
    sock.sendall(b"hi")
    assert sock.recv(100) == b"echo:hi"
    row = _rows(home)[0]
    assert row["lane"] == "lane-a" and row["tool"] == "proxy" and row["external"] == [f"https 127.0.0.1:{ports['echo']}"]
    assert row["runtime"] is True and row["action"].startswith("the agent: CONNECT")  # pid 4242 is the lane's agent


def test_the_proxy_refuses_a_blocked_domain_and_its_subdomains(home: Path) -> None:
    write_lane(home, {"id": "lane-a", "project": "demo", "pid": 1})
    netblock.add(home, "example.org")
    ports, _ = _serve(home)
    head, _ = _connect(ports["proxy"], "www.example.org:443")
    assert head.startswith("HTTP/1.1 403") and "X-Tirith-Blocked: example.org" in head
    row = _rows(home)[0]
    assert row["blocked"] == "example.org" and row["action"].startswith("curl was refused CONNECT www.example.org:443")


def test_an_address_range_rule_blocks_what_a_name_resolves_to(home: Path) -> None:
    netblock.add(home, "127.0.0.0/8")
    ports, _ = _serve(home)
    head, _ = _connect(ports["proxy"], f"localhost:{ports['echo']}", lane=None)
    assert head.startswith("HTTP/1.1 403")


def test_a_lane_behind_the_guard_gets_the_proxy_and_the_sandbox(home: Path, monkeypatch) -> None:
    monkeypatch.setattr(netproxy, "status", lambda h: {"guard": "on", "port": 47811})
    monkeypatch.setattr(netproxy, "available", lambda: True)
    monkeypatch.delenv(netproxy.SANDBOXED_ENV, raising=False)
    env = netproxy.lane_env(home, "lane-a", "tok", {"PATH": "/bin"})
    assert env["HTTPS_PROXY"] == "http://lane-a:tok@127.0.0.1:47811" == env["http_proxy"] == env["ALL_PROXY"]
    assert env["NO_PROXY"] == "localhost,127.0.0.1,::1" and env["NODE_USE_ENV_PROXY"] == "1"
    argv = netproxy.argv(home, ["agent", "-p", "x"], env)
    assert argv[:2] == ["sandbox-exec", "-f"] and argv[3:] == ["agent", "-p", "x"]
    assert "(deny network-outbound)" in Path(argv[2]).read_text()
    monkeypatch.setenv(netproxy.SANDBOXED_ENV, "1")  # started from inside a lane: confined already
    assert netproxy.argv(home, ["agent"], env) == ["agent"]
    monkeypatch.setattr(netproxy, "status", lambda h: {"guard": "off", "why": "the proxy is not running"})
    assert netproxy.lane_env(home, "lane-a", "tok", {"PATH": "/bin"}) == {"PATH": "/bin"}


def test_the_guard_can_be_turned_off(home: Path) -> None:
    (home / "config.toml").write_text("[network]\nsandbox = false\nproxy_port = 50000\n")
    assert netproxy.settings(home) == {"sandbox": False, "port": 50000}
    assert netproxy.status(home) == {"guard": "off", "why": "[network] sandbox = false in config.toml"}


@pytest.mark.skipif(sys.platform != "darwin" or not shutil.which("sandbox-exec") or netproxy.sandboxed(),
                    reason="the macOS sandbox, from outside one")
def test_inside_the_sandbox_only_localhost_is_reachable(home: Path) -> None:
    code = ("import socket\n"
            "try:\n    socket.create_connection(('192.0.2.1', 443), timeout=2); print('reached')\n"
            "except OSError as e:\n    print('errno', e.errno)\n")
    out = subprocess.run(netproxy.argv(home, [sys.executable, "-c", code], {netproxy.SANDBOXED_ENV: "1"}),
                         capture_output=True, text=True, timeout=30, env={**os.environ, netproxy.SANDBOXED_ENV: ""}).stdout
    assert out.strip() == "errno 1"  # Operation not permitted: the sandbox, not a timeout


def test_swift_in_the_lane_sandbox_gets_disable_sandbox(monkeypatch) -> None:
    """SwiftPM's own sandbox-exec cannot nest in ours: swift build/test/run/package gets --disable-sandbox."""
    box = {netproxy.SANDBOXED_ENV: "1"}
    monkeypatch.delenv(netproxy.SANDBOXED_ENV, raising=False)
    assert netproxy.swiftpm(["swift", "test", "--filter", "A/b"], box) == ["swift", "test", "--disable-sandbox", "--filter", "A/b"]
    assert netproxy.swiftpm(["/usr/bin/swift", "build"], box) == ["/usr/bin/swift", "build", "--disable-sandbox"]
    assert netproxy.swiftpm("cd Packages/Z && swift test --filter X && swift build", box) == \
        "cd Packages/Z && swift test --disable-sandbox --filter X && swift build --disable-sandbox"
    assert netproxy.swiftpm("swift test --disable-sandbox", box) == "swift test --disable-sandbox"
    assert netproxy.swiftpm(["swift", "test", "--disable-sandbox"], box) == ["swift", "test", "--disable-sandbox"]
    for other in (["swift", "--version"], ["swiftlint", "test"], ["echo", "swift", "test"], ["pytest"]):
        assert netproxy.swiftpm(other, box) == other
    assert netproxy.swiftpm("echo swift tests; myswift test", box) == "echo swift tests; myswift test"
    assert netproxy.swiftpm(["swift", "test"], {}) == ["swift", "test"]  # outside a lane sandbox: untouched
    monkeypatch.setenv(netproxy.SANDBOXED_ENV, "1")  # a stage run from inside a lane inherits the marker
    assert netproxy.swiftpm(["swift", "run"], {}) == ["swift", "run", "--disable-sandbox"]


@pytest.mark.skipif(sys.platform != "darwin" or not shutil.which("sandbox-exec") or not shutil.which("swift")
                    or netproxy.sandboxed(), reason="macOS sandbox and swift, from outside a sandbox")
def test_swift_package_builds_under_the_sandbox(home: Path, tmp_path: Path) -> None:
    (tmp_path / "Sources" / "T").mkdir(parents=True)
    (tmp_path / "Package.swift").write_text('// swift-tools-version:5.5\nimport PackageDescription\n'
                                            'let package = Package(name: "T", targets: [.target(name: "T")])\n')
    (tmp_path / "Sources" / "T" / "T.swift").write_text("public let x = 1\n")
    env = {**os.environ, netproxy.SANDBOXED_ENV: ""}
    command = netproxy.swiftpm(["swift", "build", "--scratch-path", str(tmp_path / ".b")], {netproxy.SANDBOXED_ENV: "1"})
    out = subprocess.run(netproxy.argv(home, command, {netproxy.SANDBOXED_ENV: "1"}), cwd=tmp_path,
                         capture_output=True, text=True, timeout=300, env=env)
    assert out.returncode == 0, out.stdout + out.stderr
