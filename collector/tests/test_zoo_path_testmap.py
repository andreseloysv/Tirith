from pathlib import Path

from tirith import ceo, fasttests, testmap


def test_zoo_paths_pick_their_test_commands() -> None:
    sel = fasttests.select_paths("zoo", "test", ["cms/web/src/a.ts", "cms/server/x.js", "docs/a.md"])
    assert not sel.full
    assert sel.commands == ["node --test cms/server/tests", "npx vitest run --root cms/web"]
    assert fasttests.command(sel) == " && ".join(sel.commands)
    swift = fasttests.select_paths("zoo", "test", ["Packages/ZooCore/Sources/A.swift"])
    assert swift.commands == ["swift test --package-path Packages/ZooCore"]


def test_zoo_unmapped_code_runs_everything_and_docs_nothing() -> None:
    assert fasttests.select_paths("zoo", "test", ["tools/x.py"]).full
    docs = fasttests.select_paths("zoo", "test", ["README.md"])
    assert not docs.full and fasttests.command(docs) == "true"


def test_for_lane_uses_the_path_map_and_ceo_lists_it(tmp_path: Path) -> None:
    import subprocess
    repo = tmp_path / "repo"
    (repo / "cms/server").mkdir(parents=True)
    subprocess.run(["git", "init", "-q", "-b", "main", str(repo)], check=True)
    (repo / "cms/server/a.js").write_text("x")
    sel = fasttests.for_lane(tmp_path / "home", "zoo", repo, "test")
    assert sel.commands == ["node --test cms/server/tests"]
    from datetime import datetime
    assert ceo._test_maps(tmp_path / "home", datetime.now())["zoo"]["present"]
    assert testmap.path_map("tirith") == []
