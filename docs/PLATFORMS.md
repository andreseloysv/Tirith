# Platforms: agents and Tirith parts on macOS, Windows and Linux

Agents are named by alias (`docs/CONTRACT-hooks.md`, "Agent aliases"). Facts below come from the
official install pages (read 2026-10-29) or from this repo's code; "not checked" means neither says.

## Agent CLIs

| Alias | macOS | Windows | Linux |
| - | - | - | - |
| `anthropic` | yes, 13.0+ (native installer, Homebrew cask) | yes, 10 1809+ / Server 2019+, native (PowerShell, CMD, WinGet) or WSL; Git for Windows optional (without it the agent uses PowerShell as its shell tool); sandboxing not supported natively, supported in WSL 2 | yes: Ubuntu 20.04+, Debian 10+, Alpine 3.19+ (native installer, apt, dnf, apk; Alpine also needs `bash curl libgcc libstdc++ ripgrep`) |
| `codex` | yes (curl script, npm, Homebrew cask) | yes (PowerShell install script; the page does not say whether WSL is needed) | yes (curl script, npm, release binary) |
| `copilot` | yes (Homebrew cask, install script, npm) | yes (WinGet, npm; needs PowerShell v6+; WSL not mentioned) | yes (Homebrew cask, install script, npm; npm needs Node.js 22+) |
| `muse` | yes (installed here, 1.4.14, in `~/.local/bin`) | not checked | not checked |

`muse` has no install page we could cite; only the macOS install is known from this machine. Run
`tirith doctor --agent muse` on each platform; the owner should add its sources here.

## Tirith support per agent

| Alias | Hooks (`tirith hook`) | Shim (`tirith shim install`) | Hook file | Streaming chat (`chat_args`) |
| - | - | - | - | - |
| `anthropic` | yes (own adapter) | yes | its `settings.json` | yes |
| `muse` | yes (generic adapter) | yes | `~/.config/muse/settings.json` | no (see below) |
| `codex` | yes (generic adapter) | yes | `~/.codex/hooks.json` | no |
| `copilot` | yes (generic adapter, flat entries) | yes | `~/.copilot/hooks/tirith.json` | no |

Shims are per command name (`sh` wrapper on macOS and Linux, `.cmd` on Windows), so they do not depend
on the alias.

### `muse serve` and the chat

`muse serve` by default is a stdio host that speaks JSON-RPC 2.0 (a connection must `initialize`
first); `--listen` would open a socket daemon, which Tirith does not use (no local servers or ports).
The desktop chat only speaks the `anthropic` streaming JSON dialect, and `muse` has no flag for it
(`muse exec` is one headless prompt). So `muse` has no `chat_args`: the JSON-RPC methods are not
documented where we could read them and one probe (`initialize` on stdin) printed nothing we could
capture. Blocked until the protocol is known.

Streaming chat (`chat_args`, `docs/CONTRACT-agents.md`) is the same on every platform for `anthropic`:
it is the agent's print mode with streaming JSON on stdin and stdout. It was run end to end on macOS
only (`tools/chat_check.py`).

Sources for `codex` and `copilot` (read 2026-10-29):

- https://github.com/openai/codex
- https://docs.github.com/en/copilot/how-tos/set-up/install-copilot-cli

Sources for `anthropic`:

The anthropic CLI's official docs cover these topics:

- Setup (install and system requirements).
- Headless mode (streaming JSON).
- CLI reference (flags such as `--input-format`, `--permission-prompt-tool`).
- Agent SDK (message and permission types: `SDKUserMessage`, `PermissionResult`).

## Tirith parts

| Part | macOS | Linux | Windows |
| - | - | - | - |
| Collector, `tirith` CLI, web UI (`file://`) | yes (developed here) | expected (plain Python 3.12, `uv`) | partly: see below |
| `tirith lane run`, hooks, usage, diffs | yes | expected | partly |
| Pipeline stages and actions (`/bin/sh -c`) | yes | yes | no native; use WSL |
| `tirith run` locking (`fcntl.flock` in `runstage.py`) | yes | yes | no native (module is Unix-only); use WSL |
| Agent shims (`tirith shim install`) | `sh` wrapper | `sh` wrapper | `.cmd` wrapper (written, untested on a real machine) |
| Commit guard (`sh` git hooks) | yes | yes | via Git for Windows' own `sh` (untested) |
| `tirith://` link handler (`launch.py`, `osacompile`, `Info.plist`) | yes | no | no |
| Desktop app (Tauri) and chat | yes | builds on Linux per Tauri; not run here | builds on Windows per Tauri; not run here; `collector_running()` is Unix-only so the app starts its own collector |
| Stop a lane (SIGTERM, then SIGKILL) | yes | yes | app falls back to kill; CLI signals not checked |

Sources for the tools Tirith builds on:

- Tauri 2 prerequisites per platform: https://v2.tauri.app/start/prerequisites/
- `uv` install: https://docs.astral.sh/uv/getting-started/installation/
- Git for Windows: https://git-scm.com/downloads/win

"Expected" means the code uses nothing platform specific that we found, but it was not run there. The
only platform exercised in this repo is macOS.
