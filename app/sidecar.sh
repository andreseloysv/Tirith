#!/bin/sh
# The daemon the bundle carries: builds tirith in release and copies it to
# src-tauri/binaries/tirith-<target triple>, where bundle.externalBin looks for it. On macOS it is universal.
# `tauri build` runs it first (beforeBuildCommand) and names the target in TAURI_ENV_TARGET_TRIPLE.
# Each build has its own --target, so target/release/tirith is never touched.
set -eu

cd "$(dirname "$0")/.."
target="${CARGO_TARGET_DIR:-target}"
triple="${TAURI_ENV_TARGET_TRIPLE:-$(rustc -vV | sed -n 's/^host: //p')}"
out="app/src-tauri/binaries/tirith-$triple"
mkdir -p app/src-tauri/binaries

case "$triple" in
  *-apple-darwin)
    export MACOSX_DEPLOYMENT_TARGET=10.15 # the app's minimumSystemVersion
    cargo build --release --locked -p tirith --target aarch64-apple-darwin --target x86_64-apple-darwin
    lipo -create -output "$out" \
      "$target/aarch64-apple-darwin/release/tirith" "$target/x86_64-apple-darwin/release/tirith"
    ;;
  *)
    cargo build --release --locked -p tirith --target "$triple"
    cp "$target/$triple/release/tirith" "$out"
    ;;
esac
echo "sidecar: $out"
