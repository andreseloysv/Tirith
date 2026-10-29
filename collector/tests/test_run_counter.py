"""`tirith run --new-run` started at the same moment gets distinct run numbers."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

from tirith import runstage
from tirith.store import read_json

CODE = "import sys; from pathlib import Path; from tirith import runstage; " \
       "print(runstage.resolve_run(Path(sys.argv[1]), 'p', 'ci', None, True))"


def test_parallel_new_runs_get_distinct_numbers(tmp_path: Path) -> None:
    home = tmp_path / "home"
    home.mkdir()
    root = Path(__file__).resolve().parents[1]
    procs = [subprocess.Popen([sys.executable, "-c", CODE, str(home)], cwd=root, stdout=subprocess.PIPE, text=True)
             for _ in range(12)]
    runs = [p.communicate()[0].strip() for p in procs]
    assert sorted(runs, key=lambda r: int(r.split("#")[1])) == [f"ci#{n}" for n in range(1, 13)]
    assert read_json(home / "pipelines.json", {})["p/ci"]["counter"] == 12
    assert runstage.resolve_run(home, "p", "ci", None, False) == "ci#12"
