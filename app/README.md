# Tirith desktop app (Tauri 2)

The existing web UI (`../web`) in a native window. No servers and no open ports: the page and Rust
talk through Tauri IPC only. The web UI is not copied; `frontendDist` points at `../../web`.

## Live web (no rebuild)

The window can show the current files of `web/` instead of the copy embedded at build time, so a merged
web fix is visible without rebuilding the app.

- A custom URI scheme, `tirithweb://localhost/index.html` (Windows: `http://tirithweb.localhost/`), is
  answered by Rust from disk (`src/liveweb.rs`). No HTTP server, no port. Paths are decoded; `..`,
  backslashes, NUL and symlinks leading outside the folder are refused (403/400). MIME types come from the
  extension; responses are `no-store`.
- Folder: `[app] web_dir = "<path>"` in `~/.tirith/config.toml`. Without it, debug builds
  (`npm run tauri dev`) use this repo's `web/`; release builds keep the embedded page. With `web_dir`
  set, a release build uses the live folder too (it needs an `index.html`, else the embedded page is used).
- Cmd/Ctrl+R reloads the files (the URL hash stays, so the same view returns). Rust checks the folder every
  2 s (names, sizes, mtimes) and emits `tirith://web-changed`; the page then shows a "New version of the
  page · Reload" pill (`web/js/live.js`).
- `capabilities/default.json` lists the scheme as a remote origin so that page can use the app's commands.

## What it does

- **State**: Rust reads `~/.tirith/state.json` (or `$TIRITH_HOME`) every 2 s and emits the
  `tirith://state` event. The page detects `window.__TAURI__` and uses the event instead of polling
  `state.js`. In a normal browser nothing changes.
- **Collector**: on start the app runs `uv run --project <collector> tirith collect --interval 2`
  unless a collector holds `<TIRITH_HOME>/collector.lock`, and stops it on exit only if it started it.
  Collector folder: `$TIRITH_COLLECTOR_DIR`, else `[app] collector_dir` in `~/.tirith/config.toml`,
  else this repo's `collector/`. `[app] uv = "<path>"` overrides the `uv` executable.
- **Launch**: command `launch_step(project, step)` runs `tirith launch --project P --step N`
  (the CLI command comes from another lane) and returns its output.
  From the page: `window.__TAURI__.core.invoke('launch_step', { project: 'tirith', step: 9 })`.
- **Manager session**: the one manager is a session the collector owns (`tirith manager ...`,
  `docs/CONTRACT-inbox.md`, "One manager"); the chat in the page shows it as it works. The app only sends
  controls: command `manager_control(op)` with `interrupt` (Stop the running turn), `stop` or `restart`
  runs `tirith manager <op>`. The old app-only agent chat (`chat_start` and friends, `web/js/v-chat.js`)
  is gone: it ran a second, separate session; its permission questions are now Allow / Deny items in the
  manager chat. The streaming arguments still come from `[agents.<alias>] chat_args`
  (`docs/CONTRACT-agents.md`); `tools/chat_check.py` runs that exchange once by hand.

## Run

Needs Rust, node/npm and `uv`.

```sh
cd app
npm install
npm run tauri dev      # opens the window; hot reload is not needed, the page is plain files
npm run tauri build    # bundles for the current platform
cd src-tauri && cargo test     # permission reply, line classification, live web paths and MIME types
```

## Agents per platform

Aliases the code knows (`agentrun.PRESET_DEFAULTS`): `anthropic`, `muse`. Which of them has a build
for Windows or Linux is listed in `docs/PLATFORMS.md` (`muse` is not checked). Check with
`tirith doctor --agent <alias>` on each platform. Nothing in the app assumes one.

## Windows

- Agent shims: `tirith shim install` writes `<name>.cmd` (marker `REM tirith-shim v1`) instead of an
  `sh` script. A batch file cannot tell a terminal from a pipe, so it runs the real binary only for a
  lane token that matches the lane file (use `tirith lane run --interactive` to chat from a terminal).
  See `docs/CONTRACT-gates.md`. Untested on a real Windows machine.
- Commit guard: `tirith_guard` renders the hooks for Git for Windows' own `sh` (LF, read-only attribute),
  and `tirith safegit install` writes `git.cmd` plus a Python core that refuses the flags that skip
  hooks (`docs/CONTRACT-gates.md`, "Windows"). Both are simulated in tests, not run on Windows.
- `collector_running()` asks PowerShell (`Get-CimInstance Win32_Process`) for a `tirith collect` command
  line; if that fails the app starts its own collector. Not tried on Windows.

## Packaging

```sh
scripts/package.sh            # from the repo root: release build for this platform, prints the bundle paths
scripts\package.ps1           # the same on Windows (PowerShell); prints the .msi and .exe paths
```

It runs `npm run tauri build` in `app/` (installing `node_modules` if missing) and lists what it made under
`app/src-tauri/target/release/bundle/`: macOS `.app` + `.dmg`, Windows `.msi` + NSIS `.exe`, Linux `.deb`,
`.rpm` + `.AppImage`. Each platform builds only its own bundles, so build on that OS.
Linux needs `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`.

CI: `.github/workflows/app.yml` builds the three platforms on `v*` tags (or by hand) and uploads the
bundles as artifacts (`tirith-macOS`, `tirith-Windows`, `tirith-Linux`). They are **unsigned**.

Icons: `src-tauri/icons/` is generated from `scripts/make_icon.py` (needs Pillow) with
`npm run tauri icon <1024px.png>`; remove the `android/` and `ios/` folders it creates.

Python is not bundled as a sidecar, so every platform needs `uv` installed.

Built here: macOS, unsigned, `Tirith.app` 19.5 MiB and `Tirith_0.1.0_aarch64.dmg` 12.3 MiB. The
window was not opened, and the Windows and Linux bundles were not built.

## Signing: what the owner must provide

Unsigned apps run, but macOS Gatekeeper and Windows SmartScreen warn.

macOS: `scripts/package.sh` signs with hardened runtime and `src-tauri/entitlements.plist` (JIT and
unsigned executable memory for the web view, no library validation because the app starts `uv` and agent
executables). The identity is `$APPLE_SIGNING_IDENTITY`; if unset, the script takes the first
"Developer ID Application" identity in the keychain and prints which one it uses. The name is never
stored in the repo. The `.dmg` is signed too. Check a build with:

```sh
codesign --verify --deep --strict --verbose=2 <path>/Tirith.app
spctl --assess --type execute --verbose=2 <path>/Tirith.app   # rejected until notarized
```

Notarization is off until one full set of credentials is in the environment; then the script notarizes
and staples the `.app` (Tauri does it) and the `.dmg` (the script does it):
- App Store Connect API key: `APPLE_API_KEY_PATH` (the `.p8` file), `APPLE_API_KEY` (key id), `APPLE_API_ISSUER`
- or Apple ID: `APPLE_ID`, `APPLE_PASSWORD` (app-specific password), `APPLE_TEAM_ID`

One command builds, signs, notarizes and staples both the `.app` and the `.dmg` using a `notarytool`
keychain profile (default `tirith-notary`, override with `TIRITH_NOTARY_PROFILE`):

```sh
scripts/notarize-macos.sh
```

CI (`app.yml`) signs the macOS job when the secret `APPLE_SIGNING_IDENTITY` exists and imports the
certificate from `APPLE_CERTIFICATE` (base64 `.p12`) and `APPLE_CERTIFICATE_PASSWORD`. For notarization add
`APPLE_API_KEY`, `APPLE_API_ISSUER` and `APPLE_API_KEY_CONTENT` (the `.p8` text), or `APPLE_ID`,
`APPLE_PASSWORD`, `APPLE_TEAM_ID`.

macOS checklist (Apple Developer Program, paid):
- [ ] "Developer ID Application" certificate, exported as `.p12` with its password
- [ ] `APPLE_SIGNING_IDENTITY` (name of that certificate, e.g. `Developer ID Application: Name (TEAMID)`)
- [ ] Notarization credentials, either Apple ID: `APPLE_ID`, `APPLE_PASSWORD` (app-specific password),
      `APPLE_TEAM_ID`; or an App Store Connect API key: `APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH`
- [ ] For CI: `APPLE_CERTIFICATE` (base64 of the `.p12`), `APPLE_CERTIFICATE_PASSWORD`, and the variables
      above as repository secrets; then pass them as `env` in the Build step of `app.yml`

Windows:
- [ ] Code-signing certificate (OV, or EV to avoid SmartScreen warnings at once) as `.pfx` with its password
- [ ] Its SHA-1 thumbprint for `bundle.windows.certificateThumbprint` in `tauri.conf.json`, and a timestamp
      server URL for `timestampUrl`
- [ ] For CI: import the `.pfx` into the runner's certificate store from secrets before the Build step

Linux: signing is optional; nothing needed.

## Not done

- Loading `tirithweb://` in a real window was not tried (no GUI session); the handler is covered by
  unit tests only. Opening the window in dev mode and looking at it was not done in this session;
  `cargo check` and `cargo test` pass.
- Auto-update, actual signing, a Windows shim/guard tested on Windows (`scripts/package.ps1` was not
  run on Windows either).
- `muse serve` as a chat agent: the app only speaks the `anthropic` streaming JSON dialect (see Chat).
  `muse serve` speaks JSON-RPC 2.0 over stdio (or a socket with `--listen`, not used: no ports), not that
  dialect; its methods are not documented where we could read them, so no `chat_args` exists for it
  (docs/PLATFORMS.md).
- Mobile: out of this plan. Tauri 2 can target Android and iOS, but nothing here is set up for it, and
  the chat spawns local processes.
- Python as a sidecar.
- Mobile (for later).
- Icon is a simple generated one, not a designed logo.
