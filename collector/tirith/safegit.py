"""Safe-git shim for Windows: a `git.cmd` plus this file as its Python core.

Refuses what the owner's macOS git wrapper refuses: the flag that skips hooks (`--no-verify`, also its
abbreviations), `git commit -n` (also `-an`), any `core.hooksPath` override (`-c`, `--config-env`, `git
config`, the `GIT_CONFIG_*` environment) and `commit-tree`; every other call runs the real git unchanged.
`tirith safegit install` copies this file next to `git.cmd` in the shim folder that `lane run` puts first
in PATH. The macOS wrapper (`~/.local/bin/git`) is never touched. The top of this file imports the
standard library only: the installed copy runs without Tirith.
"""
from __future__ import annotations

import hashlib
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

MARKER = "REM tirith-safegit v1"
CORE_NAME = "tirith_safegit.py"
CMD_NAME = "git.cmd"
REAL_ENV = "TIRITH_REAL_GIT"

_HOOKS_PATH = "core.hookspath"
_NO_VERIFY = "--no-verify"
# commit options whose value is the next argument (the value is data, never a flag)
_VALUE_OPTS = {"-m", "-F", "-C", "-c", "-t", "--message", "--file", "--reuse-message", "--reedit-message",
               "--template", "--author", "--date", "--cleanup"}
_SHORT_WITH_VALUE = set("mFCct")
# global options (before the subcommand) that take a value
_GLOBAL_VALUE = {"-C", "-c", "--git-dir", "--work-tree", "--namespace", "--exec-path", "--config-env"}


def _refusal(rule: str, what: str, instead: str) -> str:
    return f"tirith: {rule}: {what}. Do this instead: {instead}"


def _is_no_verify(word: str) -> bool:
    """`--no-verify` or a unique-prefix abbreviation git accepts (`--no-ver`)."""
    return len(word) >= len("--no-ve") and _NO_VERIFY.startswith(word)


def _subcommand(args: list[str]) -> tuple[str | None, int]:
    """The git subcommand and the index after it (global options skipped)."""
    i = 0
    while i < len(args):
        a = args[i]
        if not a.startswith("-"):
            return a, i + 1
        i += 2 if a in _GLOBAL_VALUE else 1
    return None, len(args)


def check(args: list[str], env: dict[str, str] | None = None) -> str | None:
    """The refusal message for these git arguments, or None when they are fine."""
    env = os.environ if env is None else env
    for key, value in env.items():
        k = key.upper()
        if (k.startswith("GIT_CONFIG_KEY_") and value.lower() == _HOOKS_PATH) \
                or (k == "GIT_CONFIG_PARAMETERS" and _HOOKS_PATH in value.lower()):
            return _refusal("hooks-path", "core.hooksPath set through the environment",
                            "edit [[hooks]] in tirith.toml, run tirith check")
    sub, rest = _subcommand(args)
    skip_value = False
    for i, a in enumerate(args):
        if _HOOKS_PATH in a.lower():
            return _refusal("hooks-path", "a command naming core.hooksPath",
                            "edit [[hooks]] in tirith.toml, run tirith check")
        if skip_value:  # the message or file named by the previous option is data
            skip_value = False
            continue
        if i >= rest and a in _VALUE_OPTS and sub == "commit":
            skip_value = True
            continue
        if a == "--":
            break
        if any(_is_no_verify(w) for w in a.split()):
            return _refusal("no-verify", "the git flag that skips hooks",
                            "run the git command without it; fix what the hook reports")
        if sub == "commit" and i >= rest and re.fullmatch(r"-[A-Za-z]+", a):
            for ch in a[1:]:
                if ch == "n":
                    return _refusal("no-verify", "git commit -n (skips hooks)",
                                    "run git commit without -n; fix what the hook reports")
                if ch in _SHORT_WITH_VALUE:  # the rest of the cluster (or the next word) is its value
                    skip_value = a.endswith(ch)
                    break
    if sub == "commit-tree":
        return _refusal("commit-tree", "git commit-tree (writes a commit without the hooks)",
                        "use git commit")
    return None


def main(argv: list[str] | None = None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    message = check(args)
    if message:
        print(message, file=sys.stderr)
        return 2
    real = os.environ.get(REAL_ENV)
    if not real:
        print("tirith safegit: no real git configured; run tirith safegit install", file=sys.stderr)
        return 2
    return subprocess.call([real, *args])


# -- install (run from Tirith, not from the installed copy) ------------------------------------


def cmd_script(real_git: str, python: str) -> str:
    """The `git.cmd` text: CRLF, marker on line 2, runs the core with the real git path."""
    return (f"@echo off\r\n{MARKER}\r\n"
            "REM Written by `tirith safegit install`; do not edit. Refuses the git flags that skip hooks.\r\n"
            f'set "{REAL_ENV}={real_git}"\r\n'
            f'"{python}" "%~dp0{CORE_NAME}" %*\r\n'
            "exit /b %ERRORLEVEL%\r\n")


def is_ours(path: Path) -> bool:
    try:
        with path.open("r", encoding="utf-8", errors="replace") as handle:
            handle.readline()
            return handle.readline().startswith(MARKER)
    except OSError:
        return False


def _real_git(shim_dir: Path, path_env: str | None) -> str | None:
    path = path_env if path_env is not None else os.environ.get("PATH", "")
    parts = [p for p in path.split(os.pathsep) if p and Path(p).expanduser().resolve() != shim_dir.resolve()]
    found = shutil.which("git", path=os.pathsep.join(parts))
    return found if found and not is_ours(Path(found)) else None


def _digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def install(shim_dir: Path, path_env: str | None = None, python: str | None = None) -> tuple[str, str]:
    """Write `git.cmd` and the core into `shim_dir`; returns ``(status, detail)``."""
    target = shim_dir / CMD_NAME
    if target.exists() and not is_ours(target):
        return "skipped", f"{target} exists and is not a Tirith safe-git shim"
    real = _real_git(shim_dir, path_env)
    if real is None:
        return "skipped", f"'git' is not on PATH (outside {shim_dir})"
    shim_dir.mkdir(parents=True, exist_ok=True)
    (shim_dir / CORE_NAME).write_bytes(Path(__file__).read_bytes())
    target.write_bytes(cmd_script(real, python or sys.executable).encode("utf-8"))
    return "installed", str(target)


def uninstall(shim_dir: Path) -> tuple[str, str]:
    target = shim_dir / CMD_NAME
    if not target.exists():
        return "skipped", f"no {target}"
    if not is_ours(target):
        return "skipped", f"{target} is not a Tirith safe-git shim"
    target.unlink()
    (shim_dir / CORE_NAME).unlink(missing_ok=True)
    return "removed", str(target)


def verify(shim_dir: Path, path_env: str | None = None) -> list[str]:
    """Problems with the installed safe-git shim (``[]`` = intact)."""
    target, core = shim_dir / CMD_NAME, shim_dir / CORE_NAME
    fix = f"Fix: tirith safegit install --dir {shim_dir}"
    if not target.exists():
        return [f"safe-git: no shim at {target}. {fix}"]
    if not is_ours(target):
        return [f"safe-git: {target} is not a Tirith safe-git shim; move it away. {fix}"]
    problems = []
    text = target.read_bytes().decode("utf-8", errors="replace")
    if not core.exists():
        problems.append(f"safe-git: {core} is missing. {fix}")
    elif _digest(core) != _digest(Path(__file__)):
        problems.append(f"safe-git: {core} differs from this Tirith's core. {fix}")
    m = re.search(rf'set "{REAL_ENV}=(.*)"', text)
    if not m or not Path(m.group(1)).exists():
        problems.append(f"safe-git: real git {m.group(1) if m else '(unset)'} does not exist. {fix}")
    m = re.search(r'^"(.*)" "%~dp0', text, re.M)
    if not m or not Path(m.group(1)).exists():
        problems.append(f"safe-git: python {m.group(1) if m else '(unset)'} does not exist. {fix}")
    if path_env is not None:
        first = next((Path(p) for p in path_env.split(os.pathsep)
                      if p and any((Path(p) / n).exists() for n in ("git.cmd", "git.exe", "git"))), None)
        if first is None or first.expanduser().resolve() != shim_dir.resolve():
            problems.append(f"safe-git: {shim_dir} is not first for 'git' in PATH. "
                            f"Fix: put {shim_dir} before the real git's folder in PATH")
    return problems


if __name__ == "__main__":
    sys.exit(main())
