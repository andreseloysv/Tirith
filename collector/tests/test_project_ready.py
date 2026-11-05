"""``tirith project add`` leaves the project ready: plan, minimal tirith.toml, guard on the owner's list."""

from __future__ import annotations

from pathlib import Path

from conftest import make_repo
from tirith import cli, manifest as manifest_mod, plangate, projects
from tirith.store import read_jsonl


def test_project_ready_after_add(home: Path, tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "app")
    pid, folder, created = projects.add_project(home, repo)
    from tirith import projectready
    done = projectready.make_ready(home, pid, folder, "app")
    assert done == {"plan": True, "manifest": True, "guard": True}
    assert plangate.plan_file(repo) is not None
    assert manifest_mod.load(repo).exists and not manifest_mod.load(repo).problems
    (wait,) = [r for r in read_jsonl(home / "waits.jsonl") if r["id"] == f"guard-install-{pid}"]
    assert wait["actions"][0]["command"].endswith(f"tirith guard install {pid}")


def test_project_ready_twice_changes_nothing(home: Path, tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "app")
    pid, folder, _ = projects.add_project(home, repo)
    from tirith import projectready
    projectready.make_ready(home, pid, folder, "app")
    again = projectready.make_ready(home, pid, folder, "app")
    assert again["plan"] is False and again["manifest"] is False
    assert len(read_jsonl(home / "waits.jsonl")) == 1


def test_roles_default_on_demand_new_project(home: Path, tmp_path: Path) -> None:
    from tirith import projectready
    repo = make_repo(tmp_path / "app")
    pid, folder, _ = projects.add_project(home, repo)
    projectready.make_ready(home, pid, folder, "app")
    m = manifest_mod.load(repo)
    assert m.roles == {"ceo": "on-demand", "manager": "on-demand"} and not m.problems


def test_roles_default_on_demand_keeps_existing_setting(home: Path, tmp_path: Path) -> None:
    from tirith import projectready
    repo = make_repo(tmp_path / "app")
    (repo / "tirith.toml").write_text('[roles]\nceo = "always"\nmanager = "off"\n', encoding="utf-8")
    pid, folder, _ = projects.add_project(home, repo)
    assert projectready.make_ready(home, pid, folder, "app")["manifest"] is False
    assert manifest_mod.load(repo).roles == {"ceo": "always", "manager": "off"}


def test_project_ready_cli_prints_what_it_did(home: Path, tmp_path: Path, capsys) -> None:
    repo = make_repo(tmp_path / "app")
    assert cli.main(["project", "add", str(repo)]) == 0
    out = capsys.readouterr().out
    assert "plan created" in out and "tirith.toml written" in out
