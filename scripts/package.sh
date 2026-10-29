#!/bin/sh
# Release build of the desktop app for the current platform; prints the bundle path(s).
# Unsigned unless the signing variables in app/README.md are set. Extra args go to `tauri build`.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root/app"

[ -d node_modules ] || npm install
npm run tauri build -- "$@"

bundle="$root/app/src-tauri/target/release/bundle"
[ -d "$bundle" ] || { echo "no bundle directory at $bundle" >&2; exit 1; }
echo "bundles:"
find "$bundle" -mindepth 2 -maxdepth 2 \( -name '*.app' -o -name '*.dmg' -o -name '*.msi' -o -name '*.exe' \
  -o -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' \) -print
