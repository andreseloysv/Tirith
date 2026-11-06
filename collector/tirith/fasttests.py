"""Fast tests in a lane: ``tirith run --stage test`` runs only the tests related to the lane's changes.

A lane runs the full suite only with ``--full``, in the merge queue, or when code changed that no test maps to. With an execution map (``tirith.testmap``)
and only ``collector`` Python files changed, pytest-testmon runs the tests whose recorded code changed. Without
one, Python modules map to tests by name
(``tirith/x.py`` -> ``tests/test_x*.py``), the test files the lane changed run themselves, and the import graph
(a test that imports ``tirith.x``, directly or through another module that imports it) adds tests only while the
selection stays under ``MOST`` of the suite. A Rust change runs only the crate touched (and the crates that
depend on it); a web change only the matching ``*.test.mjs``. When code changed but nothing maps (shared files, unknown kinds of
files, a project without a map) the full test stage runs and the pass is recorded in ``selections.jsonl``; docs-only
changes run nothing."""

from __future__ import annotations

import ast
import re
import shlex
from dataclasses import dataclass, field
from pathlib import Path

from tirith import testmap as testmap_mod
from tirith import timeouts as timeouts_mod
from tirith.home import now_iso
from tirith.store import append_jsonl

RUST_CRATES = {"rust/tirith-core": "tirith-core", "rust/tirith-cli": "tirith-cli"}
RUST_DEPENDENTS = {"tirith-core": ["tirith-core", "tirith-cli"], "tirith-cli": ["tirith-cli"]}
DOC_SUFFIXES = (".md", ".txt")
MOST = 0.6  # the import graph adds tests only while the selection stays under this share of the suite


@dataclass
class Selection:
    stage: str
    full: bool = False
    reason: str = ""
    tests: list[str] = field(default_factory=list)   # pytest paths relative to ``collector``
    crates: list[str] = field(default_factory=list)  # cargo packages
    web: list[str] = field(default_factory=list)     # web test files relative to the repo root
    changed: list[str] = field(default_factory=list)
    mapped: bool = False                             # the Python tests come from the execution map (testmon)

    def record(self) -> dict:
        mode = "full" if self.full else "map" if self.mapped else "changed"
        return {"stage": self.stage, "mode": mode, "reason": self.reason,
                "tests": self.tests, "crates": self.crates, "web": self.web, "changed": self.changed}

    def summary(self) -> str:
        if self.full:
            return f"full suite ({self.reason})"
        if not (self.tests or self.crates or self.web or self.mapped):
            return f"changed only: nothing to run ({self.reason}); the merge queue runs the full suite"
        parts = ["python tests by the execution map"] if self.mapped else \
            [f"{len(self.tests)} python test files"] if self.tests else []
        if self.crates:
            parts.append("crates " + ", ".join(self.crates))
        if self.web:
            parts.append(f"{len(self.web)} web tests")
        return f"changed only: {', '.join(parts) or 'nothing to run'} ({self.reason})"


def _git(folder: Path, *args: str) -> list[str]:
    done = timeouts_mod.run(["git", "-C", str(folder), *args], "git", text=True)
    return [line for line in (done.stdout or "").splitlines() if line.strip()] if done.returncode == 0 else []


def changed_files(folder: Path, base: str = "main") -> list[str]:
    """Files changed by the lane: committed since it left ``base``, plus staged, unstaged and new files."""
    found: set[str] = set()
    for ref in (base, f"origin/{base}"):
        point = _git(folder, "merge-base", "HEAD", ref)
        if point:
            found.update(_git(folder, "diff", "--name-only", point[0], "HEAD"))
            break
    found.update(_git(folder, "diff", "--name-only", "HEAD"))
    found.update(_git(folder, "ls-files", "--others", "--exclude-standard"))
    return sorted(found)


def _imports(path: Path) -> set[str]:
    """Names of the ``tirith`` modules a Python file imports."""
    try:
        tree = ast.parse(path.read_text(encoding="utf-8"))
    except (OSError, SyntaxError, UnicodeDecodeError):
        return set()
    names: set[str] = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            names.update(a.name.split(".")[1] for a in node.names if a.name.startswith("tirith."))
        elif isinstance(node, ast.ImportFrom) and node.module:
            if node.module == "tirith":
                names.update(a.name for a in node.names)
            elif node.module.startswith("tirith."):
                names.add(node.module.split(".")[1])
    return names


def _python(folder: Path, files: list[str]) -> tuple[list[str], str]:
    """Pytest files for the changed Python files: by name and changed tests, plus the import graph if it stays small."""
    pkg, tests_dir = folder / "collector" / "tirith", folder / "collector" / "tests"
    modules = {p.stem for p in pkg.glob("*.py")}
    tests = sorted(tests_dir.glob("test_*.py"))
    graph = {m: _imports(pkg / f"{m}.py") & modules for m in modules}
    test_imports = {t: _imports(t) for t in tests}
    changed = {Path(f).stem for f in files if f.startswith("collector/tirith/") and f.endswith(".py")}
    affected = set(changed)
    named = {Path(f).name for f in files if f.startswith("collector/tests/") and Path(f).name.startswith("test_")
             and f.endswith(".py") and (folder / f).is_file()}
    grew = True
    while grew:  # modules that import an affected module are affected too
        more = {m for m, deps in graph.items() if deps & affected} - affected
        affected |= more
        grew = bool(more)
    reached = set(named)
    for test in tests:
        stem = test.stem.removeprefix("test_")
        if any(stem == m or stem.startswith(m + "_") for m in changed):
            named.add(test.name)
        if test_imports[test] & affected or any(stem == m or stem.startswith(m + "_") for m in affected):
            reached.add(test.name)
    reached |= named
    if len(reached) > MOST * len(tests):
        return sorted(f"tests/{n}" for n in named), f"{len(changed)} modules changed; the import graph reaches most of the suite"
    return sorted(f"tests/{n}" for n in reached), f"{len(affected)} modules affected"


def _web(folder: Path, files: list[str]) -> list[str]:
    tests = sorted((folder / "web").rglob("*.test.mjs"))
    picked: set[str] = set()
    for f in files:
        if f.endswith(".test.mjs"):
            picked.add(f)
        elif f.startswith("web/") and f.endswith(".js"):
            name = Path(f).name
            for test in tests:
                try:
                    text = test.read_text(encoding="utf-8")
                except (OSError, UnicodeDecodeError):
                    continue
                if test.stem.removesuffix(".test") == Path(f).stem or re.search(re.escape(name), text):
                    picked.add(str(test.relative_to(folder)))
    return sorted(p for p in picked if (folder / p).is_file())


def select(folder: Path, stage: str, files: list[str] | None = None, base: str = "main",
           mapped: bool = False) -> Selection:
    """What the ``test`` or ``rust-test`` stage should run for the lane's changes (never the whole suite).

    ``mapped``: an execution map exists; it picks the Python tests when every changed ``collector`` file is Python
    (testmon does not see data files, so any other change falls back to names and the import graph)."""
    folder = Path(folder)
    files = changed_files(folder, base) if files is None else files
    sel = Selection(stage=stage, changed=files)
    if not files:
        sel.reason = "no change found"
        return sel
    code = [f for f in files if not f.endswith(DOC_SUFFIXES) and not f.startswith("docs/")]
    py = [f for f in code if f.startswith("collector/") and f.endswith(".py")]
    rust = [f for f in code if f.endswith(".rs") or Path(f).name == "Cargo.toml"]
    web = [f for f in code if f.startswith("web/") and f.endswith((".js", ".mjs"))]
    if stage == "rust-test":
        crates: set[str] = set()
        for f in rust:
            crate = next((c for prefix, c in RUST_CRATES.items() if f.startswith(prefix + "/")), None)
            if crate:
                crates.update(RUST_DEPENDENTS[crate])
        sel.crates, sel.reason = sorted(crates), "only the crates touched" if crates else "no crate touched"
        return sel
    if mapped and py and all(f.endswith(".py") for f in code if f.startswith("collector/")):
        sel.mapped, sel.reason = True, f"{len(py)} python files changed"
    else:
        sel.tests, sel.reason = _python(folder, py) if py else ([], "no python change")
    sel.web = _web(folder, web)
    if not (sel.tests or sel.web or sel.mapped):
        others = [f for f in code if f not in rust]
        sel.reason = f"no tests map to {others[0]}" if others else "only docs or Rust changed"
        if code:  # code changed but no test maps to it: run the whole stage and record the pass, not "nothing to run"
            sel.full, sel.reason = True, f"no test is mapped to the changed files ({sel.reason})"
    return sel


def command(sel: Selection) -> str | None:
    """Shell line for a changed-only selection; None means the manifest's own command runs."""
    if sel.full:
        return None
    steps = []
    if sel.stage == "rust-test":
        steps = [f"cargo test -q {' '.join('-p ' + c for c in sel.crates)}"] if sel.crates else []
    else:
        if sel.mapped:
            steps.append(testmap_mod.command())
        elif sel.tests:  # a subshell: the web tests below run from the repo root
            steps.append("(cd collector && uv run -q pytest -q -n auto --maxprocesses 4 "
                         + " ".join(shlex.quote(t) for t in sel.tests) + ")")
        steps.extend(f"node {shlex.quote(w)}" for w in sel.web)
    return " && ".join(steps) if steps else "true"


def supported(folder: Path) -> bool:
    """Whether the project has the layout this selection reads (``collector/tirith``); others keep their own command."""
    return (Path(folder) / "collector" / "tirith").is_dir()


def for_lane(home: Path, project: str, folder: Path, stage: str) -> Selection:
    """The selection for a lane's worktree: from the execution map when there is one (copied into the worktree),
    else by names and the import graph. Recorded in ``selections.jsonl``."""
    folder = Path(folder)
    sel = select(folder, stage, mapped=stage == "test" and testmap_mod.usable(home, project))
    if sel.mapped and not testmap_mod.install(home, project, folder):
        sel = select(folder, stage, sel.changed)
    append_jsonl(home / "selections.jsonl", {"at": now_iso(), "project": project, "worktree": str(folder),
                                              **sel.record()})
    return sel


PARALLEL_ARGS = ["-n", "auto", "--maxprocesses", "8"]
_SERIAL_FLAGS = ("-n", "--numprocesses", "--dist", "no:xdist", "--testmon", "--collect-only", "--co", "--pdb", "-s")


def parallel_pytest(command: list[str]) -> list[str]:
    """An agent's pytest command with xdist added: agents ran ``uv run pytest -q tests`` in one process (1842
    tests in 13 min, against 2 min in parallel). Left as it is when it is no pytest run, or it already chooses
    (``-n``, ``-p no:xdist``, ``--testmon``), or it debugs (``--pdb``, ``-s``). In ``sh -c '...'`` the line gets it
    after its first ``pytest`` (agents wrapped the command that way and ran 1868 tests in one process, 19 min)."""
    if len(command) >= 3 and Path(command[0]).name in ("sh", "bash", "zsh") and command[1] in ("-c", "-lc"):
        return [*command[:2], _parallel_line(command[2]), *command[3:]]  # sh -c '... pytest ...': inside the line
    if "pytest" not in command or any(arg.startswith(_SERIAL_FLAGS) for arg in command):
        return command
    return [*command, *PARALLEL_ARGS]


def _parallel_line(line: str) -> str:
    """A shell line's first ``pytest`` with xdist right behind it, unless the line already chooses or debugs."""
    words = line.split()
    if not any(w == "pytest" or w.endswith("/pytest") for w in words) or \
            any(w.startswith(_SERIAL_FLAGS) for w in words):
        return line
    return re.sub(r"(?<![\w/-])((?:\S*/)?pytest)(?=\s|$)", lambda m: m.group(1) + " " + " ".join(PARALLEL_ARGS), line, count=1)

