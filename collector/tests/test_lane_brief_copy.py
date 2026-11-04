"""Tirith keeps a copy of each lane's brief in the lane's folder, so a cut lane can restart after a reboot."""

from __future__ import annotations

from pathlib import Path

from tirith.cli import register_lane
from tirith.store import lane_brief_copy, lane_brief_path, read_lane


def _register(home: Path, brief: Path | None, lane_id: str = "l-1") -> dict:
    return register_lane(home, lane_id=lane_id, project="demo", hero_class="warden", feature="f",
                         workspace=None, log=None, pid=None, brief=str(brief) if brief else None)


def test_brief_copy_saved_in_lane_folder(home: Path, tmp_path: Path) -> None:
    brief = tmp_path / "b.md"
    brief.write_text("# Lane: x\nDo it.\n", encoding="utf-8")
    _register(home, brief)
    assert lane_brief_copy(home, "l-1") == home / "lanes" / "l-1" / "brief.md"
    assert lane_brief_copy(home, "l-1").read_text(encoding="utf-8") == "# Lane: x\nDo it.\n"
    assert read_lane(home, "l-1")["id"] == "l-1"  # the folder does not disturb the registration


def test_brief_falls_back_to_copy_when_original_is_gone(home: Path, tmp_path: Path) -> None:
    brief = tmp_path / "b.md"
    brief.write_text("task\n", encoding="utf-8")
    lane = _register(home, brief)
    assert lane_brief_path(home, lane) == brief
    brief.unlink()
    assert lane_brief_path(home, lane) == lane_brief_copy(home, "l-1")


def test_no_brief_no_copy(home: Path) -> None:
    lane = _register(home, None)
    assert not lane_brief_copy(home, "l-1").exists()
    assert lane_brief_path(home, lane) is None
