"""Manual proof: `tirith lane run --interactive` with the real `anthropic` agent in a real terminal.

Not collected by pytest. Run it by hand (one tiny real agent call):

    python3 collector/tests/manual/interactive_real.py [--home DIR] [--keep]

It runs the lane inside a pseudo-terminal, types one tiny prompt, waits for the reply, exits the
agent and checks the lane record and the hook events tagged with the lane. By default it uses a
throw-away TIRITH_HOME holding a copy of ~/.tirith/config.toml; `--home` uses another one
(for example ~/.tirith itself). Needs the agent's hooks installed for the event check
(`tirith hook install --agent anthropic`); without them that check is reported as skipped.
"""
import argparse
import fcntl
import json
import os
import pty
import re
import select
import shutil
import struct
import sys
import tempfile
import termios
import time
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
TIRITH = ROOT / "bin" / "tirith"
PROMPT = "reply with the word ok and nothing else"
ANSI = re.compile(rb"\x1b\[[0-9;?<>=]*[ -/]*[@-~]|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b[()][A-Za-z0-9]|\x1b[=>]")


class Term:
    def __init__(self, argv, env):
        self.buf = b""
        pid, fd = pty.fork()
        if pid == 0:
            os.execvpe(argv[0], argv, env)
        self.pid, self.fd = pid, fd
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 140, 0, 0))
        self.status = None

    def pump(self, seconds):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            if select.select([self.fd], [], [], 0.2)[0]:
                try:
                    data = os.read(self.fd, 65536)
                except OSError:
                    return False
                if not data:
                    return False
                self.buf += data
        return True

    def text(self):
        return ANSI.sub(b"", self.buf).decode("utf-8", "replace")

    def wait_for(self, pattern, timeout, after=0):
        end = time.monotonic() + timeout
        while time.monotonic() < end:
            if re.search(pattern, self.text()[after:]):
                return True
            if not self.pump(0.5):
                return bool(re.search(pattern, self.text()[after:]))
        return False

    def send(self, data):
        os.write(self.fd, data)

    def reap(self, timeout):
        end = time.monotonic() + timeout
        while time.monotonic() < end:
            pid, status = os.waitpid(self.pid, os.WNOHANG)
            if pid:
                self.status = os.waitstatus_to_exitcode(status)
                return True
            self.pump(0.3)
        return False


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--home", default=None, help="TIRITH_HOME to use (default: temp copy of the config)")
    ap.add_argument("--keep", action="store_true")
    args = ap.parse_args()
    real = Path.home() / ".tirith"
    tmp = Path(tempfile.mkdtemp(prefix="tirith-interactive-"))
    home = Path(args.home).expanduser() if args.home else tmp / "home"
    home.mkdir(parents=True, exist_ok=True)
    if not args.home:
        shutil.copy(real / "config.toml", home / "config.toml")
    config = tomllib.loads((home / "config.toml").read_text(encoding="utf-8"))
    agent = config["agents"]["anthropic"]["command"]
    brief = tmp / "brief.md"
    brief.write_text("# Interactive proof\nReply to the owner's prompt. No file changes.\n", encoding="utf-8")
    lane_id = f"interactive-proof-{int(time.time())}"
    env = dict(os.environ, TIRITH_HOME=str(home), TERM="xterm-256color")
    argv = [str(TIRITH), "lane", "run", "--project", "tirith", "--class", "stonewright",
            "--feature", "interactive proof", "--id", lane_id, "--workspace", str(ROOT),
            "--brief", str(brief), "--interactive", "--", agent]
    results = []

    def check(name, ok, detail=""):
        results.append((name, ok))
        print(f"{'ok  ' if ok else 'FAIL'} {name}" + (f": {detail}" if detail else ""))

    term = Term(argv, env)
    # the agent may ask to trust the folder first: accept the default
    if term.wait_for(r"trust|Yes, proceed|Do you want", 12):
        term.send(b"\r")
    ready = term.wait_for(r"(?m)^\s*[>❯]|\? for shortcuts|Try \"", 60)
    check("agent prompt appeared", ready)
    mark = len(term.text())
    term.send(PROMPT.encode())
    time.sleep(0.5)
    term.send(b"\r")
    replied = term.wait_for(r"\bok\b", 90, after=mark + len(PROMPT))
    check("agent replied ok", replied)
    time.sleep(3)  # let the Stop hook land
    term.send(b"/exit")
    time.sleep(0.5)
    term.send(b"\r")
    if not term.reap(20):
        term.send(b"\x03")
        time.sleep(0.5)
        term.send(b"\x03")
        term.reap(20)
    check("tirith exited", term.status is not None, f"exit code {term.status}")
    if term.status is None:
        os.kill(term.pid, 9)
    term.pump(1)

    lane_file = home / "lanes" / f"{lane_id}.json"
    lane = json.loads(lane_file.read_text(encoding="utf-8")) if lane_file.is_file() else {}
    print("\nlane record:\n" + json.dumps(lane, indent=2, ensure_ascii=False))
    check("lane registered", bool(lane))
    check("lane ended", bool(lane.get("ended_at")) and lane.get("result") in ("done", "failed"),
          f"result={lane.get('result')}")
    events = []
    ev = home / "events.jsonl"
    if ev.is_file():
        for line in ev.read_text(encoding="utf-8").splitlines():
            try:
                row = json.loads(line)
            except ValueError:
                continue
            if row.get("kind") == "hook" and row.get("lane") == lane_id:
                events.append(row)
    print("\nhook events tagged with the lane: " + (", ".join(e.get("event", "?") for e in events) or "none"))
    settings = Path(str(config["agents"]["anthropic"].get("settings", ""))).expanduser()
    hooked = settings.is_file() and "hook --agent anthropic" in settings.read_text(encoding="utf-8")
    if hooked:
        check("hook events tagged with the lane", bool(events), f"{len(events)} events")
        if events:
            check("Stop event seen", any(e.get("event") == "Stop" for e in events))
    else:
        print("skip hook events: hooks are not installed for the agent")
    print("\nterminal tail:\n" + "\n".join(term.text().splitlines()[-15:]))
    if not args.keep and not args.home:
        shutil.rmtree(tmp, ignore_errors=True)
    else:
        print(f"\nkept: {tmp}")
    return 0 if all(ok for _, ok in results) else 1


if __name__ == "__main__":
    sys.exit(main())
