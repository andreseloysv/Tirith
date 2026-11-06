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

## Manual check (needs a graphical session)

Not automatable; run on a Mac/Windows/Linux desktop:

1. `cd app && npm ci && npm run tauri dev`.
2. Chat: attach a file with the paperclip, then drag an image and a file onto the chat; both appear as attachments
   (`~/.tirith/logs/app.log` shows `over chat: true` for drops).
3. Send long text, a long path and a long code block: nothing is cut off and the page has no horizontal scrollbar.

Note: the `web/` folder and its `chatdrop`/`attach`/`chatoverflow` tests no longer exist; the app now shows the
daemon's page (`rust/tirith/ui/`), so there is no static test left to cover this.
