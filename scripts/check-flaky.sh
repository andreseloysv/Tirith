#!/bin/sh
# The plan check "no Python test fails at random": runs the tests that used to flake 10 times and exits
# non-zero on any failure. Order is random with pytest-randomly when installed, else a different fixed
# shuffle of the files per run (seeded by the run number, so a failure can be replayed).
# FLAKY_RUNS=<n> changes the number of runs.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root/collector"

runs=${FLAKY_RUNS:-10}
files="tests/test_plans.py tests/test_lane_interactive.py tests/test_autopilot.py tests/test_dispatch.py
tests/test_first_cycle.py tests/test_cycle_budget.py tests/test_msession.py tests/test_managerd.py
tests/test_autoreview.py tests/test_asks.py"

if uv run -q python -c 'import pytest_randomly' >/dev/null 2>&1; then
  mode="random order (pytest-randomly)"
else
  mode="fixed shuffled order"
fi

failed=0
n=1
while [ "$n" -le "$runs" ]; do
  if [ "$mode" = "fixed shuffled order" ]; then
    order=$(uv run -q python -c 'import random, sys; f = sys.argv[2:]; random.Random(int(sys.argv[1])).shuffle(f); print(" ".join(f))' "$n" $files)
  else
    order=$files
  fi
  echo "flaky check $n/$runs: $mode"
  # shellcheck disable=SC2086
  if ! uv run -q pytest -q -p no:cacheprovider $order; then
    echo "flaky check: run $n FAILED" >&2
    failed=1
  fi
  n=$((n + 1))
done

if [ "$failed" -ne 0 ]; then
  echo "flaky check: at least one run failed" >&2
  exit 1
fi
echo "flaky check: $runs runs green"
