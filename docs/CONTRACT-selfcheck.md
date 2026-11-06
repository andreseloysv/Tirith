# Contract: Tirith checks itself

Code: `collector/tirith/selfcheck.py`, called by `dispatch.run` every pass (right before the on-call manager,
so a restart it asks for happens in the same pass). State: `dispatch.json` under `health`; the web gets
`state.health`. Web: `web/js/v-health.js` (the dot in the header).

## Checks

| id | fails when | remedy by code | when it cannot be fixed |
|---|---|---|---|
| `collector` | (info only) a gap of more than 150 s between two passes; shown for 10 min | none: the app supervisor restarts the collector | — |
| `manager` | the oldest owner chat message has no first manager answer for more than 60 s (an `autopilot-proposal`: more than 10 min), no manager turn is working on it (a busy session counts as working only on the owner's own turn, or with a permission question open) and nobody is attached; or a manager process is still alive 60 s after its `result` | clear the manager's retry state (`tries`, `key`, `at`, `error`) so the next pass starts a turn, with the evidence in the brief (`## Self-check`); or end the process group (`managerd.terminate_group`) | a queue item; a pause (`sq-manager-paused-*`) is never overridden |
| `autopilot` | an open agent item (whatever its note) with no lane working and no review running, that the manager was not told about for 2 min; or an `autopilot-proposal` unanswered for 10 min (amber, the `manager` check restarts the turn); the check counts open proposals per step, and more than one for a step is amber until the autopilot collapses them | none that is safe | one queue item with the items |
| `disk` | below `[disk] warn_free_gb` (amber) or `[disk] min_free_gb` (red) | the disk guard (`diskdiet`) removes merged and stale lane folders | amber, then red until space is free |
| `reviews` | a review `running` for 20 min with its process gone, or for 3 h with it alive (it waits for a stage slot or tests; the stage has its own time limit); or `tests-failed` on tests the lane's diff does not touch | end the stuck process, run `tirith lane review <id>` again; run the unrelated failure again once | a queue item with the output |
| `wasted` | (info only, never red) work thrown away: `repeated` suites run again for a lane whose suite already ran, `discarded` merges tested and then not needed because another queued lane held their commits, `absorbed` lanes dropped before testing for the same reason | none: it only shows the totals in plain words | — |
| `ownerenv` | the owner's login shell could not be read at collector start (failed, empty or over 10 s; `CONTRACT-agents.md`, "The owner's environment") | none: it is read again on the next collector start | red with the reason, no variable names or values |

"Unrelated": the failing test files named in the review output (`test_x.py`, `x.test.mjs`) are neither in the
lane's diff nor the test of a module it changed. A failure that names no test file is never called unrelated.

When the unrelated failure is still there after its one rerun, code looks at the other lanes' reviews. When one of
them fails in the same test files too, main (or the machine) is the cause and the manager gets the queue item
("look at main"). When none does, the lane's own change is the likely cause, through a side effect the file names do
not show (shared state, a new step that runs on every pass, a real command run from the tests): the lane goes back
to its agent once with the failing output (the same relaunch as a failed claim check), the check stays `healing`,
and the manager says "The tests of lane X fail only in that lane (…): I sent it back to its agent to find the side
effect." If the next review fails the same way, the manager gets the queue item ("was sent back to its agent … and
that did not fix them. Start a fresh lane on main for its item, or drop it"); so it does when nothing came of the
send-back in 2 h (`SENDBACK_WAIT_S`: no new lane, same review result), instead of `healing` for ever. A lane whose
work another lane carries on (`retry_of` or `fallback_lane` names it: a relaunch, a send-back, a claim-check
retry) is not checked: its failure is the new lane's, even when that one died with a restart. The lane a send-back
started is never sent back itself (one send-back per chain): when it fails the same way, the manager decides.

## Remedies

At most 3 tries per episode (1 for unrelated test failures), never twice within 90 s. When the tries are used up
(or there is no safe remedy) one manager queue item (`kind: selfcheck`, id `sq-selfcheck-<key>`) holds the
evidence. It is never an owner item. The item closes by itself when the problem is gone. An episode lasts until
the owner has been told how it ended.

## What it fixed alone

After a remedy, one calm line goes into `health.fixed` (last 20, `{text, at}`), once; it is **not** written in the
chat (`inbox.add_say` is not used). The owner sees it by clicking the health dot. The chat only shows what needs
the owner. Lines:

- manager restart (after the waiting messages are answered): "The manager got stuck for 3 min; I restarted it and
  answered your 2 message(s)."
- manager process: "A manager process stayed open after answering; I closed it."
- review: "The review of lane X was stuck for 22 min; I started it again." / "The tests of lane X failed on
  something the lane did not touch; I ran them again."
- send-back: "The tests of lane X fail only in that lane (…): I sent it back to its agent to find the side effect."

## `state.health`

```json
{"level": "green|amber|red", "at": "<iso>",
 "checks": [{"id": "manager", "label": "The manager", "level": "ok|healing|failed", "text": "plain words"}],
 "fixed": [{"text": "plain words", "at": "<iso>"}]}
```

`null` before the first pass. `level`: red if any check `failed` (a remedy failed or none exists), else amber if
any `healing`, else green. No raw errors, ids or paths except a lane id.

## Web

A dot in the header (hidden without `state.health`): green, amber while a remedy works, red only when one
failed. Click: the checks in plain words, then "Fixed by Tirith" with `fixed` (newest first); Escape closes. Test: `node web/js/health.test.mjs`.

## Stable copy

Tirith runs from `<home>/stable/current`, a symlink to `<home>/stable/releases/<id>/` (a repo-shaped copy: `collector/` with `tirith/`,
`tests/`, `pyproject.toml`, `uv.lock`, `state.schema.json` and the other files the tests read, plus `fixtures/`, `bin/`, `run` and the root `pyproject.toml` next to `collector/`, so ROOT-based tests such as `test_schema.py` collect). `tirith stable promote [--source <collector>] [--test <cmd>]` copies the
source to a new release and runs the full test (default `uv run -q pytest -q tests`) inside the copy; only a pass
moves `current` (the old one becomes `previous`), a red test deletes the copy and records `last_rejected`.
A promoted release is on trial for 30 min: when `state.health.level` stays red for 2 min (checked every
dispatcher pass), `previous` is put back as `current` by itself (`last_rollback`) and the bad release is deleted.
`tirith stable rollback` does it by hand, `tirith stable status` prints `<home>/stable/stable.json`'s summary.
Once a release is current, everything Tirith starts (`tirithcmd.resolved`: launch, the link handler, launchd, the app's
run actions) runs `uv run --project <home>/stable/current/collector tirith` unless `[app] tirith` is set. After each lane the merge
queue merges into main, it starts a detached `tirith stable promote --source <main>/collector` (log
`<home>/logs/stable-promote.log`), so the stable copy follows main only through a passing full test.
Three releases are kept. Code: `tirith/stable.py`; test: `pytest -k stable`.

## Wasted work

`collector/tirith/wasted.py`: `mergeq._finish` calls `wasted.record` for each row that leaves the queue; the totals
(`repeated`, `discarded`, `absorbed`, plus a per-lane suite count) live in `<home>/wasted.json`. The merge queue
marks a settled row `wasted: "discarded"` (the suite ran, then another queued lane absorbed the merge) or
`wasted: "absorbed"` (dropped before testing). A suite counts as run for `merged`, `push-failed` and a `red` whose
message starts with `stage `. Test: `pytest -k wasted_work`.
