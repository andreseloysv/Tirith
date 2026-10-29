# Tirith desktop app (Tauri 2)

The existing web UI (`../web`) in a native window. No servers and no open ports: the page and Rust
talk through Tauri IPC only. The web UI is not copied; `frontendDist` points at `../../web`.

## What it does

- **State**: Rust reads `~/.tirith/state.json` (or `$TIRITH_HOME`) every 2 s and emits the
  `tirith://state` event. The page detects `window.__TAURI__` and uses the event instead of polling
  `state.js`. In a normal browser nothing changes.
- **Collector**: on start the app runs `uv run --project <collector> tirith collect --interval 2`
  unless a `tirith collect` process already exists, and stops it on exit only if it started it.
  Collector folder: `$TIRITH_COLLECTOR_DIR`, else `[app] collector_dir` in `~/.tirith/config.toml`,
  else this repo's `collector/`. `[app] uv = "<path>"` overrides the `uv` executable.
- **Launch**: command `launch_step(project, step)` runs `tirith launch --project P --step N`
  (the CLI command comes from another lane) and returns its output.
  From the page: `window.__TAURI__.core.invoke('launch_step', { project: 'tirith', step: 9 })`.
- **Chat** (off by default): enable with `TIRITH_CHAT=1` or `[app] chat = true`. A "Chat" button opens
  a panel: pick a project and an agent alias, type a message. Each conversation is a lane: the app
  runs `tirith lane run --interactive --project P --class chat --id chat-<ms> --brief
  ~/.tirith/chats/<id>.md -- <command> <chat_args>` with piped stdin/stdout (the agent inherits them),
  so `lane run` registers the lane, forwards signals and ends it however the agent exits. The app
  writes one streaming JSON line per message to stdin and shows the text streamed back. Lines that are
  not JSON (the `lane run` summary) are not shown. Stop = SIGTERM to `lane run` (Windows: kill).
  The command and arguments are never hardcoded; they come from `config.toml`:

  ```toml
  [agents.<alias>]
  command = "<executable>"
  chat_args = ["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose", "--permission-prompt-tool", "stdio"]
  ```

  The example is the `anthropic` alias (print mode, streaming JSON both ways, permission questions on
  stdio). Each message the app sends is one line:
  `{"type":"user","message":{"role":"user","content":[{"type":"text","text":"..."}]}}`.
  Source: the headless and Agent SDK pages listed in `docs/PLATFORMS.md` (`SDKUserMessage`,
  `PermissionResult`). `tools/chat_check.py` runs this exchange once against the
  real agent (manual, spends a few tokens).

  Only aliases with `chat_args` are offered.
- **Permission questions**: when the agent writes a `control_request` line with subtype
  `can_use_tool` (tool name and input), the panel shows a card with Allow / Deny. The command
  `chat_answer(id, requestId, allow, input)` writes the matching `control_response` line to the agent's
  stdin (allow echoes the input back as `updatedInput`; deny carries a message). The agent only asks
  when `chat_args` turns that mode on; checked against the real `anthropic` agent: the request carries `request_id` and
  `request.{subtype,tool_name,input}`, and this reply shape is accepted (a deny reaches the agent as an
  error tool result).

## Run

Needs Rust, node/npm and `uv`.

```sh
cd app
npm install
npm run tauri dev      # opens the window; hot reload is not needed, the page is plain files
npm run tauri build    # bundles for the current platform
cd src-tauri && cargo test     # permission reply and line classification
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
- `collector_running()` is still Unix-only: Windows always starts its own collector.

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

Nothing is signed until these exist. Unsigned apps run, but macOS Gatekeeper and Windows SmartScreen warn.

macOS (Apple Developer Program, paid):
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

- Resuming chats, tool-call display.
- Opening the window in dev mode and looking at it was not done in this session;
  `cargo check` and `cargo test` pass.
- Auto-update, actual signing, a Windows shim/guard tested on Windows (`scripts/package.ps1` was not
  run on Windows either).
- `muse serve` as a chat agent: the app only speaks the `anthropic` streaming JSON dialect (see Chat).
  Muse's stdin/stdout protocol is not documented where we could read it, so no `chat_args` default
  exists for it; add one only after a real run.
- Mobile: out of this plan. Tauri 2 can target Android and iOS, but nothing here is set up for it, and
  the chat spawns local processes.
- Python as a sidecar.
- Mobile (for later).
- Icon is a simple generated one, not a designed logo.
