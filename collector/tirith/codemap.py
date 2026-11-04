"""The code map of a lane's brief: the files that matter for its item, found before the agent starts.

``section(repo, item)`` reads the item's words (the backticked paths, the ``-k`` word of its check, the plain
words), looks them up in the tracked files of the repo (by name and by content) and returns a short
``## Code map`` block for the brief: the files to read, the tests that cover them and the contracts. It runs
git read-only and never fails: with nothing found the block is left out. See ``docs/CONTRACT-launch.md``.
"""

from __future__ import annotations

import re
from collections import Counter
from pathlib import Path

from tirith import timeouts as timeouts_mod

MAX_FILES = 8  # code files listed
MAX_TESTS = 5
MAX_DOCS = 3
MAX_TERMS = 8
MIN_TERM = 4
CODE_EXT = {".py", ".js", ".mjs", ".rs", ".ts", ".css", ".html", ".toml", ".json"}
SKIP_DIRS = ("fixtures/", "site/", "node_modules/", "tools/")
STOP = set("""about after again also another because before being between cada como cuando cuantas cuantos cuántas
cuántos check deben desde donde each from have into lane lanes make muestra para pero porque que quien quién
recibe sin sobre solo todo todos todas tiene tienen tirith uno una unos esta este esto estos with without
work works when where which while will your pytest collector tests test item items plan""".split())
BACKTICK = re.compile(r"`([^`]+)`")
CHECK_K = re.compile(r"\buv run -q pytest\b[^`]*?-k\s+([A-Za-z0-9_]+)")
WORD = re.compile(r"[A-Za-zÁÉÍÓÚÑáéíóúñ_]{%d,}" % MIN_TERM)


def tracked(repo: Path) -> list[str]:
    done = timeouts_mod.run(["git", "-C", str(repo), "ls-files"], "git", text=True)
    return [l for l in (done.stdout or "").splitlines() if l] if done.returncode == 0 else []


def terms(item: str) -> tuple[list[str], list[str], str | None]:
    """(paths named, search words, the check's ``-k`` word) of an item's text."""
    kw = CHECK_K.search(item)
    paths, words = [], []
    for quoted in BACKTICK.findall(item):
        if "/" in quoted and " " not in quoted:
            paths.append(quoted.strip("/"))
    prose = BACKTICK.sub(" ", item)
    prose = re.sub(r"\(check:.*?\)", " ", prose)
    for w in WORD.findall(prose):
        w = w.lower()
        if w not in STOP and w not in words:
            words.append(w)
    words.sort(key=len, reverse=True)  # the long words are the most specific
    return paths, words[:MAX_TERMS], kw.group(1).lower() if kw else None


def _content_hits(repo: Path, words: list[str], files: list[str]) -> Counter:
    hits: Counter = Counter()
    for w in words:
        done = timeouts_mod.run(["git", "-C", str(repo), "grep", "-l", "-i", "-F", "-e", w, "--", "."],
                                "git", text=True)
        if done.returncode == 0:
            for f in (done.stdout or "").splitlines():
                hits[f] += 1
    return hits


def rank(repo: Path, item: str) -> dict[str, list[str]]:
    """``{"code": [...], "tests": [...], "docs": [...]}`` for the item, best first."""
    files = [f for f in tracked(repo) if not f.startswith(SKIP_DIRS)]
    if not files:
        return {"code": [], "tests": [], "docs": []}
    paths, words, kw = terms(item)
    present = set(files)
    named = [p for p in paths if p in present or any(f.startswith(p + "/") for f in files)]
    score: Counter = Counter()
    for f in files:
        low = f.lower()
        for w in words:
            if w in low:
                score[f] += 3
        if kw and kw in Path(f).stem.lower():
            score[f] += 5
    content = _content_hits(repo, ([kw] if kw else []) + words, files)
    for f, n in content.items():
        if f in present:
            score[f] += n
    for p in named:
        for f in files:
            if f == p or f.startswith(p + "/"):
                score[f] += 20
    ranked = [f for f, _ in sorted(score.items(), key=lambda kv: (-kv[1], kv[0])) if score[f] > 1]
    tests = [f for f in ranked if "/tests/" in f or f.startswith("tests/") or ".test." in f]
    docs = [f for f in ranked if f.startswith("docs/CONTRACT") and f not in tests]
    code = [f for f in ranked if f not in tests and f not in docs and Path(f).suffix in CODE_EXT
            and not f.startswith("docs/")]
    return {"code": code[:MAX_FILES], "tests": tests[:MAX_TESTS], "docs": docs[:MAX_DOCS]}


def section(repo: Path | None, item: str | None) -> str:
    """The ``## Code map`` block (with a trailing blank line), or ``""`` when nothing relevant was found."""
    if repo is None or not item:
        return ""
    try:
        found = rank(Path(repo), item)
    except Exception:  # a map is a help, never a reason to stop a lane
        return ""
    labels = (("code", "Code"), ("tests", "Tests"), ("docs", "Contracts"))
    if not any(found[k] for k, _ in labels):
        return ""
    out = ["## Code map", "Files likely relevant to your item, found by name and content (start here; "
           "this is a guide, not a limit):"]
    for key, label in labels:
        out += [f"- {label}: " + ", ".join(f"`{f}`" for f in found[key])] if found[key] else []
    return "\n".join(out) + "\n\n"
