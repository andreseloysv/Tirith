"""Opt-in live model test: runs only when a Meta Model API key is in the environment."""

from __future__ import annotations

import os

import pytest

from tirith import brain as brain_mod
from tirith.home import Config

# a real, paid call to the brain's API: only when asked (TIRITH_LIVE_TESTS=1). Every full suite of every lane ran it
# whenever the key was in the environment, and the network watch showed lanes reaching the API from their tests.
pytestmark = pytest.mark.skipif(
    os.environ.get("TIRITH_LIVE_TESTS") != "1" or brain_mod.api_key_from_env() is None,
    reason="live test: set TIRITH_LIVE_TESTS=1 (and the brain's API key) to run it")


def test_live_summary_round_trip() -> None:
    client = brain_mod.make_client(Config())
    assert client is not None
    summary = brain_mod.job_summary(
        client, "running device tests\n19 tests passed, 0 failed\n")
    # A real sentence about the log, not an empty or placeholder answer.
    assert len(summary.split()) >= 5, summary
    assert summary.strip().lower() not in {"none", "working."}, summary
    assert "test" in summary.lower(), summary
