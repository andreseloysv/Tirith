# Agents contract: presets, usage, diffs, doctor

## Presets: `tirith lane run --agent <alias>`

```
tirith lane run --project <id> --class <class> --feature "<text>" --brief <file> \
  --agent <alias> [--model M] [--effort E] (--prompt "<text>" | --prompt-file F)
```

Builds the headless command from `~/.tirith/config.toml`; `-- <cmd>` keeps working (not together with
`--agent`). Without `--prompt` / `--prompt-file` the brief file is the prompt.

Without `--workspace` the lane gets its own worktree (`<repo>-lanes/<lane-id>`, branch `lane/<lane-id>`);
`--detach` starts it in its own session and prints the lane id at once (the default inside the manager).
The lane record's `request` is the brief it was given.

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
preset problem). `codex` is confirmed end to end: a real lane wrote a file, its `agent_message` became the report, the
`turn.completed` usage was parsed and the lane ended `done`. `copilot` is confirmed too: after `/login` a real lane
created a file, its `assistant.message` with `phase: final_answer` became the report and the lane ended
`done`; the `result` record carries only `premiumRequests` (no tokens). The preset's `unset_env`
(`GITHUB_TOKEN`, `GH_TOKEN`) is removed from the child environment of a `copilot` lane, so the CLI uses its
own login instead of a classic `ghp_` token from the shell (which it refuses). The `--allow-tool` list
was enough for a file write in non-interactive mode. Config needed for each:

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

## State size and agent detail files

`state.json` (and `state.js`) is written compact and stays under 800 KB; the desktop app and the page
re-parse it every 2 s. `state.agents` is therefore trimmed:

- At most 60 agents, working ones first, then the most recently ended.
- Working agents and agents ended in the last 2 hours keep their detail, capped: 30 `log_entries`, 12
  `events` (only `ts`, `agent`, `event`, `tool`, `summary`, `source`), 10 `files`, 5 `tests`, 5 `log_tail` lines,
  every string of an entry cut at 160 characters (80 for `system` entries), `fields` dropped when over
  120 bytes, `report` cut at 4 KB. Past a byte budget for all agents (480 KB) the oldest ended ones
  drop to summaries too.
- Older ended agents keep only the summary fields (id, name, feature, status, times, project, step,
  review, usage, the first paragraph of the report); `log_entries`, `log_tail`, `events`, `files` and
  `tests` are `[]`.
- `report_clipped` is true when `report` was cut; `detail_more` is true when anything was cut.
- `agents[].detail` = `{path, hash}`: `~/.tirith/agents/<id>.js` sets `window.TIRITH_AGENT` to
  `{id, log_entries, log_tail, events, files, tests, report}` in full (the report bounded at 16 KB). The
  collector rewrites it only when `hash` (of that content) changes, and writes one for every agent,
  also those not listed in the state. The file name is the id with characters outside `[A-Za-z0-9_.-]`
  (and a leading dot) replaced by `_`.

The drawer loads the file with a `<script>` tag when it opens an agent with `detail_more` (works from
`file://`); the desktop app asks the Tauri command `agent_detail(id)`, which reads and parses the same
file. The app pushes `state.json` to the page only when `generated_at` changed and the content (without
`generated_at`) did; when only the time moved it sends `tirith://alive` with the new `generated_at`.

## Doctor

`tirith doctor --agent <alias>` prints `ok` / `FAIL` per check and exits 1 on any failure:
`executable` (the configured command is found in the lane PATH), `path-order` (`~/.local/bin` is
first in the lane PATH), `hooks-skip` (a commit that skips hooks is refused, simulated in a temp repo
with the wrapper on PATH; reported as FAIL "cannot test" when there is no `~/.local/bin/git`).

## Chat arguments: `chat_args`

The persistent manager session (`docs/CONTRACT-inbox.md`, "One manager") starts `<command> <chat_args>` under
`tirith lane run --interactive`; without `chat_args` in the config the same arguments are used for `anthropic`.
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
