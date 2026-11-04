# The single merge queue

Agents never touch main. A lane whose contract is complete (ended `done`, `tirith plan report --status done`)
is enqueued (`tirith merge request`, or by the dispatcher). Tirith merges (`collector/tirith/mergeq.py`, state in
`<home>/mergeq.json`) one pass at a time; a pass is one lane, or a train of several (see "Fast paths" below):

1. In a temporary worktree cut from main it merges `lane/<id>`.
2. It runs every build and test stage of the manifest there (recorded pipeline runs): the full suite, once per
   pass (a train is one pass). The Python test stage holds the machine's one full-suite lock, so two full suites
   never run at once (`docs/CONTRACT-launch.md`, "Test selection"); a lane's own review only ran what its change reaches.
   After a landing it starts `tirith testmap build` in the background when the execution map is missing or old.
3. Under the plan writer lock (stale `index.lock` recovery) it fast-forwards the main checkout to exactly the
   tested commit. If main moved meanwhile only by commits that touch the plan file(s) (the manifest `roadmap`
   and `plans`; Tirith commits every plan change there, so main moves every few minutes), it merges the new main
   into the tested result without running the suite again (the lane cannot touch the plan, so the merge is clean),
   checks that the new tree differs from the tested one only in the plan file(s), and fast-forwards to that. If
   main moved in any other file it starts again (3 tries); if it still moves, the entry goes back to the queue
   with its reason and the runner goes on to the next entry (it does not stop). Then it pushes `origin`, ticks the plan
   item and removes both worktrees and the branch.
4. A conflict or a red stage goes back to the same agent: a relaunch whose brief continues the lane's branch
   with the conflict / failure appended (marker `[merge queue]`; after 3 returns the manager's queue decides).
   Never the owner. A git command cut off at its time limit (exit 124, e.g. the merge) is not a conflict: the
   attempt is retried (up to 3 times), then the lane waits in the queue as `waiting` ("timed out"); never sent back.
   Before that, a red test stage is repeated test by test: each `FAILED` id of the report runs alone with `-n 0`
   (recorded as stage `<name>-rerun`). If all pass, the merge goes on and each test is counted as flaky in
   `<home>/flaky.json` (`{project: {test id: {count, last}}}`, shown as `merge_queue.flaky`; the merged history row
   carries `flaky`). If one fails again, or the failed tests cannot be told (`ERROR` lines, a timeout, no report),
   it is a real red. No test is skipped or deleted.
   On the return the lane's plan item is reopened with the reason (`(reabierto: <result>: ...)`),
   and the queue checks that the relaunched agent started (its new lane registers within `START_WAIT`, 20 s):
   the lane's `review` records `sent_to` and `started`; when it did not start, the manager's queue is told.

5. Before any of this it commits what the agent left in the worktree. If the branch then has no commits main lacks
   there is nothing to merge: no suite and no merge commit, the lane's `review.status` becomes `no-work`, the entry
   closes (`closed`, "nothing to merge"), the worktree and branch are removed and the queue ticks nothing (the claim
   check in `docs/CONTRACT-launch.md` decides the item). "Already merged" for a gone worktree comes from the same check.

6. A commit that fails on an abandoned `index.lock` in the lane's worktree (older than 60 s, held by nobody) removes the
   lock and is tried again; it is not a red suite.
7. A lane whose commits are all already in another queued lane's branch (a continuation cut from it) stops like step 5:
   no suite, no merge, `review.status` `no-work`, message "already in the queued branch of <lane>", worktree and
   branch removed. Two branches on the same commit keep the one queued first.

Send-back details. The branch used to continue a returned lane is `mergeq.branch_of` (the branch recorded on the lane or
checked out in its worktree), not `lane/<id>`, so a continuation lane that reuses another lane's worktree can be returned.
A lane with no registered agent (`agent` null) is returned to the configured worker; one whose agent is not in
`config.toml` is not returned and the manager's queue is told why.

Continuation lanes. A lane whose workspace is another lane's worktree (a folder of `<repo>-lanes`) is enqueued by the
dispatcher when it ends `done` with outcome `done`, like a lane that owns its worktree.

Tested trees. A stage whose name contains `test` that passes is remembered by the git tree hash of the worktree (tracked
and untracked files, ignored ones left out) in `<home>/tested.json` (`{project|pipeline|stage|command hash: [{tree, at,
run, log}]}`, last 40). `runstage.run_stage(reuse=True)` (the lane's `tirith run --stage`, the review run with its claim check and the
queue) does not run the same stage with the same command on a tree that already passed: it records a passing run
(`cached: true` in `events.jsonl`) whose log keeps the earlier output, so the claim check still reads the counts. A failed
or timed out run is never remembered. `TIRITH_RETEST=1` forces the run; `*-rerun` stages always run.

WIP limit (`mergeq.WIP_LIMIT` = 3, `mergeq.wip_block`): with 3 or more lanes waiting in the queue (any project,
queued or running) no new lane starts, whoever asks (manager, autopilot, owner-less launch): `launch_step`,
`launch_plan` and `launch_verify` refuse with a Tirith block that says how many wait and what to do; the plan
item stays open and can be launched once the queue is below 3. Lanes already running and lanes the queue sends
back (`relaunch_lane`) are not affected. Decision: a `running` entry counts as waiting (it is not merged yet).

After a merge the lane records the merged `branch`. `state.projects[].merge_queue`: `{items: [{lane, position, status: queued|running, since, feature}], last, recent}`;
the Plan card shows it. Commands: `tirith merge request|run|status`.

Gate `main-merge` (an addition; no other gate changed): from a lane's worktree, `git -C <main checkout> merge|push`
(also after `cd <main>`) and pushes of `main`/`master`/the main branch are blocked with the instruction above.

## Fast paths: the stages that apply, and the train

**Only the stages that apply.** The stages run for a lane depend on its own diff (its commits against the merge
base with main), never on anything else:

- only files under `docs/`, `site/` or `ios/`, or `*.md` → the build stages only, no test stage (a plan-only lane
  lands in seconds);
- no file under `app/src-tauri/` and no `Cargo.*` → every stage except `rust-test`;
- anything else, or an empty diff → every stage, as before. A lane touching `collector/`, `web/`, `app/`,
  `scripts/` or `tirith.toml` always runs the Python tests.

A skipped stage is named in the history row (`skipped`) and in its message ("skipped test, rust-test: ...").

**Train.** `[mergeq] train = 5` in `config.toml` (default 5; 1 turns it off and every lane goes alone, exactly
as before). A pass takes up to `train` queued lanes of the same project, oldest first, and merges them one by one
into one temporary worktree:

1. A lane that conflicts (or whose commit fails) leaves the train with its real reason and is sent back as usual;
   the others go on.
2. The stages run once on the result: the union of what each lane needs (see above).
3. Green: main is fast-forwarded once (the "only the plan moved" rule applies as for one lane), pushed once, and
   every lane is ticked, closed and gets its own history row (`train` lists the lanes that landed together).
4. Red: the same lanes are redone one at a time with the normal single-lane flow, so only the culprit is
   rejected (sent back); the others land on their own.

## Every failure comes back with its reason

- conflict / red stage / failed commit → the same agent (send back), the manager's queue when that cannot happen.
- `waiting` (dirty main, branch not found, no temp worktree, no stage...): the entry stays queued; on the 2nd
  consecutive `waiting` (`WAIT_NOTICE`) the manager's queue gets one item with the reason.
- `push-failed` (main merged, origin not): a manager item at once, with the reason and the command to push.
- out of time: manager item on the 2nd time (as before). A lane or project that is gone: flow alert to the manager.
