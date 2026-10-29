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
- **Chat** (first version, off by default): enable with `TIRITH_CHAT=1` or
  `[app] chat = true`. A "Chat" button opens a panel: pick a project and an agent alias, type a
  message. The app starts the alias's headless process in the project's repo, writes one streaming
  JSON line per message to its stdin and shows the text it streams back. The session is registered
  as a lane (`tirith lane start --class chat`) and ended when the process exits. The command and
  arguments are never hardcoded; they come from `config.toml`:

  ```toml
  [agents.<alias>]
  command = "<executable>"
  chat_args = ["<args that select the streaming JSON input/output mode>"]
  ```

  Only aliases with `chat_args` are offered.

## Run

Needs Rust, node/npm and `uv`.

```sh
cd app
npm install
npm run tauri dev      # opens the window; hot reload is not needed, the page is plain files
npm run tauri build    # bundles for the current platform
cd src-tauri && cargo check
```

## Not done

- Permission questions from the agent in the UI, resuming chats, tool-call display.
- Signing and notarization, per-platform packaging, auto-update.
- Windows: the git wrapper and the guard have no Windows version; the collector check
  (`pgrep`) is Unix-only, so Windows always starts its own collector.
- Python is not bundled as a sidecar: `uv` must be installed.
- Mobile.
- Icon is a placeholder (`src-tauri/icons/icon.png`); run `npm run tauri icon <image>` for the full set.
