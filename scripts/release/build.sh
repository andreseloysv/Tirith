#!/bin/sh
# One release binary of tirith for one target, from a Mac or Linux: nothing needed but rustup and clang.
#   sh scripts/release/build.sh <target> [out-file]
# Targets: aarch64-apple-darwin, x86_64-apple-darwin, aarch64-unknown-linux-musl, x86_64-unknown-linux-musl.
# Linux binaries are static (no glibc, sqlite or OpenSSL of the machine). Cache: $TIRITH_RELEASE_CACHE.
# TIRITH_RELEASE names the version the binary says; TIRITH_AGENTS_FILE adds what `tirith setup` may offer.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
target=${1:?usage: build.sh <target> [out-file]}
cache=${TIRITH_RELEASE_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}/tirith-release}
build=${CARGO_TARGET_DIR:-$root/target}
host=$(rustc -vV | sed -n 's/^host: //p')

export CARGO_TARGET_DIR="$build" TIRITH_DIST=1
rustup target add "$target" >/dev/null

# No folder of the machine that built it ends up in the binary.
remap="--remap-path-prefix=$HOME=/builder --remap-path-prefix=$build=/build --remap-path-prefix=$root=/tirith"
maps="-ffile-prefix-map=$HOME=/builder -ffile-prefix-map=$build=/build -ffile-prefix-map=$root=/tirith"
flags="$remap -C strip=symbols"
var=$(echo "$target" | tr - _)
VAR=$(echo "$var" | tr '[:lower:]' '[:upper:]')

case "$target" in
  *-unknown-linux-musl)
    arch=${target%%-*}
    command -v clang >/dev/null || { echo "clang not found" >&2; exit 1; }
    rustup component add llvm-tools >/dev/null
    sh "$root/scripts/release/musl-headers.sh" "$cache/musl"
    export "CC_$var=$(command -v clang)"
    export "AR_$var=$(rustc --print sysroot)/lib/rustlib/$host/bin/llvm-ar"
    export "CFLAGS_$var=--target=$target -nostdlibinc -isystem $cache/musl/$arch/include $maps"
    export "CARGO_TARGET_${VAR}_LINKER=rust-lld"
    flags="$flags -C linker-flavor=ld.lld -C link-self-contained=yes"
    ;;
  *-apple-darwin)
    [ "$(uname -s)" = Darwin ] || { echo "$target is built on a Mac" >&2; exit 1; }
    export MACOSX_DEPLOYMENT_TARGET=11.0 "CFLAGS_$var=$maps"
    ;;
  *) echo "unknown target: $target" >&2; exit 1 ;;
esac
export "CARGO_TARGET_${VAR}_RUSTFLAGS=$flags"

cargo build --release --locked -p tirith --target "$target" --manifest-path "$root/Cargo.toml" >&2
bin="$build/$target/release/tirith"
if [ $# -ge 2 ]; then
  mkdir -p "$(dirname "$2")"
  cp "$bin" "$2"
  bin=$2
fi
echo "$bin"
