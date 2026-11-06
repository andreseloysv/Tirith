# Tirith desktop app

A window on the local daemon. On start it checks `127.0.0.1:<port>` (`TIRITH_PORT`, else `[server] port`
in `~/.tirith/config.toml`, else 7717); when nothing answers it starts `tirith serve` in its own process group
(the daemon outlives the window; log in `~/.tirith/logs/tirithd.log`) and then opens the daemon's page.
All state and actions live in the daemon.

The `tirith` binary is looked up next to the app's executable, then in `~/.local/bin`, then on `PATH`.

```bash
cargo check -p tirith-app     # compile check
cd app && npm ci && npm run tauri build
```

Signing and notarization on macOS: `scripts/package.sh` and `scripts/notarize-macos.sh`.
