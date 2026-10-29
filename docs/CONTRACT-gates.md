# Gates contract: rules as code

No rule lives in a markdown file. Every rule below is code that blocks, with a message that tells the
agent what to do instead. Agents are named by alias (`anthropic`, `muse`); the real executable of each
alias is `[agents.<alias>] command` in the owner's `~/.tirith/config.toml`.

Message format, one block per violation:
`tirith: <rule-id>: <what was blocked>. Do this instead: <exact command>`

## Rules

| Rule | Blocks | Enforced in | Do this instead |
|---|---|---|---|
| `subagent-tool` | the agent's internal sub-agent tool (`Task`, `Agent`) | `PreToolUse` hook | `tirith lane run --project <id> --class <class> --feature "<description>" --brief <file> -- <agent cmd> …` |
| `agent-cli` | a shell command that starts a configured agent CLI (any simple command of the line, also by absolute path, `npx`/`bunx`, or inside `sh -c`) without `tirith lane run` | `PreToolUse` hook, shims | the same `tirith lane run …` line |
| `no-verify` | the git flag that skips hooks (long form), `git commit -n` (also `-an`) | `PreToolUse` hook | run the git command without it; fix what the hook reports |
| `hooks-path` | any command naming `core.hooksPath` | `PreToolUse` hook | edit `[[hooks]]` in `tirith.toml`, run `tirith check` |
| `git-hooks` | editing/removing files under `.git/hooks` (shell mutators, redirects, Edit/Write tools) | `PreToolUse` hook | leave it; the owner uses `tirith guard install` |
| `raw-stage` | a stage `run` of the project's `tirith.toml` executed raw (the whole sequence, or the stage's last command with its args) | `PreToolUse` hook | `tirith run [--project <id>] [--pipeline <id>] --stage <name>` |
| `lane-brief` | `lane run` with no `--brief`, or a missing/empty brief file (exit 3) | `lane run` | write the task into a file, pass `--brief <file>` |
| `project-check` | `lane run` while `tirith check` fails for the project (exit 3) | `lane run` | fix the listed problems, run `tirith check --project <id>` |
| (guard) | `lane run` while the commit guard is not intact (exit 3) | `lane run` | `tirith guard install <project>` |

Custom rules: every `[[rules]]` entry of the project's `tirith.toml` is enforced by the same
`gates.check_call`, with the same message format and `violation` events (`CONTRACT-setup.md`).

`tirith run --stage …`, `tirith lane run … -- <agent>` and commands that merely mention an agent name
(`echo`, `cat`, file names) are never blocked.

## Blocking

`tirith hook` records the event as usual, then for `PreToolUse` runs `hooks.decide`. A violation exits
**2** with the message on stderr (the anthropic CLI feeds stderr back to the model). Muse uses the same
exit 2 (its docs give no other convention). The JSON alternative (`permissionDecision: "deny"`) is not
used.

Every block is also appended to `events.jsonl` as
`{"kind": "violation", "ts", "rule", "agent", "lane", "session", "project", "tool", "message", "cwd"}`.
Pipeline code and hook readers ignore these rows.

`TIRITH_GATES=off` in the environment never disables anything (an agent could set it). The only off
switch is the owner's config:

```toml
[gates]
enabled = false
```

## Shims

`tirith shim install|uninstall|verify [--dir <folder>]` (default `~/.local/bin`). For every
`[agents.<alias>] command` it writes an executable `sh` wrapper named like that command. The wrapper
`exec`s the real binary (resolved at install from `PATH` without the shim folder) when stdin and stdout
are terminals, or when `TIRITH_LANE_TOKEN` equals the `token` in `lanes/$TIRITH_LANE.json`; otherwise
it prints the `agent-cli` message and exits 2. A file without the marker line (`# tirith-shim v1`) is
never overwritten or removed. `verify` exits 1 when a shim is missing, foreign, not executable, points
at a missing binary, or is not first in `PATH`.

`lane run` generates the token, stores it in the lane file, passes it as `TIRITH_LANE_TOKEN` to the
child and removes it when the lane ends.

## In `state.json`

`violations`: the last 50 (`ts, rule, agent, lane, project, message`). `waiting_on_you` gets one item
(`source: "violation"`) per violation of the last hour. Each row also carries `blocked` (what was
blocked) and `instead` (the command), split from `message`.

The web shows them in an Overview "Rules" section (last 24 h: time, agent, rule label, what was
blocked, the command with a copy button), in red in the agent panel with a count on agent cards, and
as a red edge from the agent to The Guard in Flow.

## Activate

```sh
tirith hook install --agent anthropic      # and: --agent muse
tirith shim install                        # then put ~/.local/bin first in PATH
tirith shim verify
```
