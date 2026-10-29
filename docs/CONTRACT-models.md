# Models contract: the model and effort of the manager and of the agents it starts

One place decides them: `~/.tirith/config.toml`. The Settings view of the web page and `tirith models`
both edit it; launch, the dispatcher and `lane run --agent` read it.

```toml
[models]
choices = ["<model id>", "<model id>"]   # the models the Settings page offers (free text when absent)

[models.manager]          # the orchestrator session
agent = "anthropic"       # an alias, see docs/CONTRACT-hooks.md "Agent aliases"
model = "<model id>"
effort = "high"

[models.workers]          # default for every lane the manager, the dispatcher or the Plan button starts
agent = "anthropic"
model = "<model id>"
effort = "medium"

[models.workers.projects.<project-id>]        # optional override for one project
model = "<model id>"

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
| `tirith models set manager\|workers [--model M] [--effort E] [--agent A] [--project P] [--step S]` | edits `config.toml` as text: comments, other tables and keys not given stay as they are; a table that is missing is appended. `--step` needs `--project`; the manager has neither |
| `tirith models set workers --project P [--step S] --clear` | removes that override table |
| `tirith models apply-manager` | writes the manager settings file as above |

Exit 3 with the reason on stderr for a bad value, a manager without agent or model, or an unreadable
settings file. Values may only contain letters, digits and `. _ : / + - [ ]`.

## Web: Settings view

Sidebar "Settings" (`#view=settings`, `web/js/v-models.js`): Manager (agent, model, effort, **Save**,
**Apply to the manager**, and whether the settings file already holds the model), Workers (defaults, then
the overrides in a compact table with Remove and an add row). Models come from `choices` (a select) or a
text field. Saving sends an inbox request of kind `setup` for the project `settings`:

```json
{"action": "models-set", "role": "workers", "model": "m", "effort": "low", "project": "demo", "step": "3"}
{"action": "models-set", "role": "workers", "project": "demo", "clear": true}
{"action": "apply-manager"}
```

The page sends it the way the Setup card does (Tauri `inbox_add`, the `tirith://inbox` link, else it shows
the CLI line to copy). `tirith setup apply <request-id>` applies it (`models set` / `models
apply-manager`), answers and closes it. `state.models.pending` lists the open ones.

## State

`state.models` = `{manager {agent, model, effort, settings {path, model, effort} | null, in_sync,
effort_key}, workers {agent, model, effort}, projects {id: {...}}, steps {"id/step": {...}}, choices,
efforts, agents, pending}`. `state.agents[].ran` = `{model, effort}` or null: the model from the lane's
usage records (else from its command) and the effort from its command. `lane run` stores the flags it
found in the command as `ran` on the lane file. The agent drawer shows them in the usage line next to
the cost, the agent cards as a small `model · effort` line.
