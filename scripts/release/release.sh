#!/bin/sh
# A whole signed release, built on a Mac: the four binaries, their tarballs, SHA256SUMS signed with Tirith's
# release key, and the installer that checks both. Nothing is uploaded.
#   sh scripts/release/release.sh <version> [out-folder]
# TIRITH_RELEASE_KEY    the P-256 private key (default ~/.config/tirith-release/signing.key)
# TIRITH_AGENTS_FILE    what the guide may offer beyond the repository's catalog (default beside the key)
# TIRITH_SIGN_IDENTITY, TIRITH_NOTARY_PROFILE   see sign-macos.sh
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
version=${1:?usage: release.sh <version> [out-folder]}
out=${2:-$root/target/dist/$version}
conf=$HOME/.config/tirith-release
key=${TIRITH_RELEASE_KEY:-$conf/signing.key}
[ "$(uname -s)" = Darwin ] || { echo "a whole release is built on a Mac (the macOS targets need it)" >&2; exit 1; }
[ -f "$key" ] || { echo "no release key at $key" >&2; exit 1; }

agents=${TIRITH_AGENTS_FILE:-}
if [ -z "$agents" ] && [ -f "$conf/agents.toml" ]; then agents=$conf/agents.toml; fi
[ -n "$agents" ] || echo "no agents file: the guide will ask for the Anthropic CLI's command" >&2
export TIRITH_AGENTS_FILE="$agents" TIRITH_RELEASE="$version"

ssl=/usr/bin/openssl
work=$(mktemp -d "${TMPDIR:-/tmp}/tirith-release.XXXXXX")
trap 'rm -rf -- "${work:?}"' EXIT
$ssl ec -in "$key" -pubout 2>/dev/null > "$work/pub"
cmp -s "$work/pub" "$root/scripts/release/release.pub" ||
  { echo "this key is not the one install.sh trusts (scripts/release/release.pub)" >&2; exit 1; }
grep -q -F "$(sed -n 2p "$work/pub")" "$root/scripts/install.sh" ||
  { echo "scripts/install.sh carries another public key" >&2; exit 1; }

mkdir -p "$out"
: > "$work/SHA256SUMS"
for target in aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-musl x86_64-unknown-linux-musl; do
  echo "== $target"
  dir="$work/$target"
  mkdir "$dir"
  sh "$root/scripts/release/build.sh" "$target" "$dir/tirith" >/dev/null
  chmod 755 "$dir/tirith"
  cp "$root/LICENSE" "$dir/LICENSE"
  set -- tirith LICENSE
  case "$target" in
    *-apple-darwin) sh "$root/scripts/release/sign-macos.sh" "$dir/tirith" ;;
    *)
      cp "${TIRITH_RELEASE_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}/tirith-release}/musl/COPYRIGHT.musl" "$dir/"
      set -- "$@" COPYRIGHT.musl
      ;;
  esac
  file="tirith-$target.tar.gz"
  # no name of the machine or of who built it in the tarball
  (cd "$dir" && COPYFILE_DISABLE=1 /usr/bin/tar --no-xattrs --uid 0 --gid 0 --uname '' --gname '' \
    -czf "$out/$file" "$@")
  echo "$($ssl dgst -sha256 "$out/$file" | sed 's/^.*= *//')  $file" >> "$work/SHA256SUMS"
done

cp "$work/SHA256SUMS" "$out/SHA256SUMS"
$ssl dgst -sha256 -sign "$key" -out "$out/SHA256SUMS.sig" "$out/SHA256SUMS"
$ssl dgst -sha256 -verify "$work/pub" -signature "$out/SHA256SUMS.sig" "$out/SHA256SUMS"
cp "$root/scripts/install.sh" "$out/install.sh"
cp "$root/scripts/release/release.pub" "$out/release.pub"
echo "release $version in $out"
