"""Commit guard on throwaway tmp repos only, plus lane-run refusal."""

from __future__ import annotations

import os
import sys
from pathlib import Path

from conftest import make_hooks_mutable, make_repo, write_config, write_lane_manifest, write_plan, REPORT_DONE
from tirith.cli import main
from tirith.collector import Collector
from tirith.guardwrap import default_words, verify_guard


def test_guard_missing_then_install_then_ok(home: Path, tmp_path: Path, capsys) -> None:
    repo = make_repo(tmp_path / "repo")
    try:
        write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo),
                             "color": "#123456", "guard_words": ["bannedword"]}])
        assert main(["guard", "verify", "demo"]) == 1
        assert "NOT OK" in capsys.readouterr().out
        assert main(["guard", "install", "demo"]) in (0, 1)
        problems = verify_guard(repo, ["bannedword"])
        # Hooks exist with the marker; immutability needs BSD flags (macOS).
        assert not any("missing" in p or "marker" in p for p in problems), problems
        hooks = repo / ".git" / "hooks"
        assert "FORBIDDEN_WORDS_GUARD" in (hooks / "pre-commit").read_text()
        assert "bannedword" in (hooks / "pre-commit").read_text()
    finally:
        make_hooks_mutable(repo)


def test_guard_default_words_and_tamper_detection(home: Path, tmp_path: Path) -> None:
    repo = make_repo(tmp_path / "repo")
    try:
        write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo),
                             "color": "#123456"}])
        main(["guard", "install", "demo"])
        clean = verify_guard(repo, default_words())
        assert not any("missing" in p or "differs" in p for p in clean), clean
        make_hooks_mutable(repo)
        hook = repo / ".git" / "hooks" / "pre-commit"
        hook.write_text(hook.read_text(encoding="utf-8") + "# tampered\n", encoding="utf-8")
        problems = verify_guard(repo, default_words())
        assert any("differs" in p for p in problems), problems
        # Collector surfaces the broken guard as a waiting item + project field.
        state = Collector(home, client=None).cycle()
        proj = state["projects"][0]
        assert proj["guard"]["ok"] is False and proj["guard"]["problems"]
        assert any(w.get("source") == "guard" and w["project"] == "demo"
                   for w in state["waiting_on_you"])
    finally:
        make_hooks_mutable(repo)


def test_lane_run_refuses_without_guard_then_runs(home: Path, tmp_path: Path, capsys) -> None:
    repo = make_repo(tmp_path / "repo")
    write_lane_manifest(repo)
    write_plan(repo)
    brief = str(repo / "brief.md")
    try:
        write_config(home, [{"id": "demo", "name": "Demo", "repo": str(repo),
                             "color": "#123456"}])
        code = main(["lane", "run", "--project", "demo", "--class", "warden",
                     "--feature", "Guarded work", "--id", "g-1", "--brief", brief, "--step", "1", "--item", "Do the thing.", "--",
                     sys.executable, "-c", "print('should not run')"])
        assert code == 3
        assert "refusing to start" in capsys.readouterr().err
        assert not (home / "lanes" / "g-1.json").exists()
        main(["guard", "install", "demo"])
        capsys.readouterr()
        # On platforms without immutable flags the guard still reports the
        # missing uchg bit; only run the happy path when fully intact.
        if verify_guard(repo, default_words()):
            return
        code = main(["lane", "run", "--project", "demo", "--class", "warden",
                     "--feature", "Guarded work", "--id", "g-1", "--brief", brief, "--step", "1", "--item", "Do the thing.", "--",
                     sys.executable, "-c", REPORT_DONE + "import os; print(' '.join(os.environ['PATH'].split(':')[:2]))"])
        assert code == 0
        log_text = (home / "logs" / "g-1.log").read_text(encoding="utf-8")
        shims, wrapper = log_text.strip().split(" ")[-2:]  # agent shims first, then the git wrapper
        assert shims.endswith("shims") and wrapper.endswith(os.path.join(".local", "bin"))
        state = Collector(home, client=None).cycle()
        assert state["agents"][0]["status"] == "done"
    finally:
        make_hooks_mutable(repo)
