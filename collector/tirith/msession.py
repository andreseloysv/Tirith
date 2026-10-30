"""The manager's one persistent session, owned by the collector (``docs/CONTRACT-inbox.md``, "One manager").

The collector starts ONE long-lived agent process per Tirith home (``tirith lane run --interactive`` around the
agent's ``chat_args``: stream-json in and out, partial messages, permission questions on stdout) in its own
process group. Pending chat messages are written to its stdin as user messages, one batch per turn; a turn
ends with the agent's ``result`` event. A reader thread writes every stream line to
``<home>/manager/transcript.jsonl`` (rotated), feeds the CLI-style steps of the turn (``livesteps``), keeps the
context size, and answers permission questions by code: denied when a gate or the "manager delegates edits" rule says so,
allowed otherwise; the owner is never asked.
``tirith manager say|interrupt|stop|restart`` reach the collector through ``<home>/manager/control.jsonl``.
"""

from __future__ import annotations

import json
import os
import subprocess
import threading
import uuid
from collections import deque
from datetime import datetime, timezone
from pathlib import Path
from typing import Callable

from tirith import gates as gates_mod
from tirith import lanegates as lanegates_mod
from tirith.home import Config, now_iso
from tirith.store import append_jsonl, read_json, read_jsonl, write_json_atomic

DIR = "manager"
TRANSCRIPT = "transcript.jsonl"
CONTROL = "control.jsonl"
TURNS_DIR = "turns"
TRANSCRIPT_MAX = 16 * 1024 * 1024
TRANSCRIPT_KEEP = 3  # rotated files kept: transcript.jsonl.1 ... .N
TURN_PREFIX = "manager-turn-"
TURNS_KEPT = 30
PARTIAL_FLAG = "--include-partial-messages"
DEFAULT_CHAT_ARGS = ["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose",
                     "--permission-prompt-tool", "stdio"]
MAX_TURNS = 40  # a session this many turns old is compacted, then restarted with a handoff
CONTEXT_TOKENS = 150000  # context use that triggers /compact
OPS = ("interrupt", "stop", "restart")
SAFE_TOOLS = ("Read", "Glob", "Grep", "TodoWrite", "NotebookRead", "LS")
EDIT_TOOLS = ("Edit", "MultiEdit", "Write", "NotebookEdit")


def manager_dir(home: Path) -> Path:
    return Path(home) / DIR


def transcript_path(home: Path) -> Path:
    return manager_dir(home) / TRANSCRIPT


def control_path(home: Path) -> Path:
    return manager_dir(home) / CONTROL


def user_message(text: str) -> dict:
    return {"type": "user", "message": {"role": "user", "content": [{"type": "text", "text": text}]}}


def permission_reply(request_id: str, allow: bool, tool_input, message: str = "") -> dict:
    """The ``control_response`` for a ``can_use_tool`` question (docs/CONTRACT-agents.md)."""
    body = ({"behavior": "allow", "updatedInput": tool_input if isinstance(tool_input, dict) else {}} if allow
            else {"behavior": "deny", "message": message or "The owner denied this in Tirith."})
    return {"type": "control_response",
            "response": {"subtype": "success", "request_id": request_id, "response": body}}


# -- the command ------------------------------------------------------------------------------


def chat_command(config: Config, alias: str, model: str | None = None, effort: str | None = None,
                 resume: str | None = None) -> list[str]:
    """The agent's streaming command: ``command`` + ``chat_args`` (+ partial messages, model, effort, resume)."""
    from tirith import agentrun as agentrun_mod
    s = agentrun_mod.agent_settings(config, alias, None)
    exe = s.get("command")
    if not exe:
        raise SystemExit(f"[agents.{alias}] has no command in config.toml")
    args = [str(a) for a in (s.get("chat_args") or DEFAULT_CHAT_ARGS)]
    cmd = [str(exe), *args]
    if "stream-json" in args and PARTIAL_FLAG not in args:
        cmd.append(PARTIAL_FLAG)  # the text arrives token by token
    model = model or s.get("model")
    if model and s.get("model_flag"):
        cmd += [str(s["model_flag"]), str(model)]
    effort = effort or s.get("effort")
    if effort and s.get("effort_flag"):
        cmd += [str(s["effort_flag"]), str(effort)]
    if resume:
        cmd += ["--resume", resume]
    return cmd


# -- permissions decided by code --------------------------------------------------------------


def _pattern(rule: str, tool: str) -> str | None:
    """The part inside the parentheses of ``Tool(...)`` ("" for a bare ``Tool``), None for another tool."""
    name, _, rest = rule.partition("(")
    return rest.rstrip(")").strip() if name.strip() == tool else None


def allowed_by_rules(rules: list[str], tool: str, data: dict) -> bool:
    """``Tool`` or ``Tool(prefix:*)`` / ``Tool(exact)`` rules against one call. A Bash line is allowed when every
    command of it is (a command hidden in a substitution or a redirect is the owner's to judge)."""
    patterns = [p for p in (_pattern(r, tool) for r in rules) if p is not None]
    if tool != "Bash":
        return any(p in ("", "*") for p in patterns)
    line = gates_mod.strip_heredocs(str(data.get("command") or ""))
    if any(mark in line for mark in ("$(", "`", ">", "<(")):
        return False
    segments = gates_mod.split_commands(line)

    def ok(tokens: list[str], pattern: str) -> bool:
        if pattern in ("", "*"):
            return True
        if pattern.endswith(":*"):
            prefix = pattern[:-2].split()
            return tokens[:len(prefix)] == prefix
        return tokens == pattern.split()

    return bool(segments) and all(any(ok(tokens, p) for p in patterns) for tokens in segments)


LANE_MESSAGE = ("The manager delegates edits: start a lane with `tirith lane run` instead of changing "
                "project files yourself. Things only the owner can do go through `tirith wait add --why-owner ...`.")
_GIT_WRITES = {"commit", "merge", "push", "reset", "checkout", "switch", "rebase", "cherry-pick", "revert", "add",
               "rm", "mv", "restore", "clean", "stash", "pull", "apply", "am", "tag", "branch", "worktree"}


def _bash_writes(home: Path, data: dict, cwd) -> str | None:
    """The first write of a Bash line outside the Tirith home (a redirect to a file, ``tee``, ``sed -i``,
    ``mv``/``cp``/``rm``, a changing ``git`` command), or None. Harmless redirects (``2>&1``, ``>/dev/null``) and
    function definitions are not writes."""
    line = gates_mod.strip_heredocs(str(data.get("command") or ""))
    here, roots = str(cwd or os.getcwd()), [lanegates_mod._real(home)]
    for tokens in lanegates_mod.expand_segments(line):
        moved = lanegates_mod._cd_target(tokens)
        if moved is not None:
            here = lanegates_mod._resolve(moved, here, os.environ) or here
            continue
        for target in lanegates_mod.write_targets(tokens):
            real = lanegates_mod._resolve(target, here, os.environ)
            if real is None or not lanegates_mod._inside(real, roots):
                return target
        folder, sub = lanegates_mod._git_c(tokens)
        name, args = gates_mod._program(tokens)
        if name == "git":
            if folder is None:
                sub = next((a for a in args if not a.startswith("-")), "")
            where = lanegates_mod._resolve(folder, here, os.environ) if folder is not None else here
            if sub in _GIT_WRITES and (where is None or not lanegates_mod._inside(where, roots)):
                return f"git {sub}"
    return None


def decide(config: Config, home: Path, alias: str, tool: str, data, cwd, project: str | None = None
           ) -> tuple[str, str]:
    """``("allow"|"deny", message)`` for a tool call the manager asks to make; never a question for the owner.

    The gates first (their deny rules), then the manager delegates edits (a file edit or a Bash write outside the
    Tirith home is denied with the lane message), everything else is allowed. A deny is logged as a flow."""
    data = data if isinstance(data, dict) else {}
    verdict, message = _verdict(config, home, tool, data, cwd, project)
    if verdict == "deny":
        try:
            from tirith import flows as flows_mod
            flows_mod.append_flow(home, "tirith", "manager", "deny", f"{tool} denied: {message}",
                                  session=None, project=project)
        except OSError:
            pass
    return verdict, message


def _verdict(config: Config, home: Path, tool: str, data: dict, cwd, project: str | None) -> tuple[str, str]:
    hits = [v for v in gates_mod.check_call(config, tool, data, cwd, project, home) if v.action == "block"]
    if hits:
        return "deny", hits[0].message()
    if tool in EDIT_TOOLS:
        path = str(data.get("file_path") or data.get("notebook_path") or "")
        inside = path and Path(path).expanduser().resolve().is_relative_to(Path(home).resolve())
        return ("allow", "") if inside else ("deny", LANE_MESSAGE)
    if tool == "Bash" and _bash_writes(home, data, cwd):
        return "deny", LANE_MESSAGE
    return "allow", ""


# -- one turn and the session -----------------------------------------------------------------


class Turn:
    def __init__(self, requests: list[str], workspace, now: datetime):
        self.id = TURN_PREFIX + uuid.uuid4().hex[:8]
        self.requests = list(requests)
        self.started_at = now_iso(now)
        self.ended_at: str | None = None
        from tirith import livesteps as livesteps_mod  # late: logview -> activity -> hooks -> managerd -> here
        self.parser = livesteps_mod.StepParser(workspace)
        self.stamped = 0
        self.error = False
        self.reason = ""  # why it failed, when the session ended under it
        self.answer = ""
        self.interrupted = False

    def feed(self, line: str, now: datetime) -> None:
        self.parser.feed(line)
        stamp = now_iso(now)
        for row in self.parser.rows[self.stamped:]:
            row["at"] = stamp
        self.stamped = len(self.parser.rows)

    def view(self) -> dict:
        steps = self.parser.steps()
        summary = {**self.parser.summary, "steps": len(steps)}
        for row in steps:
            row.setdefault("at", self.started_at)
            row["at"] = row["at"] or self.started_at
        return {"id": self.id, "requests": list(self.requests), "started_at": self.started_at,
                "ended_at": self.ended_at, "summary": summary, "steps": steps, "error": self.error,
                "reason": self.reason, "answer": self.answer, "interrupted": self.interrupted}


class Session:
    """The running agent process and everything the reader thread learns from its stdout."""

    def __init__(self, home: Path, lane_id: str, proc, decide_fn: Callable, workspace, resumed: bool = False):
        self.home, self.lane_id, self.proc, self.decide, self.workspace = Path(home), lane_id, proc, decide_fn, workspace
        self.resumed = resumed
        self.lock = threading.RLock()
        self.session_id: str | None = None
        self.turn: Turn | None = None
        self.finished: deque = deque(maxlen=TURNS_KEPT)  # turn views, oldest first
        self.asks: dict[str, dict] = {}  # request id -> {tool, input, summary, at, wait}
        self.context_tokens = 0
        self._request_seen = False  # an assistant request of this turn reported its own usage
        self.turns_done = 0
        self.last_event = datetime.now(timezone.utc)
        self.exit_code: int | None = None
        self.interrupted = False
        self.started = datetime.now(timezone.utc)
        self.got_event = False
        self._out = None
        self.thread = threading.Thread(target=self._read, daemon=True)
        self.thread.start()

    # -- state ----------------------------------------------------------------------------

    @property
    def alive(self) -> bool:
        return self.exit_code is None

    @property
    def busy(self) -> bool:
        return self.turn is not None

    def write(self, obj: dict) -> bool:
        with self.lock:
            try:
                self.proc.stdin.write((json.dumps(obj) + "\n").encode("utf-8"))
                self.proc.stdin.flush()
                return True
            except (OSError, ValueError):
                return False

    def send(self, text: str, requests: list[str], now: datetime) -> bool:
        """Start a turn: one user message on stdin. Only when the session is idle."""
        with self.lock:
            if self.turn is not None or not self.alive:
                return False
            turn = Turn(requests, self.workspace, now)
            if not self.write(user_message(text)):
                return False
            self.turn, self.interrupted = turn, False
            return True

    def interrupt(self) -> bool:
        """Stop the running turn (the session lives on)."""
        with self.lock:
            if self.turn is None:
                return False
            self.interrupted = True
            return self.write({"type": "control_request", "request_id": "int-" + uuid.uuid4().hex[:8],
                               "request": {"subtype": "interrupt"}})

    def answer_ask(self, request_id: str, allow: bool) -> bool:
        with self.lock:
            ask = self.asks.pop(request_id, None)
            if ask is None:
                return False
            return self.write(permission_reply(request_id, allow, ask["input"]))

    def open_asks(self) -> dict[str, dict]:
        with self.lock:
            return {k: dict(v) for k, v in self.asks.items()}

    def views(self) -> list[dict]:
        """Turn views, oldest first: the finished ones and the running one."""
        with self.lock:
            return [*self.finished, *([self.turn.view()] if self.turn else [])]

    def terminate(self) -> None:
        pgid = self.proc.pid
        try:
            os.killpg(pgid, 15)
        except (OSError, AttributeError):
            try:
                self.proc.terminate()
            except OSError:
                pass

    # -- the reader -----------------------------------------------------------------------

    def _append(self, line: str) -> None:
        path = transcript_path(self.home)
        try:
            if self._out is None:
                path.parent.mkdir(parents=True, exist_ok=True)
                self._out = open(path, "ab")
            self._out.write(line.encode("utf-8", errors="replace") + b"\n")
            self._out.flush()
            if self._out.tell() > TRANSCRIPT_MAX:
                self._out.close()
                self._out = None
                rotate(path)
        except OSError:
            self._out = None

    def _read(self) -> None:
        try:
            for raw in iter(self.proc.stdout.readline, b""):
                line = raw.decode("utf-8", errors="replace").rstrip("\n")
                if not line.strip().startswith("{"):
                    continue
                self.got_event = True
                self._append(line)
                try:
                    self._event(line)
                except Exception:  # one odd line never stops the reader
                    continue
        finally:
            code = None
            try:
                code = self.proc.wait(timeout=5)
            except Exception:
                pass
            with self.lock:
                self.exit_code = code if code is not None else -1
                if self.turn is not None:
                    self.turn.error = True
                    self.turn.reason = self._exit_reason()
                    self._finish_turn(datetime.now(timezone.utc))
                if self._out is not None:
                    self._out.close()

    def _exit_reason(self) -> str:
        """Why the process is gone: the last lines of its launch log (``tirith lane run`` says why it refused), else its code."""
        try:
            log = (self.home / "logs" / f"{self.lane_id}.launch.log").read_text(encoding="utf-8", errors="replace")
        except OSError:
            log = ""
        tail = " / ".join(log.strip().splitlines()[-3:])[-300:]
        return f"the session exited ({tail or f'code {self.exit_code}'})"

    def _event(self, line: str) -> None:
        rec = json.loads(line)
        if not isinstance(rec, dict):
            return
        now = datetime.now(timezone.utc)
        with self.lock:
            self.last_event = now
            sid = rec.get("session_id")
            if isinstance(sid, str) and sid:
                self.session_id = sid
            kind = rec.get("type")
            if kind == "control_request":
                self._ask(rec)
                return
            if kind == "assistant":
                usage = (rec.get("message") or {}).get("usage") if isinstance(rec.get("message"), dict) else None
                self._usage(usage)
                self._request_seen = self._request_seen or isinstance(usage, dict)
            if self.turn is None:
                return  # a system line between turns
            self.turn.feed(line, now)
            if kind == "result":
                # the result's usage adds up every request of the turn; the last request is the context size
                if not self._request_seen:
                    self._usage(rec.get("usage"))
                self._request_seen = False
                self.turn.answer = str(rec.get("result") or "")[:4000]
                self._finish_turn(now)

    def _usage(self, usage) -> None:
        if isinstance(usage, dict):
            total = sum(int(usage.get(k) or 0) for k in
                        ("input_tokens", "cache_read_input_tokens", "cache_creation_input_tokens"))
            if total:
                self.context_tokens = total

    def _finish_turn(self, now: datetime) -> None:
        turn, self.turn = self.turn, None
        if turn is None:
            return
        turn.ended_at = now_iso(now)
        turn.interrupted = self.interrupted
        turn.error = turn.error or bool(turn.parser.summary.get("error"))
        view = turn.view()
        self.finished.append(view)
        self.turns_done += 1
        self.asks.clear()  # a question of the turn that ended can no longer be answered
        try:
            folder = manager_dir(self.home) / TURNS_DIR
            folder.mkdir(parents=True, exist_ok=True)
            write_json_atomic(folder / f"{turn.id}.json", view)
            for old in sorted(folder.glob("*.json"), key=lambda p: p.stat().st_mtime)[:-TURNS_KEPT]:
                old.unlink(missing_ok=True)
        except OSError:
            pass

    def _ask(self, rec: dict) -> None:
        request = rec.get("request") if isinstance(rec.get("request"), dict) else {}
        rid = str(rec.get("request_id") or "")
        if request.get("subtype") != "can_use_tool" or not rid:
            return
        tool, data = str(request.get("tool_name") or ""), request.get("input")
        try:
            verdict, message = self.decide(tool, data)
        except Exception as err:  # a rule that cannot be checked denies
            verdict, message = "deny", f"The rule check failed ({err}); try a simpler command."
        if verdict == "allow":
            self.write(permission_reply(rid, True, data))
        else:
            self.write(permission_reply(rid, False, data, message))


def rotate(path: Path) -> None:
    for i in range(TRANSCRIPT_KEEP, 0, -1):
        older = path.with_name(f"{path.name}.{i}")
        newer = path if i == 1 else path.with_name(f"{path.name}.{i - 1}")
        try:
            if newer.exists():
                os.replace(newer, older)
        except OSError:
            pass


# -- control (CLI and app -> collector) -------------------------------------------------------


def send_control(home: Path, op: str, text: str = "") -> dict:
    if op not in OPS:
        raise ValueError(f"op must be one of {', '.join(OPS)}")
    manager_dir(home).mkdir(parents=True, exist_ok=True)
    row = {"id": uuid.uuid4().hex[:10], "op": op, "at": now_iso(), "text": text}
    append_jsonl(control_path(home), row)
    return row


def new_controls(home: Path, seen: str | None) -> list[dict]:
    """Control rows after the one with id ``seen``."""
    rows = [r for r in read_jsonl(control_path(home)) if isinstance(r, dict) and r.get("op") in OPS]
    if seen:
        ids = [r.get("id") for r in rows]
        if seen in ids:
            rows = rows[ids.index(seen) + 1:]
    return rows


# -- registry: the sessions of this process, by home ------------------------------------------

_SESSIONS: dict[str, Session] = {}


def current(home: Path) -> Session | None:
    return _SESSIONS.get(str(home))


def register(home: Path, session: Session | None) -> None:
    if session is None:
        _SESSIONS.pop(str(home), None)
    else:
        _SESSIONS[str(home)] = session


def spawn(argv: list[str], home: Path, stderr_log: Path, popen: Callable | None = None):
    """Start the session process: own process group, pipes for stdin and stdout."""
    stderr_log.parent.mkdir(parents=True, exist_ok=True)
    from tirith import launch as launch_mod
    env = dict(os.environ, TIRITH_HOME=str(home), PYTHONPATH=str(Path(__file__).resolve().parent.parent))
    extra = {"start_new_session": True} if os.name == "posix" else {}
    with stderr_log.open("ab") as err:
        return (popen or subprocess.Popen)(argv, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=err,
                                           env=env, **extra)


def read_turn(home: Path, turn_id: str) -> dict | None:
    data = read_json(manager_dir(home) / TURNS_DIR / f"{turn_id}.json", None)
    return data if isinstance(data, dict) else None
