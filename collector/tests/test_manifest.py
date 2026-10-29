"""tirith.toml parsing, collector auto-declaration, `tirith run` resolution and `tirith check`."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

from conftest import make_repo
from tirith import manifest as mf
from tirith.cli import main
from tirith.collector import Collector
from tirith.guardwrap import default_words, install_guard
from tirith.store import read_jsonl

FULL = '''
[[pipelines]]
id = "ci"
name = "CI"
[[pipelines.stages]]
name = "build"
run = "make build"
[[pipelines.stages]]
name = "test"
run = "make test"
[[pipelines.stages]]
name = "ship"
run = "make ship"

[[hooks]]
event = "pre-commit"
run = "make lint"
'''


def _rules(problems) -> list[str]:
    return [p.rule for p in problems]


def test_parse_pipelines_hooks_and_kinds(tmp_path: Path) -> None:
    m = mf.parse_text(FULL, tmp_path / "tirith.toml")
    assert not m.problems
    assert [s.name for s in m.pipelines[0].stages] == ["build", "test", "ship"]
    assert [mf.stage_kind(s) for s in m.pipelines[0].stages] == ["build", "test", "deploy"]
    assert m.hooks == [mf.Hook("pre-commit", "make lint")]


def test_parse_errors_have_lines(tmp_path: Path) -> None:
    bad = mf.parse_text('[[pipelines]]\nid = "a"\n[[pipelines.stages]]\nname = "x"\nkind = "zap"\n'
                        '[[hooks]]\nevent = "nope"\nrun = "x"\n', tmp_path / "tirith.toml")
    by_rule = {p.rule: p for p in bad.problems}
    assert by_rule["stage-kind"].line == 3
    assert by_rule["hook-event"].line == 6
    syntax = mf.parse_text("[[pipelines\nid = 1", tmp_path / "tirith.toml")
    assert _rules(syntax.problems) == ["toml-syntax"] and syntax.problems[0].line >= 1


def test_load_missing_and_explicit_kind(tmp_path: Path) -> None:
    assert not mf.load(tmp_path).exists
    assert mf.stage_kind(mf.Stage("go", None, "deploy")) == "deploy"
    assert mf.stage_kind(mf.Stage("misc", None, None)) is None


def test_check_rules(tmp_path: Path, home: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    words = default_words()
    toml = repo / "tirith.toml"
    assert _rules(mf.check(repo, words))[0] == "manifest-missing"
    toml.write_text("[[pipelines\n", encoding="utf-8")
    assert "toml-syntax" in _rules(mf.check(repo, words))
    no_deploy = FULL.replace('name = "ship"', 'name = "x"')
    toml.write_text(no_deploy, encoding="utf-8")
    assert "pipeline-coverage" in _rules(mf.check(repo, words))
    toml.write_text('deploy = "none"\n' + no_deploy, encoding="utf-8")
    assert "pipeline-coverage" not in _rules(mf.check(repo, words))
    toml.write_text(FULL + '[[hooks]]\nevent = "pre-commit"\n', encoding="utf-8")
    assert "hook-run" in _rules(mf.check(repo, words))
    toml.write_text(FULL, encoding="utf-8")
    assert "commit-guard" in _rules(mf.check(repo, words))
    assert install_guard(repo, words) == []
    assert mf.check(repo, words) == []


def test_check_cli_exit_codes_and_line_format(tmp_path: Path, home: Path, capsys,
                                              monkeypatch: pytest.MonkeyPatch) -> None:
    repo = make_repo(tmp_path / "repo")
    assert main(["check", str(repo)]) == 1
    line = capsys.readouterr().out.splitlines()[0]
    assert line.startswith(f"{repo.resolve()}/tirith.toml:1: manifest-missing: ")
    assert ". Fix: " in line
    (repo / "tirith.toml").write_text(FULL, encoding="utf-8")
    assert install_guard(repo, default_words()) == []
    assert main(["check", str(repo)]) == 0
    assert main(["project", "add", str(repo)]) == 0
    capsys.readouterr()
    monkeypatch.chdir(repo)
    assert main(["check"]) == 0


def test_run_resolves_stage_from_manifest(tmp_path: Path, home: Path, capsys,
                                          monkeypatch: pytest.MonkeyPatch) -> None:
    repo = make_repo(tmp_path / "repo")
    (repo / "tirith.toml").write_text(
        '[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "test"\nrun = "echo manifest-ok"\n'
        '[[pipelines]]\nid = "rel"\n[[pipelines.stages]]\nname = "test"\nrun = "true"\n'
        '[[pipelines.stages]]\nname = "pack"\nrun = "echo packed"\n', encoding="utf-8")
    assert main(["project", "add", str(repo)]) == 0
    capsys.readouterr()
    monkeypatch.chdir(repo)
    assert main(["run", "--stage", "pack"]) == 0
    assert "packed" in capsys.readouterr().out
    event = read_jsonl(home / "events.jsonl")[0]
    assert event["pipeline"] == "rel" and event["project"] == "repo"
    with pytest.raises(SystemExit, match="several pipelines"):
        main(["run", "--stage", "test"])
    assert main(["run", "--stage", "test", "--pipeline", "ci"]) == 0
    assert "manifest-ok" in capsys.readouterr().out
    assert main(["run", "--stage", "test", "--pipeline", "ci", "--",
                 sys.executable, "-c", "print('explicit')"]) == 0
    assert "explicit" in capsys.readouterr().out


def test_collector_declares_manifest_pipelines(tmp_path: Path, home: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    (repo / "tirith.toml").write_text(FULL, encoding="utf-8")
    assert main(["project", "add", str(repo)]) == 0
    state = Collector(home, client=None).cycle()
    runs = [r for r in state["pipelines"] if r["pipeline"] == "ci"]
    assert len(runs) == 1 and runs[0]["project"] == "repo"
    assert [(s["name"], s["status"]) for s in runs[0]["stages"]] == [
        ("build", "pending"), ("test", "pending"), ("ship", "pending")]


def test_declared_pipeline_that_never_ran_is_pending(tmp_path):
    """A pipeline only declared in tirith.toml must not look like it is running."""
    from tirith.collector import Collector
    home = tmp_path / "home"
    repo = tmp_path / "repo"
    repo.mkdir()
    (repo / "tirith.toml").write_text(
        '[[pipelines]]\nid = "ci"\n[[pipelines.stages]]\nname = "build"\nrun = "true"\n',
        encoding="utf-8")
    home.mkdir()
    (home / "config.toml").write_text(
        f'[[projects]]\nid = "p"\nname = "P"\nrepo = "{repo}"\n', encoding="utf-8")
    state = Collector(home, client=None).cycle()
    runs = [r for r in state["pipelines"] if r["pipeline"] == "ci"]
    assert runs and runs[0]["status"] == "pending"
