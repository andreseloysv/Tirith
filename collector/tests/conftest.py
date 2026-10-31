"""Shared fixtures: an isolated Tirith home and throwaway git helpers."""

from __future__ import annotations

import os
import shutil
import stat
import subprocess
import sys
import time
import types
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

GIT = "/usr/bin/git" if Path("/usr/bin/git").exists() else (shutil.which("git") or "git")


@pytest.fixture
def home(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """A fresh TIRITH_HOME with the brain key removed from the env."""
    folder = tmp_path / "tirith-home"
    folder.mkdir()
    monkeypatch.setenv("TIRITH_HOME", str(folder))
    monkeypatch.delenv("META_API_KEY", raising=False)
    monkeypatch.delenv("META_SI_API_KEY", raising=False)
    return folder


def cpu_seconds() -> float:
    """CPU time spent by this process and the children it waited for. Budgets on it hold on a loaded machine,
    where the wall clock also counts the time the process spent waiting for a core."""
    t = os.times()
    return t.user + t.system + t.children_user + t.children_system


def make_repo(path: Path) -> Path:
    """Create a throwaway git repo with one commit (tmp dirs only)."""
    path.mkdir(parents=True, exist_ok=True)
    subprocess.run([GIT, "init", "-q", str(path)], check=True)
    (path / "README.md").write_text("# demo repo\n", encoding="utf-8")
    subprocess.run([GIT, "-C", str(path), "add", "."], check=True)
    subprocess.run(
        [GIT, "-C", str(path), "-c", "user.email=t@example.invalid",
         "-c", "user.name=Tirith Test", "commit", "-q", "-m", "initial"],
        check=True)
    return path


def make_hooks_mutable(repo: Path) -> None:
    """Clear immutable flags the guard sets, so tmp cleanup can proceed."""
    try:
        out = subprocess.run(
            [GIT, "-C", str(repo), "rev-parse", "--path-format=absolute", "--git-common-dir"],
            capture_output=True, text=True, check=True).stdout.strip()
    except (OSError, subprocess.SubprocessError):
        return
    hooks = Path(out) / "hooks"
    if not hooks.is_dir():
        return
    for hook in hooks.iterdir():
        try:
            flags = hook.stat().st_flags  # type: ignore[attr-defined]
            if flags & stat.UF_IMMUTABLE:
                os.chflags(hook, flags & ~stat.UF_IMMUTABLE)  # type: ignore[attr-defined]
        except (OSError, AttributeError):
            pass


def write_config(home: Path, projects: list[dict], extra: str = "") -> Path:
    """Write a minimal config.toml with ``[[projects]]`` rows."""
    lines: list[str] = []
    for proj in projects:
        lines.append("[[projects]]")
        for key, value in proj.items():
            if isinstance(value, list):
                rendered = "[" + ", ".join(f'"{item}"' for item in value) + "]"
                lines.append(f"{key} = {rendered}")
            else:
                lines.append(f'{key} = "{value}"')
        lines.append("")
    if extra:
        lines.append(extra)
    target = home / "config.toml"
    target.write_text("\n".join(lines), encoding="utf-8")
    return target


def _unlock_tree(root: Path, budget_s: float | None = None) -> None:
    """Clear the protection flag Tirith's guard set on files below ``root`` (a pytest temp dir only).
    ``budget_s`` stops the walk after that long: the temp root can hold thousands of old folders."""
    import tirith_guard
    deadline = None if budget_s is None else time.monotonic() + budget_s
    for path in _walk(root):
        if deadline is not None and time.monotonic() > deadline:
            return
        try:
            if not path.is_symlink() and tirith_guard._is_locked(path):
                tirith_guard._unlock(path)
        except OSError:
            pass


def _walk(root: Path):
    yield root
    try:
        for path in root.rglob("*"):
            yield path
    except OSError:
        return


@pytest.fixture(autouse=True)
def _unlock_guard_hooks(tmp_path):
    """Guard hooks a test installs are protected; clear the flag so pytest can delete its tmp_path."""
    yield
    _unlock_tree(tmp_path)


@pytest.fixture(autouse=True, scope="session")
def _unlock_basetemp(tmp_path_factory):
    """Same for this run's basetemp, and (within a time budget, so one test never waits for the walk) for the
    other dirs under pytest's temp root: kept dirs of failed runs and the ``garbage-*`` folders earlier runs
    left, so pytest's own cleanup at the end of the session can remove them."""
    base = tmp_path_factory.getbasetemp()
    root = base.parent if base.parent.name.startswith("pytest-of-") else base
    _unlock_tree(base)
    yield
    _unlock_tree(base)
    if root != base:
        _unlock_tree(root, budget_s=5.0)


@pytest.fixture(autouse=True)
def _temp_projects_allowed(monkeypatch):
    """Test projects live in pytest temp folders, which real discovery refuses (test_setupfix turns it off)."""
    from tirith import projects
    monkeypatch.setattr(projects, "ALLOW_TEMP", True)


@pytest.fixture(autouse=True)
def _clean_env(monkeypatch):
    """Tests run the same inside a lane or agent session as outside: drop every TIRITH_* variable
    (TIRITH_LANE, TIRITH_SESSION, TIRITH_AGENT_SESSION, TIRITH_HOME, ...); tests set what they need explicitly."""
    for name in list(os.environ):
        if name.startswith("TIRITH_"):
            monkeypatch.delenv(name, raising=False)


@pytest.fixture(autouse=True)
def _no_real_temp_cleanup(monkeypatch):
    """A test never lets the collector's cleanup thread touch the real temp folder."""
    from tirith import diskdiet
    monkeypatch.setattr(diskdiet, "TEMP_CLEANUP_ENABLED", False)


@pytest.fixture(autouse=True)
def _plenty_of_disk(monkeypatch):
    """The dispatcher and autopilot start no lane when the host disk is nearly full; tests must not depend on it."""
    from tirith import diskdiet
    monkeypatch.setattr(diskdiet, "free_bytes", lambda folder: 1000 * 1024 ** 3)


@pytest.fixture(autouse=True)
def _no_owner_shell(monkeypatch):
    """``tirith collect`` reads the owner's login shell at start; tests must not run it (test_ownerenv does, with a fake)."""
    import types
    from tirith import cli, ownerenv
    monkeypatch.setattr(cli, "ownerenv_mod", types.SimpleNamespace(load=lambda *a, **k: ownerenv.status()))


@pytest.fixture(autouse=True)
def _no_autopilot(request, monkeypatch):
    """The autopilot is on by default and would start lanes and raise owner items in every collector test;
    only ``test_autopilot`` runs it."""
    if request.module.__name__.rpartition(".")[2] == "test_autopilot":
        return
    from tirith import autopilot
    monkeypatch.setattr(autopilot, "run", lambda *args, **kwargs: False)


DEFAULT_PLAN = """# TODO

## 1. Work

- [ ] Do the thing.
- [ ] Do the other thing.
"""


def write_plan(repo: Path, text: str = DEFAULT_PLAN, name: str = "docs/TODO.md") -> Path:
    """The project's plan file, committed (Tirith only writes a plan whose file is clean)."""
    path = repo / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    subprocess.run([GIT, "-C", str(repo), "add", "--", name], check=True)
    subprocess.run([GIT, "-C", str(repo), "-c", "user.email=t@example.invalid", "-c", "user.name=Tirith Test",
                    "commit", "-q", "-m", "plan", "--", name], check=True)
    subprocess.run([GIT, "-C", str(repo), "config", "user.email", "t@example.invalid"], check=True)
    subprocess.run([GIT, "-C", str(repo), "config", "user.name", "Tirith Test"], check=True)
    return path


REPORT_DONE = ("import sys; sys.path.insert(0, %r); from tirith.cli import main; "
               "main(['plan', 'report', '--status', 'done']); " % str(Path(__file__).resolve().parent.parent))
# python code for a lane's child process that reports the outcome of its item, as an agent must

PLAN_ARGS = ["--step", "1", "--item", "Do the thing."]  # the open item of ``DEFAULT_PLAN``


def write_lane_manifest(repo: Path) -> None:
    """A tirith.toml that passes ``tirith check`` and a brief file, as ``lane run`` requires."""
    (repo / "tirith.toml").write_text(
        'deploy = "none"\n[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "build"\n'
        'run = "make build"\n[[pipelines.stages]]\nname = "test"\nrun = "make test"\n',
        encoding="utf-8")
    (repo / "brief.md").write_text("Do the thing.\n", encoding="utf-8")


@pytest.fixture(autouse=True)
def _calm_machine(monkeypatch: pytest.MonkeyPatch) -> None:
    """The adaptive lane limit must not depend on the load or the free disk of the machine running the tests."""
    from tirith import diskdiet, loadlimit
    monkeypatch.setattr(loadlimit, "load_average", lambda: 0.0)
    monkeypatch.setattr(diskdiet, "free_bytes", lambda folder: 500 * 1024 ** 3)


@pytest.fixture(autouse=True)
def _no_hourly_canary_in_dispatch(monkeypatch):
    """``dispatch.run`` starts the hourly end-to-end canary through its spawner; tests with their own fake
    spawner only expect the commands they ask for. test_canary drives ``canary.tick`` directly."""
    from tirith import dispatch
    monkeypatch.setattr(dispatch, "canary_mod", types.SimpleNamespace(tick=lambda home, now, spawn: False))


@pytest.fixture(autouse=True)
def _tick_merged_every_call(monkeypatch):
    """``tick_merged`` looks at a lane once a minute in the collector; a test calls it again at once."""
    from tirith import plangate
    monkeypatch.setattr(plangate, "TICK_EVERY_S", 0.0)
    plangate._ticked_at.clear()


@pytest.fixture(autouse=True)
def _no_wait_for_returned_agents(monkeypatch):
    """The merge queue waits for a returned lane's agent to register; tests with a fake relauncher must not."""
    from tirith import mergeq
    monkeypatch.setattr(mergeq, "START_WAIT", 0.0)
