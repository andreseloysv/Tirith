# Contract addition: commands for the owner are "Run" buttons (v1)

An agent that wants the owner to run a command never writes it as text to copy. It declares an
**action**; the page shows a row with a **Run** button. Code enforces this: text that carries a shell
command is refused.

```
agent:    tirith inbox answer <id> "<text>" --run "<command>" --label "<what it does>" [--cwd <dir>]
          tirith wait add "<text>" --run "<command>" ...
owner:    clicks Run -> confirm popover (exact command + folder) -> Run
app:      Tauri command run_action(action_id)     -> /bin/sh -c in the folder, log, rows
browser:  tirith://run?action=<id>&token=<t>      -> tirith url open -> tirith action run <id>
neither:  the button is "Copy" (the command goes to the clipboard)
```

The page never sends command text, only an id. Both executors look the id up in the files below and
refuse an unknown one. Nothing runs without the owner's click.

## Stored form

`actions` is a list on an **answer row** of `<home>/inbox.jsonl` and on a **wait row** of
`<home>/waits.jsonl`:

```json
{"id": "ac-5e1a0001", "label": "Install the link handler", "command": "uv run tirith url install",
 "cwd": "~/DEVELOP/Tirith/collector", "created_by": "manager", "at": "2026-10-28T19:41:00Z"}
```

`id` is `ac-` plus 8 hex digits; `label` (at most 120 characters) defaults to the command's first line;
`command` is at most 2000 characters; `cwd` is optional (default: the project's folder, else the user's
home); `created_by` is the answerer (`manager`, `brain`).

## Run rows

Every run appends to `inbox.jsonl` (no `id`, so the request reader ignores them):

```json
{"kind": "action-run", "action_id": "ac-5e1a0001", "status": "running", "exit_code": null, "at": "..."}
{"kind": "action-run", "action_id": "ac-5e1a0001", "status": "passed", "exit_code": 0, "at": "..."}
```

`status` goes `running` -> `passed` (exit 0) or `failed` (any other code; 127 if the command could not
start). Output (both streams) goes to `<home>/actions/<action_id>.log`, overwritten on each run. One run
at a time per action: a `running` row younger than an hour refuses another run (older ones are treated as
a run whose process died).

## CLI

| Command | Does |
|---|---|
| `tirith inbox answer <id> "<md>" --run C [--label L] [--cwd D]` | `--run` is repeatable; `--label` and `--cwd` match `--run` by position (one `--cwd` applies to all) |
| `tirith wait add "<text>" --run C [--label L] [--cwd D]` | same, on the owner's waiting list |
| `tirith action list [--json]` | every action with its project, source and `last_run` |
| `tirith action run <id>` | runs it (exit 0 passed, 1 failed, 3 unknown or already running) |

## The hard rule

`tirith inbox answer` and `tirith wait add` **refuse** (exit 2, nothing written) text that contains

- a fenced block whose language is `sh`, `bash`, `zsh`, `shell`, `console`, `shell-session`, `terminal`
  or `fish`;
- an unlabelled fence with a line that looks like a command (starts with `cd `, `uv `, `npm `, `git `,
  `./`, `$ `, ... any word of `actions.COMMAND_WORDS`);
- an inline code span that is a whole command line (a command word or prompt plus an argument, e.g.
  `` `uv run tirith url install` ``). Paths, single words and code spans in other fences are fine.

```
tirith: owner-command: commands for the owner must be actions. Do this instead: tirith inbox answer <id> "<text without the command>" --run "<command>" --label "<what it does>"
```

(`wait add` names `tirith wait add "<text without the command>" --run ...`.)

## `tirith://run` link

`tirith url open "tirith://run?action=<id>&token=<t>"` goes through the same handler and the same token
check as launch (`docs/CONTRACT-launch.md`): a wrong or missing token is refused (exit 3) and recorded as
a `launch-token` violation. It then runs `tirith action run <id>`. Any other query field is ignored.

## App

`run_action(action_id)` in `app/src-tauri/src/lib.rs` looks the id up, checks the folder exists, appends
the `running` row, and returns at once; a thread runs `/bin/sh -c`, then appends `passed` or `failed`.
Errors (unknown id, already running, missing folder) come back to the page as the rejection text.

## `state`

`state.inbox[].answers[].actions[]` and `state.waiting_on_you[].actions[]` carry the stored fields plus

```json
"last_run": {"status": "failed", "exit_code": 2, "at": "...", "log_tail": ["last", "20", "lines"]}
```

`last_run` is `null` when the action never ran.

## Web (`web/js/actions.js`)

Used by the plan thread (`v-plan.js`) and the "Needs you" cards (`ui.js`). Each action is a row: label,
the command in a monospace chip (the whole command on hover) and a primary **Run** button.

- Click -> a popover with the exact command and the folder, **Run** / **Cancel** (Esc cancels).
- Running: a spinner, "Running...", and the live log tail (refreshed with the state).
- Done: a green check "Passed" or a red cross "Failed · exit N", **Show output** (the last 20 lines) and
  **Run again**.
- No app and no link handler (`state.launch.available`): the button is **Copy** and a note says the page
  cannot run it.

## Auto-close and the manager hears the result

**The owner sees as little as possible.** An item of `waits.jsonl` whose actions have all a `passed`
`last_run` closes by itself on the next collector cycle: a row is appended with `done_at` (the time of the
last passing run) and `"done_reason": "action-passed"`. It leaves `waiting_on_you` and shows in
`answered_waits` (the folded "Answered" list) with `answer: {value: "action-passed", label: "✓ <label>"}`;
several actions give `✓ <label>, <label>`. An item with a failed (or still running) action stays open and
shows the failure. A re-run that passes closes it. Items Tirith raises itself (guard) are not affected.

A passed action of a plan answer (`state.inbox[].answers[].actions[]`) collapses in the page to one line
`✓ <label>`.

**The manager is told about every finished run on a wait** (requests keep the behaviour of
`docs/CONTRACT-inbox.md`: the request is pending again). `tirith inbox watch` emits, once per run:

```json
{"kind": "action-result", "wait": "w-2", "project": "tirith", "action_id": "ac-5e1a0001",
 "label": "Install the link handler", "status": "failed", "exit_code": 2, "at": "..."}
```

(`request` instead of `wait` for an action of an inbox answer.) On start, only failures still owed are
emitted; later runs are emitted as they finish. Passed results are information only: they are never
pending and never block.

A **failed** result on a wait is pending for the manager (its hook context, `SessionStart` /
`UserPromptSubmit`, lists it) until the action's latest run passes, or `tirith inbox done <wait>` /
`tirith wait done <wait>` closes it. It does not block `Stop`.
