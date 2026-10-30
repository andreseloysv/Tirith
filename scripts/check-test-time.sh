#!/usr/bin/env bash
# The plan check: the Python suite, run like the `test` stage, must pass in under 240 s of wall clock.
set -u
LIMIT=240
cd "$(dirname "$0")/../collector" || exit 1
start=$(date +%s)
uv run -q pytest -q -n auto --maxprocesses 8
status=$?
elapsed=$(( $(date +%s) - start ))
echo "python test suite: ${elapsed}s (limit ${LIMIT}s)"
if [ "$status" -ne 0 ]; then
  echo "FAIL: the suite failed (pytest exit $status)" >&2
  exit "$status"
fi
if [ "$elapsed" -gt "$LIMIT" ]; then
  echo "FAIL: the suite took ${elapsed}s, more than ${LIMIT}s" >&2
  exit 1
fi
