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
