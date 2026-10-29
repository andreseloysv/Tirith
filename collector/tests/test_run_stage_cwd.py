"""A stage taken from tirith.toml runs from the project root, wherever `tirith run` is called."""

from __future__ import annotations

import os

from tirith.cli import main


def test_manifest_stage_runs_from_the_project_root(tmp_path, monkeypatch):
    home = tmp_path / "home"
    home.mkdir()
    repo = tmp_path / "repo"
    (repo / "sub").mkdir(parents=True)
    (repo / "tirith.toml").write_text(
        '[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "test"\n'
        'run = "test -d sub && pwd > where.txt"\n', encoding="utf-8")
    (home / "config.toml").write_text(
        f'[[projects]]\nid = "p"\nname = "P"\nrepo = "{repo}"\n', encoding="utf-8")
    monkeypatch.setenv("TIRITH_HOME", str(home))
    monkeypatch.chdir(repo / "sub")
    assert main(["run", "--project", "p", "--stage", "test"]) == 0
    assert os.path.realpath((repo / "where.txt").read_text().strip()) == os.path.realpath(repo)
