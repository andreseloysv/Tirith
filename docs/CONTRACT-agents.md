# Agents contract: presets, usage, diffs, doctor

## Presets: `tirith lane run --agent <alias>`

```
tirith lane run --project <id> --class <class> --feature "<text>" --brief <file> \
  --agent <alias> [--model M] [--effort E] (--prompt "<text>" | --prompt-file F)
```

Builds the headless command from `~/.tirith/config.toml`; `-- <cmd>` keeps working (not together with
`--agent`). Without `--prompt` / `--prompt-file` the brief file is the prompt.

```toml
[agents.anthropic]
command = "<executable name>"      # always from config, never from code
args = ["-p", "--output-format", "stream-json", "--verbose", "--permission-mode", "acceptEdits"]
permissions = ["Read", "Edit", "Bash(git status:*)"]   # allowed tool patterns
model = "<model id>"
effort = "<level>"
add_dirs = ["~/extra"]

[agents.anthropic.projects.tirith]  # per-project override: any key above replaces the global one
permissions = ["Read", "Edit"]
```

Defaults in code (`agentrun.PRESET_DEFAULTS`, flags only): `anthropic` = print mode, stream JSON with
verbose, accept-edits, allowed-tools list (comma separated), `--model`, `--effort`, `--add-dir`;
`muse` = its `exec --json` subcommand and `--model` only; `codex` = `exec --json --sandbox
workspace-write`, `--model`, `--add-dir`; `copilot` = `--output-format json --no-ask-user`, an
`--allow-tool` list (one flag, many values), `--model`, `--add-dir`, prompt via `-p`. The `codex` and
`copilot` flags were checked against each CLI's real help, read through `tirith lane run -- codex exec
--help` / `-- copilot --help` (`copilot` also takes `--reasoning-effort`; `codex` has no effort flag, it
only accepts `-c model_reasoning_effort=...`). Both CLIs must be logged in for the user running the lane
(a `401` from `codex` or "No authentication information found" from `copilot` is a login problem, not a
preset problem). `copilot --allow-all-tools` is documented as required for non-interactive mode; the
preset uses the `--allow-tool` list instead and that is unconfirmed (owner: run one tiny task per alias
and check the JSON log, usage and permissions). Config needed for each:

```toml
[agents.codex]
command = "codex"

[agents.copilot]
command = "copilot"
``` An unsupported option (for example `--effort`
for `muse`) exits with an error. Order: executable, args, model, effort, permissions, extra dirs,
prompt (last).

## Usage

`agents[].usage` (null when the log has no usage record) =
`{input_tokens, output_tokens, cache_tokens, cost_usd (null if unknown), model, turns, duration_s}`,
parsed from the lane's JSONL log (`codex`: its `turn.completed` records count like `run.usage`, with `cached_input_tokens` as cache; cost stays null): `result` records (`usage`, `total_cost_usd`, `num_turns`,
`duration_ms`, `modelUsage`) win; else `run.usage` records (payload token counts, optional cost).
`cache_tokens` = cache creation + cache read. `sessions[].usage` and `projects[].usage` are the sums
over their agents (null when none has usage). The agent drawer shows one line under Status; the
Overview shows one `Spent …` line.

## Diff

At lane end (`lane run`, `lane end`) Tirith writes `git diff <start_commit>` plus untracked files as
added-file patches (text bounded at 1 MB, a file over 256 KB is cut, binary files listed only) to
`~/.tirith/diffs/<lane>.patch`, and the same text as `diffs/<lane>.js` (a script that sets
`window.TIRITH_DIFFS[<lane>]`, because a `file://` page cannot fetch). `agents[].diff` =
`{path, size, script}` or null. The Files section of the drawer has "View diff".

## Doctor

`tirith doctor --agent <alias>` prints `ok` / `FAIL` per check and exits 1 on any failure:
`executable` (the configured command is found in the lane PATH), `path-order` (`~/.local/bin` is
first in the lane PATH), `hooks-skip` (a commit that skips hooks is refused, simulated in a temp repo
with the wrapper on PATH; reported as FAIL "cannot test" when there is no `~/.local/bin/git`).

## Chat arguments: `chat_args`

The desktop chat (`app/README.md`) starts `<command> <chat_args>` under `tirith lane run --interactive`.
For `anthropic`: `["-p", "--input-format", "stream-json", "--output-format", "stream-json",
"--verbose", "--permission-prompt-tool", "stdio"]`. stdin takes one JSON object per line:
`{"type":"user","message":{"role":"user","content":[{"type":"text","text":"..."}]}}`. stdout streams
`system`, `assistant`, `user` (tool results) and `control_request` lines, and finally `result`. A
permission question is
`{"type":"control_request","request_id":R,"request":{"subtype":"can_use_tool","tool_name":T,"input":{...}}}`;
the answer is
`{"type":"control_response","response":{"subtype":"success","request_id":R,"response":{"behavior":"allow","updatedInput":{...}}}}`
or a `behavior` of `deny` with a `message`. Sources: `docs/PLATFORMS.md`. `tools/chat_check.py`
exercises it by hand.
