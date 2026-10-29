# Models contract: the model and effort of the manager and of the agents it starts

One place decides them: `~/.tirith/config.toml`. The Settings view of the web page and `tirith models`
both edit it; launch, the dispatcher and `lane run --agent` read it.

```toml
[models]
choices = ["<model id>", "<model id>"]   # the models the Settings page offers (free text when absent)
efforts = ["low", "high"]                # the efforts it offers (default: low medium high xhigh max)
out_of_credits_patterns = ["quota"]      # optional: regexes that replace the built-in list (see Fallback)

[models.manager]          # the orchestrator session
agent = "anthropic"       # an alias, see docs/CONTRACT-hooks.md "Agent aliases"
model = "<model id>"
effort = "high"

[models.workers]          # default for every lane the manager, the dispatcher or the Plan button starts
agent = "anthropic"
model = "<model id>"
effort = "medium"

fallback = ["muse"]       # agents to continue with, in order, when the agent is out of credits

[models.workers.projects.<project-id>]        # optional override for one project
model = "<model id>"
fallback = []             # an override may set its own chain; an empty list means none

[agents.<alias>]
model_prefixes = ["<prefix>"]   # optional: the choices that start with one of these belong to this agent

[models.workers.steps."<project-id>/<step>"]  # optional override for one plan step
effort = "low"
```

Every field is optional. Order of precedence for a worker: step override, project override,
`[models.workers]`; a field missing at one level comes from the next.

## Workers

`models.resolve_workers(config, project, step)` returns `{agent, model, effort}`.

- **`tirith lane run --agent <alias>`** (`agentrun.build_command`): model and effort come from the
  explicit `--model` / `--effort`, else from the resolved workers setting, else from
  `[agents.<alias>]`. A setting made for another agent (`agent` set and not this alias) is ignored: a
  model id means nothing to another agent. The effort flag comes from the preset
  (`PRESET_DEFAULTS[alias]["effort_flag"]`), else from `models.EFFORT_FLAGS` (`muse` uses
  `--reasoning-effort`). An alias with no effort flag only records the effort in config; the command
  does not get one. An explicit `--effort` for an alias without a preset flag is still an error.
- **`tirith launch`, the dispatcher and the Plan button** (`launch.agent_command`): with `--agent` or
  `[models.workers] agent` naming a configured alias, that alias's `launch` template is used, else
  `[launch] command`. The explicit `[launch] command` keeps working, but **its model flag is replaced by
  the configured one** when both exist: `--model X`, `--model=X` and `-m X` become the configured model,
  `--effort X` / `--reasoning-effort X` the configured effort. A flag the command does not have is not
  added. With no `[models]` the command is used exactly as written.

## Fallback: an agent that ran out of credits

The chain of a worker is its agent, then `fallback` (step override, project override, `[models.workers]`;
the first level that sets one wins): `models.resolve_chain`. Code: `tirith/fallback.py`.

- **Detection** (`fallback.out_of_credits`): the dispatcher looks at every lane that ended `failed` in the
  last hour (not one the owner stopped). The last 40 lines of its log count: plain lines as they are, JSON
  events only when they are errors (`error`, `is_error`, an `*error*` / `*fail*` type). A line matching one
  of the patterns (case-insensitive regexes; defaults: HTTP/status 402 or 429, "payment required", "too many
  requests", "quota", "credit(s)", "insufficient", "rate limit", "usage limit"; `[models]
  out_of_credits_patterns` replaces them) marks the lane `out_of_credits_at`.
- **Relaunch** (`cli.relaunch_lane`, called by `dispatch.run` once per lane, state `fallbacks` in
  `dispatch.json`): the same brief is started with the next alias of the chain that this brief has not used
  (the lane's `tried` plus its `agent`), in the same worktree when it has no uncommitted changes, else in a
  new one. The new lane records `agent`, `fallback_from` (the alias that ran out), `fallback_lane` and
  `tried`, and keeps the `step` and `request_id`. The Plan thread gets the answer "muse ran out of credits →
  continued with anthropic" (by Tirith) and the flows a message. An agent is used at most once per brief, so
  a chain never loops; when it is used up the lane stays failed with its usual owner item.
- **Picking** (`fallback.pick_alias`): `tirith launch` and the dispatcher (the autopilot) without an
  explicit `--agent` use the first alias of the chain none of whose lanes ended out of credits in the last
  hour; when all did, the first.
- `lane run` takes `--as-agent`, `--fallback-from`, `--tried` (set by launch and the relaunch).

## Manager

`tirith models apply-manager` writes `[models.manager]` into the manager agent's user settings file: the
path is `[agents.<alias>] settings` (the same file `tirith hook install` edits), the JSON key is `model`.
Every other key stays; a missing file is created; an unreadable one is refused and left untouched. Effort
goes to the agent's documented settings key when it has one (`models.SETTINGS_KEYS`); neither `anthropic`
nor `muse` has one yet, so the command says the effort was recorded in `config.toml` only.
It never runs by itself: it is a button the owner clicks (below) or a command the owner types.

## CLI

| Command | Does |
|---|---|
| `tirith models show` | the configured manager, workers, overrides and choices, and what the manager's settings file holds |
| `tirith models set manager\|workers [--model M] [--effort E] [--agent A] [--fallback A,B] [--project P] [--step S]` | edits `config.toml` as text: comments, other tables and keys not given stay as they are; a table that is missing is appended. `--step` needs `--project`; the manager has neither |
| `tirith models set workers --project P [--step S] --clear` | removes that override table |
| `tirith models apply-manager` | writes the manager settings file as above |

`--fallback` (workers only) takes aliases separated by commas; `--fallback ""` empties the chain.

Exit 3 with the reason on stderr for a bad value, a manager without agent or model, or an unreadable
settings file. Values may only contain letters, digits and `. _ : / + - [ ]`.

## Web: Settings view

Sidebar "Settings" (`#view=settings`, `web/js/v-models.js`): Manager (agent, model, effort, **Save**,
**Apply to the manager**, and whether the settings file already holds the model), Workers (defaults, then
the overrides in a compact table with Remove and an add row). Model and effort are
selects (`choices`, narrowed to the agent's `model_prefixes`; `efforts`) with an "Other…" entry that turns
into a text field; the manager's model defaults to what its settings file holds. Workers also have a
**Fallback** list (ordered chips, add / move up / remove) and one status chip per agent: `ok` or `out of
credits since 09:10`. Saving sends an inbox request of kind `setup` for the project `settings`:

```json
{"action": "models-set", "role": "workers", "model": "m", "effort": "low", "project": "demo", "step": "3"}
{"action": "models-set", "role": "workers", "fallback": ["muse"]}
{"action": "models-set", "role": "workers", "project": "demo", "clear": true}
{"action": "apply-manager"}
```

The page sends it the way the Setup card does (Tauri `inbox_add`, the `tirith://inbox` link, else it shows
the CLI line to copy). `tirith setup apply <request-id>` applies it (`models set` / `models
apply-manager`), answers and closes it. `state.models.pending` lists the open ones.

## State

`state.models` = `{manager {agent, model, effort, settings {path, model, effort} | null, in_sync,
effort_key}, workers {agent, model, effort}, projects {id: {...}}, steps {"id/step": {...}}, choices,
efforts, choices_by_agent {alias: [model]}, agents, agent_status {alias: {state: "ok"|"out", since}}, pending}`
(`workers` also has `fallback`). `state.agents[]` also has `agent` and `fallback_from`. `state.agents[].ran` = `{model, effort}` or null: the model from the lane's
usage records (else from its command) and the effort from its command. `lane run` stores the flags it
found in the command as `ran` on the lane file. The agent drawer shows them in the usage line next to
the cost, the agent cards as a small `model · effort` line.
