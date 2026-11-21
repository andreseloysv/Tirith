#!/bin/sh
# Every code file that is not a test has 100 lines or less (docs/BOTS.md, part 0): the daemon's Rust as
# `cargo fmt` writes it, the page's JavaScript and CSS, the app, the tools and the site the same. Tests
# (rust/tirith/checks, *.test.*, scripts/checks, tools/perf), docs/ and vendored code are not counted.
# The merge queue runs main's copy of this file (tirith.toml), so a branch cannot loosen it for itself.
#
#   sh tools/size.sh             every file; the ones still waiting below may stay over the limit
#   sh tools/size.sh <path>...   only the files under those paths, none of them excused
limit=100
cargo fmt --check -p tirith >/dev/null 2>&1 || { echo 'cargo fmt --check failed: run cargo fmt -p tirith'; exit 1; }

# Files still waiting for their split (docs/BOTS.md, part 0). The last split item empties this list.
pending='
app/src-tauri/icons/source/glow3d/build.py
rust/tirith/src/app.rs
rust/tirith/src/attach.rs
rust/tirith/src/bots.rs
rust/tirith/src/chat.rs
rust/tirith/src/codex.rs
rust/tirith/src/config.rs
rust/tirith/src/factory.rs
rust/tirith/src/guard.rs
rust/tirith/src/lanes.rs
rust/tirith/src/main.rs
rust/tirith/src/manager.rs
rust/tirith/src/merge.rs
rust/tirith/src/metrics.rs
rust/tirith/src/models.rs
rust/tirith/src/plan.rs
rust/tirith/src/projects.rs
rust/tirith/src/rooms.rs
rust/tirith/src/rooms/sealed.rs
rust/tirith/src/rules.rs
rust/tirith/src/sandbox.rs
rust/tirith/src/sandbox/proxy.rs
rust/tirith/src/server.rs
site/style.css
web/css/agents.css
web/css/bots.css
web/css/deck.css
web/css/style.css
web/js/app.js
web/js/common.js
web/js/md.js
web/js/model-controls.js
web/js/store.js
web/js/views/addproject.js
web/js/views/agents.js
web/js/views/boards/logview.js
web/js/views/bots.js
web/js/views/bots/face.js
web/js/views/chat.js
web/js/views/overview.js
web/js/views/overview/plan.js
web/js/views/pipelines.js
web/js/views/rooms.js
web/js/views/settings/models.js
'

over=$(git ls-files '*.rs' '*.js' '*.mjs' '*.cjs' '*.ts' '*.py' '*.sh' '*.css' |
  grep -vE '^rust/tirith/checks/|\.test\.|^scripts/checks/|^tools/perf/|^docs/|(^|/)vendor/' |
  while read -r f; do
    [ -f "$f" ] || continue
    if [ $# -gt 0 ]; then
      mine=no
      for p in "$@"; do case "$f" in ("$p"*) mine=yes ;; esac; done
      [ "$mine" = yes ] || continue
    elif printf '%s\n' "$pending" | grep -qxF "$f"; then
      continue
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
