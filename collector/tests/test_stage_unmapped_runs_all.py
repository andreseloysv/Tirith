"""``tirith run --stage test`` with no test mapped to the changed files runs the whole stage and records it."""

from __future__ import annotations

import subprocess
from pathlib import Path

from conftest import GIT, make_repo
from tirith import fasttests
from tirith.cli import main
from tirith.store import read_jsonl


def test_select_goes_full_when_nothing_maps(tmp_path):
    sel = fasttests.select(tmp_path, "test", ["scripts/x.sh"])
    assert sel.full and "no test is mapped" in sel.reason
    assert fasttests.command(sel) is None and sel.record()["mode"] == "full"
    assert "nothing to run" not in sel.summary()


def test_docs_only_still_runs_nothing(tmp_path):
    assert not fasttests.select(tmp_path, "test", ["README.md"]).full


def test_run_without_a_map_runs_the_full_stage_and_records_it(home, tmp_path, capsys, monkeypatch):
    repo = make_repo(tmp_path / "proj")
    (repo / "collector" / "tirith").mkdir(parents=True)
    (repo / "tirith.toml").write_text('[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "test"\nrun = "echo FULLRUN"\n')
    subprocess.run([GIT, "-C", str(repo), "add", "."], check=True)
    subprocess.run([GIT, "-C", str(repo), "-c", "user.email=t@example.invalid", "-c", "user.name=T",
                    "commit", "-q", "-m", "manifest"], check=True)
    (repo / "unmapped.sh").write_text("x\n")
    (home / "config.toml").write_text(f'[[projects]]\nid = "p"\nname = "P"\nrepo = "{repo}"\n', encoding="utf-8")
    monkeypatch.chdir(repo)
    assert main(["--home", str(home), "run", "--project", "p", "--stage", "test", "--changed"]) == 0
    out = capsys.readouterr().out
    assert "FULLRUN" in out and "full suite" in out
    rows = read_jsonl(home / "selections.jsonl")
    assert rows and rows[-1]["mode"] == "full"
