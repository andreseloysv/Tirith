# Hooks contract: `tirith hook`

Agent CLIs run a command on lifecycle events with a JSON payload on stdin. One command,
`tirith hook --agent <name> [--event <name>]`, turns each call into a line of
`~/.tirith/events.jsonl` (`TIRITH_HOME` wins). It is fast and never breaks the agent: append only, no
network, every exception caught, exit 0. `PreToolUse` can block (exit 2): see `docs/CONTRACT-gates.md`.

```json
{"kind": "hook", "ts": "2026-10-28T10:00:00Z", "agent": "anthropic", "event": "PreToolUse",
 "lane": "w1-data", "session": "s-1", "project": "chromasol", "tool": "Edit",
 "summary": "Edit web/js/ui.js", "agent_session_id": "…", "cwd": "/abs/dir"}
```

- `lane`: env `TIRITH_LANE` or `null`; `session`: env `TIRITH_SESSION` / current session;
  `project`: longest-prefix match of `cwd` against the project folders, or `null`.
- `summary`: `Edit <path>`, `Bash: <first line>`, the prompt's first line,
  `Sub-agent <type>: <description>` (tools `Task` and `Agent`), the last assistant message for
  `Stop` / `SubagentStop`.
- Pipeline code ignores rows with `kind: "hook"`; hook rows have no `run`.
- `UserPromptSubmit` also writes `~/.tirith/requests/<agent_session_id>.json`:
  `{"agent_session_id", "prompt", "at"}` (the owner's last request, for lanes to link to).

## Adapters

`anthropic` (Anthropic's coding CLI): events `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `SubagentStop`, `Stop`
(payload `hook_event_name`, `session_id`, `cwd`, `tool_name`, `tool_input`, `prompt`,
`last_assistant_message`). `muse` and unknown agents use the generic adapter, which reads common field
names (`hook_event_name`/`event`, `session_id`/`sessionId`, `tool_name`/`tool`, `tool_input`/`input`,
`prompt`…). Muse Code's docs list the same event names but publish no payload fields or hook format.

## Install

`tirith hook install|uninstall --agent anthropic|muse [--settings <path>] [--tirith <exe>]` merges
entries into the agent's user-level settings (the path comes from `[agents.<alias>] settings` in
`~/.tirith/config.toml`; Muse defaults to `~/.config/muse/settings.json`),
keeping every other hook; idempotent. Uninstall only removes entries whose command contains
`hook --agent <name>`. The Muse entry layout assumes the same `hooks` block layout as the anthropic CLI (unverified).

`tirith hook verify --agent <alias>` checks that all hook entries are in the settings file: prints
`Hooks protected · last check <n> s ago` (the collector's last check) and exits 0, or says what is
missing and exits 1. The collector does the same every cycle and reinstalls removed hooks, see
`docs/CONTRACT-gates.md` ("Hooks stay installed"); editing the settings file is blocked by the
`settings-edit` rule.

## In `state.json`

`agents[].events`: the last 50 hook events of that lane (newest last). `loose_events`: the last 50
events whose `lane` is null or unknown. Shape: `ts, agent, event, lane, session, project, tool, summary,
agent_session_id`.

## Agent aliases

Code and docs name agents by alias (`anthropic`, `muse`, …). The real executable and settings path of
each alias live only in the owner's `~/.tirith/config.toml`, written at setup:

```toml
[agents.anthropic]
command = "<executable name>"
settings = "<path to its user settings.json>"
```
