"""End to end: `tirith run --project P --stage test` from a lane agent's shell runs swift with --disable-sandbox."""

from __future__ import annotations

import os
from pathlib import Path

import pytest

from conftest import make_repo, write_config
from tirith import netproxy
from tirith.cli import main


@pytest.fixture
def swiftrepo(home: Path, tmp_path: Path, monkeypatch) -> tuple[Path, Path]:
    repo = make_repo(tmp_path / "repo")
    write_config(home, [{"id": "zoo", "name": "Zoo", "repo": str(repo), "color": "#123456"}])
    fake = tmp_path / "bin"
    fake.mkdir()
    seen = tmp_path / "swift-args.txt"
    (fake / "swift").write_text(f'#!/bin/sh\necho "$@" >> "{seen}"\n')
    (fake / "swift").chmod(0o755)
    (repo / "run-tests.sh").write_text("#!/bin/sh\nswift test --filter Z\n")
    (repo / "run-tests.sh").chmod(0o755)
    monkeypatch.setenv("PATH", f"{fake}{os.pathsep}{os.environ['PATH']}")
    monkeypatch.chdir(repo)
    # the launcher's env for a lane agent
    monkeypatch.setenv("TIRITH_LANE", "zoo-31")
    monkeypatch.setenv("TIRITH_PROJECT", "zoo")
    monkeypatch.setenv(netproxy.SANDBOXED_ENV, "1")
    return repo, seen


def _pipeline(repo: Path, line: str) -> None:
    (repo / "tirith.toml").write_text(
        '[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "test"\nkind = "test"\n'
        f'run = "{line}"\n', encoding="utf-8")


def test_lane_agent_entry_literal_swift_test_gets_disable_sandbox(swiftrepo) -> None:
    repo, seen = swiftrepo
    _pipeline(repo, "swift test --filter Z")
    assert main(["run", "--project", "zoo", "--stage", "test"]) == 0
    assert seen.read_text().strip() == "test --disable-sandbox --filter Z"


def test_lane_agent_entry_script_reaching_swift_gets_disable_sandbox(swiftrepo) -> None:
    repo, seen = swiftrepo
    _pipeline(repo, "./run-tests.sh")
    assert main(["run", "--project", "zoo", "--stage", "test"]) == 0
    assert seen.read_text().strip() == "test --disable-sandbox --filter Z"


def test_swift_sandbox_lane_root_zoo_stage_runs_in_zoocore(swiftrepo) -> None:
    """The zoo test stage launched from a lane root (zoo-lanes/<lane>) runs swift in Packages/ZooCore with the flag."""
    repo, seen = swiftrepo
    (repo / "Packages" / "ZooCore").mkdir(parents=True)
    (repo / "Packages" / "ZooCore" / "Package.swift").write_text("// pkg\n")
    fake = Path(os.environ["PATH"].split(os.pathsep)[0])
    (fake / "swift").write_text(f'#!/bin/sh\necho "$(basename "$PWD") $@" >> "{seen}"\n[ -f Package.swift ] || exit 1\n')
    _pipeline(repo, "(cd Packages/ZooCore && swift test --skip 'A/test(B|C)|D') && (test ! -d cms/server/tests || true)")
    assert main(["run", "--project", "zoo", "--stage", "test"]) == 0
    assert seen.read_text().strip() == "ZooCore test --disable-sandbox --skip A/test(B|C)|D"
