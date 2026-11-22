#!/bin/sh
# Signs one macOS binary with the hardened runtime.
#   sh scripts/release/sign-macos.sh <binary>
# By default ad hoc: no identity and no keychain. It runs on any Mac when `install.sh` or curl brought it; one
# downloaded with a browser asks for Gatekeeper's yes. With TIRITH_SIGN_IDENTITY ("Developer ID Application: ...")
# it is signed with that identity, and with TIRITH_NOTARY_PROFILE (notarytool's keychain profile) also notarized.
set -eu
bin=${1:?usage: sign-macos.sh <binary>}
here=$(cd "$(dirname "$0")" && pwd)
id=${TIRITH_SIGN_IDENTITY:--}
profile=${TIRITH_NOTARY_PROFILE:-}

set -- --force --options runtime --identifier app.tirith.cli --entitlements "$here/tirith.entitlements" --sign "$id"
[ "$id" = - ] || set -- "$@" --timestamp
/usr/bin/codesign "$@" "$bin"
/usr/bin/codesign --verify --strict "$bin"

if [ "$id" = - ]; then
  echo "signed ad hoc: $bin"
  exit 0
fi
echo "signed as $id: $bin"
[ -n "$profile" ] || exit 0
work=$(mktemp -d "${TMPDIR:-/tmp}/tirith-notary.XXXXXX")
trap 'rm -rf -- "${work:?}"' EXIT
/usr/bin/ditto -c -k "$bin" "$work/tirith.zip"
out=$(xcrun notarytool submit "$work/tirith.zip" --keychain-profile "$profile" --wait --output-format json)
case "$out" in
  *'"status":"Accepted"'* | *'"status": "Accepted"'*) echo "notarized: $bin" ;;
  *) echo "notarization failed: $out" >&2; exit 1 ;;
esac
