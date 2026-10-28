"""Forbidden-words commit guard for every repository Tirith manages.

Installs the owner's FORBIDDEN_WORDS_GUARD hooks (pre-commit: staged file names and added lines;
commit-msg: the message) into a repository's common git dir, so every worktree of that repository
is covered. The hooks are the owner's original ones; only the word list is configurable. The default
list is the owner's six words (attribution words and model names; see DEFAULT_WORDS). It is stored
ROT13-encoded here because this file is itself committed to repositories that the guard protects.

Repositories carrying the FORBIDDEN_WORDS_GUARD marker are also protected by the owner's git wrapper
(~/.local/bin/git), which refuses --no-verify, commit -n, hooksPath overrides and commit-tree; Tirith
launches agents with that wrapper first in PATH.

Installed hooks are made immutable (chflags uchg on macOS). This module never edits or removes an
existing guard: if the installed hooks differ from the expected ones it reports it, and only the owner
changes them.

usage: python3 tirith_guard.py install|verify <repo> [--words w1,w2,...]
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


def render(words: list[str]) -> dict[str, str]:
    """Hook file name -> content for this word list (regex alternatives, in order)."""
    attribution = codecs.decode('pbnhgube/pb-nhguberq', "rot13")
    listed = ", ".join(attribution if w.startswith("co-") else w for w in words)
    args = {"listed": listed, "words": ",".join(words), "regex": "|".join(words)}
    return {"pre-commit": PRE_COMMIT.format(**args), "commit-msg": COMMIT_MSG.format(**args)}


def hooks_dir(repo: Path) -> Path:
    out = subprocess.run(["/usr/bin/git", "-C", str(repo), "rev-parse", "--path-format=absolute",
                          "--git-common-dir"], capture_output=True, text=True, check=True).stdout.strip()
    return Path(out) / "hooks"


def digest(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def verify(repo: Path, words: list[str]) -> list[str]:
    """Problems with the guard of `repo` (empty list = intact and as configured)."""
    problems = []
    d = hooks_dir(repo)
    for name, want in render(words).items():
        p = d / name
        if not p.exists():
            problems.append(f"{name}: missing")
            continue
        have = p.read_text()
        if MARKER not in have:
            problems.append(f"{name}: present but without the {MARKER} marker")
        elif digest(have) != digest(want):
            problems.append(f"{name}: differs from the expected guard for words {','.join(words)}")
        st = p.stat()
        if not st.st_mode & stat.S_IXUSR:
            problems.append(f"{name}: not executable")
        if hasattr(st, "st_flags") and not st.st_flags & stat.UF_IMMUTABLE:
            problems.append(f"{name}: not immutable (uchg)")
    return problems


def install(repo: Path, words: list[str]) -> list[str]:
    """Installs missing hooks; never touches an existing one. Returns remaining problems."""
    d = hooks_dir(repo)
    d.mkdir(parents=True, exist_ok=True)
    for name, want in render(words).items():
        p = d / name
        if p.exists():
            continue
        p.write_text(want)
        p.chmod(0o755)
        if hasattr(os, "chflags"):
            os.chflags(p, p.stat().st_flags | stat.UF_IMMUTABLE)
    return verify(repo, words)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("action", choices=["install", "verify"])
    ap.add_argument("repo", type=Path)
    ap.add_argument("--words", default=",".join(DEFAULT_WORDS))
    a = ap.parse_args(argv)
    words = [w.strip() for w in a.words.split(",") if w.strip()]
    problems = (install if a.action == "install" else verify)(a.repo.expanduser().resolve(), words)
    if problems:
        print("guard: NOT OK\n  " + "\n  ".join(problems))
        return 1
    print(f"guard: OK ({', '.join(words)})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
