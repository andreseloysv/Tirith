#!/bin/sh
# Build, sign, notarize and staple the macOS app and dmg with one command.
# Needs a "Developer ID Application" identity in the keychain ($APPLE_SIGNING_IDENTITY, else the first one)
# and a notarytool keychain profile ($TIRITH_NOTARY_PROFILE, default tirith-notary). No secret is read or printed.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
profile="${TIRITH_NOTARY_PROFILE:-tirith-notary}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/Users/andreseloyasanchezva/DEVELOP/Tirith-lanes/tauri/app/src-tauri/target}"

[ "$(uname -s)" = Darwin ] || { echo "macOS only" >&2; exit 1; }

if [ -z "${APPLE_SIGNING_IDENTITY:-}" ]; then
  APPLE_SIGNING_IDENTITY=$(security find-identity -v -p codesigning \
    | sed -n 's/^[[:space:]]*[0-9][0-9]*) [0-9A-F]* "\(Developer ID Application:.*\)"$/\1/p' | head -n 1)
fi
[ -n "$APPLE_SIGNING_IDENTITY" ] || { echo "no Developer ID Application identity found" >&2; exit 1; }
export APPLE_SIGNING_IDENTITY
echo "signing with: $APPLE_SIGNING_IDENTITY"

# This script notarizes; keep Tauri from doing it a second time.
unset APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID APPLE_API_KEY APPLE_API_ISSUER APPLE_API_KEY_PATH

cd "$root/app"
[ -d node_modules ] || npm install
npm run tauri build -- --bundles app,dmg

bundle="$CARGO_TARGET_DIR/release/bundle"
app=$(find "$bundle/macos" -maxdepth 1 -name '*.app' | head -n 1)
dmg=$(find "$bundle/dmg" -maxdepth 1 -name '*.dmg' | head -n 1)
[ -n "$app" ] && [ -n "$dmg" ] || { echo "app or dmg missing under $bundle" >&2; exit 1; }

codesign --verify --deep --strict --verbose=2 "$app"
codesign -dv "$app" 2>&1 | grep -q 'flags=.*runtime' || { echo "hardened runtime is off on $app" >&2; exit 1; }
codesign --force --timestamp --sign "$APPLE_SIGNING_IDENTITY" "$dmg"
codesign --verify --strict --verbose=2 "$dmg"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# submit <file>: wait for the verdict and print the id; on failure print the log summary and return non-zero.
submit() {
  out="$work/submit.json"
  xcrun notarytool submit "$1" --keychain-profile "$profile" --wait --output-format json >"$out" || true
  id=$(sed -n 's/.*"id" *: *"\([^"]*\)".*/\1/p' "$out" | head -n 1)
  status=$(sed -n 's/.*"status" *: *"\([^"]*\)".*/\1/p' "$out" | head -n 1)
  echo "submission id: ${id:-unknown}  status: ${status:-unknown}  ($(basename "$1"))"
  if [ "$status" != Accepted ]; then
    if [ -n "$id" ]; then
      xcrun notarytool log "$id" --keychain-profile "$profile" | head -n 60
    fi
    return 1
  fi
}

zip="$work/$(basename "$app" .app).zip"
ditto -c -k --keepParent "$app" "$zip"
submit "$zip"
xcrun stapler staple "$app"

# The dmg is a separate submission, so it gets its own ticket to staple.
submit "$dmg"
xcrun stapler staple "$dmg"

spctl -a -vv "$app"
spctl -a -vv -t open --context context:primary-signature "$dmg"
echo "stapled app: $app"
echo "stapled dmg: $dmg"
