"""`tirith` for agents lives in the repo: bin/tirith, on every lane's PATH and the collector's."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

import pytest

from tirith import cli, launch

ROOT = Path(__file__).resolve().parents[2]


def test_bin_tirith_runs_the_collector_cli_in_place() -> None:
    script = ROOT / "bin" / "tirith"
    text = script.read_text(encoding="utf-8")
    assert os.access(script, os.X_OK) and text.startswith("#!")
    assert 'uv run -q --project "$ROOT/collector" tirith "$@"' in text
    assert "cd " not in text.split("exec ")[0].replace('ROOT=$(cd "$(dirname', "")  # the current folder is kept


def test_bin_tirith_finds_uv_without_a_login_path(tmp_path: Path) -> None:
    if not any((Path(d) / "uv").exists() for d in ("/opt/homebrew/bin", "/usr/local/bin")):
        pytest.skip("no uv in the fallback folders")
    env = dict(HOME=str(tmp_path), PATH="/usr/bin:/bin", TIRITH_HOME=str(tmp_path / "th"))
    done = subprocess.run([str(ROOT / "bin" / "tirith"), "--help"], env=env, capture_output=True, text=True, timeout=120)
    assert done.returncode == 0, done.stderr
    assert "usage" in done.stdout.lower()


def test_lane_path_has_the_repo_bin_next_to_the_shims(home: Path) -> None:
    parts = cli._wrapper_path_env(home)["PATH"].split(os.pathsep)
    assert str(ROOT / "bin") in parts
    assert parts.index(str(home / "shims")) < parts.index(str(ROOT / "bin"))


def test_run_script_puts_bin_on_the_collector_path() -> None:
    assert 'PATH="$ROOT/bin:$PATH"' in (ROOT / "run").read_text(encoding="utf-8")


def test_brief_tells_agents_how_to_run_tests() -> None:
    brief = launch.build_brief("T", "", ["x"], "1")
    assert "tirith run --project <id> --stage test" in brief
