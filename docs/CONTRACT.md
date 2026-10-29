# Tirith data contract (v1)

Tirith has two halves that only meet through files in `~/.tirith/` (overridable with `TIRITH_HOME`):

- **`tirith` CLI + collector** (`collector/`, Python 3.12 via uv): the orchestrator, agents and
  pipelines write small events; the collector merges them with live facts (processes, logs, git) and
  the Muse "brain", and writes `state.json` atomically (write temp + rename) every 2 s.
- **The world** (`world/`, Godot 4.7): reads `state.json` (polling its mtime every second), never
  writes. It must run against `fixtures/state.example.json` with no collector at all.

No servers, no open ports. The only network access is the collector's optional calls to the
Meta Model API for the brain (see below); everything works without it.

## Inputs written by the CLI (`~/.tirith/`)

| Path | Written by | Content |
|---|---|---|
| `config.toml` | owner | projects: `id`, `name`, `repo` path, `color`; brain settings |
| `lanes/<lane-id>.json` | `tirith lane start` / `tirith lane end` | lane registration (below) |
| `events.jsonl` | `tirith run`, `tirith hook` | pipeline stage events (below) and agent hook events (`kind: "hook"`, see `CONTRACT-hooks.md`), one JSON object per line |
| `notes.jsonl` | `tirith note` | `{"at","project","text"}` orchestrator decisions |
| `waits.jsonl` | `tirith wait add/done` | `{"id","at","project","text","link","done_at"}` |
| `orchestrator.json` | `tirith steward "<activity>"` | `{"activity","since"}` what the Steward is doing now |
| `agents.json` | collector | persistent hero identity per agent id: name, epithet, class, renown |
| `brain/` | collector | cached Muse outputs (summaries, architecture) keyed by input hash |

Lane registration (`tirith lane start --project chromasol --class stonewright --feature "Room database"
--workspace <dir> --log <file> --pid <pid> --brief <file>`):
```json
{"id": "w1-data", "project": "chromasol", "class": "stonewright", "feature": "Room database",
 "workspace": "/abs/dir", "log": "/abs/log.txt", "pid": 12345, "brief": "/abs/brief.md",
 "started_at": "2026-10-28T10:00:00Z", "ended_at": null, "result": null}
```
`tirith lane end <id> --result done|failed` sets `ended_at` and `result`.

`tirith lane run --project P --class C --feature F --brief <file> [--interactive] -- <agent>` registers
the lane (pid, parent, session, start commit, token, step, request), runs the agent with `TIRITH_LANE`
and the token in its environment, then ends the lane (`done` on exit 0, `failed` otherwise) and prints
`lane <id>: <result>, <n> files +<a> -<d>, <time>`. Same refusals either way (brief, `tirith check`,
guard: exit 3). Default: output goes to a log (`log`), the answer is extracted from it.
`--interactive`: the child inherits stdin/stdout/stderr (the owner chats with it), `log` is `null`;
activity comes from `tirith hook` events tagged with the lane. SIGINT/SIGTERM/SIGHUP received by
`tirith` are forwarded to the child, which is always waited for, so the lane always ends (a child killed
by a signal exits `128+signal` and the lane is `failed`). The final answer is the summary of the lane's
last `Stop` hook event, if any (else the lane ends without an answer and a wait is added).

Pipeline events (`tirith run --project chromasol --pipeline android-release --stage build -- make android-build`):
```json
{"at": "...", "run": "android-release#7", "project": "chromasol", "pipeline": "android-release",
 "stage": "build", "event": "start|end", "status": "running|passed|failed", "log": "/abs/stage.log"}
```
`tirith run` executes the command, streams it to the stage log, and returns its exit code.
`tirith pipeline declare <pipeline> --stages build,test,lint,sign,device,store,review` fixes the order
(stages not yet run show as `pending`).

## Output: `state.json`

```json
{
  "version": 1,
  "generated_at": "2026-10-28T19:40:00Z",
  "brain": {"enabled": true, "model": "muse-spark", "last_error": null},
  "steward": {"name": "Aldric of the Tower", "title": "The Steward", "activity": "Reviewing W6",
              "since": "...", "notes": [{"at": "...", "project": "chromasol", "text": "Merged W5"}]},
  "waiting_on_you": [{"id": "w-3", "project": "chromasol", "text": "Tick the AI asset declaration",
                      "since": "...", "link": null}],
  "projects": [{
    "id": "chromasol", "name": "ChromaSol", "color": "#2f7d5b",
    "architecture": {
      "nodes": [{"id": "engine", "label": "Colour engine (C)", "kind": "core", "files": 14}],
      "edges": [{"from": "subject", "to": "engine", "label": "masks"}],
      "generated_at": "...", "source": "muse|manual|none"}
  }],
  "agents": [{
    "id": "w1-data", "project": "chromasol", "parent": "steward|<lane id of the spawning agent>",
    "name": "Halvard", "epithet": "Stonehand", "class": "stonewright", "rank": "sergeant", "renown": 340,
    "feature": "Room database", "status": "working|done|failed|stalled",
    "started_at": "...", "ended_at": null, "elapsed_s": 1840,
    "summary": "Writing the v3→v4 migration test; 19 device tests green so far.",
    "log_tail": ["...last 20 lines..."], "files_changed": 23, "insertions": 1240, "deletions": 80,
    "report": "final report text or null"
  }],
  "pipelines": [{
    "run": "android-release#7", "project": "chromasol", "pipeline": "android-release",
    "status": "running|passed|failed", "started_at": "...", "ended_at": null,
    "stages": [{"name": "build", "status": "passed|running|failed|pending|skipped",
                "started_at": "...", "ended_at": "...", "duration_s": 34, "log_tail": ["..."]}]
  }]
}
```

Rules: times are ISO-8601 UTC; unknown values are `null`, never missing keys; arrays may be empty.
`status: stalled` = process alive but log silent for > 10 min. Agents whose lane ended more than
24 h ago are dropped; the last 20 pipeline runs are kept. Ranks follow `docs/LORE.md`
(renown thresholds 0/100/300/700/1500; +100 lane done, +20 per passed stage it ran, −30 failed lane).

## The brain (Meta models doing real work)

The collector calls the **Meta Model API** (Muse Spark) only when its input changed, caches by hash,
and never blocks the 2 s loop (background worker):
1. **Summary**: last ~200 log lines of a lane → one plain sentence of what the agent is doing now.
2. **Waiting detector**: log tail / final report → "needs the owner" items (added to `waiting_on_you`
   with `source: "brain"`), e.g. a lane asking for a decision.
3. **Architecture**: a project's file tree + key files → `architecture` nodes/edges (refreshed on
   demand with `tirith map <project>` and when the tree changes a lot).
4. **Hero epithet**: the first summary of a new agent → a fitting epithet.
API key from env `META_API_KEY` (never written to disk or logs). With no key, the brain is
`enabled: false` and the collector falls back to: a plain status sentence as summary (never raw log lines), no new waits,
architecture from `config.toml` if given.
