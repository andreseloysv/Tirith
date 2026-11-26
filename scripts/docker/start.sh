#!/bin/sh
# Container entry: a forwarder for the page (NeoBot's page listens on 127.0.0.1 only: publish 7802 on the host's
# 127.0.0.1 and reach it through an SSH tunnel), then the daemon as the user tirith, as the main process.
set -eu
CFG=/home/tirith/.tirith/config.toml
PORT=$(sed -n 's/^port *= *\([0-9][0-9]*\).*/\1/p' "$CFG" 2>/dev/null | head -1)
PORT=${PORT:-7801}
mkdir -p /run/user/1001 && chown tirith /run/user/1001 && chmod 700 /run/user/1001
socat "TCP-LISTEN:7802,fork,reuseaddr,bind=0.0.0.0" "TCP:127.0.0.1:$PORT" &
exec runuser -u tirith -- env HOME=/home/tirith USER=tirith LOGNAME=tirith XDG_RUNTIME_DIR=/run/user/1001 neobot serve
