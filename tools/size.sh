#!/bin/sh
# Every code file that is not a test has 100 lines or less (docs/BOTS.md, part 0): the daemon's Rust as
# `cargo fmt` writes it, the page's JavaScript and CSS, the app, the tools and the site the same. Tests
# (rust/tirith/checks, *.test.*, scripts/checks, tools/perf), docs/ and vendored code are not counted.
# The merge queue runs main's copy of this file (tirith.toml), so a branch cannot loosen it for itself.
#
#   sh tools/size.sh             every file
#   sh tools/size.sh <path>...   only the files under those paths
limit=100
cargo fmt --check -p tirith >/dev/null 2>&1 || { echo 'cargo fmt --check failed: run cargo fmt -p tirith'; exit 1; }

over=$(git ls-files '*.rs' '*.js' '*.mjs' '*.cjs' '*.ts' '*.py' '*.sh' '*.css' |
  grep -vE '^rust/tirith/checks/|\.test\.|^scripts/checks/|^tools/perf/|^docs/|(^|/)vendor/' |
  while read -r f; do
    [ -f "$f" ] || continue
    if [ $# -gt 0 ]; then
      mine=no
      for p in "$@"; do case "$f" in ("$p"*) mine=yes ;; esac; done
      [ "$mine" = yes ] || continue
    fi
    n=$(wc -l < "$f" | tr -d ' ')
    [ "$n" -le "$limit" ] || echo "  $f: $n lines"
  done)

if [ -n "$over" ]; then
  echo "over $limit lines (split them by responsibility, see docs/BOTS.md part 0):"
  echo "$over"
  exit 1
fi
echo "every code file has $limit lines or less"
