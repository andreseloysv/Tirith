#!/bin/sh
# Installs NeoBot on Linux or macOS (x86_64, arm64) and starts its guide, which connects the agents.
#   curl -fsSL https://github.com/andreseloysv/Tirith/releases/latest/download/install.sh | sh -s -- --server
#   sh install.sh [--from <folder with a release>] [--no-setup] [options of `neobot setup`]
# `tirith`, the old name of the command, is a link to `neobot`. As root on Linux it makes the user `tirith`: agents never run as root. TIRITH_BASE_URL: another place to download.
set -eu
KEY='-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEEMqh12EKmR6Eeh0WfAy2RUc8O2XK
0vydFEXZCLAwXgH8fRKBIyFIiZF8f2ZchMGUb52KgAT+Tz/T4NZXyF9Tbg==
-----END PUBLIC KEY-----'
BASE=${TIRITH_BASE_URL:-https://github.com/andreseloysv/Tirith/releases/latest/download}
die() { echo "install: $*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

target() {
  os=$(uname -s) arch=$(uname -m) # (under Rosetta a Mac says x86_64)
  [ "$os" != Darwin ] || [ "$(sysctl -n hw.optional.arm64 2>/dev/null || echo 0)" != 1 ] || arch=arm64
  case "$arch" in arm64 | aarch64) arch=aarch64 ;; x86_64 | amd64) arch=x86_64 ;; *) die "no build for $arch" ;; esac
  case "$os" in
    Darwin) echo "$arch-apple-darwin" ;;
    Linux) echo "$arch-unknown-linux-musl" ;;
    *) die "no build for $os" ;;
  esac
}

tools() {
  miss='' need='curl tar openssl'
  [ "$(id -u)$(uname -s)" != 0Linux ] || need="$need git bash runuser" # what the guide would ask sudo for, and what leaves root
  for t in $need; do have "$t" || miss="$miss $t"; done
  [ -n "$miss" ] || return 0
  if have apt-get; then how='apt-get install -y -q ca-certificates'
  elif have dnf; then how='dnf install -y -q ca-certificates'
  elif have apk; then how='apk add ca-certificates npm' # Copilot's official installer for musl is npm's
  else die "install$miss first"; fi
  have apk || miss=$(echo "$miss" | sed 's/runuser/util-linux/') # its package elsewhere
  [ "$(id -u)" = 0 ] || die "install$miss first: sudo $how$miss"
  echo "Installing$miss..."
  if have apt-get; then apt-get update -q >/dev/null; fi
  $how $miss >/dev/null # words on purpose
}

fetch() {
  if [ -n "$from" ]; then cp "$from/$1" "$tmp/$1"; else curl -fsSL -m 600 -o "$tmp/$1" "$BASE/$1"; fi ||
    die "cannot get $1"
}

# The files come only if SHA256SUMS is signed with NeoBot's release key and names their sum.
verified() {
  file="neobot-$(target).tar.gz"
  for f in "$file" SHA256SUMS SHA256SUMS.sig; do fetch "$f"; done
  printf '%s\n' "$KEY" > "$tmp/key"
  openssl dgst -sha256 -verify "$tmp/key" -signature "$tmp/SHA256SUMS.sig" "$tmp/SHA256SUMS" >/dev/null 2>&1 ||
    die "SHA256SUMS is not signed with NeoBot's release key: nothing was installed"
  want=$(awk -v f="$file" '$2 == f { print $1 }' "$tmp/SHA256SUMS")
  got=$(openssl dgst -sha256 "$tmp/$file" | sed 's/^.*= *//')
  [ -n "$want" ] && [ "$want" = "$got" ] || die "$file does not match its signed sum: nothing was installed"
  tar -xzf "$tmp/$file" -C "$tmp" neobot
  echo "Signature and sum are right."
}

service_user() {
  id tirith >/dev/null 2>&1 || { have useradd && useradd --create-home --shell /bin/sh tirith; } ||
    adduser -D -s /bin/sh tirith || die "cannot make the user tirith"
  have loginctl && loginctl enable-linger tirith 2>/dev/null || true # the guide waits for its systemd
  [ "$(cat /proc/sys/kernel/apparmor_restrict_unprivileged_userns 2>/dev/null || echo 0)" != 1 ] ||
    "$bin/neobot" net allow || echo "install: the network sandbox needs: $bin/neobot net allow" >&2
}

main() {
  from='' setup=1 n=$#
  while [ "$n" -gt 0 ]; do
    a=$1; shift; n=$((n - 1))
    case "$a" in
      --from) [ "$n" -gt 0 ] || die "--from needs a folder"; from=$1; shift; n=$((n - 1)) ;;
      --no-setup) setup=0 ;;
      *) set -- "$@" "$a" ;;
    esac
  done
  tools
  tmp=$(mktemp -d "${TMPDIR:-/tmp}/neobot-install.XXXXXX")
  trap 'rm -rf -- "${tmp:?}"' EXIT
  verified
  asroot=0 bin="$HOME/.local/bin"
  if [ "$(id -u)" = 0 ] && [ "$(uname -s)" = Linux ]; then asroot=1 bin=/usr/local/bin; fi
  mkdir -p "$bin"
  cp "$tmp/neobot" "$bin/.neobot.new" && chmod 755 "$bin/.neobot.new" && mv -f "$bin/.neobot.new" "$bin/neobot"
  ln -sf neobot "$bin/tirith"; echo "Installed $("$bin/neobot" --version) in $bin (also as tirith)"
  set -- "$bin/neobot" setup "$@"
  if [ "$asroot" = 1 ]; then
    service_user
    home=$(awk -F: '$1 == "tirith" { print $6 }' /etc/passwd)
    set -- runuser -u tirith -- env HOME="$home" USER=tirith LOGNAME=tirith TIRITH_VIA="${SUDO_USER:-root}" \
      XDG_RUNTIME_DIR="/run/user/$(id -u tirith)" "$@"
    cd "$home"
  fi
  [ "$setup" = 1 ] || { echo "Next: $*"; return 0; }
  # `curl | sh` took stdin: the guide's questions come from the terminal.
  if ( : </dev/tty ) 2>/dev/null; then "$@" </dev/tty; else "$@" --yes </dev/null; fi
}
main "$@"
