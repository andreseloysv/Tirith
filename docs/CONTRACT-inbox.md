# Contract addition: the owner's inbox (v1)

Next to every step of the Plan the owner has two buttons: **Finish it** (a flag) and **Ask** (a speech
bubble). They send the manager a request without typing in its terminal. No server, no port: the request
is one line in a local file that the manager watches.

```
button -> app: Tauri command inbox_add          -> append to <home>/inbox.jsonl
       -> browser: tirith://inbox?...&token=..  -> tirith url open -> tirith inbox add
       -> neither: the page shows the `tirith inbox add ...` line to copy
manager: tirith inbox watch (monitor) -> tirith inbox answer <id> "<markdown>" -> tirith inbox done <id>
collector: state.inbox -> the Plan shows the chip and the answers under the step
```

## `<home>/inbox.jsonl` (append-only)

`<home>` is `$TIRITH_HOME`, else `~/.tirith`. One JSON object per line; rows are never rewritten.

| Row | Fields |
|---|---|
| request | `id`, `at`, `kind` (`finish`, `ask`, `setup`, `chat`, `autopilot`, `autopilot-proposal`, `plan-refresh`, `plan-new` or `plan-edit`), `project`, `step`, `text`, `from` (`"owner"`, or `"tirith"` for `autopilot-proposal`), `status` (`"open"`) |
| answer | `id` (of the request), `at`, `kind: "answer"`, `by` (`manager`, `brain` or `tirith`), `text` (markdown), `auto` (bool), optional `lane` (id of the lane that took the request) |
| done | `id`, `at`, `kind: "done"` |

- `id` is `in-` plus 8 hex digits. `step` is the roadmap step number as a string.
- `text` is the question (markdown), at most 4096 bytes (cut on a character boundary). A `finish` may have
  no text; an `ask` must.
- Answer and done rows for an unknown id, and lines that are not JSON, are ignored.

### Kinds `plan-refresh` and `plan-new`

Two requests about a project's whole plan, sent by the buttons in the Plan card header. `step` is `"plan"`
(it may be left out). Tirith handles both by code (see the dispatcher below); the manager is not involved.

- `plan-refresh` ("Update the plan"): no text.
- `plan-new` ("Start a new plan"): `text` is a JSON object `{"title", "goal", "deadline", "steps"}` of strings;
  only `title` is required, `deadline` is free text (the form sends `YYYY-MM-DD`), `steps` is free text.
  Anything else is rejected, so a bad request is never stored. `state.inbox[]` rows of this kind carry the
  parsed object as `plan`.

### Kind `plan-edit`

The owner edits, deletes or adds one item of a plan with the buttons on the Plan card. `project` is the
project, `step` the roadmap step id (required), `text` a JSON object with exactly one change:

| Change | `text` |
|---|---|
| edit | `{"plan": "", "item_text": "<exact original>", "new_text": "<new>"}` |
| delete | `{"plan": "", "item_text": "<exact original>", "delete": true}` |
| add | `{"plan": "", "add": "<text>"}` |

`plan` is the plan id (file name without `.md`, see `CONTRACT-roadmap.md`); empty means the current plan.
`item_text` is the item's `text` as in `state.projects[].roadmap` (whitespace-normalised, tag included). Anything
else is rejected, so a bad request is never stored. `state.inbox[]` rows of this kind carry the parsed object
as `edit` and are never `manager_pending`: Tirith applies them (see the dispatcher below).

### Kind `setup`

Asks the manager to change or run a project's setup (`CONTRACT-setup.md`). `step` is `"setup"` (it may be
left out), `text` is a JSON object `{"action": "add-stage" | "remove-stage" | "add-rule" | "remove-rule" |
"run-stage", ...fields}` (anything else is rejected). `state.inbox[]` rows of this kind carry the parsed
object as `setup`. The manager handles one with `tirith setup apply <id>`: it edits `tirith.toml` (or runs
the stage), answers and closes the request.

### Kind `chat`

The owner's conversation with the manager (the Overview's chat panel). `{id, at, kind: "chat", project, step: "",
text (markdown), from: "owner", status}`: `project` is optional (empty = about nothing in particular), there is
no step, `text` is required. It is `manager_pending` until the manager answers (a brain or Tirith answer does
not clear it), and again after the owner runs one of the manager's Run `actions` until the manager answers the
result, like every other kind. The dispatcher adds no flow for it (the thread is the chat) and, when the
manager stays silent, its escalation offers no "Start an agent". Answers are the usual answer rows (`by:
"manager"`, markdown, optional `actions`); `tirith inbox answer <id> ...` is unchanged.

### Kind `autopilot`

The owner's switch for the autopilot (`CONTRACT-roadmap.md`): `text` is `on` or `off` (anything else is
rejected), `project` is `all` and `step` is `autopilot` (both may be left out). It is never
`manager_pending`: the dispatcher applies it in its next cycle (the last request wins over
`[autopilot] enabled`), records the flow `owner` -> `tirith` ("Autopilot off") and closes it. Running lanes
are not stopped; no new lane is started while it is off. Sent by the Plan header's Pause / Resume button
(`tirith inbox add --kind autopilot --text off`).

### Kind `autopilot-proposal`

Tirith's own request to the manager (`from: "tirith"`), written by the autopilot (`CONTRACT-roadmap.md`) for an
open plan item nobody works on. `project`/`step` name the item; the first line of `text` is
`Autopilot: <project> step <n> — <item>`, then why it is open, the plan file, the last attempts of the step
(lane, outcome, review status, claims, report summary) and the options. It is `manager_pending` like an owner
message: the manager (on-call turn or persistent session) decides launch / combine / wait / drop, answers with
one short line ("Autopilot: step 9 item … -> started lane X") and closes it (`tirith inbox done`). The dispatcher
adds no flow and never escalates it to the owner. One open proposal per item; a new one only when the item's text or
note changes; when the item is ticked or gone, Tirith answers `by: tirith` and closes it. Unanswered for more
than 10 min: `selfcheck` (manager and autopilot checks). The chat shows the first line as an "Autopilot" line.

## CLI

| Command | Does |
|---|---|
| `tirith inbox add --kind finish\|ask\|setup --project P --step N [--text T]` | appends a request, prints its id |
| `tirith inbox add --kind finish\|ask\|setup --project P [--step N] [--text T]` | appends a request, prints its id (`--step` is for plan steps; a `setup` request takes its JSON as `--text`) |
| `tirith inbox add --kind chat [--project P] --text T` | appends a chat message (`--project` and `--step` optional) |
| `tirith inbox add --kind finish\|ask\|setup\|plan-refresh\|plan-new\|plan-edit --project P [--step N] [--text T]` | appends a request, prints its id (`--step` is for plan steps; a `setup`, `plan-new` or `plan-edit` request takes its JSON as `--text`) |
| `tirith inbox list [--open] [--json]` | lists requests (`--open`: only those with no answer and not done) |
| `tirith inbox answer <id> "<markdown>" [--by manager\|brain] [--lane L]` | appends an answer (default `manager`, `auto: false`); `--lane` names the agent that has the request |
| `tirith inbox done <id>` | closes the request |
| `tirith inbox brain <id>` | answers the request with the brain now (`by: brain`, `auto: true`) |
| `tirith inbox watch [--new-only] [--interval S]` | blocks; prints each open request as one JSON line, then each new one as it arrives |

`watch` prints the requests the manager still owes something (`manager_pending`) when it starts (so none is lost between manager sessions);
`--new-only` skips them. Requests the manager already answered, or that are done, are not printed. Use it with a monitor:
every printed line is something the owner is waiting on. Unknown id or bad input: exit 3, reason on stderr.

### `tirith chat say`: the manager writes first

`tirith chat say "<markdown>" [--project P] [--run CMD --label L] [--option "Label|value"]` writes to the owner
without a request. With options or Run actions it is an owner item, exactly `tirith wait add` (same rules: no
shell commands in the text, `waits.jsonl`, replies as `kind: "reply"`). Without them it is a plain message: one
row `{id, at, kind: "say", project, text, from: "manager"}` in `inbox.jsonl`, listed in `state.says` (last 100,
oldest first). A `say` row is not a request: it is not in `state.inbox`, not `manager_pending`, `inbox watch`
does not print it and nothing answers it.

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
 "status": "answered", "manager_pending": true, "answers": [{"at": "...", "by": "manager", "text": "...", "auto": false}]}
```

Each row also has `lanes`: the lane ids from `answers[].lane`, then the agents whose `request_id` is the
request. A lane gets its `request_id` from `--request <id>` on `tirith lane run`, `lane start` and
`tirith launch` (`state.agents[].request_id`). All optional; without them nothing changes.

`status` is `open` (no answer yet), `answered` (at least one answer) or `done` (a done row).

`manager_pending` is true while the manager owes the request something: a `finish` with no answer at
all (Tirith's dispatcher normally settles it within a cycle), any other kind until the manager itself has
answered (`by: manager`) or it is done. A brain or Tirith answer never clears it for an `ask` or `setup`.

### Automatic answer

With `[dispatch] enabled = false` an `ask` that is still `open` 120 s after `at` gets one answer from the brain (the same worker and
`brain/` cache as the other brain jobs, job `inbox-answer`, keyed by the request id), stored in
`inbox.jsonl` as `by: "brain", auto: true`. The brain receives the step title, its open and done items,
the agents on that step (goal, what they are doing now, last tests, files), the recent flows of the
project, and the question. A manager answer within the 120 s means the brain never runs. Failed calls are
retried at most every 5 minutes. Brain disabled: nothing happens. With the dispatcher on (the default) the brain
answers an `ask` in the first cycle, not after 120 s.

### Client id (`cid`) and safe retries

A request may carry `cid`: the id the page made for this send (1 to 64 characters of `[A-Za-z0-9_-]`; the
`tirith://` link takes `&cid=`, the CLI `--cid`, the Tauri command `cid`). It is stored on the row and shown as
`cid` in `state.inbox`. A request whose `cid` is already on a row is ignored (the existing row is returned), so
the page can retry a send without ever writing it twice. Rows without a `cid` never collide.

The app writes every `inbox_add` attempt and its result (`ok id=...` or `error: ...`, never the text) to
`<home>/logs/chat-send.log`.

## Web (`web/js/v-manager.js`): chat with the manager

- The Overview's right column is the chat "Manager" (it replaces Activity, whose flows stay in the Flow view).
  From every other view a **Chat** button in the header opens the same panel as a drawer.
- One thread built only from `state.inbox`, oldest first with the newest at the bottom: the owner's chat
  messages (right, blue), the questions and "Finish this step" requests of the Plan, and every answer to them
  (left, rendered markdown, who answered and when, Run `actions` under the answer through `T.actions`).
- **Your turn is in the thread.** Every owner item (`state.waiting_on_you`, see "Asks" above) is a manager
  message at its `since` time: the text as markdown, its one-click options, command/link and Run actions under
  it, a small **Reply** link and the mark "Waits for you". **Reply** puts "Replying to <item>" over the composer;
  the next message sent is the free reply to that item (`inbox_reply`, same paths as an option click), not a chat
  message. A typed draft aimed at an item goes along with its next option click. Answered items (`answered_waits`,
  and those sent from this page that the collector has not shown yet) stay in the thread, followed by the
  owner's choice as a message ("You: Done"). `state.says` messages are plain manager messages. There is no
  separate "Your turn" section in the Overview.
- The chat header shows a badge with the number of items that still wait for the owner and **Jump to next**
  (scrolls to the next one, round and round). The header **Chat** button of other views gets its dot too.
- While a chat, ask or finish request is `manager_pending`, the thread ends with what is really happening:
  "Manager is typing…" (animated dots, age) only if a live manager session's heartbeat (`state.manager.live`,
  from `manager.json`, at most 5 min old) moved after the message, or a lane of that request is working.
  Otherwise a plain line without dots: "Waiting for the manager · 12 min", and after 30 min "The manager has
  not answered yet" in amber. A request reopened by an action result shows that result ("✗ Install the agent
  shims failed") as the reason instead. Timestamps without a zone are read as UTC.
- Composer: Enter sends, Shift+Enter is a new line, an optional project chip (default: the sidebar's project
  filter). Sending: `window.__TAURI__` -> `inbox_add` (kind `chat`); else, with the link handler installed, the
  `tirith://inbox?kind=chat&project=..&step=&text=..&token=..` link; else the `tirith inbox add --kind chat ...`
  line is shown to copy. A sent message shows (dimmed) until the collector lists it, at most 30 s.
- The old feature-flagged agent chat (`web/js/v-chat.js`, Tauri only) is removed: the manager chat is the only chat.

## Web (`web/js/v-plan.js`)

- Buttons are pills with words: **Finish this step** and **Ask**, with a tooltip saying what happens.
- The Plan card header has two more pills, always visible: **Update the plan** (sends `plan-refresh`; shows
  "Updating the plan…" while the request is in flight or its lane is working) and **Start a new plan** (opens
  a form: title, goal, deadline, first steps; "Start the plan" sends `plan-new`, a title is required).
  They use the same send paths as the step buttons, with the key `<project>|plan`. Under the header the
  card shows the "sent" chip, errors and the CLI fallback, and the last 2 plan requests as timelines like the
  step ones (asked, answered by Tirith with the summary, handled by the lane).
- On hover, an open item of the current plan shows two icon buttons: **Edit** (inline field, Enter saves, Esc
  cancels) and **Delete** (popover "Delete this item from the plan?"); a step shows **Add item** at the end of
  its open items and a `+` on its title row. Each sends a `plan-edit` with the key `<project>|<step>` over the
  same send paths. The item shows the new text (or struck through) with "Saving…" until the plan file in
  `state` has the change; a refused change shows its reason with Dismiss. The "sent" chip is not used for it.
  Other plans (read only) have no buttons. In History the request reads "You edited / deleted / added".
- Under the step, one timeline per request (newest first, built only from `state.inbox`): asked, answered
  (manager, Tirith started an agent, or Muse Spark automatically), handled by the agent in `lanes` with its
  live status, done. Unanswered: "Waiting for the manager · N s"; after 2 min a note that Tirith will
  escalate. The local "sent" chip lasts only until the request shows up in the state (max 10 s).

- Every step row has the two buttons on the right: visible on hover or focus, always on touch screens.
- **Ask** opens a text box under the step: Enter sends, Esc closes (the draft is kept). **Finish it** sends
  at once. Requests for done steps are allowed.
- Sending: `window.__TAURI__` -> `inbox_add`; else, with the link handler installed (`state.launch`), the
  `tirith://inbox` link; else the exact `tirith inbox add ...` command is shown to copy.
- A step with an open request (or one sent in the last 30 s that the collector has not shown yet) shows the
  chip "Sent · waiting for the manager".
- Answers show under the step as rendered markdown, newest last, with "Manager" or "Muse Spark, automatic";
  each request is collapsible (closed by default once done). The last 9 entries per step are shown.
- The Plan header shows the number of open requests. Below it: "Autopilot on · 2 steps in progress · 1 waiting for a review"
  (or "Paused") with a Pause / Resume button that sends the `autopilot` request.
- Under a step the latest entry (a request thread, or "Autopilot started <agent>" from `state.autopilot.started`)
  stays open; every older entry folds into one "N earlier messages" line (click to unfold, answers inside closed).
- The "Finish with an agent" launch button is only in the expanded step, labelled "Start an agent now"
  (the direct path).

## Dispatcher (`collector/tirith/dispatch.py`)

No rule depends on an agent's good will: every cycle the collector runs the dispatcher (idempotent; what it
did is in `<home>/dispatch.json`: `handled`, `escalated`, `stop`). `[dispatch] enabled = true` and
`auto_launch = true` are the defaults; `enabled = false` switches the whole thing off (the manager then
handles everything by hand), `auto_launch = false` leaves `finish` to the manager.

| Request | The dispatcher |
|---|---|
| `finish`, a working lane already has that project and step | answers `by: tirith`, naming the lane and its now-doing line |
| `finish`, no such lane | runs the `tirith launch` code in-process (detached, like the CLI) and answers with the new lane id |
| `finish`, launch refused (step done, `tirith check` fails, no agent command, ...) | answers with the reason and the exact fix command, and queues a manager item `launch-refused` (`sq-dispatch-<request id>`) with the `Fix it` and `Try again` commands; nothing reaches the owner |
| `ask` | the brain answers at once (`by: brain`, `auto: true`); the request stays `manager_pending` |
| `plan-refresh`, no working lane of that project with `step: "plan-refresh"` | starts the auditor lane (`tirith.planlane`, same path as `tirith launch`: worktree `<repo>-lanes/<id>`, branch `lane/<id>`, generated brief in `<home>/briefs/`), `step: "plan-refresh"`, and answers with the lane id |
| `plan-refresh`, one already running for the project | answers naming that lane (one refresh at a time per project) |
| `plan-new` | starts the planner lane (`step: "plan-new"`) and answers with the lane id; several may run |
| `plan-refresh` / `plan-new`, launch refused | answers with the reason and queues a manager item `launch-refused` (with the `tirith inbox add` retry command for a refresh) |
| `plan-edit` | applies the change to the project's plan file by code: the step's open item whose text is exactly `item_text` (a wrapped item counts as one) is replaced, removed, or the new item is appended after the step's last open item; commits **only that file** with `Roadmap: <edit\|delete\|add> in step N (owner)` through the normal git hooks (the commit guard); answers `by: tirith` and queues one `plan-refresh` |
| `plan-edit`, item text changed meanwhile (or ticked), step or plan gone, plan file has uncommitted changes, commit refused | changes nothing, answers `Could not <verb> the plan: <reason>` and adds an alert flow |
| `plan-edit` after another edit with a `plan-refresh` of that project still unanswered | no second refresh: the answer says one is already queued (at most one queued refresh per project) |
| `autopilot` | sets the autopilot on or off, closes the request |
| `autopilot-proposal` | left to the manager (`manager_pending`); no flow, never escalated to the owner |
| `setup` and other kinds | left to the manager (`manager_pending`) |

**The plan lanes.** The auditor's brief is generated: the current plan file, the project's merged lanes with
the first line of their reports, the recent inbox requests, the paths of `events.jsonl` (pipeline results)
and `inbox.jsonl`, and the job: compare every open and done item with the code, tick what is really done,
untick what is not, add items for work asked in the inbox or in chat that the plan lacks, change only the
plan file, and finish with a summary report in three parts (**Done since last update**, **Still missing**,
**Needs the owner**). The planner's brief carries the title, goal, deadline and first steps of the request
and asks for `docs/plans/<slug>.md` (same format as `docs/TODO.md`; `<slug>` comes from the title and is
unique) and its registration in `tirith.toml` (`plans`, and `roadmap` set to it). The lane leaves the files
in its worktree; the automatic review (`docs/CONTRACT-launch.md`) runs the tests, commits and merges them,
and answers the request with the merge line followed by the lane's report: that report is the summary the
owner reads under the Plan header. These lanes never tick steps from their report (`update_plan` skips them).
A refusal is the usual one (`tirith check` fails, no agent command, no repo, no plan file).

Every action is a flow (`flows.jsonl`): `owner` -> `tirith` (the request), then `tirith` -> `agent:<lane>`
(`brief` when started, `message` when already running), `tirith` -> `brain` and `brain` -> `owner` for an
answered `ask`, `tirith` -> `owner` (`alert`) for a refusal or an escalation. `tirith` and `brain` are flow
endpoints.

### The manager is forced to answer (hooks)

For the manager session only: `TIRITH_ROLE=manager` in the environment, or a working folder inside
`[manager] session_cwd` of `config.toml`. A lane (`TIRITH_LANE` set) is never the manager. Agent alias
`anthropic`; `tirith hook install --agent anthropic --role manager` writes the hook commands with
`TIRITH_ROLE=manager` in front (`--settings <file>` to write elsewhere).

| Hook | Does |
|---|---|
| `SessionStart`, `UserPromptSubmit` | prints `{"hookSpecificOutput": {"hookEventName": ..., "additionalContext": ...}}` listing every `manager_pending` request: id, kind, project, step, text, age, and the commands `tirith inbox answer <id> "<markdown>"` / `tirith inbox done <id>` |
| `Stop` | exit 2 with the same list on stderr while a `manager_pending` request older than 60 s has no manager answer. After 3 blocks in a row for the same set of requests it lets the session stop and records one `inbox-ignored` violation. A new set starts counting again |

Each injection (`kind: "inbox-injection"`) and block (`kind: "inbox-block"`) is a row in `events.jsonl`.

### Escalation

A `manager_pending` request unanswered for 10 minutes: once per request, a macOS notification
(`osascript display notification`; nothing on other systems), a `tirith` -> `owner` alert flow, and the owner
item "The manager has not answered: <text>" with options `Wait`, `Start an agent`
(`tirith launch --project P --step N`) and `Answer it with the brain` (`tirith inbox brain <id>`). The item
closes itself once the manager answers.
# Asks: "Your turn" (addition)

What the manager needs from the owner is an **ask**: a question with a clear call to action. Every card
on the Overview has one-click options and a box to write a free reply. Only what truly needs the owner
gets there; the manager resolves the rest first.

```
manager: tirith wait add --project P --text "<markdown>" [--option "Label|value" ...] [--command "<shell>"]
                         [--link URL] [--kind decision|action|approval|info]      -> waits.jsonl
owner:   button or text -> Tauri inbox_reply | tirith://inbox?kind=reply&... | `tirith inbox reply ...`
                                                                                   -> inbox.jsonl (kind "reply")
manager: tirith inbox watch prints the reply at once -> acts -> tirith inbox done <reply id>
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
- `state.steward_queue[]` is the old key of `manager_queue` (same value); `<home>/steward_queue.jsonl` is read while
  `manager_queue.jsonl` does not exist.
- `state.manager_queue[]`: open items addressed to the manager, `{id, at, kind: "review", project, lane,
  text, to: "manager", status}`, from `<home>/manager_queue.jsonl`.
- A manual ask that only says to install the commit guard disappears once the project's guard is intact.
  Brain-detected waits exist only for working agents and are **not** in `waiting_on_you` (see
  "Only owner items reach the owner").

## Lane endings

- Every launched lane that ends `done` is reviewed automatically by the dispatcher (see
  `docs/CONTRACT-launch.md`, "Automatic review"); its merge answers the request that started it
  (`by: "tirith"`), and its steward queue item (`kind: "review"`, closed with `result`) blocks the steward's
  `Stop` while it is older than 60 s and open. Owner items of a failed review carry Run `actions`.

- A lane that ends `done` with an "open / needs the owner / left for you" section in its report creates
  **no** ask for the owner. It adds one `kind: "review"` item to the manager queue (`tirith lane end` and
  `tirith lane run` both do it). The manager triages it (`tirith queue list`, `tirith queue done <id>`)
  and re-asks the owner with `tirith wait add ... --option ...` only when needed.
- A lane that fails under `tirith lane run`, or ends without an answer, adds one manager queue item
  `sq-lane-<id>` (`kind: lane-failed`, with the log tail), never an owner ask: retrying or dropping is the
  manager's call. `tirith lane end --result failed` run by the manager adds none.

## Web (`web/js/v-needs.js`, `web/js/v-manager.js`)

- There is no "Your turn" section: each ask is a manager message in the chat thread of the Overview, at its
  `since` time (`docs/CONTRACT-inbox.md`, "Web: chat with the manager"). The message is the question as
  markdown; the owner's answer follows it as a message ("You: Done").
- Options are buttons under the message (the first is primary); `command` has a **Copy** button, `link` an
  **Open** button; an `action` ask has **Done**; an `approval` without options gets Approve / Decline; an
  `info` ask without options gets "Got it". **Reply** aims the chat composer at the ask: Enter sends a free
  reply. A typed draft goes along with the next option click.
- The manager can also write a plain message with `tirith chat say` (see the inbox contract).
- Sending: `window.__TAURI__` -> `inbox_reply`; else, with the link handler installed, the
  `tirith://inbox?kind=reply` link; else the `tirith inbox reply ...` line is shown to copy.
- After sending, the item shows as answered at once (kept in `localStorage` until the collector shows the
  answer).

## Every item has a call to action (rule in code)

No item reaches the owner without at least one one-click **option** or one **Run action**. A command to
copy, a link or a text box is not enough.

- `tirith wait add` (`cli.add_wait`) refuses a wait with neither `--option` nor `--run`, with an
  agent-style message (`tirith: call-to-action: ... Do this instead: add --option "Label|value" or --run
  ...`). What only the manager must see goes to its own queue (`tirith queue`), not to the owner.
- `dispatch.add_owner_item` raises on an empty option list.
- Items the collector generates carry options or actions: dispatcher items (`Wait` / `Start an agent` when the manager is silent), guard checks (below), and violations,
  brain-detected waits and room waits (`Got it|ack`).
- The collector enforces it again on the final list (`_enforce_cta`, through `asks.is_owner_item`): an item
  without one (an old row of `waits.jsonl`, a future generator) is **not** shown. It becomes one manager
  queue item (`kind: "no-call-to-action"`, id `sq-cta-<wait id>`, added once).
- A reply to a generated item (`room-*`) answers it: it moves to `answered_waits`.
- **Your turn is only for what the manager refuses or cannot do.** There is no "Let the manager handle it" (or
  any hand-back) option anywhere: the manager's work is the default. What enforces it is "The owner gate" below.
- **Text matches buttons.** `add_owner_item` rewrites a text that promises a button the item lacks ("botón
  Run", "click Run", "Run button", same for Open / Done; `asks.missing_buttons` / `drop_button_claims`).
- The web message shows the item's Run actions (`web/js/actions.js`) between its option buttons and the text box.

### The owner gate (`tirith/ownergate.py`)

Nothing reaches the owner unless only the owner can do it, enforced by one function, `ownergate.admit`. Every
path that raises an owner item or an owner-directed message calls it: `dispatch.add_owner_item`,
`tirith wait add` / `tirith chat say` with options (`cli.add_wait`), a roadmap `(dueño: …)` tag
(`autopilot`), the "Needs the owner" list of a plan-refresh report (`ownergate.filter_report`, called by the
automatic review before the report is posted), and the manager-silent escalation. The autoreview no longer asks the owner anything: it merges.

- `owner-decision` is never for a merge / drop / review / relaunch question: `admit` rejects it (to the manager
  queue as `kind: "not-owner"`, "not for the owner: ...") by `kind` (`merge`, `drop`, `review`, `relaunch`) and
  by those words in the text or `why_detail`.
- The item must name `why_owner`, one of `asks.WHY_OWNER`: `credentials` (passwords, keys, store/developer
  accounts), `payment`, `personal-account` (a login only the owner can do), `physical-device`, `owner-decision`
  (a product choice with options). `tirith wait add --why-owner KEY [--why-detail "Apple account"]`; in a
  roadmap tag `(dueño: personal-account (Apple account) …)`; in a report bullet `- [credentials] …` or
  `- [personal-account: Apple ID] …`. Old keys map (`account` → `personal-account`, `device` →
  `physical-device`, `legal` → `owner-decision`); `security` and a missing key are not reasons.
- Missing or unknown `why_owner`: not an owner item. It becomes a manager queue item `kind: "agent-can-do"`,
  text `agent can do: <text>`, id `sq-acd-<hash>` (once). `tirith wait add` prints why on stderr and exits 0
  with nothing written to `waits.jsonl`.
- **The dispatcher turns each `agent-can-do` item into one verification lane** (`tirith/verifylane.py`,
  `cli.launch_verify`, one per cycle, step `verify`, none while the disk is low, none for an item without a
  project). The brief says: open `tirith://` links with `open`, take screenshots with playwright, read state
  or files, and report **Checked / Result / Needs the owner**. The queue item closes with the lane id; a
  refused launch stays in the queue for the manager. State in `<home>/verify-state.json`.
- Text with `(fuera de este plan` / `(out of this plan`, or one that mostly repeats an item that has it, is
  dropped: never shown, one row in `<home>/owner_dropped.jsonl` (`at, id, project, via, text`).
- A launch Tirith could not do (dispatcher, plan lanes, autopilot) is the manager's, not the owner's: a
  queue item `kind: "launch-refused"` (id `sq-dispatch-…` / `sq-autopilot-…`) with the fix and retry commands.
- The owner item shows its reason in plain words: `why_owner`, `why_detail` and `reason_tag` ("Needs your
  Apple account", "Needs your decision: pick a date") are on the wait and in `state.waiting_on_you`, shown
  as a chip on the card.
- On the first cycle of the collector every open row of `waits.jsonl` is re-checked (`ownergate.migrate`):
  one with an old reason key stays and gets its `why_owner`; the others are closed
  (`done_reason: "moved to the manager queue"`) and queued as `agent-can-do` (or dropped when out of plan).
### Concrete how-to (`tirith/howto.py`)

An owner item may carry a `howto` (`tirith wait add ... --howto '<json>|<file>'`, `add_owner_item(howto=...)`),
stored on the `waits.jsonl` row and shown in `state.waiting_on_you[].howto`:

```json
{"steps": [{"text": "…", "url": "https://…", "terminal": "xcrun notarytool store-credentials …"}],
 "done_check": "xcrun notarytool history --keychain-profile tirith-notary",
 "already_done": ["Developer ID certificate exists (Team AB12CD34EF)"]}
```

- **Validation (in `ownergate.admit`).** No placeholder (`<TEAMID>`, `YOUR_…`, `xxx`, `{{…}}`, `[your …]`, `TODO`,
  `...`) in a step, in the item text of an item with steps, or in a command; every `terminal` and the `done_check`
  must parse as a shell line as they are; `url` is a full `http(s)://` or `tirith://` address. A how-to that fails
  does not reach the owner: a manager queue item `kind: "make-concrete"`, text `make it concrete: look up …`.
- **UI.** A step with `url` is an "Open page" link; a step with `terminal` is an "Open in Terminal" button (Tauri
  command `open_in_terminal(wait_id, step)`: the app reads the command from `waits.jsonl`, never from the page,
  and runs `osascript` → `tell application "Terminal" to do script …`, so a password is typed only into the
  Terminal prompt). Without the app the button is "Copy command". `already_done` shows as ✓ facts.
- **Done check.** Each collector cycle runs `howto.check_open`: for every open item with a `done_check` not run in
  the last 3 minutes, it runs the command (30 s limit); exit 0 closes the item (`done_reason: "howto-done"`, shown
  in "Answered" as "✓ Done: the check passed") and adds an `alert` flow `tirith → manager`. Last run per item in
  `<home>/howto-checks.json`.
- **Prompts.** `howto.RULES` is in the manager rules, the plan-refresh brief and the verification brief: look up
  every fact the machine can tell, skip steps already done, pre-fill commands, never ask for a secret in chat.

- The plan-refresh brief and the verification brief tell the lane this list, that a check an agent can do
  is never an owner check, and that out-of-plan items are never listed.
- `asks.is_owner_item` (the collector's last gate) also requires a valid `why_owner`; the guard reminder is an
  `owner-decision`. A brain guess or a room message that names the owner has none: it goes to the manager
  queue (`not-owner-item`).

### Only owner items reach the owner

Agents report to the manager. Everything the collector generates for the owner passes one function,
`asks.is_owner_item(item)`: it needs a one-click option or a Run action **and** a `source` in
`asks.OWNER_SOURCES` = `manual` (asks the manager or the dispatcher wrote with `tirith wait add`, plan owner
items, lane failures), `guard` (guard/setup actions), `setup`, `plan`, `lane`, `room` (a room message that
names the owner). `_enforce_cta` applies it to the final list; an item that fails it is queued for the manager
(`kind: "no-call-to-action"` without a call to action, `kind: "not-owner-item"` with a source that is not
allowed, id `sq-cta-<wait id>`, once). A new generator needs its source in the allow-list on purpose.

What the brain guesses from a **working** agent's log ("it seems to wait for X") is never an owner item. It is a
manager queue item, `kind: "agent-waiting"` (`asks.sync_agent_waiting`, every cycle):

- one open item per lane + text (id `sq-aw-<hash of lane and text>`), not added twice;
- closed by itself (`result: "no longer waiting"`) when the lane ends or leaves the list, or when the brain's
  answer for that lane no longer has the text (no change while the brain's answer is still pending). The same
  text said again after an automatic close opens a new item; one the manager closed (`tirith queue done`)
  stays closed;
- listed in the manager hook context (`SessionStart` / `UserPromptSubmit`): "Tirith queue: N working agent(s)
  look to be waiting ...", each with its `tirith queue done <id>`; it does not block `Stop`.

### Guard items (`guard-<project>`)

One item per project whose commit guard is not intact (`kind: "action"`). Its **actions** are generated by
the collector under stable ids and stored in `<home>/generated-actions.json` (`{id: action + project}`),
so the Run path (`tirith action run`, `tirith://run`, the app's `run_action`) finds them by id like any
other action; the command text only ever comes from there:

| Problem | Run action (label; command, run in the project's folder) |
|---|---|
| missing, differs, not executable, not immutable | `Install the word guard in <name>`: `tirith guard install <project>` |
| `pre-commit: present but without the FORBIDDEN_WORDS_GUARD marker` (the repo has its own hook) | `Add the word guard and keep <name>'s hook`: `tirith guard install <project> --chain` |

Options on both: `Not now|snooze` (hides the item for 7 days from the reply, stateless; it comes back
after) and `Never for this project|never` (the collector writes `guard = false` in the project's
`[[projects]]` block of `config.toml`; the check stops reporting, `projects[].guard` is
`{ok: true, problems: [], disabled: true}` and `lane run` skips the guard check).

## Photos and files in the chat (attachments)

The owner can attach images and any other file to a chat message (Overview "Manager" column, and its drawer):
Cmd+V in the composer, drag and drop onto the chat, or the paperclip. While files are dragged over the chat it
shows "Drop files to attach them"; a drop anywhere else does nothing (the page never navigates to the file).
Above the text box a picture shows as a thumbnail and any other file as a chip with its name and size; each
can be removed before sending. Up to 6 files per message, at most 10 MB each, none empty. A PNG, JPEG, GIF or
WebP is a picture only when its bytes say so, never because of its name.

```
app:     drop on the window -> HTML5 drop with the files (the main window turns the native drop handler off)
         (native handler on) -> tauri://drag-drop {paths, position} -> attachment_dropped(paths)
                                                         -> [{name, type, size, b64} | {name, error}]
         attachment_save(message_id, name, mime, bytes_base64) -> <home>/attachments/<message-id>/<n>.<ext> | <n>-<name>
         inbox_add(kind: "chat", ..., id, attachments)   -> inbox.jsonl row with `attachments`
browser: drop target as in the app, but it cannot write files -> "Attach files in the desktop app" (nothing is sent)
CLI:     tirith inbox add --kind chat --text T --attach FILE ...    (the file stays where it is)
manager: tirith inbox watch line / hook context list the absolute paths -> open the files
         tirith inbox answer <id> "<markdown>" --attach FILE ...    -> the answer shows them too
```

- The main window is built with `disable_drag_drop_handler()`, so the page gets the HTML5 `dragover`/`drop`
  events with the files, as in a browser, and reads them itself. A window that keeps Tauri's native handler
  takes the drop instead (the page gets no HTML5 drop with the files): Tauri's
  `tauri://drag-enter|over|leave|drop` events carry the pointer in physical pixels, the page divides by
  `devicePixelRatio` and marks the chat under it. `attachment_dropped` reads only paths of the last native drop
  (the app remembers them from `WindowEvent::DragDrop`), refuses folders, empty files and files over 10 MB with a
  reason per file, and types a picture by its bytes, anything else by its extension.
- `<message-id>` is the request id (`in-` and 8 hex digits). The page picks it, saves every file with
  `attachment_save`, then calls `inbox_add` with the same `id`. `attachment_save` refuses a bad id, an empty file
  or one over 10 MB, picks the file name itself (`<n>.png|jpg|gif|webp` for a picture, `<n>-<name>` with the name
  reduced to `[A-Za-z0-9._-]` for any other file; `n` counts from 1, at most 6 per message) and cannot write outside
  `<home>/attachments/`. A file that is not a picture keeps the `mime` the page gave when well formed and not one of
  the four picture types, else `application/octet-stream`. It returns `{path, type, size, name}`; the page adds
  `width` and `height` of a picture.
- `inbox_add` takes optional `id` and `attachments` (chat only): each path must be inside
  `<home>/attachments/` without `..` and each type a well-formed `type/subtype`, otherwise the request is refused.
  A chat message needs text **or** an attachment.
- Rows: a request or an answer may carry `attachments: [{path, type, size, width, height, name?}]` (absolute
  `path`; `width`/`height` are 0 unless `type` is one of the four picture types; `name` is the original file
  name). Malformed entries are dropped when the rows are read (at most 6 kept); `state.inbox[]` and
  `state.inbox[].answers[]` pass the list on unchanged.
- `tirith inbox add|answer --attach FILE` (repeatable) describes an existing file (checks size, reads a
  picture's type and dimensions from its bytes, guesses any other type from the name) and stores its absolute
  path; exit 3 with the reason otherwise.
- `tirith inbox watch` lines of a request with attachments gain `attachments` (the objects above, so the absolute
  `path`s). The manager hook context (`SessionStart`, `UserPromptSubmit`) lists each one under the request as
  `image: <path> (<type>, <w>x<h>)` or `file: <path> (<name>, <type>, <size> bytes)`.
- Web: thumbnails and file chips sit above the bubble, in the owner's messages and in the manager's answers; a
  click on a thumbnail opens the image full size in a lightbox (click or Esc closes); a file chip shows its path on
  hover. In the app the thumbnail of a message sent from this window comes from the picked file; others load
  `file://<path>`.

## The manager inside Tirith (`collector/tirith/managerd.py`)

The owner can close the terminal: chat messages are still answered. No server, no port.

- **Presence.** The interactive manager session (the hooks with `TIRITH_ROLE=manager`) writes
  `<home>/manager.json` `{pid, session, at}` on every `SessionStart`, `UserPromptSubmit`, `PostToolUse` and
  `Stop`. *Attached* = `at` younger than 120 s and `pid` alive. Lanes, headless or not, never write it.
- **On call.** Each dispatcher cycle, when nobody is attached and something is pending for the manager (a
  `manager_pending` request of any kind, a failed action of the owner, an open manager-queue item; not the
  "agent looks to be waiting" guesses, and a lane review only after 5 min) and no headless manager is running,
  Tirith starts ONE: `tirith lane run --class steward --feature "Manager" --agent <alias>`, the alias and
  model from `[models.manager]` (default alias `anthropic`), the command built like the workers' presets
  (`tirith.agentrun.build_command`: headless print mode, stream JSON). It runs in the folder of the first
  pending request's project (else the first project) and exits when done; the next pending item starts
  another. A manager turn is finished as soon as its log has a `result` event (or its process exits): the
  next pending item starts the next turn at once, without the 60 s retry wait (that wait only applies to a
  retry of the same set of ids). A manager process still alive 30 s after its `result` is terminated
  (SIGTERM to its process group; `managerd.reap_finished`); lanes it started are detached and survive.
  `[dispatch] auto_launch = false` or `enabled = false` switches it off. A set of pending ids that
  survives two headless runs is not started again (the usual escalation to the owner applies); a refusal to
  start is kept in `dispatch.json` `manager.error`.
- **Brief** (`<home>/briefs/manager-<hex>.md`, generated): the manager's rules (delegate everything with
  `tirith lane run` / `tirith launch`, never edit code, answer with `tirith inbox answer` then `inbox done`,
  owner items only for what it must refuse, no OS notifications), the pending messages with their attachment
  paths (`attachments[].path` on the request, when present), failed actions, the manager queue, a plan
  summary per project, the recent flows and its own notes.
- **Notes.** `tirith manager-note "<text>"` appends a timestamped line to `<home>/manager-notes.md` (last
  20 KB kept); the last 4 KB go into the next brief, so the next run knows what was decided and launched.
- **Detached lanes.** Lanes started by `tirith launch`, the autopilot, the dispatcher, the auto review and the
  headless manager go through `launch.spawn_detached`: new session (process group on Windows), stdin from
  `/dev/null`, output to `<home>/logs/<lane>.launch.log`. Closing a terminal or the app never kills them.
  `tirith lane run` typed in a terminal stays attached, except inside a manager session
  (`TIRITH_LANE=manager-…` or `TIRITH_ROLE=manager`): there it detaches by default (same as `--detach`),
  prints the lane id and the worktree and returns at once, so the manager's turn ends. `--no-detach` (hidden)
  is what the detached copy runs with; `--by-tirith` lanes never re-detach.
- **Worktrees.** `tirith lane run` without `--workspace` creates the git worktree
  `<repo>-lanes/<lane-id>` on branch `lane/<lane-id>` from the current `main` (HEAD when there is no `main`);
  an existing one is reused. The only lane that runs in the main checkout is the manager itself (class
  `steward`, id `manager-…`).
- **Dead lanes.** A lane with a `pid` that has no `ended_at`, whose pid and process group are gone and that
  started more than 30 s ago, is closed by the collector with `result: "stopped"` (shown as failed).
- **Settling.** The manager brief tells it not to close a request at launch: the request is settled when the
  lane reports (its report comes back as a queue item).
- **State.** `state.manager.presence = {mode: "live" | "on_call", working, lane}`. The chat header shows
  "Manager: live session", "Manager: on call (starts when you write)" or, while the headless manager runs,
  "Manager is working on your message…".

### One manager: a persistent session owned by Tirith (`collector/tirith/msession.py`)

`[manager] session = true` (default). The one-shot path above runs only with `session = false`, or for a cycle
in which the session could not start (`dispatch.json` `manager.session.error` says why).

- **The process.** The collector starts ONE agent process per Tirith home, the first time something is pending
  for the manager: `tirith lane run --interactive --id manager-<hex> --class steward --feature Manager
  --as-agent <alias> --by-tirith -- <command> <chat_args>` with piped stdin/stdout, in its own process group,
  in the first pending request's project folder. The alias and model/effort come from `[models.manager]`
  (default `anthropic`). `chat_args` is `[agents.<alias>] chat_args`, default for `anthropic`: `-p
  --input-format stream-json --output-format stream-json --verbose --permission-prompt-tool stdio`; Tirith adds
  `--include-partial-messages` (text arrives token by token) and `--resume <session-id>` when restarting.
  `lane run` registers the lane and ends it however the agent exits. The session lives in the collector
  process: a collector restart ends it, the next start resumes the conversation.
- **Messages.** Pending chat messages, failed actions and queue items are written to stdin as ONE user
  message per turn, only while the session is idle; what arrives during a turn waits (the chat shows it as
  "Queued") and goes into the same session when the turn ends. The first message of a conversation carries the
  whole brief (rules, plan summary, the notes in `manager-notes.md`); later ones only what is new. A turn ends
  with the agent's `result` event. A request still open when its turn ends gets the turn's text as the inbox
  answer (`by: manager`, not closed); the manager normally answers with `tirith inbox answer` itself. A
  request is offered at most twice (`MAX_TRIES`); an interrupted turn is never offered again.
- **Dying and restarting.** If the process dies it is started again (after 15 s, times the number of fast
  deaths in a row) with `--resume <session-id>`. A resumed session that dies within 20 s is started fresh the
  next time: the first message then repeats the brief and the notes (the handoff). Failed manager lanes pause
  the manager as before (two in a row, one `sq-manager-paused-…` queue item); lanes ended on purpose
  (`stop` marker) do not count and raise no queue item. A session that cannot start falls back to the one-shot
  manager.
- **Transcript.** Every stdout line goes to `<home>/manager/transcript.jsonl`, rotated at 16 MB (3 files
  kept). Each turn is also kept as CLI-style steps: `<home>/manager/turns/<turn-id>.json` (last 30) with
  `{id, requests, started_at, ended_at, summary, steps, answer, error, interrupted}`; steps are the
  `livesteps` kinds `text` (`partial: true` while it is typed), `tool`, `result`, plus `todo` (the TodoWrite
  list, one step per turn, updated in place) and, on Edit / MultiEdit / Write tool steps, `diff: {path, rows:
  [["-"|"+", line]], more}`. Secrets are masked in steps; the transcript is raw and private to the home.
- **State.** `state.manager.turns[]`: one row per turn, shaped like an agent row (`id manager-turn-<hex>`,
  `status`, `started_at`, `ended_at`, `turn_requests`, `live` with the steps file `agents/<id>.steps.js`) plus
  `lanes` (the sub-agent lanes whose parent is the session lane and that started during the turn) and
  `interrupted`. `state.manager.presence.session = {lane, busy, turns, context_tokens, asks}` while the session
  runs; `working` is true while a turn runs.
- **Control.** `tirith manager say <text>` (a chat message, queued into the session), `interrupt` (ends the
  running turn; the session lives on), `stop` (ends the session; a chat message written after the stop starts
  it again), `restart [--fresh]` (resume, or with `--fresh` a new conversation with the handoff), `status`.
  They reach the collector through `<home>/manager/control.jsonl` (picked up on the next cycle, ~2 s). The app
  has the Tauri command `manager_control(op)`; the page's Stop button and Esc call it (in a page opened from
  a file the CLI line is shown instead).
- **Permissions.** A `can_use_tool` question from the stream is decided by code first
  (`msession.decide`): denied when a gate says so (`gates.check_call`: no `--no-verify`, protected files, no
  agent CLI outside a lane, ...) or when it edits a file outside the Tirith home (the manager delegates edits);
  allowed for read-only tools and for the allow rules of `[agents.<alias>] permissions` (`Tool`,
  `Tool(prefix:*)`, `Tool(exact)`; a Bash line only when every command in it matches, never with `$(`, a
  backtick or a redirect). Everything else is an owner decision: a `waits.jsonl` item `perm-<request-id>` (kind
  `approval`, options Allow / Deny, source manual, so it passes the owner gate) that the chat shows as buttons.
  The reply is written back as a `control_response` and the item is closed; a question left when the turn ends
  is closed too.
- **Context.** `[manager] context_tokens` (150000) and `max_turns` (40): when an idle session's last context
  size is above the first, or the conversation is longer than the second, `/compact` is sent once as its own
  turn; if the context is still large (or it gets long again) the session is ended and the next one starts
  fresh with the handoff.
- **Chat.** Under each owner message the page draws the turn that answers it (spinner, the newest steps, the
  task list, coloured diffs, the lanes it started as nested blocks), collapsing to "N steps · 48s · $0.21 · show"
  when the turn ends. Esc stops the running turn; Up on an empty box brings back the last message. The app-only
  agent chat (`v-chat.js`, `chat_start` ...) is gone.

### Sending never loses a message

- The page gives each chat send a `cid` and shows the message as "Sending…" in the thread until the next state
  has a row with that `cid`. The Tauri command failing, or no such row within 10 s, turns it into
  "Not sent · Retry · Copy text" and puts the text back in the composer. Retry resends with the same `cid`.
- Drafts are kept in `localStorage` per project (`tirith.chat.draft.<project>`), and sends still unconfirmed are
  kept in `tirith.chat.unsent`; after a reload they come back as "Sending…" and are checked against the state.
  Storage that is blocked only means nothing survives a reload.
