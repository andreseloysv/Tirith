"""Opt-in live model test: runs only when a Meta Model API key is in the environment."""

from __future__ import annotations

import os

import pytest

from tirith import brain as brain_mod
from tirith.home import Config

pytestmark = pytest.mark.skipif(
    brain_mod.api_key_from_env() is None, reason="no Meta Model API key in the environment")


def test_live_summary_round_trip() -> None:
    client = brain_mod.make_client(Config())
    assert client is not None
    summary = brain_mod.job_summary(
        client, "running device tests\n19 tests passed, 0 failed\n")
    # A real sentence about the log, not an empty or placeholder answer.
    assert len(summary.split()) >= 5, summary
    assert summary.strip().lower() not in {"none", "working."}, summary
    assert "test" in summary.lower(), summary
