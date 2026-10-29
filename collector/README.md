# Tirith collector (`tirith` CLI + Spark brain)

The Python half of Tirith: a small CLI that agents, pipelines and the
orchestrator use to report what they are doing, plus a collector loop that
merges those reports with live facts (processes, logs, git, the commit
guard) and writes `~/.tirith/state.json` every 2 seconds for the web page and the app.
The data contract lives in `../docs/CONTRACT.md`; the JSON schema that
`state.json` must satisfy is `state.schema.json` in this folder.

No servers, no open ports. The only network access is the optional brain
calling the Meta Model API (see below); everything works without it.

## Install and run

Requires Python 3.12+ and [uv](https://docs.astral.sh/uv/).

```bash
cd collector
uv sync                # create .venv and install dev tools (pytest, jsonschema)
uv run tirith --help   # the console script
# or install the command for everyday use:
uv tool install .      # puts `tirith` on your PATH
```

Run the collector in a spare terminal (it just rewrites `state.json`):

```bash
tirith collect              # every 2 s; Ctrl-C to stop
tirith collect --once       # single cycle (used by tests and scripts)
tirith status               # short text summary of the current state
```

Or use the Makefile shortcuts: `make install`, `make test`, `make collect`,
`make demo`.

### launchd (optional, owner's choice)

`tirith-collector.plist.template` is a ready-to-adapt launchd template.
Nothing installs it for you. To use it: replace the two `__REPLACE__` paths,
copy it to `~/Library/LaunchAgents/com.tirith.collector.plist`, then
`launchctl load` it yourself.

## Configure

`~/.tirith/config.toml` (override the folder with `TIRITH_HOME`):

```toml
[[projects]]
id = "chromasol"
name = "ChromaSol"
repo = "/absolute/path/to/repo"
color = "#2f7d5b"
# guard_words = ["secretword"]   # optional; default is the guard module's list
# architecture = { nodes = [{ id = "engine", label = "Colour engine", kind = "core", files = 14 }], edges = [] }

[brain]
model = "muse-spark-1.3"         # any Spark model id your key can use
# base_url = "https://api.meta.ai/v1"
# enabled = true

[manager]
name = "Aldric of the Tower"
title = "The Manager"
# guard_words = [...]            # global fallback for projects without their own
```

Guard words resolve per project first, then the top-level `guard_words`,
then the encoded default list in `tirith_guard.py`.

## The commands

```bash
# Lanes (one coding agent each)
tirith lane start --project chromasol --class stonewright \
  --feature "Room database" --workspace <dir> --log <file> --pid <pid> --brief <file>
tirith lane end <lane-id> --result done --report "Migrations green."
tirith lane run --project chromasol --class stonewright --feature "Room database" \
  -- <agent command...>     # verifies the commit guard first, refuses if broken;
                            # runs the agent with ~/.local/bin first in PATH

# Pipelines (caravans)
tirith pipeline declare android-release --project chromasol --stages build,test,lint,sign
tirith run --project chromasol --pipeline android-release --stage build --new-run -- make build
tirith run --project chromasol --pipeline android-release --stage test -- make test

# Orchestrator + owner queue
tirith manager "Reviewing the data lane"
tirith note "Merged the data lane" --project chromasol
tirith wait add "Tick the store declaration" --project chromasol
tirith wait done w-1

# Architecture map + commit guard
tirith map chromasol
tirith guard install chromasol   # installs missing hooks only; never edits existing ones
tirith guard verify --all
```

Classes come from `../docs/LORE.md`: `stonewright`, `loremaster`, `warden`,
`swiftrider`, `herald`, `glasswright`, `archivist` (plus `steward`, shown as Manager).
Hero names are generated from Tirith's own syllable tables and stay stable
per lane id; renown is +100 for a finished lane, +20 per passed stage run
with `--agent <lane-id>`, −30 for a failed lane (ranks at 0/100/300/700/1500).

## The brain (Spark models doing real work)

When `META_API_KEY` is set, a background worker calls the **Meta Model API**
(base `https://api.meta.ai/v1`, `POST /chat/completions`,
`Authorization: Bearer <key>`, OpenAI-style body; see
https://dev.meta.ai/docs/) for four jobs, cached by input hash in `brain/`
so unchanged logs cost nothing:

1. **Summary** — last ~200 log lines → one sentence of current activity.
2. **Waiting detector** — log tail/report → what a working agent seems to wait on
   (manager queue items `kind: "agent-waiting"`, never `waiting_on_you`).
3. **Architecture** — file tree + key files → map nodes/edges (`tirith map`,
   refreshed automatically when no stored map exists).
4. **Hero epithet** — an agent's first brain summary → its lasting epithet.

The key is read only from the environment, `META_API_KEY` or `META_SI_API_KEY`
(the first one set wins), and is never logged or written to disk:

```bash
export META_SI_API_KEY=...   # e.g. in ~/.zshrc
tirith collect
```

Without a key the brain reports `enabled: false` and the collector falls
back to: last non-empty log line as the summary, no new brain waits, and
architecture from `config.toml` or the last `tirith map` (which can also
build a simple folder map offline).

## The commit guard

`tirith_guard.py` installs/verifies the owner's forbidden-words hooks
(pre-commit + commit-msg) in a repo's shared hooks folder. It never edits
or removes an existing hook. The collector verifies every configured
project each cycle: results appear as `projects[].guard = {ok, problems}`
in `state.json`, and a broken guard raises a `waiting_on_you` item with
`source: "guard"`. `tirith lane run` refuses to start an agent in a
workspace whose guard is not intact.

## Demo + tests

```bash
make demo        # or: uv run python demo.py
# fills a temp TIRITH_HOME with a guarded demo repo, lanes, a pipeline,
# waits and a live ticker lane, then prints how to watch it.

make test        # or: uv run pytest -q
# covers CLI events, collector merge, stalled detection, ranks, the guard
# on throwaway tmp repos, brain fallback/cache, and schema validation of
# both the fixture and freshly collected state. One live model test runs
# only when META_API_KEY is set.
```

## Layout

- `tirith/cli.py` — the `tirith` command (argparse, subcommands above).
- `tirith/collector.py` — the merge loop that builds `state.json`.
- `tirith/brain.py` — Spark client, background worker, hash cache, 4 jobs.
- `tirith/names.py` — syllable-table hero names, renown and ranks.
- `tirith/home.py` / `tirith/store.py` — `TIRITH_HOME` layout + file IO.
- `tirith/projects.py` — project folders, auto-discovery, config editing, sessions.
- `tirith/guardwrap.py` — thin adapter over `tirith_guard.py` (untouched).
- `state.schema.json` — JSON schema for `state.json` (contract v1 + the
  `projects[].guard` and wait `source` additions documented above).
- `demo.py`, `Makefile`, `tirith-collector.plist.template`, `CREDITS.md`.

## Rooms, flows and `state.js`

- **Rooms** (agentsmessenger.com, read-only polling): `tirith room add|remove|list|tail <room>`
  and `tirith room say <room> --as <name> "<text>" [--re N]` (the only command that POSTs).
  Config `[[rooms]]`, `[owner] names`, `[messenger] interval_s` and the `rooms` state section are
  specified in `../docs/CONTRACT-rooms.md`. Code: `tirith/rooms.py`; two extra brain jobs
  (room summary, "needs the owner") in `tirith/brain.py`.
- **Flows**: `tirith flow --from <endpoint> --to <endpoint> --kind <kind> --text "<line>"`; lane
  start/end, wait add/done, guard problems and finished pipeline runs record flows automatically
  (`../docs/CONTRACT-flows.md`, code in `tirith/flows.py`) and `state.json` gets `flows`.
- **`state.js`**: every cycle also writes `state.js` next to `state.json` (atomically):
  `window.TIRITH_STATE = <the same JSON>;`, loadable from a page opened via `file://`.
  `make fixtures` regenerates `../fixtures/state*.js` from the JSON fixtures.

## Projects as folders and sessions

Spec: `../docs/CONTRACT-sessions.md`; code in `tirith/projects.py`.

- `tirith project add <folder> [--name N] [--color #hex] [--guard-words a,b]` appends a
  `[[projects]]` row to `config.toml` (absolute folder in `repo`, id = slug of the name, made
  unique; existing text and comments are kept, the write is atomic). `.` works.
  `tirith project list` shows registered and auto projects; `tirith project remove <id>` drops
  either kind. Adding the folder of an auto project promotes it (and lets you rename it).
- **Auto-discovery**: lane `workspace`, the cwd `tirith run` records in its events (`cwd`) and a
  `[[rooms]] project` that is a path outside every known project become an auto project (git
  top-level, else the folder), stored in `projects-auto.json` (`auto: true`). A removed auto project
  is remembered and does not come back. An item belongs to the project whose folder is the longest
  prefix of its folder; an explicit project id that is known always wins, an unknown one falls back
  to the folder.
- `state.projects[]` adds `folder`, `auto` and `counts` (`agents_working`, `agents_total`,
  `pipelines_running`, `flows_24h`).
- `tirith session start "<title>" [--id ID]` (prints the id and `export TIRITH_SESSION=<id>`, writes
  `~/.tirith/session`), `tirith session end [<id>]`, `tirith session list`. Every CLI event (lane,
  `run` events, flow, note, wait add/done) records `session` = env `TIRITH_SESSION`, else the file,
  else `null`; child commands of `run`/`lane run` inherit it. `state.sessions[]` lists sessions newest
  first with their `projects` and `counts`; agents, pipeline runs, flows, notes and waits carry
  `session`, and flows also `project`. Sessions live in `sessions.json`.
