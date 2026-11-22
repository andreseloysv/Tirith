#!/bin/sh
# Installs the latest released tirith for Linux (x86_64 or aarch64) in ~/.local/bin, after checking its SHA-256.
#
#   curl -fsSL https://raw.githubusercontent.com/andreseloysv/Tirith/main/scripts/install.sh | sh
#
# Running it again updates. Needs curl or wget. The binaries come from .github/workflows/cli.yml.
set -eu

releases=https://github.com/andreseloysv/Tirith/releases
tmp=

fail() {
  echo "install.sh: $*" >&2
  exit 1
}

cleanup() {
  [ -z "$tmp" ] || rm -rf "$tmp"
}

fetch() {
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL --proto '=https' --retry 3 -o "$2" "$1"
  else
    wget -q -O "$2" "$1"
  fi
}

# Everything runs from main, called on the last line, so a download cut short runs nothing.
main() {
  os=$(uname -s)
  [ "$os" = Linux ] || fail "there are binaries only for Linux, not $os (macOS has the app: $releases/latest)"

  machine=$(uname -m)
  case "$machine" in
    x86_64 | amd64) arch=x86_64 ;;
    aarch64 | arm64) arch=aarch64 ;;
    *) fail "no binary for $machine, only x86_64 and aarch64" ;;
  esac

  command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1 || fail "curl or wget is needed"
  command -v sha256sum >/dev/null 2>&1 || fail "sha256sum is needed to check the download"

  bin="${HOME:?is not set}/.local/bin"
  [ ! -d "$bin/tirith" ] || fail "$bin/tirith is a folder"
  mkdir -p "$bin"
  # Next to its destination, so the final mv is atomic and works while tirith runs.
  tmp=$(mktemp -d "$bin/.tirith-install.XXXXXX")
  trap cleanup EXIT
  trap 'exit 1' HUP INT TERM

  asset=tirith-linux-$arch
  url=$releases/latest/download/$asset
  echo "Downloading $asset from the latest Release"
  fetch "$url" "$tmp/tirith" || fail "could not download $url"
  fetch "$url.sha256" "$tmp/sum" || fail "could not download $url.sha256"

  want=$(cut -d ' ' -f 1 "$tmp/sum")
  got=$(sha256sum "$tmp/tirith" | cut -d ' ' -f 1)
  [ "$want" = "$got" ] || fail "the SHA-256 of $asset does not match, nothing was installed"

  chmod 755 "$tmp/tirith"
  mv -f "$tmp/tirith" "$bin/tirith"
  echo "Installed $bin/tirith"

  case ":${PATH:-}:" in
    *":$bin:"*) ;;
    *) echo "$bin is not on your PATH, add it: export PATH=\"\$HOME/.local/bin:\$PATH\"" ;;
  esac
}

main "$@"
