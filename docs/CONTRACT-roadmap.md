# Contract addition: the project's plan (v1)

Each project's plan lives in a markdown checklist (for Tirith itself `docs/TODO.md`). The collector
reads it every cycle; there is no second, hand-written copy.

## Where the file is

`tirith.toml`, top level:

```toml
roadmap = "docs/TODO.md"                  # path relative to the project folder (or absolute)
deadline = "2026-11-04T21:00:00-05:00"    # optional ISO-8601, shown as a countdown
plans = ["docs/TODO.md", "docs/plans/beta.md"]   # optional: every plan of the project (see "Several plans")
```

Without `roadmap` the collector tries `docs/TODO.md`, then `TODO.md`. No file → `roadmap: null`.

## File format

```markdown
Any intro text is ignored.

## 3. Hooks de agentes

- [x] A finished item.
- [ ] An open item that continues
      on the next line.
  - [ ] Nested items count too (any indent).

## Orden
Headings without a number are not steps; their text is ignored.
```

A step is a `## <n>. <title>` heading, `id` is `<n>` as a string. Items are `- [ ]` / `- [x]` lines
(`*` and `+` bullets work); indented non-bullet lines directly below an item are joined to it.
Files over 256 KB are ignored. Parsed results are cached by file mtime and size.

## Item tags

An item whose text contains one of these tags (case-insensitive) is not for an agent:

| Tag | Meaning | `who` |
|---|---|---|
| `(dueño` / `(owner` | only the owner can do it | `owner` |
| `(desde YYYY-MM-DD` / `(from YYYY-MM-DD` | blocked until that day (inclusive); from then on it is an agent's | `date` |
| `(fuera de este plan` / `(out of this plan` | not part of this plan: excluded from `done` / `total` and from the step status | `out` |
| none | an agent can do it | `agent` |

### The owner tag

`(dueño: <reason> <what>)`, with the reason key first: `credentials`, `payment`, `personal-account`,
`physical-device` or `owner-decision`, optionally followed by plain words in parentheses (`personal-account
(Apple account)` → "Needs your Apple account"). Without a valid reason the item is not an owner item: the owner
gate queues it for the manager as "agent can do" and a verification lane does it (see
`docs/CONTRACT-inbox.md`, "The owner gate"). What the tag may carry:

| In the tag | Owner item |
|---|---|
| a command in backticks, e.g. `(dueño: run `uv run tirith guard reinstall relagame`)` | a **Run** action with that command, cwd = the project folder (for Tirith, `collector/`), label = the sentence |
| a URL | the item's **Open** link, option `Done` |
| nothing | options `Done` / `Not now` |

The owner item's text is the item's sentence only: no step title, no project/step prefix, no tag
(`Reinstalar la guarda nueva en RelaGame.`). A command or URL outside the tag still works (older plans).
`roadmap.owner_parts(text)` returns `{sentence, command, url, why_owner, why_detail, label}`. Never write "botón Run" in the
item: the generator drops a promise of a button the item does not have.

`out` wins over `owner`, `owner` over `date`. A malformed date is not a tag. Done items keep their `who`
but never raise owner items.

## Item states

Tirith keeps a status note at the end of an item's text, so the plan file itself says where each item stands:

| Note | State | Meaning |
|---|---|---|
| (none) | `open` | an agent can take it |
| `(en curso: <lane>)` | `progress` | the lane works on it |
| `(en revisión)` | `review` | its agent reported it done; ticked `[x]` when the review passes |
| `(bloqueado: <reason>)` | `blocked` | its agent reported it blocked |
| `(reabierto: <reason>)` | `reopened` | a review failed or the lane ended without a report; an agent can take it again |
| `[x]` | `done` | |

An item is found by its text without the note (or by its id `i-` and 8 hex digits of that text). Items in
`progress`, `review` or `blocked` are not "agent items" (the autopilot and `tirith launch` skip them, and a step
with only those is `todo`, or `active` while a lane works on it). `state.projects[].roadmap.steps[].items[]` has
`state` and, for `progress` (the lane), `blocked` and `reopened` (the reason), `detail`. The Plan shows the state as
a chip ("In progress · <agent>", "In review", "Blocked: …", "Reopened: …") and the Agents view shows the item of
each lane (`agents[].item`, `agents[].outcome`).

Step `0` "Trabajo automático de Tirith" holds Tirith's own work (reviews of other items are sub-items instead). It
is created on first use, and the autopilot never starts a lane on it.

## The plan gate

*The plan is the only way to start and to finish an agent* (`tirith/plangate.py`):

- **Single writer.** Only Tirith writes the plan file, on the project's main checkout, one commit per change
  `Roadmap: <what> in step N (<who>)` (`<who>` is `owner`, `manager`, a lane id or `tirith`), through the normal
  hooks, serialized with a lock (`<home>/locks/roadmap-<project>.lock`). A plan with uncommitted changes is never
  written. The `roadmap-edit` gate (`docs/CONTRACT-gates.md`) blocks every agent's edit of the file.
- **Start = a linked open item**, no exceptions: `--step N --item "…"` checked by code (`docs/CONTRACT-launch.md`).
- **`tirith plan add --project P --step N --text "…" [--title "…"]`** adds an open item at the end of the step
  (`--title` creates the step when it is not in the plan). It is the manager's way to update the TODO, e.g. to add a
  review; the owner's plan edits from the Plan card (below) use the same writer.
- **Finish = an outcome**: `tirith plan report --lane L --status done|partial|blocked [--add …]… [--reason …]`.
- **Done only when verified**: the review ticks the item, or reopens it with the reason.

## `state.projects[].roadmap`

```json
{"file": "/abs/docs/TODO.md", "deadline": "2026-11-04T21:00:00-05:00", "done": 23, "total": 61,
 "steps": [{"id": "3", "title": "Hooks de agentes", "status": "active", "done": 7, "total": 11,
            "items": [{"text": "…", "done": true, "who": "agent"},
                      {"text": "… (dueño: …)", "done": false, "who": "owner", "owner_item": "owner-tirith-3-1a2b3c4d"},
                      {"text": "… (desde 2026-11-02)", "done": false, "who": "date", "until": "2026-11-02"}],
            (each item also has "state": "open|progress|review|blocked|reopened|done" and, when it has one, "detail")
            "agents": ["lane-id"]}]}
```

Step `status` (checked in this order; `out` items are ignored everywhere):

| Status | Rule |
|---|---|
| `empty` | no items |
| `done` | every item checked |
| `active` | a working lane is tagged with this step (an agent is on it) |
| `ready` | has open `agent` items and nobody on it: the autopilot starts a lane |
| `blocked-owner` | only open `owner` items (maybe also dated ones) are left; `owner_item` on each is the id of its owner item |
| `blocked-date` | only open `date` items are left; the step has `until` (the earliest date) |
| `todo` | anything else (not reachable with a well-formed plan) |

`agents` are the ids of lanes with `status == "working"` in the same project whose `step` equals the
step id.

## The autopilot

`collector/tirith/autopilot.py`, called by the dispatcher every cycle. On by default:

```toml
[autopilot]
enabled = true       # false: no lane is started by itself (the owner's on/off request wins over the file)
max_parallel = 4     # autopilot lanes running at once
```

- For every project with a roadmap, one lane per `ready` step, lowest step first (file order), never two on
  the same step, at most `max_parallel` autopilot lanes running. The launch is the same code as
  `tirith launch`, and the brief lists only the step's `agent` items (`tirith launch` does the same by
  hand; a step with no agent item is refused with "has no items an agent can do now").
- A step is not started again while a lane of that step waits for its review or has a failed review.
- Every launch is a flow `tirith -> agent:<lane>` ("Autopilot started <project> step <n>") and a row in
  `state.autopilot.started`; the Plan shows it under the step as "Autopilot started <agent>".
- **Not converging:** each ended autopilot lane (and its review settled) is counted once. When it ticked none
  of the items it was started for, the step's counter goes up, otherwise it resets. At 2 the step is paused:
  a manager queue item `sq-autopilot-<project>-<step>` (kind `autopilot`, "step N is not converging") and an
  alert flow. The step resumes by itself when its open agent items change (the queue item is closed then).
- **A refused launch** (check fails, no agent command, …) is one manager queue item `sq-autopilot-<project>-<step>`
  (`kind: "launch-refused"`, with the fix and retry commands), never an owner item; retried every 10 minutes.
- **Owner items:** for every open `owner` item one owner item with the id of `owner_item`
  (`owner-<project>-<step>-<hash of the text>`), once. The text is the item's clean sentence and
  `why_owner` / `reason_tag` say why it is the owner's (see "The owner tag"; passed through the owner gate). A command (in the tag, else in the
  text) becomes a Run action; a URL becomes the item's `link` with the option `Done`; otherwise the options
  are `Done` / `Not now`. When the item is ticked (or removed) the owner item is
  closed (`done_at`).
- State is in `<home>/dispatch.json` under `autopilot` (`on`, `lanes`, `stuck`, `paused`, `owner_items`, `refused`).

`state.autopilot`:

```json
{"enabled": true, "max_parallel": 4,
 "started": [{"lane": "t-world", "project": "tirith", "step": "3", "at": "2026-10-28T19:21:00Z"}],
 "paused_steps": [{"project": "tirith", "step": "5", "at": "…"}]}
```

## Web

The Plan shows per step: `done` ✓, `active` the agent avatar(s), `ready` "Queued for the autopilot",
`blocked-owner` a person icon and "Waiting for you" (click: the owner item in Your turn),
`blocked-date` a calendar icon and "From Nov 2". The header reads "Autopilot on · 2 agents working · 3 queued"
with a Pause / Resume button (inbox request `autopilot`, see `CONTRACT-inbox.md`). Under a step only the
latest thread entry is open; older ones fold into "N earlier messages".

## Lanes

`tirith lane run --step <id>` and `tirith lane start --step <id>` store `step` on the lane; it appears
as `agents[].step` (string or null).

## Several plans

`plans` lists the plan files of the project (paths like `roadmap`; missing files are skipped, a file listed
twice counts once). `roadmap` stays the **current plan** and is always shown first; when it is not set (or its
file is missing) the first existing entry of `plans` is the current plan, and `state.projects[].roadmap` is that
one, so everything that reads `roadmap` keeps working. A plan file has the same format as above, plus two
optional things in its head (before the first `##`): the `# <title>` heading (a leading `TODO:` or `Plan:` is
dropped) and a line `Deadline: <ISO-8601>`. "Start a new plan" makes a planner lane write
`docs/plans/<slug>.md`, add it to `plans` (with the old current plan, so it stays listed) and set `roadmap` to it.

### `state.projects[].plans`

```json
[{"id": "beta", "title": "Beta launch", "current": false, "finished": false,
  "file": "/abs/docs/plans/beta.md", "deadline": "2026-12-01", "done": 1, "total": 5, "steps": [ ... ]}]
```

Each entry is shaped like `roadmap` (`file`, `deadline`, `done`, `total`, `steps` with the same step status
rules) plus `id` (the file name without `.md`), `title` (heading, else the id), `current` (true for the first
entry only) and `finished` (`total > 0` and every item checked). The `deadline` of a non-current plan is its
`Deadline:` line (the manifest `deadline` belongs to the current plan, whose own line wins when present).
Lanes tagged with a step (`agents`) belong to the current plan only. Empty list when the project has no
plan. The Plan card shows a switcher: the current plan and the unfinished ones as chips, finished plans
folded under "N done"; other plans are read only (no step buttons).

## Plan buttons

The Plan card header has **Update the plan** and **Start a new plan**; they send the inbox requests
`plan-refresh` and `plan-new` (`docs/CONTRACT-inbox.md`), which Tirith turns into an auditor lane and a
planner lane.

## Editing the plan from the Plan card

Each open item of the current plan has **Edit** and **Delete** on hover, and each step has **Add item**. They
send a `plan-edit` request (`CONTRACT-inbox.md`) and the dispatcher changes the file by code
(`roadmap.edit(text, step_id, item_text, new_text | delete | add)`):

- The item is found by exact text inside the step (whitespace-normalised, a wrapped item is one item) and
  must be open. No match raises `roadmap.EditError` ("that item changed in the plan meanwhile") and the file is
  left as it is.
- An edit keeps the bullet, indent and `[ ]`; the new text is written on one line (a wrapped item becomes one
  line). A delete removes the item with its wrapped lines. An add appends `- [ ] <text>` after the step's last
  open item (or after its last item, or at the top of an empty step).
- The dispatcher refuses when the plan file has uncommitted changes, otherwise it writes the file and commits
  only that file with `Roadmap: <edit|delete|add> in step N (owner)` through the normal hooks; if the commit
  is refused the file is restored.
- After a change it queues one `plan-refresh` (Update the plan) for the project, unless one is already
  queued, so the auditor checks the plan again.
