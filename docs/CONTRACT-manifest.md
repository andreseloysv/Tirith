# Contract addition: `tirith.toml` per project (v1)

Every project keeps a manifest at its root. The collector reads it, so pipelines show up
without anyone running `tirith pipeline declare`, and `tirith check` tells an agent what is missing.

```toml
deploy = "none"            # optional: the project does not deploy
include = ["extra.toml"]   # optional: files next to tirith.toml whose [[pipelines]] are added (nothing else is read)

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

Stage `paths` (optional, list of non-empty strings): globs relative to the repo root (`*` inside a folder, `**`
across folders, `?` one character; a leading `!` excludes, and a list of only `!` globs means every other file). A stage
with `paths` runs in the merge queue only when a file the lane changed matches (in a merge train: any file changed by
any lane of the train); otherwise it is skipped with the usual note `skipped <stage>: nothing it tests changed`.
`tirith run --stage <name> --changed` (and any run inside a lane worktree) does the same with the lane's changed files
and just says it was skipped. An unknown or empty diff runs everything. A stage without `paths` keeps the built-in
rules: docs-only diffs skip test stages, and `rust-test` runs only when `app/src-tauri/` or `Cargo.*` changed.
Validated by `tirith check` as `stage-paths`.

```toml
[[pipelines.stages]]
name = "android"
kind = "test"
run = "cd android && ./gradlew test"
paths = ["android/**", "gradle/**", "!**/*.md"]
```

Pipelines from an `include` file do not gate merges: the merge queue and the lane review skip them.
They still run on demand with `tirith run --stage <name>` and show up like any other pipeline.

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
| `lane` | id of the lane whose `workspace` is that worktree (else `$TIRITH_LANE`), or `null` for runs by the manager |
| `worktree` | absolute path of the worktree, or `null` |
| `branch` | the checked-out branch there (`lane/<id>`), or `null` |

Status is truthful. The `start` event records the stage `pid` and its `timeout` (always; `CONTRACT-timeouts.md`). A run is `running` only
while a stage has a start, no end, a live pid and is younger than its timeout (3600 s for an older run without one); otherwise that
stage is `failed` with `reason: "interrupted"`. In a finished run (no running stage) the stages that never started
are `skipped`, and the run is `passed`/`failed` by the stages that ran.

The Overview lists only the running runs, as the same compact rows as the Pipelines view (one row per run:
status, name, who, the stage chips in order with their names, the failing stage in red, duration, time ago);
a row opens in place in the Pipelines view (stages, commands, the last 20 log lines, Full log, Run again).
Raw build and test commands are blocked by the `raw-stage` gate (`docs/CONTRACT-gates.md`).

Every stage also gets one shared `CARGO_TARGET_DIR` per project (unless already set): `cargo_target_dir` of the
project in `config.toml`, else `<repo>-lanes/.cargo-target`, so lane worktrees do not each compile their own copy
(see "Disk diet" in `CONTRACT-launch.md`).

## `tirith check [--project ID | <folder>]`

Exit 0 when all rules pass, 1 otherwise. One line per problem, written for agents:
`<file>:<line>: <rule-id>: <what is wrong>. Fix: <exact command or snippet>`.

| Rule | Meaning |
|---|---|
| `manifest-missing`, `toml-syntax` | `tirith.toml` exists and parses |
| `pipeline-id`, `pipeline-duplicate`, `pipeline-stages`, `stage-name`, `stage-duplicate`, `stage-kind`, `stage-paths` | shape of pipelines and stages |
| `pipeline-coverage` | one gating pipeline (not one from an `include` file) declares build, test and deploy (deploy waived by `deploy = "none"`) |
| `hook-event`, `hook-run` | every `[[hooks]]` entry is valid |
| `rule-id`, `rule-duplicate`, `rule-match`, `rule-regex`, `rule-paths`, `rule-action`, `rule-message`, `rule-instead`, `rule-shape` | every `[[rules]]` entry is valid (see `CONTRACT-setup.md`) |
| `commit-guard` | the commit guard is intact (`verify_guard`) |

## `lane_exclude`: heavy paths a lane does not check out

```toml
lane_exclude = ["ZooBundles/*/audio", "output"]   # git pathspec-style globs, relative to the repo root
mergeq_exclude = false                            # true: the merge queue's temporary worktrees skip them too
```

For a repo with thousands of heavy tracked files (audio, images), a lane worktree copies them all (minutes, GBs).
With `lane_exclude`, `launch.create_worktree` runs `git worktree add --no-checkout`, then
`git sparse-checkout set --no-cone` with the patterns `/*` and `!/<path>/` for each entry, then `git checkout`: the
excluded paths are not on disk. They stay tracked, so `git add -A` in the lane does not stage their deletion and a merge
never removes them. A lane must not need those files (its build and tests run without them).

The merge queue's `_mergeq-*` worktrees check out everything by default, because the project's build and test stages may
read those files. `mergeq_exclude = true` (opt-in, uses the same `lane_exclude` list) makes them sparse too; set it only
when no stage reads the excluded paths. Both keys are top-level; a missing key means nothing is excluded.

If `git worktree add` (or a sparse step) fails or times out, the half-made folder is removed and `git worktree prune`
runs, so no stale folder is left. Worktree creation has its own timeout class, `git-worktree-add` (default 600 s).

## `[roles]`: which roles a project uses

```toml
[roles]
ceo = "always"       # "always" (default) | "on-demand" | "off"
manager = "always"   # same values
```

Missing table or key = `"always"` (today's behaviour). Any other value, or an unknown key, is a `tirith check`
problem (`roles-value` / `roles-key`) naming the key. Parsed into `Manifest.roles` and exported as `roles` in
`state.roadmap`.

With `manager = "on-demand"`, `promote.promote_to_manager(mode, task, parallel_items, subagents, diff)` says when a
worker becomes a manager: the task does not fit in one lane (`plangate.size_problem` / `budget_note`), 2 or more items
run in parallel, or the task asks for sub-agents. It returns the reason, or `None` to stay a worker; `"always"` and
`"off"` never promote. `promote.manager_decision(...)` returns the same verdict as `{rule, measured, threshold, reason}`
(`task_size`, `lane_diff`, `parallel_items`, `subagents`).

With `ceo = "on-demand"`, `promote.promote_to_ceo(mode, workers, unattended_hours, selfcheck_failed)` says when a CEO is
created: more than 2 workers at once (`CEO_WORKERS`), more than 1 h of work with nobody watching
(`CEO_UNATTENDED_HOURS`), or a failure the self-check does not fix. Reason or `None`; `promote.ceo_decision(...)` gives
`{rule, measured, threshold, reason}` (`workers`, `unattended`, `selfcheck`). `"always"` and `"off"` never promote.

When the work is small again (at most `CEO_WORKERS` workers; for a manager, fewer than `PARALLEL_ITEMS` parallel items)
and an on-demand role has had nothing to do for more than `RETIRE_IDLE_SECONDS` (3600), `promote.retire_decision(mode,
role, workers, idle_seconds, parallel_items)` returns `{rule: "idle_seconds", measured, threshold, reason}` and
`promote.retire_if_idle(home, ...)` also appends the `retire` row to `company.jsonl`. `"always"` and `"off"` never retire.
The dispatcher's regular pass calls `promote.retire_on_tick(home, projects, lanes, now, state)`: it measures the idle time
of each on-demand `ceo-`/`manager-` role (since its last lane ended; 0 while one runs) and calls `retire_if_idle`, once
per period of activity (`dispatch.json` `retired`).

Every promotion or retirement is appended to `<home>/company.jsonl` by `promote.log_change(home, event, role, rule,
measured, threshold, reason)` (`event` = `promote` | `retire`; `log_decision` records a `manager_decision`), one line
`{ts, event, role, rule, measured, threshold, reason}`; `promote.read_log(home)` reads it back.

The collector exports the last 200 rows as `state.company`. The page's Company view (`web/js/v-company.js`) draws them
as a timeline, newest first: when, promotion or retirement, the role, the rule with the measured value against its
threshold, and the reason. Test: `node web/js/company.test.mjs`.

TODO: nothing else enforces `"off"` / `"on-demand"` yet. The CEO runner (`ceo.py`) and the manager start are decided per
Tirith home (`config.toml`), not per project, so there is no single per-project gate to wire.
