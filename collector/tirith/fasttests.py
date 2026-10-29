"""Fast tests in a lane: ``tirith run --stage test --changed`` runs only the tests related to the lane's changes.

Python modules map to tests by name (``tirith/x.py`` -> ``tests/test_x*.py``) and by the import graph (a test
that imports ``tirith.x``, directly or through another module that imports it). A Rust change runs only the
crate touched (and the crates that depend on it); a web change only the matching ``*.test.mjs``. When unsure
(an unknown kind of file, shared core files, a selection that covers most of the suite) the full suite runs.
The merge queue always runs the full suite."""

from __future__ import annotations

import ast
import re
import shlex
from dataclasses import dataclass, field
from pathlib import Path

from tirith import timeouts as timeouts_mod

CORE = {"collector/tirith/home.py", "collector/tirith/store.py", "collector/tirith/models.py",
        "collector/tirith/__init__.py", "collector/tests/conftest.py", "collector/pyproject.toml",
        "collector/uv.lock", "tirith.toml", "Cargo.lock", "Cargo.toml"}  # a change here can break any test
RUST_CRATES = {"rust/tirith-core": "tirith-core", "rust/tirith-cli": "tirith-cli"}
RUST_DEPENDENTS = {"tirith-core": ["tirith-core", "tirith-cli"], "tirith-cli": ["tirith-cli"]}
DOC_SUFFIXES = (".md", ".txt")
MOST = 0.6  # a selection covering more of the suite than this runs the whole suite


@dataclass
class Selection:
    stage: str
    full: bool = True
    reason: str = ""
    tests: list[str] = field(default_factory=list)   # pytest paths relative to ``collector``
    crates: list[str] = field(default_factory=list)  # cargo packages
    web: list[str] = field(default_factory=list)     # web test files relative to the repo root
    changed: list[str] = field(default_factory=list)

    def record(self) -> dict:
        return {"stage": self.stage, "mode": "full" if self.full else "changed", "reason": self.reason,
                "tests": self.tests, "crates": self.crates, "web": self.web, "changed": self.changed}

    def summary(self) -> str:
        if self.full:
            return f"full suite ({self.reason})"
        parts = [f"{len(self.tests)} python test files"] if self.tests else []
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


def _python(folder: Path, files: list[str]) -> tuple[list[str], str] | None:
    """Pytest files for the changed Python files, or None when unsure (run everything)."""
    pkg, tests_dir = folder / "collector" / "tirith", folder / "collector" / "tests"
    modules = {p.stem for p in pkg.glob("*.py")}
    tests = sorted(tests_dir.glob("test_*.py"))
    if not tests:
        return None
    graph = {m: _imports(pkg / f"{m}.py") & modules for m in modules}
    test_imports = {t: _imports(t) for t in tests}
    affected = {Path(f).stem for f in files if f.startswith("collector/tirith/") and f.endswith(".py")}
    chosen = {Path(f).name for f in files if f.startswith("collector/tests/") and Path(f).name.startswith("test_")
              and f.endswith(".py") and (folder / f).is_file()}
    grew = True
    while grew:  # modules that import an affected module are affected too
        more = {m for m, deps in graph.items() if deps & affected} - affected
        affected |= more
        grew = bool(more)
    for test in tests:
        stem = test.stem.removeprefix("test_")
        if test_imports[test] & affected or any(stem == m or stem.startswith(m + "_") for m in affected):
            chosen.add(test.name)
    if len(chosen) > MOST * len(tests):
        return None
    return sorted(f"tests/{name}" for name in chosen), f"{len(affected)} modules affected"


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


def select(folder: Path, stage: str, files: list[str] | None = None, base: str = "main") -> Selection:
    """What the ``test`` or ``rust-test`` stage should run for the lane's changes."""
    folder = Path(folder)
    files = changed_files(folder, base) if files is None else files
    sel = Selection(stage=stage, changed=files)
    if not files:
        sel.reason = "no change found"
        return sel
    core = [f for f in files if f in CORE]
    if core:
        sel.reason = f"shared file changed: {core[0]}"
        return sel
    code = [f for f in files if not f.endswith(DOC_SUFFIXES) and not f.startswith("docs/")]
    py = [f for f in code if f.startswith("collector/") and f.endswith(".py")]
    rust = [f for f in code if f.endswith(".rs") or Path(f).name == "Cargo.toml"]
    web = [f for f in code if f.startswith("web/") and f.endswith((".js", ".mjs"))]
    other = [f for f in code if f not in py and f not in rust and f not in web]
    if other:
        sel.reason = f"unmapped file changed: {other[0]}"
        return sel
    if stage == "rust-test":
        crates: set[str] = set()
        for f in rust:
            crate = next((c for prefix, c in RUST_CRATES.items() if f.startswith(prefix + "/")), None)
            if crate is None:
                sel.reason = f"rust file outside the crates: {f}"
                return sel
            crates.update(RUST_DEPENDENTS[crate])
        sel.full, sel.crates, sel.reason = False, sorted(crates), "only the crates touched"
        return sel
    picked = _python(folder, py) if py else ([], "no python change")
    if picked is None:
        sel.reason = "python change reaches most of the suite or has no tests to map to"
        return sel
    sel.full, sel.tests, sel.reason = False, picked[0], picked[1]
    sel.web = _web(folder, web)
    if not (sel.tests or sel.web):
        sel.reason = "only docs or Rust changed" if not code or rust else sel.reason
    return sel


def command(sel: Selection) -> str | None:
    """Shell line for a changed-only selection; None means the manifest's own command runs."""
    if sel.full:
        return None
    steps = []
    if sel.stage == "rust-test":
        steps = [f"cargo test -q {' '.join('-p ' + c for c in sel.crates)}"] if sel.crates else []
    else:
        if sel.tests:
            steps.append("cd collector && uv run -q pytest -q " + " ".join(shlex.quote(t) for t in sel.tests))
        steps.extend(f"node {shlex.quote(w)}" for w in sel.web)
    return " && ".join(steps) if steps else "true"
