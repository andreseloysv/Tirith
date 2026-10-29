# Contract addition: agentsmessenger.com rooms (proposal, v1)

Tirith shows the public rooms where agents talk (https://agentsmessenger.com). The collector reads
them (HTTP GET only) and adds a `rooms` array to `state.json`; readers only read it. The key is
optional for old readers (`state.example.json` has none); a collector that supports rooms always
writes it (`[]` when no room is configured).

## Input: `~/.tirith/config.toml`

```toml
[owner]
names = ["andres"]            # speaker names that are you; messages naming you raise "needs you"

[messenger]
interval_s = 15               # poll interval per room, never below 10

[[rooms]]
id = "meta-hackathon-ideas"   # https://agentsmessenger.com/r/<id>
project = "tirith"            # optional
title = "Hackathon ideas"     # optional (default: the id, capitalised)
aliases = { "muse" = "t-world" }   # room speaker name -> Tirith lane id
```

CLI: `tirith room add <room> [--project P] [--alias NAME=LANE]`, `remove`, `list`,
`tail <room> [-n N]` (GET), and `say <room> --as <name> "<text>" [--re N]` (the only POST).

Cache: `~/.tirith/rooms/<room>.json` holds the last 100 messages, the cursor (`after`), who is
present and the last error. The poller is incremental (`after=<last n>`), runs on its own thread,
backs off exponentially after errors (15 s, 30 s, 60 s ... max 300 s) and never blocks the 2 s loop.
Only `agentsmessenger.com` and the Meta Model API are contacted.

## Output: `state.json` -> `rooms`

```json
{
  "rooms": [{
    "id": "meta-hackathon-ideas", "url": "https://agentsmessenger.com/r/meta-hackathon-ideas",
    "title": "Meta Hackathon Ideas", "project": "tirith",
    "summary": "One sentence from the Muse brain on what the room discusses now.",
    "summary_source": "muse|fallback|none",
    "needs_owner": [{"n": 7, "from": "tinker-bot", "reason": "asks you to confirm the format",
                     "source": "brain|room"}],
    "message_count": 8, "last_message_at": "2026-10-28T19:36:02.000Z",
    "activity_per_hour": 4, "messages_last_day": 8,
    "polled_at": "2026-10-28T19:39:50Z", "error": null,
    "participants": [{"name": "muse", "kind": "tirith-agent|external|owner",
                      "hero_id": "t-world", "hero_name": "Marrow",
                      "last_seen": "...", "messages": 3, "present": true}],
    "reply_edges": [{"from": "muse", "to": "ghost-writer", "count": 1}],
    "messages": [{"n": 4, "at": "...", "from": "muse", "text": "...", "re": [3],
                  "agreed": ["ghost-writer"], "signed": "self|platform|none",
                  "kind": "message", "truncated": false}]
  }]
}
```

Rules:
- `messages` are the last 40 (oldest first); `re` only lists message numbers present in `messages`'
  cache. `agreed` is the list of names that signed agreement. `signed`: `self` = the sender signs with
  its own key, `platform` = the platform vouches for the name, `none` = unsigned.
- `participants[].kind`: `owner` when the name is in `[owner] names`; `tirith-agent` when the name
  is an alias, a lane id or a hero name (`hero_id` is the lane id, `hero_name` is null when that lane
  is not currently in `agents`); otherwise `external`. Sorted by message count.
- `summary` comes from the brain (cached by transcript hash); without a brain it is
  `"Latest from <name>: <text>"` with `summary_source: "fallback"`.
- "Needs the owner": the brain flags messages addressed to you (`source: "brain"`); without a brain,
  a message naming one of your names that you have not answered afterwards (`source: "room"`).
  Each also appears in `waiting_on_you` as `{"id": "room-<room>-<n>", "link": <room url>,
  "source": "brain|room"}` (the wait `source` enum gained `room`).
- Times are the service's ISO-8601 UTC strings (milliseconds kept in message `at`).

New brain jobs (cached by input hash): 5. room summary, 6. room "needs the owner" detection.

## Flows

`flows` (see `CONTRACT-flows.md`) is a sibling array in the same file; endpoint `room:<id>`
refers to the `rooms[].id` above.
