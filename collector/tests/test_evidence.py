"""A finished plan item shows its evidence: commit, test output and screenshots."""

from __future__ import annotations

from pathlib import Path

from tirith import evidence, roadmap as rm

ITEM = "Ship it. (check: `true`) (hecho: abc1234) <!-- tirith:{\"id\":\"i-a1b2c3d4\"} -->"


def _project(tmp_path: Path, text: str = ITEM) -> Path:
    (tmp_path / "docs").mkdir()
    (tmp_path / "docs" / "TODO.md").write_text(f"## 1. Step\n\n- [x] {text}\n- [ ] Open one\n")
    return tmp_path


def test_evidence_commit_of() -> None:
    assert evidence.commit_of("x (hecho: abc1234)") == "abc1234"
    assert evidence.commit_of("x (hecho: sin cambios)") is None
    assert evidence.commit_of("x") is None


def test_evidence_from_note_only(tmp_path: Path) -> None:
    item = rm.build(_project(tmp_path), "p", [])["steps"][0]["items"][0]
    assert item["evidence"] == {"commit": "abc1234", "tests": "", "shots": []}


def test_evidence_reads_folder(tmp_path: Path) -> None:
    repo = _project(tmp_path)
    folder = repo / "docs" / "evidence" / "i-a1b2c3d4"
    folder.mkdir(parents=True)
    (folder / "tests.txt").write_text("12 passed in 1s\n")
    (folder / "after.png").write_bytes(b"png")
    (folder / "notes.md").write_text("ignored")
    items = rm.build(repo, "p", [])["steps"][0]["items"]
    found = items[0]["evidence"]
    assert found["tests"] == "12 passed in 1s"
    assert found["shots"] == [str(folder / "after.png")]
    assert "evidence" not in items[1]  # open items have none


def test_evidence_none_without_proof(tmp_path: Path) -> None:
    repo = _project(tmp_path, "Plain done item <!-- tirith:{\"id\":\"i-a1b2c3d4\"} -->")
    assert "evidence" not in rm.build(repo, "p", [])["steps"][0]["items"][0]


def test_evidence_ignores_bad_id_and_truncates(tmp_path: Path) -> None:
    assert evidence.folder_of(tmp_path, "../etc") is None
    folder = tmp_path / "docs" / "evidence" / "i-00000000"
    folder.mkdir(parents=True)
    (folder / "t.log").write_text("x" * 10000 + "END")
    tests = evidence.for_item(tmp_path, "i-00000000", "")["tests"]
    assert len(tests) <= evidence.TESTS_MAX + 1 and tests.endswith("END")
