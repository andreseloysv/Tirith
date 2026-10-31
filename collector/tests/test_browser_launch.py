"""Browser tests and rendering helpers must never launch Edge and steal the owner's focus."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def test_browser_scripts_never_select_edge() -> None:
    scripts = [script for folder in ("web/js", "docs/demo", "docs/social")
               for script in (ROOT / folder).rglob("*.mjs")]
    offenders = [str(script.relative_to(ROOT)) for script in scripts
                 if "msedge" in script.read_text(encoding="utf-8").lower()]
    assert not offenders, f"Use Playwright's bundled headless Chromium instead of Edge: {offenders}"
