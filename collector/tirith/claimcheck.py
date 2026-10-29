"""A lane's final report is checked against its real change before the lane can merge.

``verify`` pulls the checkable claims out of the report text (regexes and line structure, no model) and
compares each with a fact: the lane diff, the test functions the diff adds, the recorded test runs of the
worktree, git, and the files on disk. Every claim that does not hold becomes one sentence in
``Result.mismatches``. A brief that asks to *fix* something while the diff touches only docs is flagged too.
``tirith lane review`` and the automatic review call it after the tests; a mismatch is the review result
``claims-failed``. See ``docs/CONTRACT-launch.md``.
"""

from __future__ import annotations

import re
import subprocess
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path

from tirith import timeouts as timeouts_mod
from tirith.home import parse_iso
from tirith.store import lane_path, read_json, read_jsonl

MARK = "## Claim check failed"   # the heading of the note appended to a brief that is sent back
MAX_CHAIN = 5

_PATH = re.compile(r"(?<![\w./~-])((?:~/|/|\./)?(?:[\w.@+-]+/)*[\w.@+-]+\.[A-Za-z][A-Za-z0-9]{0,7})(?![\w/-])")
_CHANGE = re.compile(r"\b(?:chang(?:ed|es)|add(?:ed|s)|creat(?:ed|es)|modif(?:ied|ies)|updat(?:ed|es)|"
                     r"edit(?:ed|s)|wrote|written|new file|touch(?:ed|es)|renam(?:ed|es)|remov(?:ed|es)|"
                     r"delet(?:ed|es)|extend(?:ed|s)|rewrote|implemented)\b", re.IGNORECASE)
_NEGATED = re.compile(r"\b(?:not|never|no|didn'?t|did not|wasn'?t|without|unchanged|untouched|except|"
                      r"instead of|rather than)\b", re.IGNORECASE)
_NUM = {"a": 1, "an": 1, "one": 1, "two": 2, "three": 3, "four": 4, "five": 5, "six": 6, "seven": 7,
        "eight": 8, "nine": 9, "ten": 10, "eleven": 11, "twelve": 12}
_COUNT = r"(\d+|a|an|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve)"
_ADDED_TESTS = re.compile(
    rf"\b(?:add(?:ed|s)?|wrote|writ(?:ten|es)|creat(?:ed|es)|new)\s+{_COUNT}\s+(?:new\s+)?"
    r"(?:[\w-]+\s+){0,2}?(?:tests?|test cases?|specs?)\b", re.IGNORECASE)
_TEST_NAME = re.compile(r"(?<![/\w.])(test_\w+)\b(?!\.\w)")
_PASSED = re.compile(r"(\d+)\s+(?:tests?\s+)?passed", re.IGNORECASE)
_PASS_CLAIM = re.compile(r"\b(?:all\s+)?tests?\s+(?:all\s+)?(?:pass(?:ed|es|ing)?|are\s+green|green)\b|"
                         r"\bpass(?:ed|es)?\s+(?:all\s+)?tests?\b", re.IGNORECASE)
_SHA = re.compile(r"\b(?:commit(?:ted)?|sha|as|at|commit id)\s*[:=]?\s*(?:as\s+)?`?([0-9a-f]{7,40})\b`?",
                  re.IGNORECASE)
_MERGED = re.compile(r"\bmerged\b", re.IGNORECASE)
_PUSHED = re.compile(r"\bpushed\b", re.IGNORECASE)
_NOT_DONE = re.compile(r"\b(?:not|never|n't|nothing|un)\s*(?:yet\s+)?(?:be\s+|been\s+)?(?:merged|pushed)\b|"
                       r"\b(?:to|will|ready to|can|must|should|before|until|then)\s+(?:be\s+)?(?:merge|push)(?:ed)?\b|"
                       r"\bunmerged\b|\bmerged\s+(?:origin/)?(?:main|master)\b|\bmerge\s+(?:origin/)?(?:main|master)\b", re.IGNORECASE)
_SHOT = re.compile(r"\b(?:screenshots?|looked at|viewed|inspected|eyeballed)\b", re.IGNORECASE)
_IMAGE = re.compile(r"\.(?:png|jpe?g|gif|webp)$", re.IGNORECASE)
_FIX_ITEM = re.compile(r"^\s*(?:[-*]|\d+[.)])\s*\**(?:fix|repair|correct)\b[^\n]*", re.IGNORECASE | re.MULTILINE)
_DO_ITEM = re.compile(r"^\s*(?:[-*]|\d+[.)])\s*\**do\b(?!\s*not|n't)[^\n]*", re.IGNORECASE | re.MULTILINE)
_DOC_WORDS = re.compile(r"\b(?:docs?|documentation|readme|wording|text|comment|typo|copy)\b", re.IGNORECASE)
_DOC_FILE = re.compile(r"(?:\.(?:md|rst)$|(?:^|/)docs?/|(?:^|/)readme)", re.IGNORECASE)
_TEST_FN = [re.compile(p) for p in (
    r"^\+\s*(?:async\s+)?def\s+(test_\w+)", r"^\+\s*(?:pub\s+)?(?:async\s+)?fn\s+(test_\w+|\w+)\s*\(",
    r"^\+\s*(?:test|it)\(\s*['\"`]([^'\"`]+)", r"^\+\s*(?:async\s+)?function\s+(test\w+)")]


@dataclass
class Result:
    claims: list[str] = field(default_factory=list)       # what was checked, one line each
    mismatches: list[str] = field(default_factory=list)   # what did not hold, one sentence each

    @property
    def ok(self) -> bool:
        return not self.mismatches

    def public(self) -> dict:
        return {"ok": self.ok, "checked": len(self.claims), "mismatches": list(self.mismatches)}


def _git(folder: Path, *args: str) -> str:
    done = timeouts_mod.run(["git", "-C", str(folder), *args], "git", text=True)
    return done.stdout if done.returncode == 0 else ""


def _ok(folder: Path, *args: str) -> bool:
    return timeouts_mod.run(["git", "-C", str(folder), *args], "git").returncode == 0


# -- the facts ---------------------------------------------------------------------------------


def chain(home: Path, lane: dict) -> list[dict]:
    """The lane and the lanes it continued (``fallback_lane``), newest first: a lane sent back works on
    the same change, so the diff base and the start time are the first lane's."""
    out, seen = [lane], {str(lane.get("id"))}
    while len(out) < MAX_CHAIN and lane.get("fallback_lane"):
        before = read_json(lane_path(home, str(lane["fallback_lane"])), None)
        if not isinstance(before, dict) or str(before.get("id")) in seen:
            break
        out.append(before)
        seen.add(str(before.get("id")))
        lane = before
    return out


def lane_diff(workspace: Path, base: str | None) -> tuple[dict[str, list[str]], list[str]]:
    """``(added lines per changed file, paths)``: ``git diff <base>`` (commits and uncommitted) plus untracked."""
    added: dict[str, list[str]] = {}
    paths: list[str] = []
    if base:
        current = None
        for line in _git(workspace, "diff", "--no-color", "-U0", base).splitlines():
            if line.startswith("+++ "):
                current = line[6:] if line.startswith("+++ b/") else None
                if current:
                    added.setdefault(current, [])
                    paths.append(current)
            elif current and line.startswith("+") and not line.startswith("+++"):
                added[current].append(line)
    for rel in _git(workspace, "ls-files", "--others", "--exclude-standard").splitlines():
        try:
            text = (workspace / rel).read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        added[rel] = ["+" + ln for ln in text.splitlines()]
        paths.append(rel)
    return added, paths


def test_functions(lines: list[str]) -> set[str]:
    """Names of the test functions among added diff lines (python, rust, js)."""
    names, rust = set(), False
    for line in lines:
        rust = rust or bool(re.match(r"^\+\s*#\[(?:tokio::)?test", line))
        for index, pattern in enumerate(_TEST_FN):
            found = pattern.match(line)
            if not found:
                continue
            if index == 1 and not (found.group(1).startswith("test_") or rust):
                continue
            names.add(found.group(1))
            rust = False
    return names


def last_change(workspace: Path, paths: list[str]) -> float:
    times = []
    for rel in paths:
        try:
            times.append((workspace / rel).stat().st_mtime)
        except OSError:
            pass
    return max(times, default=0.0)


def test_runs(home: Path, workspace: Path) -> list[dict]:
    """Passed test-stage runs recorded for this worktree: ``{at (end), passed counts}``."""
    want = str(workspace.resolve())
    runs, open_start = [], {}
    for event in read_jsonl(home / "events.jsonl"):
        where = str(event.get("worktree") or event.get("cwd") or "")
        if not where or Path(where).resolve() != Path(want):
            continue
        key = (event.get("run"), event.get("stage"))
        if event.get("event") == "start":
            open_start[key] = event
        elif event.get("event") == "end" and event.get("status") == "passed" and "test" in str(event.get("stage")):
            at = parse_iso(event.get("at"))
            counts = _log_counts(event.get("log"))
            if at:
                runs.append({"at": at.timestamp(), "counts": counts, "run": event.get("run")})
    return runs


def _log_counts(path) -> list[int]:
    try:
        text = Path(str(path)).read_text(encoding="utf-8", errors="replace")[-200_000:]
    except OSError:
        return []
    return [int(n) for n in _PASSED.findall(text)]


# -- the claims --------------------------------------------------------------------------------


def _lines(report: str) -> list[str]:
    return [ln.strip() for ln in str(report or "").splitlines() if ln.strip()]


def _clean(token: str) -> str:
    return token.strip("`'\"*_()[]<>,;:")


def _paths_in(line: str) -> list[str]:
    out = []
    for found in _PATH.findall(line):
        token = _clean(found)
        if token and not token.startswith(("http", "www.")) and not re.fullmatch(r"\d+(?:\.\d+)+", token) \
                and not re.fullmatch(r"[A-Za-z]\.[A-Za-z]\.?", token):
            out.append(token)
    return out


def _in_diff(token: str, paths: list[str]) -> str | None:
    token = token.removeprefix("./")
    for rel in paths:
        if rel == token or rel.endswith("/" + token) or (token.endswith("/" + rel)) or \
                (token.startswith(("/", "~")) and token.endswith(rel)):
            return rel
    return None


def _number(text: str) -> int:
    return int(text) if text.isdigit() else _NUM.get(text.lower(), 0)


def check_files(report: str, paths: list[str], result: Result) -> None:
    seen = set()
    for line in _lines(report):
        if not _CHANGE.search(line) or _NEGATED.search(line) or _SHOT.search(line):
            continue
        for token in _paths_in(line):
            outside = token.startswith(("/", "~")) and not _in_diff(token, paths)
            if token in seen or _IMAGE.search(token) or outside:
                continue
            seen.add(token)
            result.claims.append(f"changed {token}")
            if not _in_diff(token, paths):
                result.mismatches.append(f"said it changed or added {token}; it is not in the lane diff")


def check_tests_added(report: str, added: dict[str, list[str]], result: Result) -> None:
    per_file = {rel: test_functions(lines) for rel, lines in added.items()}
    everything = set().union(*per_file.values()) if per_file else set()
    for line in _lines(report):
        if _NEGATED.search(line) and not _ADDED_TESTS.search(line):
            continue
        counted = _ADDED_TESTS.search(line)
        if counted:
            wanted = _number(counted.group(1))
            result.claims.append(f"added {wanted} test(s)")
            if wanted > len(everything):
                result.mismatches.append(
                    f"said it added {wanted} test(s); the diff adds {len(everything)} test function(s)")
        if not (_CHANGE.search(line) or counted):
            continue
        files = [t for t in _paths_in(line) if not t.startswith(("/", "~"))]
        for name in dict.fromkeys(_TEST_NAME.findall(line)):
            result.claims.append(f"added test {name}")
            hit = [rel for rel, names in per_file.items() if name in names]
            if not hit:
                where = f" in {files[0]}" if files else ""
                result.mismatches.append(f"said it added {name}{where}; no such test in the diff")
            elif files and not any(_in_diff(f, hit) for f in files):
                result.mismatches.append(f"said it added {name} in {files[0]}; the diff adds it in {hit[0]}")


def check_tests_pass(report: str, home: Path, workspace: Path, since: float, result: Result) -> None:
    for line in _lines(report):
        passes = _PASS_CLAIM.search(line) and not _NEGATED.search(line)
        counts = [int(n) for n in _PASSED.findall(line)]
        if not passes and not counts:
            continue
        runs = [r for r in test_runs(home, workspace) if r["at"] + 1 >= since]  # event times have whole seconds
        result.claims.append("tests passed" + (f" ({', '.join(map(str, counts))})" if counts else ""))
        if not runs:
            result.mismatches.append(
                "said the tests pass; no passing test run of the lane's worktree is recorded after its last change")
            continue
        known = {n for run in runs for n in run["counts"]}
        for count in counts:
            if known and count not in known:
                result.mismatches.append(
                    f"said {count} passed; the recorded run(s) after the last change show "
                    f"{', '.join(str(n) for n in sorted(known))} passed")


def check_git(report: str, workspace: Path, repo: Path | None, result: Result) -> None:
    head = _git(workspace, "rev-parse", "HEAD").strip()
    for line in _lines(report):
        if _NOT_DONE.search(line):
            continue
        for sha in dict.fromkeys(_SHA.findall(line)):
            if not re.search(r"\d", sha):  # a plain word such as "added" or "decade" is not a sha
                continue
            result.claims.append(f"commit {sha}")
            if not _ok(workspace, "cat-file", "-e", f"{sha}^{{commit}}"):
                result.mismatches.append(f"said it committed as {sha}; there is no such commit in the repo")
    text = "\n".join(ln for ln in _lines(report) if not _NOT_DONE.search(ln))
    if _MERGED.search(text) and repo is not None and head:
        result.claims.append("merged")
        if not _ok(repo, "merge-base", "--is-ancestor", head, "HEAD"):
            result.mismatches.append("said it merged; the lane's commits are not in the repo's branch")
    if _PUSHED.search(text) and repo is not None and head:
        result.claims.append("pushed")
        if not _git(repo, "branch", "-r", "--contains", head).strip():
            result.mismatches.append("said it pushed; the lane's commits are on no remote branch")


def check_screenshots(report: str, workspace: Path, started: float, result: Result) -> None:
    for line in _lines(report):
        if not _SHOT.search(line) or _NEGATED.search(line):
            continue
        for token in _paths_in(line):
            if not _IMAGE.search(token):
                continue
            path = Path(token).expanduser()
            path = path if path.is_absolute() else workspace / path
            result.claims.append(f"screenshot {token}")
            try:
                newer = path.stat().st_mtime >= started
            except OSError:
                result.mismatches.append(f"said it looked at the screenshot {token}; the file does not exist")
                continue
            if not newer:
                result.mismatches.append(f"said it looked at the screenshot {token}; the file is older than the lane")


def fix_items(brief: str) -> list[str]:
    return [m.group(0).strip(" -*\t") for m in [*_FIX_ITEM.finditer(brief), *_DO_ITEM.finditer(brief)]]


def check_fix(brief: str, paths: list[str], result: Result) -> None:
    """The brief asks to fix or do something, the diff touches only docs (or nothing): it was explained."""
    items = [i for i in fix_items(brief) if not _DOC_WORDS.search(i)]
    if not items:
        return
    result.claims.append("the brief's fix/do items")
    if not paths or all(_DOC_FILE.search(p) for p in paths):
        shown = items[0] if len(items[0]) <= 100 else items[0][:99] + "…"
        result.mismatches.append(
            f"the brief asks \"{shown}\" but the diff {'is empty' if not paths else 'touches only docs'}; "
            "explaining or documenting is not fixing")


# -- the entry point ---------------------------------------------------------------------------


def verify(home: Path, lane: dict, repo: Path | None = None, *, report: str | None = None,
           brief: str | None = None) -> Result:
    """Check the lane's report (``report`` or ``lane["report"]``) against its worktree."""
    result = Result()
    text = str(report if report is not None else lane.get("report") or "")
    workspace = Path(str(lane.get("workspace") or ""))
    if not text.strip() or not workspace.is_dir():
        return result
    first = chain(home, lane)[-1]
    added, paths = lane_diff(workspace, first.get("start_commit") or lane.get("start_commit"))
    started = parse_iso(first.get("started_at"))
    start_ts = started.timestamp() if isinstance(started, datetime) else 0.0
    if brief is None:
        try:
            brief = Path(str(lane.get("brief") or "")).read_text(encoding="utf-8", errors="replace")
        except OSError:
            brief = ""
    check_files(text, paths, result)
    check_tests_added(text, added, result)
    check_tests_pass(text, home, workspace, last_change(workspace, paths), result)
    check_git(text, workspace, repo, result)
    check_screenshots(text, workspace, start_ts, result)
    check_fix(brief or "", paths, result)
    return result


def feedback(result: Result) -> str:
    """The note appended to a brief that is sent back (and shown in the review message)."""
    listed = "\n".join(f"- {m}" for m in result.mismatches)
    return (f"{MARK}\n\nYour final report was checked against the real change and these claims do not hold:\n"
            f"{listed}\n\nFix the work (or the report) so every claim is true, run the tests again, and "
            "report only what is in the diff.\n")
