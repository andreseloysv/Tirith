from __future__ import annotations

from pathlib import Path

from tirith import runstage


def test_stage_log_names_carry_the_project_per_project(tmp_path: Path) -> None:
    tirith = runstage.stage_log_path(tmp_path, "tirith", "ci#349", "test")
    zoo = runstage.stage_log_path(tmp_path, "zoo", "ci#349", "test")
    assert tirith.name == "ci-tirith-349-test.log"
    assert zoo.name == "ci-zoo-349-test.log"
    assert tirith != zoo


def test_stage_log_per_project_run_stage_writes_the_project_log(tmp_path: Path) -> None:
    home = tmp_path / "home"
    code, _ = runstage.run_stage(home, "zoo", "ci", "test", ["true"], tmp_path, run_id="ci#350", echo=False)
    assert code == 0
    assert (home / "logs" / "ci-zoo-350-test.log").exists()
    assert not (home / "logs" / "ci-350-test.log").exists()
