"""Forbidden-words commit guard for every repository Tirith manages.

Installs the owner's FORBIDDEN_WORDS_GUARD hooks (pre-commit: staged file names and added lines;
commit-msg: the message) into a repository's common git dir, so every worktree of that repository
is covered. The hooks are the owner's original ones; only the word list is configurable. The default
list is the owner's six words (attribution words and model names; see DEFAULT_WORDS). It is stored
ROT13-encoded here because this file is itself committed to repositories that the guard protects.

Repositories carrying the FORBIDDEN_WORDS_GUARD marker are also protected by the owner's git wrapper
(~/.local/bin/git), which refuses --no-verify, commit -n, hooksPath overrides and commit-tree; Tirith
launches agents with that wrapper first in PATH.

Installed hooks are made immutable (chflags uchg on macOS; the read-only attribute on Windows, where the
hooks run under Git for Windows' bundled sh and are always written with LF line endings). This module never edits or removes an
existing guard: if the installed hooks differ from the expected ones it reports it, and only the owner
changes them.

usage: python3 tirith_guard.py install|verify <repo> [--words w1,w2,...] [--chain]
"""
from __future__ import annotations

import argparse
import codecs
import hashlib
import os
import stat
import subprocess
import sys
from pathlib import Path

MARKER = "FORBIDDEN_WORDS_GUARD"
DEFAULT_WORDS = [codecs.decode(w, "rot13") for w in ['pb-?nhguber?q?', 'pynhqr', 'bchf', 'zhfr', 'jbaqre', 'zrgn']]

PRE_COMMIT = r'''#!/bin/bash
# FORBIDDEN_WORDS_GUARD
# Blocks the commit when a staged file name or an added line contains a forbidden word
# (whole word, any case): {listed}.
# Installed by Tirith; words: {words}
set -euo pipefail

git diff --cached --no-color --no-ext-diff -U0 --diff-filter=ACMR | perl -ne '
    BEGIN {{ $re = qr/(?<![A-Za-z0-9])({regex})(?![A-Za-z0-9])/i; $bad = 0 }}
    if (/^\+\+\+ b\/(.*)$/) {{
        $file = $1;
        if ($file =~ $re) {{ print STDERR "  $file: \"$1\" in the file name\n"; $bad = 1 }}
        next;
    }}
    if (/^@@ -\S+ \+(\d+)/) {{ $line = $1; next }}
    if (/^\+/) {{
        if (/$re/) {{ chomp; my $t = substr($_, 1, 120); print STDERR "  $file:$line: \"$1\" in: $t\n"; $bad = 1 }}
        $line++;
    }}
    END {{
        if ($bad) {{
            print STDERR "\npre-commit: commit blocked, forbidden words in the staged changes (see above).\n";
            exit 1;
        }}
    }}
'
'''

COMMIT_MSG = r'''#!/bin/bash
# FORBIDDEN_WORDS_GUARD
# Blocks the commit when the message contains a forbidden word (whole word, any case):
# {listed}. Lines starting with # are ignored.
# Installed by Tirith; words: {words}
set -euo pipefail

perl -ne '
    BEGIN {{ $re = qr/(?<![A-Za-z0-9])({regex})(?![A-Za-z0-9])/i; $bad = 0 }}
    next if /^#/;
    if (/$re/) {{ chomp; print STDERR "  line $.: \"$1\" in: $_\n"; $bad = 1 }}
    END {{
        if ($bad) {{
            print STDERR "\ncommit-msg: commit blocked, forbidden words in the commit message (see above).\n";
            exit 1;
        }}
    }}
' "$1"
'''


def _windows() -> bool:
    return sys.platform.startswith("win")


def _git() -> str:
    return "git" if _windows() else "/usr/bin/git"


GUARD_VERSION = 2
HOOKS = ("pre-commit", "commit-msg", "pre-push")
_BODY_START = "set -euo pipefail\n"

PRE_PUSH = r'''#!/bin/bash
# FORBIDDEN_WORDS_GUARD
# Records every push attempt and blocks it when `tirith check` fails.
# Installed by Tirith; words: {words}
set -euo pipefail
'''

# Recording prologue, POSIX sh syntax. @HOOK@ and @REASON@ are filled per hook.
_RECORD = r'''
# --- Tirith: record every run of this hook (TIRITH_GUARD_VERSION=2) ---
TIRITH_HOOK=@HOOK@
TIRITH_REASON="@REASON@"
tirith_json() { printf '%s' "$1" | tr '\000-\037' ' ' | sed 's/\\/\\\\/g; s/"/\\"/g'; }
tirith_record() {
    _t_home=${TIRITH_HOME:-${HOME:-.}/.tirith}
    _t_repo=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
    _t_lane=null
    if [ -n "${TIRITH_LANE:-}" ]; then _t_lane="\"$(tirith_json "$TIRITH_LANE")\""; fi
    mkdir -p "$_t_home" 2>/dev/null || return 0
    printf '{"kind":"git-hook","hook":"%s","repo":"%s","result":"%s","reason":"%s","at":"%s","lane":%s}\n' \
        "$(tirith_json "$1")" "$(tirith_json "$_t_repo")" "$2" "$(tirith_json "$3")" \
        "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$_t_lane" >> "$_t_home/events.jsonl" 2>/dev/null || true
}
tirith_exit() {
    _t_code=$?
    trap - EXIT
    if [ "$_t_code" -ne 0 ]; then tirith_record "$TIRITH_HOOK" blocked "$TIRITH_REASON"; fi
    exit "$_t_code"
}
trap tirith_exit EXIT
tirith_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
'''

_RUN_HOOKS = r'''
# --- Tirith: [[hooks]] of tirith.toml for this event, through `tirith run` ---
tirith_hook_runs() {
    [ -f "$1" ] || return 0
    awk -v ev="$2" '
        function flush() { if (e == ev && r != "") print r; e = ""; r = "" }
        function val(line) {
            sub(/^[^=]*=[ \t]*/, "", line); sub(/[ \t]*$/, "", line)
            if (line ~ /^"/) { sub(/^"/, "", line); sub(/"$/, "", line); gsub(/\\"/, "\"", line); gsub(/\\\\/, "\\", line) }
            return line
        }
        /^[ \t]*\[/ { flush(); inh = ($0 ~ /^[ \t]*\[\[[ \t]*hooks[ \t]*\]\]/); next }
        inh && /^[ \t]*event[ \t]*=/ { e = val($0) }
        inh && /^[ \t]*run[ \t]*=/ { r = val($0) }
        END { flush() }
    ' "$1"
}
if command -v tirith >/dev/null 2>&1; then
    tirith_runs=$(tirith_hook_runs "$tirith_root/tirith.toml" "$TIRITH_HOOK")
    if [ -n "$tirith_runs" ]; then
        while IFS= read -r tirith_cmd; do
            [ -n "$tirith_cmd" ] || continue
            TIRITH_REASON="tirith run --stage $TIRITH_HOOK failed: $tirith_cmd"
            (cd "$tirith_root" && tirith run --stage "$TIRITH_HOOK" -- /bin/sh -c "$tirith_cmd") </dev/null || exit 1
        done <<TIRITH_EOF
$tirith_runs
TIRITH_EOF
    fi
fi
'''

_RUN_CHECK = r'''
# --- Tirith: `tirith check` before a push ---
if command -v tirith >/dev/null 2>&1; then
    (cd "$tirith_root" && tirith check "$tirith_root") </dev/null || exit 1
fi
'''

# The repository's own hook, moved to <hook>.local by `install --chain`, runs after the guard's checks.
_CHAIN = r'''
# --- Tirith: the repository's own hook, kept as @HOOK@.local ---
tirith_local="$(cd "$(dirname "$0")" && pwd)/@HOOK@.local"
if [ -x "$tirith_local" ]; then
    TIRITH_REASON="the repository's own @HOOK@ hook failed"
    "$tirith_local" "$@" || exit 1
fi
'''

_ALLOWED = r'''
trap - EXIT
tirith_record "$TIRITH_HOOK" allowed ok
'''


def _for_windows(text: str) -> str:
    """The hook text for Git for Windows' sh: a CRLF `tirith.toml` must not leak a carriage return into
    the commands, and `sh` is found through PATH."""
    text = text.replace("        /^[ \\t]*\\[/ { flush();",
                        "        { sub(/\\r$/, \"\") }\n        /^[ \\t]*\\[/ { flush();")
    return text.replace("-- /bin/sh -c", "-- sh -c")


def _record_part(hook: str, reason: str) -> str:
    return _RECORD.replace("@HOOK@", hook).replace("@REASON@", reason)


def render_v1(words: list[str]) -> dict[str, str]:
    """The original two hooks (no recording): the owner's guard text only."""
    attribution = codecs.decode('pbnhgube/pb-nhguberq', "rot13")
    listed = ", ".join(attribution if w.startswith("co-") else w for w in words)
    args = {"listed": listed, "words": ",".join(words), "regex": "|".join(words)}
    return {"pre-commit": PRE_COMMIT.format(**args), "commit-msg": COMMIT_MSG.format(**args)}


def render(words: list[str], chained: frozenset[str] | set[str] = frozenset(),
           windows: bool | None = None) -> dict[str, str]:
    """Hook file name -> content for this word list (regex alternatives, in order).

    pre-commit and commit-msg are the owner's text split once at ``set -euo pipefail``: the header
    before it and the guard body after it stay byte for byte; the recording prologue sits in
    between, the ``[[hooks]]`` runs and the "allowed" record follow the body. A hook in ``chained``
    also runs the repository's own ``<hook>.local`` (see ``install(chain=True)``) after its checks.
    ``windows`` (default: this machine) adjusts the added parts for Git for Windows' sh; the text is LF only."""
    reasons = {"pre-commit": "forbidden words in the staged changes",
               "commit-msg": "forbidden words in the commit message"}
    out = {}
    for name, text in render_v1(words).items():
        head, sep, body = text.partition(_BODY_START)
        out[name] = (head + sep + _record_part(name, reasons[name]) + "\n" + body + _RUN_HOOKS
                     + (_CHAIN.replace("@HOOK@", name) if name in chained else "") + _ALLOWED)
    out["pre-push"] = (PRE_PUSH.format(words=",".join(words)) + _record_part("pre-push", "tirith check failed")
                       + _RUN_CHECK + _RUN_HOOKS
                       + (_CHAIN.replace("@HOOK@", "pre-push") if "pre-push" in chained else "") + _ALLOWED)
    if _windows() if windows is None else windows:
        out = {name: _for_windows(text) for name, text in out.items()}
    return out


def hooks_dir(repo: Path) -> Path:
    out = subprocess.run([_git(), "-C", str(repo), "rev-parse", "--path-format=absolute",
                          "--git-common-dir"], capture_output=True, text=True, check=True).stdout.strip()
    return Path(out) / "hooks"


def _read(p: Path) -> str:
    """File text without newline translation (a CRLF hook must show as CRLF)."""
    return p.read_bytes().decode("utf-8", errors="replace")


def digest(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def verify(repo: Path, words: list[str]) -> list[str]:
    """Problems with the guard of `repo` (empty list = intact and as configured)."""
    problems = []
    d = hooks_dir(repo)
    old = _is_v1(d, words)
    v1 = render_v1(words)
    chained = {n for n in HOOKS if (d / f"{n}.local").exists()}
    for name, want in render(words, chained).items():
        p = d / name
        if not p.exists():
            if not (old and name == "pre-push"):
                problems.append(f"{name}: missing")
            continue
        have = _read(p)
        if MARKER not in have:
            problems.append(f"{name}: present but without the {MARKER} marker")
        elif "\r" in have:
            problems.append(f"{name}: has CRLF line endings (sh would not run it); reinstall it with LF")
        elif digest(have) != digest(want) and not (old and digest(have) == digest(v1.get(name, ""))):
            problems.append(f"{name}: differs from the expected guard for words {','.join(words)}")
        st = p.stat()
        if _windows():  # no execute bit there (Git for Windows' sh runs it); the lock is the read-only attribute
            if st.st_mode & stat.S_IWRITE:
                problems.append(f"{name}: not read-only")
            continue
        if not st.st_mode & stat.S_IXUSR:
            problems.append(f"{name}: not executable")
        if hasattr(st, "st_flags") and not st.st_flags & stat.UF_IMMUTABLE:
            problems.append(f"{name}: not immutable (uchg)")
    for name in sorted(chained):
        if not _windows() and not os.access(d / f"{name}.local", os.X_OK):
            problems.append(f"{name}.local: not executable (the chained {name} hook would be skipped)")
    return problems


def _is_v1(d: Path, words: list[str]) -> bool:
    """True when pre-commit and commit-msg are the version 1 guard (and pre-push is not there)."""
    try:
        return all(_read(d / n) == t for n, t in render_v1(words).items()) \
            and not (d / "pre-push").exists()
    except OSError:
        return False


def outdated(repo: Path, words: list[str]) -> list[str]:
    """Owner steps to move an intact version 1 guard to the current one ([] = already current)."""
    d = hooks_dir(repo)
    if verify(repo, words) or not _is_v1(d, words):
        return []
    files = " ".join(str(d / n) for n in ("pre-commit", "commit-msg"))
    if _windows():
        return ["guard intact but older version (version %d also records each hook run, runs the [[hooks]] of "
                "tirith.toml and adds pre-push): reinstall as the owner:" % GUARD_VERSION,
                f"  1. attrib -r {files}", f"  2. del {files}", f"  3. tirith guard install {repo}"]
    return [f"guard intact but older version (version {GUARD_VERSION} also records each hook run, runs "
            "the [[hooks]] of tirith.toml and adds pre-push): reinstall as the owner:",
            f"  1. chflags nouchg {files}",
            f"  2. rm {files}",
            f"  3. tirith guard install {repo}"]


def _is_locked(p: Path) -> bool:
    if _windows():
        return not p.stat().st_mode & stat.S_IWRITE
    return not hasattr(os, "chflags") or bool(p.stat().st_flags & stat.UF_IMMUTABLE)


def _lock(p: Path) -> None:
    if _windows():
        os.chmod(p, stat.S_IREAD)  # the read-only attribute
    elif hasattr(os, "chflags"):
        os.chflags(p, p.stat().st_flags | stat.UF_IMMUTABLE)


def _unlock(p: Path) -> None:
    """Undo `_lock` (owner steps only: reinstall, chaining)."""
    if _windows():
        os.chmod(p, stat.S_IREAD | stat.S_IWRITE)
    elif hasattr(os, "chflags"):
        os.chflags(p, p.stat().st_flags & ~stat.UF_IMMUTABLE)


def install(repo: Path, words: list[str], chain: bool = False) -> list[str]:
    """Installs missing hooks; never edits an existing guard. Returns remaining problems.

    With ``chain``, a hook the repository already has that is not the guard (no marker) is moved to
    ``<hook>.local`` and the guard installed in its place; the guard runs the ``.local`` hook after its own
    checks and fails when it fails. A ``.local`` file that already exists is never overwritten."""
    d = hooks_dir(repo)
    d.mkdir(parents=True, exist_ok=True)
    old = _is_v1(d, words)
    if chain:
        for name in HOOKS:
            p, local = d / name, d / f"{name}.local"
            if p.exists() and MARKER not in _read(p) and not local.exists():
                _unlock(p)
                p.rename(local)
    chained = {n for n in HOOKS if (d / f"{n}.local").exists()}
    for name, want in render(words, chained).items():
        p = d / name
        if p.exists() or (old and name == "pre-push"):  # a version 1 guard is replaced as a whole
            if p.exists() and _read(p) == want and not _is_locked(p):
                _lock(p)  # the guard as expected that lost its lock: locking it edits nothing
            continue
        p.write_bytes(want.encode("utf-8"))  # LF on every platform
        p.chmod(0o755)
        _lock(p)
    return verify(repo, words)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("action", choices=["install", "verify"])
    ap.add_argument("repo", type=Path)
    ap.add_argument("--words", default=",".join(DEFAULT_WORDS))
    ap.add_argument("--chain", action="store_true",
                    help="install: keep the repository's own hooks as <hook>.local and run them after the guard")
    a = ap.parse_args(argv)
    words = [w.strip() for w in a.words.split(",") if w.strip()]
    repo = a.repo.expanduser().resolve()
    problems = install(repo, words, a.chain) if a.action == "install" else verify(repo, words)
    if problems:
        print("guard: NOT OK\n  " + "\n  ".join(problems))
        return 1
    print(f"guard: OK ({', '.join(words)})")
    for line in outdated(a.repo.expanduser().resolve(), words):
        print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
