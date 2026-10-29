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
| request | `id`, `at`, `kind` (`finish`, `ask` or `setup`), `project`, `step`, `text`, `from` (`"owner"`), `status` (`"open"`) |
| answer | `id` (of the request), `at`, `kind: "answer"`, `by` (`steward`, `brain` or `tirith`), `text` (markdown), `auto` (bool), optional `lane` (id of the lane that took the request) |
| done | `id`, `at`, `kind: "done"` |

- `id` is `in-` plus 8 hex digits. `step` is the roadmap step number as a string.
- `text` is the question (markdown), at most 4096 bytes (cut on a character boundary). A `finish` may have
  no text; an `ask` must.
- Answer and done rows for an unknown id, and lines that are not JSON, are ignored.

## CLI

| Command | Does |
|---|---|
| `tirith inbox add --kind finish\|ask\|setup --project P --step N [--text T]` | appends a request, prints its id |
| `tirith inbox list [--open] [--json]` | lists requests (`--open`: only those with no answer and not done) |
| `tirith inbox answer <id> "<markdown>" [--by steward\|brain] [--lane L]` | appends an answer (default `steward`, `auto: false`); `--lane` names the agent that has the request |
| `tirith inbox done <id>` | closes the request |
| `tirith inbox brain <id>` | answers the request with the brain now (`by: brain`, `auto: true`) |
| `tirith inbox watch [--new-only] [--interval S]` | blocks; prints each open request as one JSON line, then each new one as it arrives |

`watch` prints the requests the steward still owes something (`steward_pending`) when it starts (so none is lost between steward sessions);
`--new-only` skips them. Requests the steward already answered, or that are done, are not printed. Use it with a monitor:
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
 "status": "answered", "steward_pending": true, "answers": [{"at": "...", "by": "steward", "text": "...", "auto": false}]}
```

Each row also has `lanes`: the lane ids from `answers[].lane`, then the agents whose `request_id` is the
request. A lane gets its `request_id` from `--request <id>` on `tirith lane run`, `lane start` and
`tirith launch` (`state.agents[].request_id`). All optional; without them nothing changes.

`status` is `open` (no answer yet), `answered` (at least one answer) or `done` (a done row).

`steward_pending` is true while the steward owes the request something: a `finish` with no answer at
all (Tirith's dispatcher normally settles it within a cycle), any other kind until the steward itself has
answered (`by: steward`) or it is done. A brain or Tirith answer never clears it for an `ask` or `setup`.

### Automatic answer

With `[dispatch] enabled = false` an `ask` that is still `open` 120 s after `at` gets one answer from the brain (the same worker and
`brain/` cache as the other brain jobs, job `inbox-answer`, keyed by the request id), stored in
`inbox.jsonl` as `by: "brain", auto: true`. The brain receives the step title, its open and done items,
the agents on that step (goal, what they are doing now, last tests, files), the recent flows of the
project, and the question. A steward answer within the 120 s means the brain never runs. Failed calls are
retried at most every 5 minutes. Brain disabled: nothing happens. With the dispatcher on (the default) the brain
answers an `ask` in the first cycle, not after 120 s.

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

## Dispatcher (`collector/tirith/dispatch.py`)

No rule depends on an agent's good will: every cycle the collector runs the dispatcher (idempotent; what it
did is in `<home>/dispatch.json`: `handled`, `escalated`, `stop`). `[dispatch] enabled = true` and
`auto_launch = true` are the defaults; `enabled = false` switches the whole thing off (the steward then
handles everything by hand), `auto_launch = false` leaves `finish` to the steward.

| Request | The dispatcher |
|---|---|
| `finish`, a working lane already has that project and step | answers `by: tirith`, naming the lane and its now-doing line |
| `finish`, no such lane | runs the `tirith launch` code in-process (detached, like the CLI) and answers with the new lane id |
| `finish`, launch refused (step done, `tirith check` fails, no agent command, ...) | answers with the reason and the exact fix command, and adds an owner item with options `Fix it` / `Try again` (`waits.jsonl` row with `options: [{label, command}]`, also in `state.waiting_on_you[].options`) |
| `ask` | the brain answers at once (`by: brain`, `auto: true`); the request stays `steward_pending` |
| `setup` and other kinds | left to the steward (`steward_pending`) |

Every action is a flow (`flows.jsonl`): `owner` -> `tirith` (the request), then `tirith` -> `agent:<lane>`
(`brief` when started, `message` when already running), `tirith` -> `brain` and `brain` -> `owner` for an
answered `ask`, `tirith` -> `owner` (`alert`) for a refusal or an escalation. `tirith` and `brain` are flow
endpoints.

### The steward is forced to answer (hooks)

For the steward session only: `TIRITH_ROLE=steward` in the environment, or a working folder inside
`[steward] session_cwd` of `config.toml`. A lane (`TIRITH_LANE` set) is never the steward. Agent alias
`anthropic`; `tirith hook install --agent anthropic --role steward` writes the hook commands with
`TIRITH_ROLE=steward` in front (`--settings <file>` to write elsewhere).

| Hook | Does |
|---|---|
| `SessionStart`, `UserPromptSubmit` | prints `{"hookSpecificOutput": {"hookEventName": ..., "additionalContext": ...}}` listing every `steward_pending` request: id, kind, project, step, text, age, and the commands `tirith inbox answer <id> "<markdown>"` / `tirith inbox done <id>` |
| `Stop` | exit 2 with the same list on stderr while a `steward_pending` request older than 60 s has no steward answer. After 3 blocks in a row for the same set of requests it lets the session stop and records one `inbox-ignored` violation. A new set starts counting again |

Each injection (`kind: "inbox-injection"`) and block (`kind: "inbox-block"`) is a row in `events.jsonl`.

### Escalation

A `steward_pending` request unanswered for 10 minutes: once per request, a macOS notification
(`osascript display notification`; nothing on other systems), a `tirith` -> `owner` alert flow, and the owner
item "The steward has not answered: <text>" with options `Wait`, `Start an agent`
(`tirith launch --project P --step N`) and `Answer it with the brain` (`tirith inbox brain <id>`). The item
closes itself once the steward answers.
# Asks: "Your turn" (addition)

What the steward needs from the owner is an **ask**: a question with a clear call to action. Every card
on the Overview has one-click options and a box to write a free reply. Only what truly needs the owner
gets there; the steward resolves the rest first.

```
steward: tirith wait add --project P --text "<markdown>" [--option "Label|value" ...] [--command "<shell>"]
                         [--link URL] [--kind decision|action|approval|info]      -> waits.jsonl
owner:   button or text -> Tauri inbox_reply | tirith://inbox?kind=reply&... | `tirith inbox reply ...`
                                                                                   -> inbox.jsonl (kind "reply")
steward: tirith inbox watch prints the reply at once -> acts -> tirith inbox done <reply id>
collector: the ask is marked answered (kept, not deleted)  -> state.answered_waits
```

## `<home>/waits.jsonl` (additive fields)

`options` (up to 6 of `{"label", "value"}`; the CLI takes `"Label|value"`, the value defaults to the
label), `command` (a shell line the owner can copy), `kind` (`decision`, `action`, `approval` or
`info`; default: `decision` with options, `action` with a command, else `info`). A plain `tirith wait add
"text"` still writes the old row. `text` may also be given as `--text`.

## Reply rows in `inbox.jsonl`

`{"id": "in-xxxxxxxx", "at", "kind": "reply", "wait": "<ask id>", "project", "value", "text", "from":
"owner", "status": "open"}`. `value` is the option chosen (empty for a free reply), `text` the owner's
words (at most 4096 bytes); one of them is required. `tirith inbox watch` prints every open reply as one
JSON line (`kind: "reply"`, with `wait`, `value`, `text`), also those written before it started;
`tirith inbox done <reply id>` closes one. Replies never appear in `state.inbox`.

| Command | Does |
|---|---|
| `tirith inbox reply --wait ID [--project P] [--value V] [--text T]` | appends a reply, prints its id (the project defaults to the ask's) |

The `tirith://inbox` link takes `kind=reply&wait=..&value=..&text=..&project=..&token=..`, with the same
token check as before. The app has `inbox_reply(wait, project, value, text)` next to `inbox_add`.

## State

- `state.waiting_on_you[]` keeps its fields and gains `kind`, `options`, `command` and `status` (`open`).
- `state.answered_waits[]` (the last 10): the same shape with `status: "answered"` and `answer`
  `{at, value, label, text}`; the first reply to an ask is its answer.
- `state.steward_queue[]`: open items addressed to the steward, `{id, at, kind: "review", project, lane,
  text, to: "steward", status}`, from `<home>/steward_queue.jsonl`.
- A manual ask that only says to install the commit guard disappears once the project's guard is intact.
  Brain-detected asks exist only for working agents.

## Lane endings

- A lane that ends `done` with an "open / needs the owner / left for you" section in its report creates
  **no** ask for the owner. It adds one `kind: "review"` item to the steward queue (`tirith lane end` and
  `tirith lane run` both do it). The steward triages it (`tirith queue list`, `tirith queue done <id>`)
  and re-asks the owner with `tirith wait add ... --option ...` only when needed.
- A lane that fails under `tirith lane run`, or ends without an answer, adds one owner ask `lane-<id>`
  (`kind: decision`, options `Retry|retry`, `Show the log|log`, `Drop it|drop`, and a `tail` command for
  the log). `tirith lane end --result failed` run by the steward adds none.

## Web (`web/js/v-needs.js`)

- Overview section "Your turn" (above the Plan): one card per ask, newest first. Title is the question
  as markdown (2 lines, full on click), project chip, age.
- Options are buttons (the first is primary); `command` has a **Copy** button, `link` an **Open** button;
  an `action` ask has **Done**; an `approval` without options gets Approve / Decline; an `info` ask without
  options gets "Got it". A text box "Or write what you need…" sends a free reply on Enter. A typed draft
  goes along with the next option click.
- Sending: `window.__TAURI__` -> `inbox_reply`; else, with the link handler installed, the
  `tirith://inbox?kind=reply` link; else the `tirith inbox reply ...` line is shown to copy.
- After sending, the card moves to the "Answered" fold with "Sent to the steward" (kept in
  `localStorage` until the collector shows the answer). Items older than 48 h with no options, command or
  link fold into "Older". Empty: "Nothing needs you. The steward will ask here."
