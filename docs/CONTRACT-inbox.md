# Contract addition: the owner's inbox (v1)

Next to every step of the Plan the owner has two buttons: **Finish it** (a flag) and **Ask** (a speech
bubble). They send the steward a request without typing in its terminal. No server, no port: the request
is one line in a local file that the steward watches.

```
button -> app: Tauri command inbox_add          -> append to <home>/inbox.jsonl
       -> browser: tirith://inbox?...&token=..  -> tirith url open -> tirith inbox add
       -> neither: the page shows the `tirith inbox add ...` line to copy
steward: tirith inbox watch (monitor) -> tirith inbox answer <id> "<markdown>" -> tirith inbox done <id>
collector: state.inbox -> the Plan shows the chip and the answers under the step
```

## `<home>/inbox.jsonl` (append-only)

`<home>` is `$TIRITH_HOME`, else `~/.tirith`. One JSON object per line; rows are never rewritten.

| Row | Fields |
|---|---|
| request | `id`, `at`, `kind` (`finish` or `ask`), `project`, `step`, `text`, `from` (`"owner"`), `status` (`"open"`) |
| answer | `id` (of the request), `at`, `kind: "answer"`, `by` (`steward` or `brain`), `text` (markdown), `auto` (bool), optional `lane` (id of the lane that took the request) |
| done | `id`, `at`, `kind: "done"` |

- `id` is `in-` plus 8 hex digits. `step` is the roadmap step number as a string.
- `text` is the question (markdown), at most 4096 bytes (cut on a character boundary). A `finish` may have
  no text; an `ask` must.
- Answer and done rows for an unknown id, and lines that are not JSON, are ignored.

## CLI

| Command | Does |
|---|---|
| `tirith inbox add --kind finish\|ask --project P --step N [--text T]` | appends a request, prints its id |
| `tirith inbox list [--open] [--json]` | lists requests (`--open`: only those with no answer and not done) |
| `tirith inbox answer <id> "<markdown>" [--by steward\|brain] [--lane L]` | appends an answer (default `steward`, `auto: false`); `--lane` names the agent that has the request |
| `tirith inbox done <id>` | closes the request |
| `tirith inbox watch [--new-only] [--interval S]` | blocks; prints each open request as one JSON line, then each new one as it arrives |

`watch` prints the requests that are open when it starts (so none is lost between steward sessions);
`--new-only` skips them. Requests already answered or done are not printed. Use it with a monitor:
every printed line is something the owner is waiting on. Unknown id or bad input: exit 3, reason on stderr.

## `tirith://inbox` link (browser fallback)

`tirith://inbox?kind=ask&project=P&step=N&text=..&token=T` goes through the same handler as launch
(`docs/CONTRACT-launch.md`): `tirith url open` checks the launch token (a wrong or missing one is refused,
exit 3, and recorded as a `launch-token` violation) and runs `tirith inbox add`.

## App

`inbox_add(kind, project, step, text)` (`app/src-tauri/src/lib.rs`) appends the row itself (no shell, one
write call) and returns the id. The page detects the app with `window.__TAURI__`.

## `state.inbox`

The last 100 requests, oldest first:

```json
{"id": "in-3f9a21c0", "at": "...", "kind": "ask", "project": "tirith", "step": "4", "text": "...",
 "status": "answered", "answers": [{"at": "...", "by": "steward", "text": "...", "auto": false}]}
```

Each row also has `lanes`: the lane ids from `answers[].lane`, then the agents whose `request_id` is the
request. A lane gets its `request_id` from `--request <id>` on `tirith lane run`, `lane start` and
`tirith launch` (`state.agents[].request_id`). All optional; without them nothing changes.

`status` is `open` (no answer yet), `answered` (at least one answer) or `done` (a done row).

### Automatic answer

An `ask` that is still `open` 120 s after `at` gets one answer from the brain (the same worker and
`brain/` cache as the other brain jobs, job `inbox-answer`, keyed by the request id), stored in
`inbox.jsonl` as `by: "brain", auto: true`. The brain receives the step title, its open and done items,
the agents on that step (goal, what they are doing now, last tests, files), the recent flows of the
project, and the question. A steward answer within the 120 s means the brain never runs. Failed calls are
retried at most every 5 minutes. Brain disabled: nothing happens. `finish` requests are never answered
automatically.

## Web (`web/js/v-plan.js`)

- Buttons are pills with words: **Finish this step** and **Ask**, with a tooltip saying what happens.
- Under the step, one timeline per request (newest first, built only from `state.inbox`): asked, answered
  (steward, Tirith started an agent, or Muse Spark automatically), handled by the agent in `lanes` with its
  live status, done. Unanswered: "Waiting for the steward · N s"; after 2 min a note that Tirith will
  escalate. The local "sent" chip lasts only until the request shows up in the state (max 10 s).

- Every step row has the two buttons on the right: visible on hover or focus, always on touch screens.
- **Ask** opens a text box under the step: Enter sends, Esc closes (the draft is kept). **Finish it** sends
  at once. Requests for done steps are allowed.
- Sending: `window.__TAURI__` -> `inbox_add`; else, with the link handler installed (`state.launch`), the
  `tirith://inbox` link; else the exact `tirith inbox add ...` command is shown to copy.
- A step with an open request (or one sent in the last 30 s that the collector has not shown yet) shows the
  chip "Sent · waiting for the steward".
- Answers show under the step as rendered markdown, newest last, with "Steward" or "Muse Spark, automatic";
  each request is collapsible (closed by default once done). The last 3 requests per step are shown.
- The Plan header shows the number of open requests.
- The "Finish with an agent" launch button is only in the expanded step, labelled "Start an agent now"
  (the direct path).
