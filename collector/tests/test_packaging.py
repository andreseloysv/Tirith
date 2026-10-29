"""Packaging files stay consistent: bundle config, icons, build scripts and the CI workflow."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def test_bundle_icons_exist() -> None:
    conf = json.loads((ROOT / "app/src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
    assert conf["bundle"]["active"] is True
    for icon in conf["bundle"]["icon"]:
        assert (ROOT / "app/src-tauri" / icon).is_file(), icon


def test_build_scripts_exist_for_unix_and_windows() -> None:
    assert (ROOT / "scripts/package.sh").read_text(encoding="utf-8").startswith("#!/bin/sh")
    ps = (ROOT / "scripts/package.ps1").read_text(encoding="utf-8")
    assert "npm run tauri build" in ps and "*.msi" in ps


def test_ci_builds_the_three_platforms() -> None:
    text = (ROOT / ".github/workflows/app.yml").read_text(encoding="utf-8")
    for runner in ("macos-latest", "windows-latest", "ubuntu-22.04"):
        assert runner in text
