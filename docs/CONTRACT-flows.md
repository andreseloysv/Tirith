# Contract addition: information flows (v1)

Tirith shows how information moves between the Steward (orchestrator), the agents, the owner and
outside rooms, not only the results.

## Input: `~/.tirith/flows.jsonl` (one JSON object per line)

Written by `tirith flow --from <endpoint> --to <endpoint> --kind <kind> --text "<one line>" [--ref <path>]`
and automatically by other commands:

| Trigger | from → to | kind |
|---|---|---|
| `tirith lane start` | `steward` → `agent:<lane>` | `brief` (ref = brief file) |
| `tirith lane end` | `agent:<lane>` → `steward` | `report` (text = done/failed + first line of the report) |
| `tirith wait add` | `steward` → `owner` | `request` |
| `tirith wait done` | `owner` → `steward` | `answer` |
| guard problem appears | `guard` → `owner` | `alert` |
| pipeline run ends | `pipeline:<run>` → `steward` | `report` |
| git hook run (`events.jsonl` `kind: "git-hook"`) | `guard` → `pipeline:git-hooks#<n>` | `message` (allowed) / `alert` (blocked) |

Git hook runs also form a `git-hooks` pipeline per project (stages `pre-commit`, `commit-msg`,
`pre-push`; the latest result of each, blocked = failed). Installed hooks (guard version 2) write
`{kind:"git-hook", hook, repo, result, reason, at, lane}` and run `tirith.toml` `[[hooks]]` through
`tirith run --stage <event> -- <run>`; `pre-push` runs `tirith check`.

Endpoints: `steward`, `owner`, `guard`, `tirith`, `brain`, `agent:<lane-id>`, `room:<room-id>`, `pipeline:<run>`.
Kinds: `brief`, `message`, `report`, `request`, `answer`, `decision`, `merge`, `alert`.

```json
{"id": "f-17", "at": "2026-10-28T20:40:00Z", "from": "steward", "to": "agent:s-world",
 "kind": "message", "text": "Owner wants a much smaller compact world", "ref": null}
```

## Output: `state.json` → `flows`

The last 100 flows, oldest first, same shape plus resolved display names:
`{"id","at","from","to","from_name","to_name","kind","text","ref"}` where names are the hero name for
agents ("Halvard Stonehand"), "The Steward", "You" (owner), the room id, or the run id.

## Decisions (`kind: decision`)

`tirith decide "<decision>" --why "<reason>" [--lane <id>]` writes a `decision` flow with an extra
`why` field: `agent:<lane>` → `steward` (lane from `--lane` or `$TIRITH_LANE`), or `steward` → `owner`
without a lane. The collector also extracts decisions from lane reports, once per decision: only lines
that start with `Decision:` or `Decided:` (bullets and bold allowed). The reason is the inline
`because …` / `(why: …)` part or a `Why:` line right after. Lines under a heading such as "Left open",
"Next steps" or "Questions" are never decisions. A decision already recorded with `tirith decide` is
not duplicated.

`state.json` flows carry `why` when there is one. The web shows decisions as small labels on the
agent ↔ steward edge in Flow (click: the reason) and as a "Decisions" list in the agent panel.
