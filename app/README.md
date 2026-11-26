# NeoBot desktop app

A window on the local daemon, or on a team server (below). On start it checks `127.0.0.1:<port>` (`TIRITH_PORT`, else `[server] port`
in `~/.tirith/config.toml`, else 7717); when nothing answers it starts `neobot serve` in its own process group
(the daemon outlives the window; log in `~/.tirith/logs/tirithd.log`) and then opens the daemon's page as the owner,
with the one-use link of `neobot open link`.
All state and actions live in the daemon.

## A team server instead of this Mac

*Server*, *Servers...* (Cmd/Ctrl+Shift+S) opens a window where you add the address of a team server (`https://neobot.team.example`
behind a TLS proxy, or `http://127.0.0.1:7801` through an SSH tunnel), choose between *This Mac* and a server, and
*Sign in with Microsoft*. The addresses, and the name and role last seen, are in `team-servers.json` in the app's
config folder; the choice survives a restart. While a server is shown the app starts no daemon and needs none.

Signing in (`app/signin`, crate `tirith-signin`): the server's `GET /api/auth/oidc` names the provider's endpoints, the
client id and the scope; the app opens the provider's page in the system browser (Authorization Code with PKCE S256, a
public client, redirect `http://localhost:<random port>` that the app listens on until the one answer comes, RFC 8252),
trades the code for an access token and hands it to the server's `POST /api/auth/oidc`, which opens a session; the
window then loads the server's one-use link and shows its page, its title saying your name and role. No refresh token is
asked for (no `offline_access`): the access token is used once and dropped, nothing of the provider's is stored, and
when the server's session ends (`session_hours`, 12 by default) you sign in again. Endpoints that are not https (a
loopback provider apart) or not the issuer's host are refused. `cargo test -p tirith-signin` runs the flow against a
fake provider and server on local ports.

The `neobot` binary is looked up next to the app's executable, then in `~/.local/bin`, then on `PATH`.

The macOS and Linux bundles carry it next to the executable (`bundle.externalBin`). `tauri build` runs
`sidecar.sh`, which builds it into `src-tauri/binaries/neobot-<target triple>`; on macOS it is universal, so both
targets are needed: `rustup target add aarch64-apple-darwin x86_64-apple-darwin`. Windows bundles carry none,
because the daemon is Unix-only.

A bundled app links `~/.local/bin/neobot` and `~/.local/bin/tirith` (the alias) to its own binary when there is none, and links again when its old
link points into a bundle that is gone. It never replaces anything else, and makes no link when it runs from
a dmg or other volume, from App Translocation or from an AppImage.

The daemon still reads the page from the `web/` folder of the checkout it was compiled in (`[app] web_dir` in
`config.toml` overrides it). On a computer without that folder the bundled daemon starts but has no page to serve.

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
