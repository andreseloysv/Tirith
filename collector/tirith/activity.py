"""Per-agent goal and live activity: brief goal, git files, tools, tests, log events.

Everything is cheap by design (the collector cycle runs every 2 s): the lane log is read
incrementally and bounded, the brief is cached by mtime, git runs one diff pair per working lane.
Contract: ``docs/CONTRACT-activity.md``.
"""

from __future__ import annotations

import json
import os
import re
import shlex
import subprocess
import threading
import time
from collections import Counter, deque
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
from pathlib import Path

from tirith import timeouts as timeouts_mod
from tirith import hooks as hooks_mod
from tirith.home import now_iso, parse_iso

LOG_BYTES = 512 * 1024
BRIEF_BYTES = 32 * 1024
GOAL_MAX = 400
ACTIVE_WINDOW_S = 300
EVENTS_MAX = 50
TESTS_MAX = 20
FILES_MAX = 100
UNTRACKED_COUNT_BYTES = 256 * 1024
GIT_TIMEOUT_S = 10
PREFETCH_WORKERS = 8
FILES_TTL_S = 15  # a live lane's git file list is re-read at most this often (in the background)

# -- goal -------------------------------------------------------------------------------------

_HEADING = re.compile(r"^\s{0,3}#{1,6}\s+(.*?)\s*#*\s*$")
_GOAL_HEADING = re.compile(r"^(why|goal|goals|objective|purpose|por qu[eé]|objetivo)\b", re.I)


def strip_markdown(text: str) -> str:
    """Plain text of one markdown paragraph (links, emphasis, code marks, bullets removed)."""
    text = re.sub(r"!?\[([^\]]*)\]\([^)]*\)", r"\1", text)
    text = re.sub(r"(\*\*|__|\*|`)", "", text)
    text = re.sub(r"(?m)^\s*(?:[-*+]|\d+[.)])\s+", "", text)
    return re.sub(r"\s+", " ", text).strip()


def _paragraphs(lines: list[str]) -> list[tuple[str | None, str]]:
    """``(heading, paragraph)`` pairs; code fences are skipped, a heading ends a paragraph."""
    out: list[tuple[str | None, str]] = []
    heading: str | None = None
    block: list[str] = []
    fenced = False

    def flush() -> None:
        if block:
            out.append((heading, "\n".join(block)))
            block.clear()

    for line in lines:
        if line.strip().startswith("```"):
            flush()
            fenced = not fenced
        elif fenced:
            continue
        elif (match := _HEADING.match(line)):
            flush()
            heading = match.group(1)
        elif not line.strip():
            flush()
        else:
            block.append(line.strip())
    flush()
    return out


_SETUP_START = re.compile(
    r"^\W*(?:(?:workspace|repo|repository|tests?|branch|worktree|folder|collector)\W*:|read\b)", re.I)
_LANE_PREFIX = re.compile(r"^\s*lane\s*[:\-\u2013\u2014]\s*", re.I)


def _is_setup(paragraph: str) -> bool:
    """True for a paragraph that is set-up instructions (``Workspace: ...``, ``Read the ...``)."""
    return bool(_SETUP_START.match(paragraph))


def _clip_goal(text: str) -> str:
    text = text.strip()
    return text if len(text) <= GOAL_MAX else text[:GOAL_MAX - 1].rstrip() + "\u2026"


def goal_from_text(text: str) -> str | None:
    """First paragraph under a Why/Goal heading (markdown kept, at most 400 chars), else the
    ``#`` title without a "Lane:" prefix. Set-up paragraphs never count."""
    lines = text.splitlines()
    paras = [(h, p) for h, p in _paragraphs(lines) if p.strip()]
    for heading, para in paras:
        if heading and _GOAL_HEADING.match(strip_markdown(heading)) and not _is_setup(para):
            return _clip_goal(para)
    for line in lines:
        match = re.match(r"^#\s+(.*?)\s*#*\s*$", line)
        if match:
            title = _LANE_PREFIX.sub("", match.group(1)).strip()
            if title:
                return _clip_goal(title)
    return None


# -- test detection ---------------------------------------------------------------------------

_OPERATORS = re.compile(r"&&|\|\||[;|\n]")
_ENV_ASSIGN = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")
_WRAPPERS = {"time", "env", "sudo", "nohup", "exec", "command", "noglob", "nice", "caffeinate"}
_RUNNER_PREFIXES = {"uv": ("run",), "poetry": ("run",), "pipenv": ("run",), "pdm": ("run",),
                    "hatch": ("run",), "rye": ("run",), "uvx": (), "npx": (), "bunx": (),
                    "pnpx": ()}
_VALUE_OPTS = {"--project", "--with", "--with-requirements", "--python", "-p", "--directory",
               "--package", "--extra", "--group", "--env-file", "--from", "-C", "--prefix"}
_PKG_MANAGERS = {"npm", "pnpm", "yarn", "bun"}
_PASSED_N = re.compile(r"\b(\d+) (?:passed|passing)\b")
_FAILED_N = re.compile(r"\b(\d+) (?:failed|failing|errors?)\b")
_FAIL_LINE = re.compile(r"^(?:FAIL\b|--- FAIL\b|FAILED\b)|\bBUILD FAILED\b|\bTests? failed\b", re.M)
_OK_LINE = re.compile(r"^ok\s|\bBUILD SUCCESSFUL\b|\btest result: ok\b", re.M)


def _split_commands(command: str) -> list[list[str]]:
    """Simple commands of a shell line as word lists (operators outside quotes split)."""
    text, simple, buf, quote, i = str(command or ""), [], [], None, 0
    while i < len(text):
        ch = text[i]
        if quote:
            quote = None if ch == quote else quote
            buf.append(ch)
        elif ch in "'\"":
            quote = ch
            buf.append(ch)
        else:
            m = _OPERATORS.match(text, i)
            if m:
                simple.append("".join(buf))
                buf, i = [], m.end()
                continue
            buf.append(ch)
        i += 1
    simple.append("".join(buf))
    out = []
    for part in simple:
        try:
            words = shlex.split(part)
        except ValueError:
            words = part.split()
        while words and (_ENV_ASSIGN.match(words[0]) or words[0] in _WRAPPERS):
            words.pop(0)
        if words:
            out.append(words)
    return out


def _skip_opts(words: list[str]) -> list[str]:
    """Words after leading ``-x`` / ``--opt value`` options."""
    i = 0
    while i < len(words) and words[i].startswith("-"):
        i += 2 if words[i] in _VALUE_OPTS else 1
    return words[i:]


def _runs_tests(words: list[str]) -> bool:
    """True when one simple command's program (and subcommand) is a test runner."""
    if not words:
        return False
    prog, rest = os.path.basename(words[0]), words[1:]
    if prog in _RUNNER_PREFIXES:
        need = _RUNNER_PREFIXES[prog]
        if rest[:len(need)] != list(need):
            return False
        return _runs_tests(_skip_opts(rest[len(need):]))
    if re.fullmatch(r"python[\d.]*", prog):
        return rest[:2] == ["-m", "pytest"] or rest[:2] == ["-m", "unittest"]
    if prog in ("pytest", "py.test", "vitest", "jest"):
        return True
    if prog in _PKG_MANAGERS:
        rest = _skip_opts(rest)
        if rest[:1] == ["run"]:
            rest = rest[1:]
        return bool(rest) and re.fullmatch(r"test(?::\S+)?", rest[0]) is not None
    if prog in ("go", "cargo", "swift"):
        return rest[:1] == ["test"]
    if prog in ("gradle", "gradlew"):
        return any(not w.startswith("-") and re.search(r"(^|:)(test\w*|\w*Test)$", w) for w in rest)
    if prog == "make":
        return any(w in ("test", "check") for w in rest)
    if prog == "xcodebuild":
        return any(w in ("test", "test-without-building") for w in rest)
    if prog == "tirith":
        return rest[:1] == ["run"] and any(
            w == "--stage=test" or (w == "--stage" and rest[i + 1:i + 2] == ["test"])
            for i, w in enumerate(rest))
    return False


def _runner_kind(words: list[str]) -> str | None:
    """``"test"`` / ``"build"`` when one simple command's program is a test or build runner."""
    if not words:
        return None
    prog, rest = os.path.basename(words[0]), words[1:]
    if prog == "tirith":
        return None
    if prog in _RUNNER_PREFIXES:
        need = _RUNNER_PREFIXES[prog]
        return _runner_kind(_skip_opts(rest[len(need):])) if rest[:len(need)] == list(need) else None
    if _runs_tests(words):
        return "test"
    if prog in ("go", "cargo", "swift"):
        return "build" if rest[:1] == ["build"] else None
    if prog in _PKG_MANAGERS:
        rest = _skip_opts(rest)
        if rest[:1] == ["run"]:
            rest = rest[1:]
        return "build" if rest and re.fullmatch(r"build(?::\S+)?", rest[0]) else None
    if prog in ("gradle", "gradlew"):
        return "build" if any(not w.startswith("-") and re.search(r"(^|:)(build|assemble\w*)$", w)
                              for w in rest) else None
    if prog == "make":
        return "build" if "build" in rest else None
    if prog == "xcodebuild":
        return "build" if any(w in ("build", "archive", "build-for-testing") for w in rest) else None
    if prog == "node":
        return "build" if "--check" in rest or "-c" in rest else None
    return None


def runner_kinds(command) -> list[tuple[str, str]]:
    """``(kind, command text)`` of each test or build runner in the shell line (kind: test | build)."""
    out = []
    for words in _split_commands(command):
        kind = _runner_kind(words)
        if kind:
            out.append((kind, " ".join(words)))
    return out


def is_test_command(command) -> bool:
    """True when a simple command of the shell line runs a test suite (not just mentions one)."""
    return any(_runs_tests(words) for words in _split_commands(command))


def readable_command(command, workspace=None) -> str:
    """The shell line without leading ``cd … &&``, paths relative to the workspace."""
    text = str(command or "").strip()
    while True:
        m = re.match(r"cd\s+(?:'[^']*'|\"[^\"]*\"|\S+)\s*(?:&&|;)\s*", text)
        if not m:
            break
        text = text[m.end():]
    ws = str(workspace or "").rstrip("/")
    if ws and len(ws) > 1:
        text = text.replace(ws + "/", "").replace(ws, ".")
    return text.replace(str(Path.home()) + "/", "~/")


def run_outcome(text, failed_flag=False) -> tuple[str, int | None, int | None]:
    """``(result, passed, failed)`` from a test run's output; the flag only decides when the
    output has no summary (a pipe to ``tail`` hides the exit code)."""
    text = str(text or "")
    passed = sum(int(n) for n in _PASSED_N.findall(text)) if _PASSED_N.search(text) else None
    failed = sum(int(n) for n in _FAILED_N.findall(text)) if _FAILED_N.search(text) else None
    if failed or _FAIL_LINE.search(text):
        return "failed", passed, failed
    if passed or _OK_LINE.search(text):
        return "passed", passed, failed
    return ("failed" if failed_flag else "passed"), passed, failed


def _one_line(text, limit: int = 200) -> str:
    line = " ".join(str(text or "").split())
    return line if len(line) <= limit else line[:limit - 1] + "…"


def _result_text(content) -> str:
    """Text of a tool_result ``content`` (string or list of text blocks)."""
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "\n".join(str(b.get("text") or "") for b in content if isinstance(b, dict))
    return ""


# -- log parsing ------------------------------------------------------------------------------


class _LaneLog:
    """Incremental parse state of one lane log."""

    def __init__(self) -> None:
        self.offset = 0
        self.stamp: tuple[int, float] | None = None  # (size, mtime) last seen
        self.events: deque[dict] = deque(maxlen=EVENTS_MAX)
        self.tools: Counter = Counter()
        self.tests: deque[dict] = deque(maxlen=TESTS_MAX)
        self.calls: dict[str, dict] = {}  # tool call id -> its test row (or {} when not a test)
        self.seen_ids: set[str] = set()
        self.has_tools = False
        self.workspace = None

    # -- one record --------------------------------------------------------------------------

    def _event(self, agent: str, lane: str, name: str, tool, summary: str, ts: str) -> None:
        self.events.append({"ts": ts, "agent": agent, "event": name, "lane": lane, "tool": tool,
                            "summary": summary, "source": "log"})

    def _tool(self, agent, lane, tool, summary, ts, call_id=None, command=None,
              count=True) -> None:
        if count:
            self.tools[tool] += 1
            self.has_tools = True
        self._event(agent, lane, "tool", tool, summary, ts)
        if call_id:
            self.seen_ids.add(str(call_id))
        if command is not None and is_test_command(command):
            row = {"at": ts, "command": _one_line(readable_command(command, self.workspace)),
                   "result": "running"}
            if _one_line(command) != row["command"]:
                row["full"] = _one_line(command, 400)
            self.tests.append(row)
            if call_id:
                self.calls[str(call_id)] = row
        if len(self.calls) > 4 * TESTS_MAX:
            self.calls = {k: v for k, v in self.calls.items() if v in self.tests}

    def _finish(self, call_id, failed: bool, text="") -> None:
        row = self.calls.get(str(call_id))
        if row is not None:
            row["result"], passed, bad = run_outcome(text, failed)
            for key, n in (("passed", passed), ("failed", bad)):
                if n is not None:
                    row[key] = n

    def feed(self, record: dict, lane: str, cwd, ts: str) -> None:
        """Turn one JSON record from the log into events/tool counts/test rows."""
        ts = _record_ts(record) or ts
        self.workspace = cwd
        kind = record.get("type")
        if kind == "assistant":
            self._assistant_tools(record, lane, cwd, ts)
        elif kind == "user":
            self._tool_results(record)
        elif kind in ("item.started", "item.completed"):
            self._item(record, lane, ts)
        elif kind == "result":
            self._event("anthropic", lane, "done", None,
                        hooks_mod._clip(record.get("result") or record.get("subtype") or "done"), ts)
        elif "payload_type" in record:
            self._muse(record, lane, ts)

    def _assistant_tools(self, record, lane, cwd, ts) -> None:
        message = record.get("message")
        content = message.get("content") if isinstance(message, dict) else None
        for block in content if isinstance(content, list) else []:
            if not isinstance(block, dict) or block.get("type") != "tool_use":
                continue
            tool = str(block.get("name") or "tool")
            data = block.get("input") if isinstance(block.get("input"), dict) else {}
            command = data.get("command") if tool == "Bash" else None
            self._tool("anthropic", lane, tool, hooks_mod._tool_summary(tool, data, cwd), ts,
                       block.get("id"), command)

    def _tool_results(self, record) -> None:
        message = record.get("message")
        content = message.get("content") if isinstance(message, dict) else None
        for block in content if isinstance(content, list) else []:
            if isinstance(block, dict) and block.get("type") == "tool_result":
                self._finish(block.get("tool_use_id"), bool(block.get("is_error")),
                             _result_text(block.get("content")))

    def _item(self, record, lane, ts) -> None:
        item = record.get("item")
        if not isinstance(item, dict):
            return
        started = record["type"] == "item.started"
        item_id = str(item.get("id") or "")
        first = not item_id or item_id not in self.seen_ids
        if item.get("type") == "command_execution":
            command = item.get("command")
            if first:
                self._tool("agent", lane, "Bash", "Bash: " + _one_line(command, 160), ts,
                           item_id, command)
            code = item.get("exit_code")
            if not started and code is not None:
                self._finish(item_id, code != 0, item.get("aggregated_output"))
        elif item.get("type") == "file_change" and first:
            changes = item.get("changes") if isinstance(item.get("changes"), list) else []
            paths = ", ".join(str(c.get("path")) for c in changes if isinstance(c, dict))
            self._tool("agent", lane, "file_change", "file_change " + (paths or "(files)"), ts,
                       item_id)

    def _muse(self, record, lane, ts) -> None:
        kind = str(record.get("payload_type") or "")
        payload = record.get("payload") if isinstance(record.get("payload"), dict) else {}
        if kind.startswith("run.terminal"):
            self._event("muse", lane, "done", None, hooks_mod._clip(payload.get("text") or kind), ts)
            return
        toolish = kind.startswith("tool.") or (
            not kind.startswith("runtime.") and re.search(r"(^|\.)(command|file)(\.|_|$)", kind))
        if not toolish:
            return
        parts = kind.split(".")
        tool = str(payload.get("tool") or payload.get("tool_name") or payload.get("name")
                   or (parts[1] if len(parts) > 1 else parts[0]))
        command = payload.get("command")
        path = payload.get("path") or payload.get("file_path")
        summary = ("Bash: " + _one_line(command, 160)) if command else (
            f"{tool} {path}" if path else f"{tool} ({kind})")
        call_id = payload.get("tool_call_id") or payload.get("call_id") or payload.get("id")
        code = payload.get("exit_code")
        if call_id and str(call_id) in self.seen_ids:
            if code is not None:
                self._finish(call_id, code != 0, _muse_output(payload))
            return
        self._tool("muse", lane, tool, summary, ts, call_id, command)
        if code is not None:
            self._finish(call_id, code != 0, _muse_output(payload))


def _muse_output(payload: dict) -> str:
    return "\n".join(str(payload.get(k) or "") for k in ("output", "stdout", "stderr", "text"))


def _record_ts(record: dict) -> str | None:
    """ISO time of a record that carries its own ``timestamp``/``ts``."""
    for key in ("timestamp", "ts"):
        value = record.get(key)
        if isinstance(value, str) and parse_iso(value):
            return now_iso(parse_iso(value))
    return None


def _read_new(path: Path, offset: int, size: int) -> tuple[bytes, int]:
    """Complete lines in ``[offset, size)`` (at most the last 512 KB); returns ``(data, offset)``."""
    start = max(offset, size - LOG_BYTES)
    with path.open("rb") as handle:
        handle.seek(start)
        raw = handle.read(size - start)
    if start > offset:  # skipped ahead: the first line is cut in half
        cut = raw.find(b"\n")
        if cut < 0:
            return b"", start
        raw, start = raw[cut + 1:], start + cut + 1
    end = raw.rfind(b"\n")
    return (raw[:end + 1], start + end + 1) if end >= 0 else (b"", start)


# -- git files --------------------------------------------------------------------------------

_CHANGES = {"A": "added", "M": "modified", "D": "deleted", "R": "renamed", "T": "modified",
            "C": "added"}


def _git(workspace: str, *args: str) -> str | None:
    try:
        out = timeouts_mod.run(["git", "-C", workspace, *args], "git", timeout=GIT_TIMEOUT_S)
    except (OSError, subprocess.SubprocessError):
        return None
    return out.stdout.decode("utf-8", errors="replace") if out.returncode == 0 else None


def _numstat(text: str) -> dict[str, tuple[int, int]]:
    """``git diff --numstat -z`` -> ``{path: (insertions, deletions)}`` (new path for renames)."""
    parts = text.split("\0")
    stats: dict[str, tuple[int, int]] = {}
    i = 0
    while i < len(parts):
        fields = parts[i].split("\t")
        i += 1
        if len(fields) < 3:
            continue
        path = fields[2]
        if not path:  # rename: "ins\tdel\t\0old\0new\0"
            path = parts[i + 1] if i + 1 < len(parts) else ""
            i += 2
        ins = int(fields[0]) if fields[0].isdigit() else 0
        dele = int(fields[1]) if fields[1].isdigit() else 0
        if path:
            stats[path] = (ins, dele)
    return stats


def _name_status(text: str) -> list[tuple[str, str]]:
    """``git diff --name-status -z`` -> ``[(change, path)]`` (new path for renames/copies)."""
    parts = text.split("\0")
    out: list[tuple[str, str]] = []
    i = 0
    while i < len(parts) and parts[i]:
        code = parts[i][0]
        if code in "RC":
            path = parts[i + 2] if i + 2 < len(parts) else ""
            i += 3
        else:
            path = parts[i + 1] if i + 1 < len(parts) else ""
            i += 2
        if path:
            out.append((_CHANGES.get(code, "modified"), path))
    return out


def _count_lines(path: Path) -> int:
    try:
        if path.stat().st_size > UNTRACKED_COUNT_BYTES:
            return 0
        data = path.read_bytes()
    except OSError:
        return 0
    return 0 if b"\0" in data else data.count(b"\n") + (1 if data and not data.endswith(b"\n") else 0)


def git_files(workspace, start_commit) -> tuple[list[dict], tuple[int, int, int]]:
    """Files changed since ``start_commit`` (diff pair + untracked) and tracked totals.

    Returns ``(files, (tracked_files, insertions, deletions))``; ``files`` is capped at 100.
    """
    if not workspace or not start_commit or not Path(workspace).is_dir():
        return [], (0, 0, 0)
    base = str(start_commit)
    stat = _git(str(workspace), "diff", "--numstat", "-z", "-M", base)
    names = _git(str(workspace), "diff", "--name-status", "-z", "-M", base)
    if stat is None or names is None:
        return [], (0, 0, 0)
    stats = _numstat(stat)
    files = [{"path": path, "change": change, "insertions": stats.get(path, (0, 0))[0],
              "deletions": stats.get(path, (0, 0))[1]} for change, path in _name_status(names)]
    totals = (len(files), sum(f["insertions"] for f in files), sum(f["deletions"] for f in files))
    untracked = _git(str(workspace), "ls-files", "--others", "--exclude-standard", "-z") or ""
    for path in filter(None, untracked.split("\0")):
        if len(files) >= FILES_MAX:
            break
        files.append({"path": path, "change": "untracked",
                      "insertions": _count_lines(Path(workspace) / path), "deletions": 0})
    return files[:FILES_MAX], totals


# -- tools / tests from hook rows ---------------------------------------------------------------


def hook_tools_and_tests(rows: list[dict]) -> tuple[Counter, list[dict]]:
    """Tool counts and test runs from a lane's hook rows (one count per call)."""
    tooled = [r for r in rows if r.get("tool")]
    calls = [r for r in tooled if r.get("event") == "PreToolUse"] or \
        [r for r in tooled if r.get("event") == "PostToolUse"]
    tools = Counter(str(r["tool"]) for r in calls)
    tests = []
    for row in calls:
        summary = str(row.get("summary") or "")
        if row["tool"] == "Bash" and is_test_command(summary[len("Bash: "):]):
            tests.append({"at": row.get("ts"), "command": _one_line(summary[len("Bash: "):]),
                          "result": None})
    return tools, tests[-TESTS_MAX:]


# -- tracker ------------------------------------------------------------------------------------


def _iso(ts: float) -> str:
    return now_iso(datetime.fromtimestamp(ts, timezone.utc))


class ActivityTracker:
    """Keeps per-lane caches (log parse, brief goal, ended-lane files) across cycles."""

    def __init__(self) -> None:
        self._logs: dict[str, _LaneLog] = {}
        self._goals: dict[str, tuple[float, str | None]] = {}
        self._ended_files: dict[str, tuple[list[dict], tuple[int, int, int]]] = {}
        self._live_files: dict[str, tuple[float, tuple[list[dict], tuple[int, int, int]]]] = {}
        self._refreshing: set[str] = set()

    def _log(self, lane: dict) -> _LaneLog | None:
        """Bring the lane's log state up to date (read only what is new)."""
        lane_id, path = str(lane["id"]), lane.get("log")
        if not path:
            return None
        state = self._logs.setdefault(lane_id, _LaneLog())
        try:
            info = Path(path).stat()
        except OSError:
            return state
        stamp = (info.st_size, info.st_mtime)
        if state.stamp == stamp:
            return state
        if info.st_size < state.offset:
            state = self._logs[lane_id] = _LaneLog()
        try:
            data, state.offset = _read_new(Path(path), state.offset, info.st_size)
        except OSError:
            return state
        state.stamp = stamp
        for line in data.decode("utf-8", errors="replace").splitlines():
            if not line.startswith("{"):
                continue
            try:
                record = json.loads(line)
            except ValueError:
                continue
            if isinstance(record, dict):
                state.feed(record, lane_id, lane.get("workspace"), _iso(info.st_mtime))
        return state

    def goal(self, lane: dict) -> str | None:
        """Goal text from the lane's brief file (cached by mtime), else the feature."""
        path = lane.get("brief")
        goal = None
        if path:
            try:
                mtime = os.stat(path).st_mtime
                cached = self._goals.get(str(path))
                if cached and cached[0] == mtime:
                    return cached[1] or str(lane.get("feature") or "") or None
                with open(path, "rb") as handle:
                    text = handle.read(BRIEF_BYTES).decode("utf-8", errors="replace")
                goal = goal_from_text(text)
                self._goals[str(path)] = (mtime, goal)
            except OSError:
                goal = None
        return goal or str(lane.get("feature") or "") or None

    def prefetch(self, lanes: list[dict]) -> None:
        """Read the git files of every lane not seen yet at once (a cold start would run 3 git calls per lane in turn)."""
        todo = [l for l in lanes if str(l["id"]) not in (self._ended_files if l.get("ended_at") else self._live_files)]
        if len(todo) < 2:
            return
        with ThreadPoolExecutor(max_workers=PREFETCH_WORKERS) as pool:
            results = list(pool.map(lambda l: git_files(l.get("workspace"), l.get("start_commit")), todo))
        at = time.monotonic()
        for lane, result in zip(todo, results):
            if lane.get("ended_at"):
                self._ended_files[str(lane["id"])] = result
            else:
                self._live_files[str(lane["id"])] = (at, result)

    def files(self, lane: dict, ended: bool) -> tuple[list[dict], tuple[int, int, int]]:
        """Live git files; an ended lane keeps the list computed once."""
        lane_id = str(lane["id"])
        if ended and lane_id in self._ended_files:
            return self._ended_files[lane_id]
        if ended:
            result = git_files(lane.get("workspace"), lane.get("start_commit"))
            self._ended_files[lane_id] = result
            self._live_files.pop(lane_id, None)
            return result
        self._ended_files.pop(lane_id, None)
        cached = self._live_files.get(lane_id)
        if cached is None:  # first sight: the caller waits for it
            result = git_files(lane.get("workspace"), lane.get("start_commit"))
            self._live_files[lane_id] = (time.monotonic(), result)
            return result
        if time.monotonic() - cached[0] >= FILES_TTL_S and lane_id not in self._refreshing:
            self._refreshing.add(lane_id)
            workspace, start = lane.get("workspace"), lane.get("start_commit")

            def refresh() -> None:
                try:
                    self._live_files[lane_id] = (time.monotonic(), git_files(workspace, start))
                finally:
                    self._refreshing.discard(lane_id)

            threading.Thread(target=refresh, daemon=True).start()
        return cached[1]

    def lane(self, lane: dict, status: str, hook_rows: list[dict], now: datetime) -> dict:
        """The activity fields of one agent row (see ``docs/CONTRACT-activity.md``)."""
        ended = bool(lane.get("ended_at"))
        log = self._log(lane)
        use_log = log is not None and log.has_tools
        if use_log:
            tools, tests = log.tools, [dict(t) for t in log.tests]
        else:
            tools, tests = hook_tools_and_tests(hook_rows)
        if status != "working":
            for row in tests:
                if row["result"] == "running":
                    row["result"] = None
        hooks = [{**hooks_mod.public(r), "source": "hook"} for r in hook_rows][-EVENTS_MAX:]
        merged = sorted(hooks + list(log.events if log else []), key=lambda e: str(e["ts"]))
        files, totals = self.files(lane, ended)
        times = [e["ts"] for e in merged[-1:]]
        mtime = None
        if log is not None and log.stamp:
            mtime = _iso(log.stamp[1])
            times.append(mtime)
        last = max(times) if times else None
        recent = parse_iso(last)
        active = status == "working" and recent is not None and \
            (now - recent).total_seconds() <= ACTIVE_WINDOW_S
        return {
            "goal": self.goal(lane), "active": bool(active), "last_activity_at": last,
            "files": files, "totals": totals,
            "tools": [{"name": n, "count": c} for n, c in
                      sorted(tools.items(), key=lambda kv: (-kv[1], kv[0]))],
            "tests": tests[-TESTS_MAX:], "events": merged[-EVENTS_MAX:],
        }
