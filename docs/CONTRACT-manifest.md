# Contract addition: `tirith.toml` per project (v1)

Every project keeps a manifest at its root. The collector reads it, so pipelines show up
without anyone running `tirith pipeline declare`, and `tirith check` tells an agent what is missing.

```toml
deploy = "none"            # optional: the project does not deploy

[[pipelines]]
id = "ci"                  # required, unique
name = "CI"                # optional display name

[[pipelines.stages]]
name = "build"             # required, unique within the pipeline
run = "make build"         # the command `tirith run --stage build` executes
kind = "build"             # optional: build | test | deploy

[[hooks]]
event = "pre-commit"       # pre-commit | commit-msg | pre-push | post-commit | post-merge
run = "make lint"
```

Stage kind: the explicit `kind`, else recognised from the stage name (`build`/`compile` → build,
`test`/`check`/`lint` → test, `deploy`/`release`/`publish`/`ship` → deploy).

## How the collector uses it

For every registered or auto project folder it reads `<folder>/tirith.toml` each cycle. Each pipeline
is declared with its stage order (same effect as `tirith pipeline declare`). A pipeline that never ran
appears in `state.json` `pipelines` as run `<id>#0` with every stage `pending`. A broken manifest is
skipped silently by the collector; `tirith check` reports it.

## `tirith run`

`tirith run --stage test` needs no `--project` (the project containing the cwd) and no `--pipeline`
(resolved from the manifest when the stage name is unique, else `--pipeline` is required). With no
command after `--` it runs the stage's `run` through `/bin/sh -c`.

### Every run is recorded (`runstage.run_stage`)

Everything Tirith runs goes through one function, `tirith.runstage.run_stage(home, project, pipeline,
stage, command, cwd, lane, run_id)`: `tirith run` and `tirith lane review` (the test stage in the
lane's worktree) both call it, so each review appears as a pipeline run with its stage, log and result.
It appends the `start` and `end` rows of `events.jsonl` and returns `(exit code, output tail)`.

Inside a lane's worktree (a linked git worktree of the project) `tirith run` takes the manifest from the
**worktree** root, runs the stage commands from there (not from the main repo) and tags both rows, and
the run in `state.json` `pipelines[]`, with:

| Field | Meaning |
|---|---|
| `lane` | id of the lane whose `workspace` is that worktree (else `$TIRITH_LANE`), or `null` for runs by the steward |
| `worktree` | absolute path of the worktree, or `null` |
| `branch` | the checked-out branch there (`lane/<id>`), or `null` |

The Overview lists these runs under "Pipelines": the running ones first, then those that ended in the
last 30 minutes (result, duration, the lane or "steward"); each row opens the Pipelines view on the run.
Raw build and test commands are blocked by the `raw-stage` gate (`docs/CONTRACT-gates.md`).

## `tirith check [--project ID | <folder>]`

Exit 0 when all rules pass, 1 otherwise. One line per problem, written for agents:
`<file>:<line>: <rule-id>: <what is wrong>. Fix: <exact command or snippet>`.

| Rule | Meaning |
|---|---|
| `manifest-missing`, `toml-syntax` | `tirith.toml` exists and parses |
| `pipeline-id`, `pipeline-duplicate`, `pipeline-stages`, `stage-name`, `stage-duplicate`, `stage-kind` | shape of pipelines and stages |
| `pipeline-coverage` | one pipeline declares build, test and deploy (deploy waived by `deploy = "none"`) |
| `hook-event`, `hook-run` | every `[[hooks]]` entry is valid |
| `commit-guard` | the commit guard is intact (`verify_guard`) |
