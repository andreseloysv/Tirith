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

## `state.projects[].roadmap`

```json
{"file": "/abs/docs/TODO.md", "deadline": "2026-11-04T21:00:00-05:00", "done": 23, "total": 61,
 "steps": [{"id": "3", "title": "Hooks de agentes", "status": "active", "done": 7, "total": 11,
            "items": [{"text": "…", "done": true}], "agents": ["lane-id"]}]}
```

Step `status`:

| Status | Rule |
|---|---|
| `empty` | no items |
| `done` | every item checked |
| `active` | some checked, or a working lane is tagged with this step |
| `todo` | none checked and no working lane |

`agents` are the ids of lanes with `status == "working"` in the same project whose `step` equals the
step id.

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
