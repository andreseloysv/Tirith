# The single merge queue

Agents never touch main. A lane whose contract is complete (ended `done`, `tirith plan report --status done`)
is enqueued (`tirith merge request`, or by the dispatcher). Tirith merges one lane at a time
(`collector/tirith/mergeq.py`, state in `<home>/mergeq.json`):

1. In a temporary worktree cut from main it merges `lane/<id>`.
2. It runs every build and test stage of the manifest there (recorded pipeline runs).
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
   On the return the lane's plan item is reopened with the reason (`(reabierto: <result>: ...)`),
   and the queue checks that the relaunched agent started (its new lane registers within `START_WAIT`, 20 s):
   the lane's `review` records `sent_to` and `started`; when it did not start, the manager's queue is told.

`state.projects[].merge_queue`: `{items: [{lane, position, status: queued|running, since, feature}], last, recent}`;
the Plan card shows it. Commands: `tirith merge request|run|status`.

Gate `main-merge` (an addition; no other gate changed): from a lane's worktree, `git -C <main checkout> merge|push`
(also after `cd <main>`) and pushes of `main`/`master`/the main branch are blocked with the instruction above.
