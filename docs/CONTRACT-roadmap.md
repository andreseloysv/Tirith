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
  written. A git `index.lock` left in the main checkout (older than 60 s) is removed
  before the commit, and the commit retried once if git reports it. The `roadmap-edit` gate (`docs/CONTRACT-gates.md`) blocks every agent's edit of the file.
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
| `ready` | has open `agent` items and nobody on it: the autopilot tells the manager (it starts nothing) |
| `blocked-owner` | only open `owner` items (maybe also dated ones) are left; `owner_item` on each is the id of its owner item |
| `blocked-date` | only open `date` items are left; the step has `until` (the earliest date) |
| `todo` | anything else (not reachable with a well-formed plan) |

`agents` are the ids of lanes with `status == "working"` in the same project whose `step` equals the
step id.

## The autopilot

`collector/tirith/autopilot.py`, called by the dispatcher every cycle. **The autopilot never starts an agent**:
three actors starting agents collided (duplicates, generic briefs replacing detailed ones), so one brain decides:
the manager. On by default:

```toml
[autopilot]
enabled = true       # false: no proposals (the owner's on/off request wins over the file)
max_parallel = 4     # kept for old configs; nothing is launched by the autopilot any more
```

- **Proposals.** Every open `agent` item (not ticked, not owner-only, not `out`, not in step 0) that nobody works
  on is reported to the manager: no lane `working` on its step and no review `pending`, `running` (a review whose
  process died counts as running: the self-check runs it again) or `ready` (being merged). Whatever the item's
  note says (open, `en curso`, `en revisión`, `reabierto`, `bloqueado`, or a `desde` date that has passed) it is
  reported after a ~30 s debounce (`DEBOUNCE_S`), so a normal handoff ("lane ended -> review starting") is not.
- A proposal is an inbox request of kind `autopilot-proposal` from `tirith` (project, step, text): the item,
  why it is open, the plan file and step, and the linked lane, the last 3 attempts of that step with outcome,
  review status (merged or not), claims result and the first paragraph of the report. **One per item and
  state**: a new proposal follows only when the item's text or note changes (the old unanswered one is closed
  quietly, with no answer in the chat). When the item is ticked or removed an unanswered proposal is closed
  the same way. A flow `tirith -> manager` ("Autopilot proposes <project> step <n>: …") records each.
- **The manager decides** like for an owner message: launch (with a written brief and the plan link), combine with
  a lane, wait (say why), or drop (a `plan-edit` through Tirith; `tirith plan done` when the work is on main). It
  answers on the proposal with one short line ("Autopilot: step 9 item … -> started lane X") and closes it; the
  chat shows the proposal as an "Autopilot" line and the answer as a manager line. A proposal is never escalated
  to the owner. A proposal unanswered for more than 10 min is a manager-health problem (`CONTRACT-selfcheck.md`).
- **The switch** is the Plan's Pause / Resume (inbox request `autopilot`). Off means no proposals; opening
  proposals stay with the manager. The value is kept in `<home>/autopilot.json` (`{"on": bool}`), which wins
  over a lost or stale `dispatch.json`, so it survives any restart or reload.
- **Skipped reasons** (plain words, never "waiting for you to review": reviews are Tirith's own job): "an agent is
  already on it", "the work of an earlier agent on it is being checked by Tirith", "… is checked and Tirith is
  merging it", "only the owner's items are left". A failed review is not skipped: it is proposed, with its facts.
- **Owner items:** for every open `owner` item one owner item with the id of `owner_item`
  (`owner-<project>-<step>-<hash of the text>`), once. The text is the item's clean sentence and
  `why_owner` / `reason_tag` say why it is the owner's (see "The owner tag"; passed through the owner gate). A command (in the tag, else in the
  text) becomes a Run action; a URL becomes the item's `link` with the option `Done`; otherwise the options
  are `Done` / `Not now`. When the item is ticked (or removed) the owner item is
  closed (`done_at`).
- State is in `<home>/dispatch.json` under `autopilot` (`on`, `owner_items`, `proposals`: item key -> request id and
  fingerprint, `seen`: when an item was first seen unattended).

`state.autopilot`:

```json
{"enabled": true, "max_parallel": 4, "working": 2, "steps_working": 2, "steps_in_review": 1,
 "proposed": 1, "queued": 0, "open_items": 6, "started": [], "paused_steps": [],
 "skipped": [{"project": "tirith", "step": "9", "why": "the work of an earlier agent on it is being checked by Tirith"}]}
```

`started` and `paused_steps` are always empty (kept for old readers). `proposed`: open items whose proposal is
still open; `queued`: items about to be proposed (inside the debounce).

## Web

The Plan shows per step: `done` ✓, `active` the agent avatar(s), `ready` "Open: the manager decides",
`blocked-owner` a person icon and "Waiting for you" (click: the owner item in Your turn),
`blocked-date` a calendar icon and "From Nov 2". The header says what is true: "Paused", or "Autopilot on · 2 steps in progress · 1 waiting for a review · 1 proposed
to the manager"; never "nothing to start" while open items exist (then "N open items for the owner or a later date",
or "all done"). It has a Pause / Resume button (inbox request `autopilot`, see `CONTRACT-inbox.md`). Under a step only the
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
