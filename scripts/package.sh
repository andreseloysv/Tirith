#!/bin/sh
# Release build of the desktop app for the current platform; prints the bundle path(s).
# On macOS it signs with a Developer ID Application identity (hardened runtime, entitlements.plist):
# $APPLE_SIGNING_IDENTITY, else the first one in the keychain. It notarizes and staples when the
# credentials in app/README.md are set. Without an identity the build is unsigned.
# Extra args go to `tauri build`.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root/app"

notarize=""
if [ "$(uname -s)" = Darwin ]; then
  if [ -z "${APPLE_SIGNING_IDENTITY:-}" ]; then
    APPLE_SIGNING_IDENTITY=$(security find-identity -v -p codesigning \
      | sed -n 's/^[[:space:]]*[0-9][0-9]*) [0-9A-F]* "\(Developer ID Application:.*\)"$/\1/p' | head -n 1)
  fi
  if [ -n "$APPLE_SIGNING_IDENTITY" ]; then
    export APPLE_SIGNING_IDENTITY
    echo "signing with: $APPLE_SIGNING_IDENTITY"
    if [ -n "${APPLE_API_KEY_PATH:-}" ] && [ -n "${APPLE_API_KEY:-}" ] && [ -n "${APPLE_API_ISSUER:-}" ]; then
      notarize=key
    elif [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_PASSWORD:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ]; then
      notarize=apple-id
    fi
    echo "notarization: ${notarize:-off}"
  else
    unset APPLE_SIGNING_IDENTITY
    echo "no Developer ID Application identity found: building unsigned" >&2
  fi
fi

[ -d node_modules ] || npm install
npm run tauri build -- "$@"

target="${CARGO_TARGET_DIR:-$root/app/src-tauri/target}"
bundle="$target/release/bundle"
[ -d "$bundle" ] || { echo "no bundle directory at $bundle" >&2; exit 1; }

if [ "$(uname -s)" = Darwin ] && [ -n "${APPLE_SIGNING_IDENTITY:-}" ]; then
  for dmg in "$bundle"/dmg/*.dmg; do
    [ -f "$dmg" ] || continue
    codesign --force --timestamp --sign "$APPLE_SIGNING_IDENTITY" "$dmg"
    if [ -n "$notarize" ]; then
      # Tauri notarizes and staples the .app itself; the .dmg is submitted here.
      if [ "$notarize" = key ]; then
        xcrun notarytool submit "$dmg" --wait --key "$APPLE_API_KEY_PATH" --key-id "$APPLE_API_KEY" \
          --issuer "$APPLE_API_ISSUER" >/dev/null
      else
        xcrun notarytool submit "$dmg" --wait --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" \
          --team-id "$APPLE_TEAM_ID" >/dev/null
      fi
      xcrun stapler staple "$dmg"
    fi
  done
fi

echo "bundles:"
find "$bundle" -mindepth 2 -maxdepth 2 \( -name '*.app' -o -name '*.dmg' -o -name '*.msi' -o -name '*.exe' \
  -o -name '*.deb' -o -name '*.rpm' -o -name '*.AppImage' \) -print
