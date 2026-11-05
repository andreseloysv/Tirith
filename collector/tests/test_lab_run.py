"""tirith lab run: scenario in a throwaway copy, scripted owner, fake agent."""

import json
import shutil
import sys
from pathlib import Path

from tirith import labrun
from tirith.cli import main

ROOT = Path(__file__).resolve().parents[2]
LAB = ROOT / "fixtures" / "company-lab"
AGENT = [sys.executable, str(Path(__file__).parent / "fake_lab_agent.py")]


def _lab(tmp_path: Path, patch=None) -> Path:
    lab = tmp_path / "lab"
    shutil.copytree(LAB, lab)
    if patch:
        f = lab / "scenarios.json"
        data = json.loads(f.read_text())
        for sc in data["scenarios"]:
            sc.update(patch.get(sc["id"], {}))
        f.write_text(json.dumps(data))
    return lab


def test_lab_run_promotion_and_throwaway(tmp_path):
    lab = _lab(tmp_path, {"login-3-parts": {"check": "test -f src/auth.py"}})
    r = labrun.run_scenario(lab, "login-3-parts", AGENT)
    assert r["items_to_main"] == 3 and r["duplicate_items"] == 1
    assert r["cost_usd"] == 0.75 and r["seconds"] == 180
    assert r["promotions"] == [{"role": "manager", "reason": "3 parallel items"}]
    assert r["checked"] is True and r["ok"]
    assert not (lab / "src" / "auth.py").exists()  # the fixture itself is untouched


def test_lab_run_owner_awake_vs_asleep(tmp_path):
    lab = _lab(tmp_path, {"typo": {"owner_request": "typo: please sleep-ask", "check": "true"}})
    assert labrun.run_scenario(lab, "typo", AGENT)["owner_interruptions"] == 1
    lab = _lab(tmp_path / "b", {"typo": {"owner_request": "typo: please sleep-ask", "check": "true",
                                         "unattended": True}})
    r = labrun.run_scenario(lab, "typo", AGENT)
    assert r["owner_interruptions"] == 0 and r["blocked_asks"] == 1


def test_lab_run_cli_writes_report(tmp_path):
    out = tmp_path / "out"
    lab = _lab(tmp_path, {"typo": {"check": "true"}})
    code = main(["lab", "run", "typo", "--agent", " ".join(AGENT), "--lab", str(lab), "--out", str(out)])
    assert code == 0
    assert json.loads((out / "typo.json").read_text())["items_to_main"] == 1
    assert "Role promotions" in (out / "typo.md").read_text()
    assert main(["lab", "run", "nope", "--agent", "x", "--lab", str(lab), "--out", str(out)]) == 2
