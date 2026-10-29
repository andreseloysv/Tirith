# Contract addition: environment variables per project (v1)

The owner adds variables to a project under Settings > Environment. Every lane, stage run and Run action of
that project gets them in its environment, on top of the inherited one (a project variable wins over an
inherited one of the same name). The values are shown in full on the page, on purpose.

```
Settings table -> Tauri project_env_set | tirith://env-set | tirith project env set -> <home>/env/<project>.toml (0600)
stage run / lane / Run action of <project> -> environment = inherited + env/<project>.toml
collector -> state.projects[].env (KEY -> value), read by the Settings page only
```

## Storage: `<home>/env/<project>.toml`

One file per project, mode `0600`, plain top-level string keys; written atomically by `tirith.projenv`:

```toml
API_URL = "https://example.invalid/v1"
DEBUG = "1"
```

Names match `[A-Za-z_][A-Za-z0-9_]*`; values are any string without NUL. The project id is `[A-Za-z0-9][A-Za-z0-9._-]*`
(it is a file name). An empty set removes the file. A missing or broken file means no variables. Nothing is
ever written to `config.toml`.

## CLI

| Command | Does |
|---|---|
| `tirith project env list [--project P] [--json]` | prints `KEY=VALUE` lines (or a JSON object) |
| `tirith project env set [--project P] KEY=VALUE ...` | adds or changes those variables |
| `tirith project env set [--project P] --replace-stdin` | stdin is a JSON object; it becomes the whole set (what the Settings page saves) |
| `tirith project env unset [--project P] KEY ...` | removes variables |

`--project` defaults to the project of the cwd and must be a known project. A bad name or project id exits 3
with `project env: <what>. Fix: <how>` on stderr and leaves the file alone.

## Sending from the page

Saving replaces the whole set. The page tries, in order:

1. the Tauri command `project_env_set(project, env)` (the app runs `tirith project env set --replace-stdin`
   and writes the values to its stdin, never to the command line);
2. a browser with the handler: `tirith://env-set?project=P&env=<JSON object>&token=T` (needs the launch token
   like every `tirith://` link; the handler log hides `token=` and `env=`);
3. else the page shows `printf %s '<json>' | tirith project env set --project P --replace-stdin` to copy.

After a send the page keeps the table as typed until `state.projects[].env` equals it ("Sent · waiting for the
state"), so a refresh never flips it back.

## Where it applies

| Where | How |
|---|---|
| `tirith run` and every recorded stage (also `review` test runs) | `runstage.run_stage` lays `env/<project>.toml` over the child's environment |
| `tirith lane run` (the agent process of a lane) | `cli._wrapper_path_env(..., project)` |
| Run actions (`tirith action run`, `tirith://run`, the app's Run button) | `actions.run_action` and the app's `run_shell` read the file of the action's project |

## State

`state.projects[].env`: object `KEY -> value` (empty `{}` when none), every project row. It is in `state.json`
like the rest of the state, which only the local page reads.
