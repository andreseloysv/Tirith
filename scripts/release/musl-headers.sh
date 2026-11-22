#!/bin/sh
# The C headers of musl for both Linux targets, in <dir>/{aarch64,x86_64}: all that sqlite and ring need to
# compile for Linux with clang (the libc itself comes with Rust's musl targets). Text only: the pinned release
# is checked by its sha256 and nothing of it is run but its header substitutions.
#   sh scripts/release/musl-headers.sh <dir>
set -eu
VERSION=1.2.5
SHA256=a9a118bbe84d8764da0ea0d28b3ab3fae8477fc7e4085d90102b8596fc7c75e4
out=${1:?usage: musl-headers.sh <dir>}
[ ! -f "$out/.musl-$VERSION" ] || exit 0

work=$(mktemp -d "${TMPDIR:-/tmp}/tirith-musl.XXXXXX")
trap 'rm -rf -- "${work:?}"' EXIT
mkdir "$work/src"
curl -fsSL -m 300 -o "$work/musl.tar.gz" "https://musl.libc.org/releases/musl-$VERSION.tar.gz"
if command -v sha256sum >/dev/null 2>&1; then
  sum=$(sha256sum "$work/musl.tar.gz")
else
  sum=$(shasum -a 256 "$work/musl.tar.gz")
fi
[ "${sum%% *}" = "$SHA256" ] || { echo "musl $VERSION: wrong checksum" >&2; exit 1; }
tar -xzf "$work/musl.tar.gz" -C "$work/src"
src="$work/src/musl-$VERSION"

for arch in aarch64 x86_64; do
  inc="$out/$arch/include"
  mkdir -p "$inc/bits"
  cp -R "$src/include/." "$inc/"
  cp "$src"/arch/generic/bits/*.h "$inc/bits/"
  cp "$src/arch/$arch/bits/"*.h "$inc/bits/"
  sed -f "$src/tools/mkalltypes.sed" "$src/arch/$arch/bits/alltypes.h.in" "$src/include/alltypes.h.in" \
    > "$inc/bits/alltypes.h"
  calls="$src/arch/$arch/bits/syscall.h.in"
  { cat "$calls"; sed -n -e 's/__NR_/SYS_/p' < "$calls"; } > "$inc/bits/syscall.h"
done
cp "$src/COPYRIGHT" "$out/COPYRIGHT.musl"
: > "$out/.musl-$VERSION"
echo "musl $VERSION headers in $out" >&2
