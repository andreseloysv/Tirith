# Contract addition: the project's plan (v1)

Each project's plan lives in a markdown checklist (for Tirith itself `docs/TODO.md`). The collector
reads it every cycle; there is no second, hand-written copy.

## Where the file is

`tirith.toml`, top level:

```toml
roadmap = "docs/TODO.md"                  # path relative to the project folder (or absolute)
deadline = "2026-11-04T21:00:00-05:00"    # optional ISO-8601, shown as a countdown
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

`out` wins over `owner`, `owner` over `date`. A malformed date is not a tag. Done items keep their `who`
but never raise owner items.

## `state.projects[].roadmap`

```json
{"file": "/abs/docs/TODO.md", "deadline": "2026-11-04T21:00:00-05:00", "done": 23, "total": 61,
 "steps": [{"id": "3", "title": "Hooks de agentes", "status": "active", "done": 7, "total": 11,
            "items": [{"text": "…", "done": true, "who": "agent"},
                      {"text": "… (dueño: …)", "done": false, "who": "owner", "owner_item": "owner-tirith-3-1a2b3c4d"},
                      {"text": "… (desde 2026-11-02)", "done": false, "who": "date", "until": "2026-11-02"}],
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
- **A refused launch** (check fails, no agent command, …) is one owner item `autopilot-<project>-<step>` with
  `Fix it` / `Try again`, retried every 10 minutes.
- **Owner items:** for every open `owner` item one owner item with the id of `owner_item`
  (`owner-<project>-<step>-<hash of the text>`), once. The text names the project, step and item. A command
  in the text (code span or fence) becomes a Run action; a URL becomes the item's `link` with the option
  `Done`; otherwise the options are `Done` / `Not now`. When the item is ticked (or removed) the owner item is
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
