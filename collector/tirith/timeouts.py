"""Every command Tirith starts has a timeout, and this module is the one mechanism behind it.

* the policy: command classes with a default, a minimum and a maximum (:data:`POLICY`);
* the value in force for a class (:func:`effective`): the owner's ``[timeouts]`` in ``config.toml``, else a
  value the owner set, else one an agent set for its lane (bounded, with a reason), else the one learned from
  the recorded durations (p95 x :data:`FACTOR`, clamped), else the default;
* the runner (:func:`run`) used by every subprocess Tirith starts: own process group, SIGTERM then SIGKILL on
  the whole group at the deadline, the class's cleanup (a stale ``index.lock``), the duration and outcome
  recorded in ``timeouts.json``, and a clear error naming the class and how to raise it.

See ``docs/CONTRACT-timeouts.md``.
"""

from __future__ import annotations

import contextlib
import fcntl
import os
import shlex
import signal
import subprocess
import threading
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from tirith.home import Config, load_config, now_iso, parse_iso, tirith_home
from tirith.store import read_json, read_lane, write_json_atomic

FILE = "timeouts.json"
FACTOR = 3.0            # learned timeout = p95 of the recorded durations x this
MIN_SAMPLES = 5         # fewer recorded durations and the default stays
KEEP_SAMPLES = 60       # durations kept per class
KEEP_TIMEOUTS = 12      # recent timeouts kept per class
KILL_GRACE_S = 3.0      # SIGTERM, then SIGKILL after this long
EXIT_TIMED_OUT = 124    # the exit code of a command that was stopped at its deadline

# class -> (default s, min s, max s, plain words)
POLICY: dict[str, tuple[float, float, float, str]] = {
    "git": (30, 5, 300, "git commands Tirith runs to read a repository"),
    "git-commit": (180, 30, 900, "a git commit, with the commit checks that run before it"),
    "stage": (1800, 60, 7200, "a build or test stage"),
    "agent-bash": (120, 10, 600, "a shell command an agent runs"),
    "agent-turn": (3600, 300, 21600, "one run of an agent in a lane"),
    "muse": (600, 60, 3600, "a request to the model"),
    "cargo": (1800, 60, 7200, "a cargo build or test"),
    "npm": (900, 60, 3600, "an npm install or script"),
    "generic": (120, 5, 1800, "any other command"),
}
LABELS = {"git": "Git (reading)", "git-commit": "Git commit", "stage": "Build and test stages",
          "agent-bash": "Agent shell commands", "agent-turn": "Agent runs", "muse": "Model requests",
          "cargo": "Cargo", "npm": "npm", "generic": "Other commands"}


def base_class(cls: str) -> str:
    """``stage:test`` is the ``stage`` policy with its own history; an unknown class is ``generic``."""
    head = str(cls or "").split(":", 1)[0]
    return head if head in POLICY else "generic"


def bounds(cls: str) -> tuple[float, float, float]:
    default, low, high, _ = POLICY[base_class(cls)]
    return float(default), float(low), float(high)


def clamp(cls: str, seconds: float) -> float:
    _, low, high = bounds(cls)
    return max(low, min(high, float(seconds)))


def classify_command(command: str) -> str:
    """The class of a shell line an agent runs (``agent-bash`` unless it is plainly cargo, npm or a commit)."""
    try:
        words = shlex.split(command or "")
    except ValueError:
        words = str(command or "").split()
    words = [w for w in words if "=" not in w.split("/")[-1] or w.startswith("-")]
    first = Path(words[0]).name if words else ""
    if first == "cargo":
        return "cargo"
    if first in ("npm", "npx", "pnpm", "yarn", "node"):
        return "npm"
    if first == "git" and "commit" in words[1:4]:
        return "git-commit"
    return "agent-bash"


# -- the history file -------------------------------------------------------------------------


def _path(home: Path) -> Path:
    return Path(home) / FILE


@contextlib.contextmanager
def _locked(home: Path):
    Path(home).mkdir(parents=True, exist_ok=True)
    with (Path(home) / (FILE + ".lock")).open("a") as handle:
        fcntl.flock(handle, fcntl.LOCK_EX)
        try:
            yield
        finally:
            fcntl.flock(handle, fcntl.LOCK_UN)


def _book(home: Path) -> dict:
    data = read_json(_path(home), {})
    data = data if isinstance(data, dict) else {}
    for key in ("durations", "timeouts", "set"):
        if not isinstance(data.get(key), dict):
            data[key] = {}
    return data


def record(home: Path, cls: str, seconds: float, outcome: str, *, command: str = "", timeout: float = 0,
           lane: str | None = None) -> None:
    """Keep one duration (``outcome``: ``ok``, ``failed`` or ``timeout``); a timeout is also listed by itself."""
    try:
        with _locked(home):
            book = _book(home)
            if outcome != "timeout":  # only commands that finished say how long the class normally takes
                rows = book["durations"].setdefault(cls, [])
                rows.append(round(float(seconds), 2))
                del rows[:-KEEP_SAMPLES]
            else:
                rows = book["timeouts"].setdefault(cls, [])
                rows.append({"at": now_iso(), "seconds": round(float(timeout)), "command": str(command)[:200],
                             **({"lane": lane} if lane else {})})
                del rows[:-KEEP_TIMEOUTS]
            write_json_atomic(_path(home), book)
    except OSError:
        pass


def p95(values: list[float]) -> float:
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int(0.95 * len(ordered)))] if ordered else 0.0


def learned(home: Path, cls: str) -> float | None:
    """clamp(p95 of the recorded durations x FACTOR) for ``cls`` (its own history, else its base class's)."""
    durations = _book(home)["durations"]
    rows = [float(x) for x in durations.get(cls) or [] if isinstance(x, (int, float))]
    if len(rows) < MIN_SAMPLES:
        return None
    return clamp(cls, p95(rows) * FACTOR)


# -- agent / owner overrides ------------------------------------------------------------------


class TimeoutRefused(ValueError):
    """A ``tirith timeout set`` the policy does not allow."""


def _lane_live(home: Path, lane_id: str | None) -> bool:
    if not lane_id:
        return False
    lane = read_lane(home, lane_id)
    return isinstance(lane, dict) and not lane.get("ended_at")


def set_timeout(home: Path, config: Config, cls: str, seconds: float, reason: str, *,
                lane: str | None = None) -> dict:
    """Store a timeout for ``cls``. An agent (``lane``) stays within min/max, must give a reason and loses
    the value when its lane ends; the owner (no lane) is bounded the same way but never expires.
    A class fixed in ``config.toml`` ``[timeouts]`` cannot be changed here."""
    if str(cls or "").split(":", 1)[0] not in POLICY:
        raise TimeoutRefused(f"unknown class '{cls}'; known: {', '.join(POLICY)}")
    default, low, high = bounds(cls)
    if cls in config.timeouts:
        raise TimeoutRefused(f"the owner fixed '{cls}' at {config.timeouts[cls]:g} s in config.toml [timeouts]")
    if lane and not str(reason or "").strip():
        raise TimeoutRefused("give the reason with --reason")
    if not low <= float(seconds) <= high:
        raise TimeoutRefused(f"'{cls}' takes {low:g} to {high:g} seconds, not {float(seconds):g}")
    row = {"seconds": float(seconds), "reason": str(reason or "").strip()[:300], "at": now_iso(),
           **({"lane": lane} if lane else {})}
    with _locked(home):
        book = _book(home)
        book["set"][cls] = row
        write_json_atomic(_path(home), book)
    return row


def _stored(home: Path, cls: str) -> dict | None:
    row = _book(home)["set"].get(cls)
    if not isinstance(row, dict) or not isinstance(row.get("seconds"), (int, float)):
        return None
    if row.get("lane") and not _lane_live(home, row["lane"]):
        return None  # an agent's value ends with its lane
    return row


def describe(home: Path, config: Config, cls: str) -> dict:
    """The value in force for ``cls`` and where it comes from."""
    default, low, high = bounds(cls)
    if cls in config.timeouts:
        return {"class": cls, "seconds": float(config.timeouts[cls]), "source": "config", "default": default,
                "min": low, "max": high}
    row = _stored(home, cls)
    if row is not None:
        return {"class": cls, "seconds": clamp(cls, row["seconds"]), "source": "agent" if row.get("lane") else "owner",
                "default": default, "min": low, "max": high, "reason": row.get("reason"), "lane": row.get("lane")}
    value = learned(home, cls)
    if value is None and ":" in cls:
        value = learned(home, base_class(cls))
    if value is not None:
        return {"class": cls, "seconds": value, "source": "learned", "default": default, "min": low, "max": high}
    return {"class": cls, "seconds": default, "source": "default", "default": default, "min": low, "max": high}


def effective(home: Path, config: Config | None, cls: str) -> float:
    return describe(home, config if config is not None else load_config(Path(home)), cls)["seconds"]


def table(home: Path, config: Config, now=None) -> list[dict]:
    """One row per policy class for Settings: the value in force, where it comes from, the last timeouts."""
    book = _book(home)
    rows = []
    for cls in POLICY:
        row = describe(home, config, cls)
        row["label"] = LABELS[cls]
        row["words"] = POLICY[cls][3]
        samples = [d for key, vals in book["durations"].items() if base_class(key) == cls for d in vals]
        row["samples"] = len(samples)
        row["last"] = max((t for key, vals in book["timeouts"].items() if base_class(key) == cls
                           for t in vals if isinstance(t, dict)), key=lambda t: str(t.get("at")), default=None)
        rows.append(row)
    return rows


def recent_timeouts(home: Path, since_s: float, now=None) -> dict[str, list[dict]]:
    """class -> the timeouts recorded within the last ``since_s`` seconds."""
    from datetime import datetime, timezone
    now = now or datetime.now(timezone.utc)
    found: dict[str, list[dict]] = {}
    for cls, vals in _book(home)["timeouts"].items():
        keep = [t for t in vals if isinstance(t, dict) and (when := parse_iso(t.get("at")))
                and (now - when).total_seconds() <= since_s]
        if keep:
            found[cls] = keep
    return found


# -- stopping a process with its children -----------------------------------------------------


def kill_group(proc: subprocess.Popen, grace: float = KILL_GRACE_S) -> None:
    """SIGTERM the process group of ``proc``, SIGKILL it when it is still there after ``grace`` seconds."""
    def signal_group(sig) -> None:
        try:
            os.killpg(proc.pid, sig)
        except (OSError, AttributeError):
            with contextlib.suppress(OSError):
                proc.send_signal(sig)
    signal_group(signal.SIGTERM)
    try:
        proc.wait(timeout=grace)
    except subprocess.TimeoutExpired:
        pass
    signal_group(signal.SIGKILL)  # the leader may be gone while a child ignored SIGTERM


def timeout_message(cls: str, seconds: float, what: str = "") -> str:
    return (f"timed out after {seconds:g} s (class {cls}{'; ' + what if what else ''}); "
            f"raise it with: tirith timeout set --class {base_class(cls)} --seconds N --reason \"…\"")


# -- cleanup after a timeout ------------------------------------------------------------------


def _git_dir(argv: list[str], cwd: str | Path | None) -> Path | None:
    folder = None
    if "-C" in argv[:-1]:
        folder = argv[argv.index("-C") + 1]
    folder = Path(folder or cwd or os.getcwd())
    if not folder.is_absolute() and cwd:
        folder = Path(cwd) / folder
    for parent in (folder, *folder.parents):
        marker = parent / ".git"
        if marker.is_dir():
            return marker
        if marker.is_file():  # a linked worktree: ".git" holds "gitdir: <path>"
            text = marker.read_text(encoding="utf-8", errors="replace").strip()
            if text.startswith("gitdir:"):
                target = Path(text.split(":", 1)[1].strip())
                return target if target.is_absolute() else parent / target
    return None


def held(path: Path) -> bool:
    """True when a process has ``path`` open (``lsof``); unknown counts as held, so nothing is removed blindly."""
    try:
        done = subprocess.run(["lsof", "-t", "--", str(path)], capture_output=True, text=True, timeout=10)
    except (OSError, subprocess.SubprocessError):
        return True
    return bool(done.stdout.strip())


def cleanup_git(argv: list[str], cwd, started: float, holders: Callable[[Path], bool] = held) -> list[str]:
    """Remove the ``index.lock`` the killed git command left: only one created since ``started``, held by nobody."""
    removed = []
    git_dir = _git_dir(argv, cwd)
    if git_dir is None:
        return removed
    lock = git_dir / "index.lock"
    try:
        if lock.is_file() and lock.stat().st_mtime >= started - 1 and not holders(lock):
            lock.unlink()
            removed.append(str(lock))
    except OSError:
        pass
    return removed


CLEANUP: dict[str, Callable] = {"git": cleanup_git, "git-commit": cleanup_git}


# -- the runner -------------------------------------------------------------------------------


@dataclass
class Result:
    """What :func:`run` returns; ``stdout``/``stderr`` as ``subprocess.CompletedProcess`` has them."""

    args: list
    returncode: int
    stdout: object = ""
    stderr: object = ""
    timed_out: bool = False
    seconds: float = 0.0
    error: str = ""
    cleaned: tuple = ()

    def check_returncode(self) -> None:
        if self.returncode:
            raise subprocess.CalledProcessError(self.returncode, self.args, self.stdout, self.stderr)


def run(argv, cls: str = "generic", cwd=None, env=None, *, home: Path | None = None, capture_output: bool = True,
        text: bool = False, stdin=None, input=None, timeout: float | None = None, check: bool = False,
        stdout=None, stderr=None, shell: bool = False, config: Config | None = None) -> Result:
    """Run ``argv`` like ``subprocess.run``, but always with a deadline.

    The deadline is ``timeout`` or the policy's value for ``cls``. A process that is not finished by then is
    stopped with its children, the class cleanup runs, and the result has ``returncode`` 124,
    ``timed_out`` and ``error`` (the plain message). Starting failures (no such program) come back as
    ``returncode`` 127 with ``error`` instead of raising."""
    home = Path(home) if home else tirith_home()
    limit = float(timeout) if timeout else effective(home, config, cls)
    out_arg = stdout if stdout is not None else (subprocess.PIPE if capture_output else None)
    err_arg = stderr if stderr is not None else (subprocess.PIPE if capture_output else None)
    began, started_wall = time.monotonic(), time.time()
    what = " ".join(map(str, argv))[:200] if isinstance(argv, (list, tuple)) else str(argv)[:200]
    try:
        proc = subprocess.Popen(argv, cwd=str(cwd) if cwd else None, env=env, shell=shell, text=text,
                                stdin=subprocess.PIPE if input is not None else stdin, stdout=out_arg,
                                stderr=err_arg, start_new_session=os.name == "posix")
    except OSError as error:
        return Result(list(argv) if isinstance(argv, (list, tuple)) else [argv], 127,
                      "" if text else b"", str(error) if text else str(error).encode(), error=str(error))
    timed_out = False
    try:
        out, err = proc.communicate(input=input, timeout=limit)
    except subprocess.TimeoutExpired:
        timed_out = True
        kill_group(proc)
        try:
            out, err = proc.communicate(timeout=KILL_GRACE_S)
        except subprocess.TimeoutExpired:
            out, err = ("" if text else b""), ("" if text else b"")
    except BaseException:
        kill_group(proc, 0.5)
        raise
    seconds = time.monotonic() - began
    lane = os.environ.get("TIRITH_LANE") or None
    cleaned: list[str] = []
    message = ""
    if timed_out:
        hook = CLEANUP.get(base_class(cls))
        if hook is not None:
            cleaned = hook(list(argv) if isinstance(argv, (list, tuple)) else [str(argv)], cwd, started_wall)
        message = timeout_message(cls, limit, "removed " + ", ".join(cleaned) if cleaned else "")
        record(home, cls, seconds, "timeout", command=what, timeout=limit, lane=lane)
    else:
        record(home, cls, seconds, "ok" if proc.returncode == 0 else "failed")
    result = Result(list(argv) if isinstance(argv, (list, tuple)) else [argv],
                    EXIT_TIMED_OUT if timed_out else proc.returncode, out if out is not None else "",
                    err if err is not None else "", timed_out, seconds, message, tuple(cleaned))
    if check:
        result.check_returncode()
    return result


def supervise(proc: subprocess.Popen, cls: str, *, home: Path | None = None, config: Config | None = None,
              timeout: float | None = None, argv=None, cwd=None, what: str = "") -> Callable[[], dict]:
    """Put an already started process (own process group) under the deadline of ``cls``; for the callers that
    stream its output (stages, agent runs). Returns ``done()``: call it after the process was waited for; it
    stops the timer, records the duration or timeout and returns ``{"timed_out", "seconds", "limit", "message"}``."""
    home = Path(home) if home else tirith_home()
    limit = float(timeout) if timeout else effective(home, config, cls)
    began, started_wall = time.monotonic(), time.time()
    fired = threading.Event()
    lane = os.environ.get("TIRITH_LANE") or None
    label = what or (" ".join(map(str, argv))[:200] if isinstance(argv, (list, tuple)) else str(argv or cls))

    def expire() -> None:
        fired.set()
        kill_group(proc)

    timer = threading.Timer(limit, expire)
    timer.daemon = True
    timer.start()

    def done() -> dict:
        timer.cancel()
        seconds = time.monotonic() - began
        info = {"timed_out": fired.is_set(), "seconds": seconds, "limit": limit, "message": ""}
        if fired.is_set():
            hook = CLEANUP.get(base_class(cls))
            cleaned = hook(list(argv) if isinstance(argv, (list, tuple)) else [], cwd, started_wall) if hook else []
            info["message"] = timeout_message(cls, limit, what)
            record(home, cls, seconds, "timeout", command=label, timeout=limit, lane=lane)
        else:
            record(home, cls, seconds, "ok" if proc.returncode == 0 else "failed")
        return info

    return done


# -- the agent hook ---------------------------------------------------------------------------


def bash_update(home: Path, config: Config, tool_input) -> dict | None:
    """The ``updatedInput`` for an agent's Bash call: its ``timeout`` (ms) from the policy of the command's
    class, keeping a larger value the agent asked for up to the class maximum. None when nothing changes."""
    if not isinstance(tool_input, dict) or not tool_input.get("command") or tool_input.get("run_in_background"):
        return None
    cls = classify_command(str(tool_input["command"]))
    policy_s = effective(home, config, cls)
    top_s = max(bounds(cls)[2], policy_s)
    asked = tool_input.get("timeout")
    asked_ms = float(asked) if isinstance(asked, (int, float)) and not isinstance(asked, bool) and asked > 0 else 0.0
    ms = int(min(max(asked_ms, policy_s * 1000), top_s * 1000))
    if tool_input.get("timeout") == ms:
        return None
    return {**tool_input, "timeout": ms}
