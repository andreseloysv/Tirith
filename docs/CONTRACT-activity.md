# Contract: per-agent goal and activity (v1)

The owner wants to see, for every agent, its goal, whether it is active, which tests it triggered,
which tools it uses and which files it is changing (with git), live. The collector computes all of it
each cycle (`collector/tirith/activity.py`); nothing needs to be asked of the agent. Every field is
optional in `collector/state.schema.json`, so old states stay valid.

New fields on each `state.agents[]` item:

| Field | Type | Meaning |
|---|---|---|
| `goal` | string or null | The lane brief's first paragraph under a "Why"/"Goal"/"Por qué"/"Objetivo" heading (markdown kept, at most 400 chars; a paragraph that starts with set-up words such as `Workspace:`, `Repo:`, `Tests:`, `Read …` never counts), else the brief's `#` title without a "Lane:" prefix, else the feature. |
| `active` | bool | `true` while status is `working` and the newest activity is at most 5 minutes old. |
| `last_activity_at` | ISO string or null | Newest of: last event (hook or log) and the log's last write. |
| `files` | list of `{path, change, insertions, deletions}` | Changed since the lane's `start_commit`: `git diff --numstat/--name-status -z -M` plus `git ls-files --others --exclude-standard`. `change` is `added`, `modified`, `deleted`, `renamed` or `untracked` (untracked: `insertions` = line count of text files up to 256 KB, else 0). Max 100, paths relative to the workspace. Ended lanes keep the list computed once. |
| `tools` | list of `{name, count}` | Tool calls, most used first. |
| `tests` | list of `{at, command, result}` | Test runs, newest last, max 20. `result` is `passed`, `failed`, `running` or null (unknown). |
| `events` | list | Hook events plus events read from the lane's log, merged by `ts`, last 50. Each has `source`: `hook` or `log`. |

`files_changed`, `insertions` and `deletions` (existing) are now the tracked totals of the same diff
(untracked files excluded), so one diff pair per working agent per cycle is the whole git cost.

## Test commands

The shell line is split into simple commands (`&&`, `||`, `;`, `|`, newline, outside quotes); `cd`,
`VAR=x` assignments and wrappers (`time`, `env`, `sudo`, `nohup`) are skipped. It counts as a test run
only when a simple command's PROGRAM (and subcommand) is a test runner: `pytest`, `python -m pytest`,
`uv run [opts] pytest` (also `poetry run`, `uvx`, `npx`), `npm|pnpm|yarn|bun [run] test[:x]`, `vitest`,
`jest`, `go test`, `cargo test`, `swift test`, `gradle`/`gradlew … test*`, `make test|check`,
`xcodebuild … test`, `tirith run … --stage test`. A path or word containing "test" is never enough
(`cd …/tests && ls`, `sed -i … tests/x.py`, `echo pytest` are not tests).

`result`: `running` until the tool result arrives, then read from the OUTPUT when it has a summary
(`N passed`, `N failed`, `N errors`, jest/vitest `Tests:` lines, gradle `BUILD SUCCESSFUL|FAILED`, go
`ok`/`FAIL`, cargo `test result: ok`): any failure or error means `failed`, else `passed`. Only when the
output has no summary does the tool's error flag / exit code decide (a pipe to `tail` hides the exit
code). When the lane is no longer working, a still-`running` test becomes null.

Each test row also carries `passed` / `failed` (integers, when the output gave counts) and, when the
readable `command` differs from what ran, `full` (the whole line, up to 400 chars). `command` has leading
`cd … &&` dropped and paths relative to the workspace; the page shows it, with `full` on hover.

## Request and report

- `agents[].request`: `{text (≤ 2 KB), at, agent_session_id}` or null. `tirith lane run` records
  the lane's own brief (the task it was given, cut at 2 KB) when it has one. `lane start` records
  the owner's request: the request file (`~/.tirith/requests/<session>.json`, written by `tirith hook`) of the agent
  session in env `TIRITH_AGENT_SESSION`, else the newest request of the last 6 hours. Children inherit
  the nearest ancestor's request in the state; the page shows "Asked by you" on the root lane only.
- `agents[].report`: the lane's final answer in markdown, whole (stored and exposed up to 16 KB, cut at a
  line boundary), never shortened for the UI. `summary` is the "now doing" line (cut at a word, at most 300 chars): the newest timed `say` entry of `log_entries` (the agent's own words, markdown, never tool output) while the lane runs, the first paragraph of the report once it ended; only without those the brain's or a plain status sentence. The page shows the report instead of the summary when there is one.
- The page renders every agent-written text (`report`, `summary`, `goal`, `request`, `say`/`done` log
  entries, room messages, waits) as markdown with `web/js/md.js` (HTML escaped first, links only
  http/https/file) and clamps long text to about 8 lines with "Show all".

## Events from the lane log

The lane's log is read incrementally (a first read takes at most the last 512 KB; later cycles read
only new bytes; unchanged logs cost one `stat`). Lines that are not JSON objects are ignored. A
record's own `timestamp`/`ts` is used when present, else the log's modification time.

- `{"type":"assistant","message":{"content":[{"type":"tool_use","id","name","input"}]}}` → event
  `tool` (`tool` = name, summary like hooks: `Edit web/js/ui.js`, `Bash: uv run pytest -q`).
- `{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id","is_error"}]}}` → sets
  the matching test's `result`.
- `{"type":"item.started"|"item.completed","item":{"type":"command_execution","id","command",
  "exit_code"}}` and `item.type` `file_change` (`changes[].path`) → `tool` events (`Bash`,
  `file_change`); `exit_code` sets the test result.
- `muse --json` records: `payload_type` starting with `tool.` or naming a `command`/`file` segment
  (`runtime.*` bookkeeping excluded) → generic `tool` event from `payload` (`tool`/`tool_name`/`name`,
  `command`, `path`, `exit_code`; the call id from `tool_call_id`/`call_id`/`id`). The field names are a
  best effort: the sample stream has no tool records yet. `run.terminal.*` → `done`.
- `{"type":"result", …}` → event `done`.

Tool counts and tests come from the log when it contains tool calls, otherwise from the lane's hook
rows (one count per `PreToolUse`, tests with unknown result).

## Cost

The git diff pair runs once per working lane per cycle (ended lanes: once). The log is parsed
incrementally per lane (cache keyed by size and mtime); the brief is cached by mtime.

## Log entries

`state.agents[].log_entries` is the lane log as structured entries, **newest first**, built each cycle
by `collector/tirith/logview.py` from the last 256 KB of the log (cached by size and mtime; an agent
without a log file gets entries from its hook events). The old `log_tail` stays for compatibility.
The web only renders them (`web/js/logview.js`, shared by the Agents drawer and the Flow node).

Each entry: `{ts, kind, text, detail, fields, path}`.

| `kind` | `text` | `detail` | `fields` |
|---|---|---|---|
| `say` | the agent's own words | rest of a long text | set for an unknown JSON object (key → value card) |
| `command` | shell command | "" | |
| `output` | first 6 lines | rest (≤ 4 KB) | |
| `edit` | path relative to the workspace | `+12 −3`, `created`, `deleted` or `edited` | |
| `read` | `Read path`, `Grep pattern in dir` | | |
| `error` | first lines of the traceback / failed output / error message | rest | |
| `test` | test command | `running`, `passed` or `failed` | `{passed, failed}` when counts are known |
| `spawn` | what the sub-agent was asked | its type | |
| `done` | the final answer (first 6 lines) | rest | `{usage: "12.4k tokens, 38 s"}` |
| `system` | bookkeeping (init, rate limit, usage ping) | the original JSON | hidden by default, counted |

`ts` is the record's own timestamp, or null for plain-text lines. `path` is the full path of an edit/read
(the page shows it on hover; `text` is shortened). Only 60 visible and 30 `system` entries are kept.

JSON is never passed through as text. A line, a command's output or an event payload that is JSON becomes:
a `test` (`{passed, failed}`), an `error` (`{error: …}`), a `system` usage line (`input_tokens`,
`duration_ms` → `12.4k tokens, 38 s`), a `done` (`Finished: <first sentence>`, from `{result: …}`), or
else a `say` with `fields` (bounded: strings ≤ 300 chars, ≤ 24 keys, depth 3) and the original
pretty-printed in `detail`. The page humanizes keys (`exit_code` → "Exit code"), shows ISO times as
relative, booleans as ✓/✗, and collapses nested values ("3 items ▸"). A traceback in plain text is one
`error` entry.
