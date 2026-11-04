"""A lane's brief carries a code map: the files relevant to its item."""

from __future__ import annotations

from pathlib import Path

from tests.conftest import make_repo
from tirith import codemap, launch


def _repo(tmp_path: Path) -> Path:
    repo = tmp_path / "repo"
    (repo / "collector" / "tirith").mkdir(parents=True)
    (repo / "collector" / "tests").mkdir(parents=True)
    (repo / "docs").mkdir()
    (repo / "collector" / "tirith" / "evidence.py").write_text("def show():\n    return 'evidence'\n")
    (repo / "collector" / "tirith" / "other.py").write_text("x = 1\n")
    (repo / "collector" / "tests" / "test_evidence.py").write_text("def test_evidence():\n    pass\n")
    (repo / "docs" / "CONTRACT-evidence.md").write_text("# evidence\n")
    return make_repo(repo)


ITEM = "Each finished item shows its evidence in the app. (check: `cd collector && uv run -q pytest -q tests -k evidence`)"


def test_codemap_lists_relevant_files(tmp_path: Path) -> None:
    text = codemap.section(_repo(tmp_path), ITEM)
    assert text.startswith("## Code map")
    assert "collector/tirith/evidence.py" in text
    assert "collector/tests/test_evidence.py" in text
    assert "docs/CONTRACT-evidence.md" in text
    assert "other.py" not in text


def test_codemap_named_path_comes_first(tmp_path: Path) -> None:
    found = codemap.rank(_repo(tmp_path), "Change `collector/tirith/other.py` for evidence")
    assert found["code"][0] == "collector/tirith/other.py"


def test_codemap_empty_without_match_or_repo(tmp_path: Path) -> None:
    assert codemap.section(_repo(tmp_path), "zzzzqqqq wwwwxxxx") == ""
    assert codemap.section(None, ITEM) == ""
    assert codemap.section(tmp_path / "missing", ITEM) == ""


def test_codemap_in_brief(tmp_path: Path) -> None:
    repo = _repo(tmp_path)
    brief = launch.build_brief("Step", "", [ITEM], "3", ITEM, repo)
    assert brief.index("## What to build") < brief.index("## Code map") < brief.index("## Hard rules")
    assert "collector/tirith/evidence.py" in brief
    assert "## Code map" not in launch.build_brief("Step", "", [ITEM], "3", ITEM)
