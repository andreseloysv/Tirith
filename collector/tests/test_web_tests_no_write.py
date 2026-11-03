"""Browser tests must not write into versioned files: captures go through web/js/shots.mjs (temp dir by default)."""
import re
from pathlib import Path

JS = Path(__file__).resolve().parents[2] / "web" / "js"
TESTS = sorted(p for p in JS.glob("*.test.mjs") if p.name != "landing.test.mjs")


def test_no_test_joins_the_versioned_captures_folder():
    bad = [p.name for p in TESTS if re.search(r"""path\.join\([^)]*['"]screenshots['"]""", p.read_text())]
    assert bad == []


def test_screenshots_go_through_helper_or_an_explicit_argument():
    ok = re.compile(r"path:\s*(shotPath\(|process\.argv|shot\b|shots\b|[a-z]*[Pp]ath\b)")
    bad = []
    for p in TESTS:
        for i, line in enumerate(p.read_text().splitlines(), 1):
            if ".screenshot(" in line and not ok.search(line):
                bad.append(f"{p.name}:{i}")
    assert bad == []


def test_helper_defaults_to_a_temp_dir():
    src = (JS / "shots.mjs").read_text()
    assert "UPDATE_SCREENSHOTS" in src and "mkdtempSync" in src


def test_planagents_never_names_versioned_folder():
    assert "screenshots" not in (JS / "planagents.test.mjs").read_text()
