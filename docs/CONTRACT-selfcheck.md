# Contract: Tirith checks itself

Code: `collector/tirith/selfcheck.py`, called by `dispatch.run` every pass (right before the on-call manager,
so a restart it asks for happens in the same pass). State: `dispatch.json` under `health`; the web gets
`state.health`. Web: `web/js/v-health.js` (the dot in the header).

## Checks

| id | fails when | remedy by code | when it cannot be fixed |
|---|---|---|---|
| `collector` | (info only) a gap of more than 150 s between two passes; shown for 10 min | none: the app supervisor restarts the collector | — |
| `manager` | the oldest owner chat message has no manager answer for more than 120 s (an `autopilot-proposal`: more than 10 min), no manager turn is working and nobody is attached; or a manager process is still alive 60 s after its `result` | clear the manager's retry state (`tries`, `key`, `at`, `error`) so the next pass starts a turn, with the evidence in the brief (`## Self-check`); or end the process group (`managerd.terminate_group`) | a queue item; a pause (`sq-manager-paused-*`) is never overridden |
| `autopilot` | an open agent item (whatever its note) with no lane working and no review running, that the manager was not told about for 2 min; or an `autopilot-proposal` unanswered for 10 min (amber, the `manager` check restarts the turn) | none that is safe | one queue item with the items |
| `disk` | below `[disk] min_free_gb` | the disk guard (`diskdiet`) already removes merged worktrees | red until space is free |
| `reviews` | a review `running` for 20 min; or `tests-failed` on tests the lane's diff does not touch | end the stuck process, run `tirith lane review <id>` again; run the unrelated failure again once | a queue item with the output |
| `ownerenv` | the owner's login shell could not be read at collector start (failed, empty or over 10 s; `CONTRACT-agents.md`, "The owner's environment") | none: it is read again on the next collector start | red with the reason, no variable names or values |

"Unrelated": the failing test files named in the review output (`test_x.py`, `x.test.mjs`) are neither in the
lane's diff nor the test of a module it changed. A failure that names no test file is never called unrelated.

## Remedies

At most 3 tries per episode (1 for unrelated test failures), never twice within 90 s. When the tries are used up
(or there is no safe remedy) one manager queue item (`kind: selfcheck`, id `sq-selfcheck-<key>`) holds the
evidence. It is never an owner item. The item closes by itself when the problem is gone. An episode lasts until
the owner has been told how it ended.

## Telling the owner

After a remedy the manager says one calm line in the chat (`inbox.add_say`), once:

- manager restart: after the waiting messages are answered: "The manager got stuck for 3 min; I restarted it and
  answered your 2 message(s)."
- manager process: "A manager process stayed open after answering; I closed it."
- review: "The review of lane X was stuck for 22 min; I started it again." / "The tests of lane X failed on
  something the lane did not touch; I ran them again."

## `state.health`

```json
{"level": "green|amber|red", "at": "<iso>",
 "checks": [{"id": "manager", "label": "The manager", "level": "ok|healing|failed", "text": "plain words"}]}
```

`null` before the first pass. `level`: red if any check `failed` (a remedy failed or none exists), else amber if
any `healing`, else green. No raw errors, ids or paths except a lane id.

## Web

A dot in the header (hidden without `state.health`): green, amber while a remedy works, red only when one
failed. Click: the checks in plain words; Escape closes. Test: `node web/js/health.test.mjs`.
