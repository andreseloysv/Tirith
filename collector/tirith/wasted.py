"""Wasted work: what Tirith ran or built and then threw away, counted so the owner can see it in the health.

Three kinds, all read from what the merge queue settles (``mergeq._finish`` calls :func:`record`):

* ``repeated``: a full suite run again for a lane whose suite already ran (red, then fixed and tested again);
* ``discarded``: a merge that was tested and then not needed, because another queued lane already held its commits;
* ``absorbed``: a lane dropped before testing because another queued lane's branch already contains all of it.

Totals live in ``<home>/wasted.json``; ``summary`` is the plain-words line shown by the ``wasted`` health check.
See ``docs/CONTRACT-selfcheck.md``.
"""

from __future__ import annotations

from pathlib import Path

from tirith.store import read_json, write_json_atomic

FILE = "wasted.json"
KINDS = ("repeated", "discarded", "absorbed")
SUITE_RESULTS = ("merged", "push-failed")  # a suite ran and passed (red is told apart by its message)
LANES_KEPT = 500


def _load(home: Path) -> dict:
    data = read_json(Path(home) / FILE, None)
    if not isinstance(data, dict):
        data = {}
    for kind in KINDS:
        data[kind] = int(data.get(kind) or 0)
    if not isinstance(data.get("suites"), dict):
        data["suites"] = {}
    return data


def suite_ran(row: dict) -> bool:
    """Did a full suite run for this settled queue row?"""
    result, message = str(row.get("result") or ""), str(row.get("message") or "")
    return result in SUITE_RESULTS or (result == "red" and message.startswith("stage ")) \
        or row.get("wasted") == "discarded"


def record(home: Path, row: dict) -> None:
    """Count one settled merge-queue row (a row that leaves the queue)."""
    kind = row.get("wasted")
    ran = suite_ran(row)
    if kind not in ("discarded", "absorbed") and not ran:
        return
    data = _load(home)
    lane = str(row.get("lane") or "")
    if ran:
        if data["suites"].get(lane):
            data["repeated"] += 1
        data["suites"][lane] = int(data["suites"].get(lane) or 0) + 1
        if len(data["suites"]) > LANES_KEPT:
            data["suites"] = dict(list(data["suites"].items())[-LANES_KEPT:])
    if kind in ("discarded", "absorbed"):
        data[kind] += 1
    write_json_atomic(Path(home) / FILE, data)


def totals(home: Path) -> dict:
    data = _load(home)
    return {kind: data[kind] for kind in KINDS}


def summary(home: Path) -> str:
    got = totals(home)
    if not any(got.values()):
        return "No work has been thrown away."
    parts = []
    if got["repeated"]:
        parts.append(f"{got['repeated']} suite(s) run again")
    if got["discarded"]:
        parts.append(f"{got['discarded']} tested merge(s) discarded")
    if got["absorbed"]:
        parts.append(f"{got['absorbed']} lane(s) absorbed by another")
    return "Work thrown away: " + ", ".join(parts) + "."
