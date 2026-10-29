# Contract: per-agent goal and activity (v1)

The owner wants to see, for every agent, its goal, whether it is active, which tests it triggered,
which tools it uses and which files it is changing (with git), live. The collector computes all of it
each cycle (`collector/tirith/activity.py`); nothing needs to be asked of the agent. Every field is
optional in `collector/state.schema.json`, so old states stay valid.

New fields on each `state.agents[]` item:

| Field | Type | Meaning |
|---|---|---|
| `goal` | string or null | The lane brief's first paragraph under a "Why"/"Goal" heading, else under the first heading, else its first paragraph (markdown stripped, at most 3 sentences and 400 chars); else the feature. |
| `active` | bool | `true` while status is `working` and the newest activity is at most 5 minutes old. |
| `last_activity_at` | ISO string or null | Newest of: last event (hook or log) and the log's last write. |
| `files` | list of `{path, change, insertions, deletions}` | Changed since the lane's `start_commit`: `git diff --numstat/--name-status -z -M` plus `git ls-files --others --exclude-standard`. `change` is `added`, `modified`, `deleted`, `renamed` or `untracked` (untracked: `insertions` = line count of text files up to 256 KB, else 0). Max 100, paths relative to the workspace. Ended lanes keep the list computed once. |
| `tools` | list of `{name, count}` | Tool calls, most used first. |
| `tests` | list of `{at, command, result}` | Test runs, newest last, max 20. `result` is `passed`, `failed`, `running` or null (unknown). |
| `events` | list | Hook events plus events read from the lane's log, merged by `ts`, last 50. Each has `source`: `hook` or `log`. |

`files_changed`, `insertions` and `deletions` (existing) are now the tracked totals of the same diff
(untracked files excluded), so one diff pair per working agent per cycle is the whole git cost.

## Test commands

A shell command counts as a test run when one of its segments (split on `&&`, `||`, `;`, `|`) runs
`pytest` (so `uv run … pytest` too), `npm test` / `npm run test`, `pnpm test`, `yarn test`, `vitest`,
`jest`, `go test`, `cargo test`, `gradle`/`gradlew … test`, `make test` / `make check`,
`xcodebuild … test`, `swift test` or `tirith run … --stage test`. Segments starting with `grep`, `cat`,
`echo`, `git`, `tail` and similar viewers are ignored. `result`: `running` until the tool result
arrives, then `failed` when the result is an error (or its text reports `N failed` / `N errors`),
else `passed`. When the lane is no longer working, a still-`running` test becomes null.

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
