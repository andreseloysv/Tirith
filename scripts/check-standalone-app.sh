#!/bin/sh
# Proves the macOS app works on a clean Mac: the bundle carries the collector and the web, and the bundled
# collector runs once with a temp HOME and a PATH that does not contain this repository.
# Uses $TIRITH_APP (a built Tirith.app), else builds one. Exit 0 = ok, 1 = a check failed.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
fail() { echo "FAIL: $*" >&2; exit 1; }
[ "$(uname -s)" = Darwin ] || fail "macOS only"

app="${TIRITH_APP:-}"
if [ -z "$app" ]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$root/target}"
  cd "$root/app"
  [ -d node_modules ] || npm install --no-audit --no-fund
  npm run tauri build -- --bundles app >&2
  app=$(find "$CARGO_TARGET_DIR/release/bundle/macos" -maxdepth 1 -name '*.app' | head -n 1)
fi
[ -d "$app" ] || fail "no app bundle found (set TIRITH_APP)"
res="$app/Contents/Resources/resources"
echo "app: $app"

for f in collector/pyproject.toml collector/uv.lock collector/tirith/cli.py collector/tirith_guard.py web/index.html web/js/main.js web/js/firstrun.js; do
  [ -f "$res/$f" ] || fail "missing in the bundle: $f"
done
for bad in collector/tests collector/.venv web/screenshots; do
  [ ! -e "$res/$bad" ] || fail "should not be in the bundle: $bad"
done
echo "resources: ok"

uv=$(command -v uv) || fail "uv is needed to run the collector"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cache=$(uv cache dir)
pydir=$(uv python dir)

# what the app does on first run: a writable copy of the collector
cp -R "$res/collector" "$work/collector"
mkdir -p "$work/home" "$work/cwd"
cd "$work/cwd"
env -i HOME="$work/home" TIRITH_HOME="$work/home/.tirith" PATH="$(dirname "$uv"):/usr/bin:/bin" \
  UV_CACHE_DIR="$cache" UV_PYTHON_INSTALL_DIR="$pydir" \
  "$uv" run --project "$work/collector" tirith collect --once >"$work/out.log" 2>&1 \
  || { cat "$work/out.log" >&2; fail "the bundled collector did not run"; }
[ -s "$work/home/.tirith/state.json" ] || { cat "$work/out.log" >&2; fail "collect --once wrote no state.json"; }
echo "collector: ok (state.json written in a temp HOME)"
echo "OK"
