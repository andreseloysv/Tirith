#!/usr/bin/env python3
"""Exercise browser persistence and the network proxy without real accounts or an LLM.

Run setup-codex-browser.py first. Uses its installed runtime and a disposable browser profile.
"""
import base64
import json
import os
from pathlib import Path
import queue
import shutil
import subprocess
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

root = Path(__file__).resolve().parents[2]
runtime = Path(os.environ.get("TIRITH_HOME", Path.home() / ".tirith")) / "browser-mcp/node_modules"
if not runtime.exists():
    raise SystemExit("Run scripts/setup-codex-browser.py first.")
authorization = "Basic " + base64.b64encode(b"browser-check:local-test").decode()
seen = []


class Proxy(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    def log_message(self, *_):
        pass

    def do_CONNECT(self):
        assert self.headers.get("Proxy-Authorization") == authorization
        seen.append("CONNECT " + self.path)
        self.send_response(403)
        self.send_header("Content-Length", "0")
        self.send_header("Connection", "close")
        self.end_headers()

    def do_GET(self):
        if self.headers.get("Proxy-Authorization") != authorization:
            self.send_response(407)
            self.send_header("Proxy-Authenticate", 'Basic realm="browser-check"')
            self.send_header("Content-Length", "0")
            self.send_header("Connection", "close")
            self.end_headers()
            return
        seen.append(self.path)
        blocked = "blocked.invalid" in self.path
        body = f"<title>{'Blocked by proxy' if blocked else 'Browser check'}</title>".encode()
        self.send_response(403 if blocked else 200)
        self.send_header("Content-Type", "text/html")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


class Client:
    def __init__(self, home):
        env = dict(os.environ, TIRITH_HOME=str(home), HTTPS_PROXY=f"http://browser-check:local-test@127.0.0.1:{server.server_port}")
        command = [shutil.which("node"), str(root / "scripts/codex-browser.mjs"), "--headless", "--timeout-navigation", "10000"]
        if Path("/usr/bin/sandbox-exec").exists():
            env["TIRITH_SANDBOXED"] = "1"
            command = ["/usr/bin/sandbox-exec", "-p", '(version 1)(allow default)(deny network-outbound)(allow network-outbound (remote ip "localhost:*"))(allow network-outbound (remote unix-socket))'] + command
        self.log = (home / "mcp.log").open("a")
        self.process = subprocess.Popen(command, env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.log, text=True)
        self.queue, self.seq = queue.Queue(), 0
        threading.Thread(target=self.read, daemon=True).start()
        self.call("initialize", dict(protocolVersion="2024-11-05", capabilities={}, clientInfo=dict(name="tirith-browser-check", version="1")))
        self.send(dict(jsonrpc="2.0", method="notifications/initialized"))

    def read(self):
        for line in self.process.stdout:
            self.queue.put(json.loads(line))
        self.queue.put(dict(error="MCP stopped; see mcp.log"))

    def send(self, message):
        self.process.stdin.write(json.dumps(message) + "\n")
        self.process.stdin.flush()

    def call(self, method, params):
        print(method, params.get("name", ""), flush=True)
        self.seq += 1
        self.send(dict(jsonrpc="2.0", id=self.seq, method=method, params=params))
        while True:
            result = self.queue.get(timeout=90)
            assert "error" not in result, result
            if result.get("id") == self.seq:
                return result["result"]

    def tool(self, name, **arguments):
        result = self.call("tools/call", dict(name=name, arguments=arguments))
        assert not result.get("isError"), result
        return "\n".join(c.get("text", "") for c in result.get("content", []))

    def close(self):
        self.process.terminate()
        self.process.wait(timeout=10)
        self.log.close()


server = ThreadingHTTPServer(("127.0.0.1", 0), Proxy)
threading.Thread(target=server.serve_forever, daemon=True).start()
try:
    with tempfile.TemporaryDirectory(prefix="tirith-browser-check-") as directory:
        home = Path(directory)
        (home / "browser-mcp").mkdir()
        (home / "browser-mcp/node_modules").symlink_to(runtime)
        for iteration in range(2):
            client = Client(home)
            try:
                names = {t["name"] for t in client.call("tools/list", {})["tools"]}
                assert {"browser_navigate", "browser_snapshot", "browser_close"} <= names
                assert "Browser check" in client.tool("browser_navigate", url="http://tirith-check.invalid/")
                if iteration == 0:
                    client.tool("browser_evaluate", function="() => { localStorage.setItem('tirith-check', 'persisted'); return 'saved'; }")
                else:
                    assert "persisted" in client.tool("browser_evaluate", function="() => localStorage.getItem('tirith-check')")
                assert "Blocked by proxy" in client.tool("browser_navigate", url="http://blocked.invalid/")
                refused = client.call("tools/call", dict(name="browser_navigate", arguments=dict(url="https://blocked.invalid/")))
                assert refused.get("isError"), refused
                assert "CONNECT blocked.invalid:443" in seen
                client.tool("browser_close")
            finally:
                client.close()
        assert sum("tirith-check.invalid" in url for url in seen) >= 2, seen
        print("PASS: MCP tools, authenticated network proxy, blocked destination, persistent profile across restarts")
finally:
    server.shutdown()
