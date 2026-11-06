"""A lane that reported done enters the merge queue in one dispatcher tick, even when the lane list the
dispatcher was handed is stale (the canary lanes stayed 'queued' and never reached the queue)."""
from __future__ import annotations

from pathlib import Path

from tirith import mergeq as mergeq_mod
from tirith.home import load_config

from test_mergeq import env, make_lane  # noqa: F401  (fixture and helper)


def test_done_lane_is_enqueued_from_a_stale_snapshot(env, home: Path) -> None:
    repo, _origin = env
    make_lane(home, repo, "alpha", {"a.txt": "a\n"})
    from tirith.store import lane_path, read_json
    lane = read_json(lane_path(home, "alpha"), None)
    stale = [{"id": "alpha", "ended_at": None, "result": None}]  # the snapshot saw it still running
    lane["outcome"] = {"status": "done"}
    from tirith.store import write_lane
    write_lane(home, lane)
    assert mergeq_mod.run(home, load_config(home), stale, spawn=lambda *a, **k: None, dry_run=True)
    assert mergeq_mod.queued(home, "alpha")


def test_done_lane_missing_from_the_snapshot_is_enqueued(env, home: Path) -> None:
    repo, _origin = env
    make_lane(home, repo, "beta", {"b.txt": "b\n"})
    from tirith.store import lane_path, read_json, write_lane
    lane = read_json(lane_path(home, "beta"), None)
    lane["outcome"] = {"status": "done"}
    write_lane(home, lane)
    assert mergeq_mod.run(home, load_config(home), [], dry_run=True)
    assert mergeq_mod.queued(home, "beta")
