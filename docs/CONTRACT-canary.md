# Contract: the hourly end-to-end test lane

Code: `collector/tirith/canary.py`; started by `dispatch.run` (only when the pass has a spawner); checked by the
`canary` check of `selfcheck.py`. Test: `pytest -k canary`.

Every piece can pass its unit tests while nothing merges in production. The canary goes through the real wiring
once an hour: plan, lane, merge queue, push, plan tick.

## The test project

`<home>/canary/` is made on first use: `repo/` (a git repo on `main` with a manifest of trivial build and test
stages and `docs/TODO.md`), `origin.git/` (a local bare repo, the repo's `origin`), `runs.json` (the last 10
results), `brief.md`. No network, no server. It is registered as the project `canary` (guard off) so the
real dispatcher, merge queue and disk guard handle it. It is left out of the owner's project list and of
`tirith project list`; the owner sees it only as the "end-to-end test" health check.

## A run (`tirith canary run`)

Started detached by the dispatcher when the last run is older than an hour, or the collector's code changed
since it (a reload), and none is under way. Steps, each written to `runs.json` when reached:

| step | what happens | stops here when |
|---|---|---|
| `plan-add` | `tirith plan add` puts a new item in step 1 | the command fails |
| `lane` | `tirith lane run` with a shell agent: edits `canary.txt`, `tirith plan report --status done`, commits | the lane does not end `done` with outcome `done` |
| `queued` | the dispatcher puts the finished lane in the merge queue | it never does |
| `merge-start` | the dispatcher's own spawner starts `tirith merge run` (the test never starts it) | the entry stays waiting |
| `merge` | merge into the canary's `main`, build and test stages | the queue reports anything but merged |
| `push` | `canary.txt` is on the bare origin's `main` | it is not |
| `tick` | the plan item is ticked | it is not |

The run waits up to 15 minutes in all. Afterwards its worktrees and `lane/*` branches are removed.

## The check

`canary` in `state.health`: `ok` when the last run reached the push and the tick (or none has run yet, or one is
under way for less than 15 minutes); `failed` otherwise, with the step it stopped at and the log path
(`<home>/logs/canary-<id>.log`). A failure is one manager queue item (`sq-selfcheck-canary`), never an owner
item; it closes by itself when a later run passes.
