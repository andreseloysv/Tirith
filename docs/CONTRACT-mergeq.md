# The single merge queue

Agents never touch main. A lane whose contract is complete (ended `done`, `tirith plan report --status done`)
is enqueued (`tirith merge request`, or by the dispatcher). Tirith merges one lane at a time
(`collector/tirith/mergeq.py`, state in `<home>/mergeq.json`):

1. In a temporary worktree cut from main it merges `lane/<id>`.
2. It runs every build and test stage of the manifest there (recorded pipeline runs).
3. Under the plan writer lock (stale `index.lock` recovery) it fast-forwards the main checkout to exactly the
   tested commit; if main moved meanwhile it starts again (3 tries). Then it pushes `origin`, ticks the plan
   item and removes both worktrees and the branch.
4. A conflict or a red stage goes back to the same agent: a relaunch whose brief continues the lane's branch
   with the conflict / failure appended (marker `[merge queue]`; after 3 returns the manager's queue decides).
   Never the owner.

`state.projects[].merge_queue`: `{items: [{lane, position, status: queued|running, since, feature}], last, recent}`;
the Plan card shows it. Commands: `tirith merge request|run|status`.

Gate `main-merge` (an addition; no other gate changed): from a lane's worktree, `git -C <main checkout> merge|push`
(also after `cd <main>`) and pushes of `main`/`master`/the main branch are blocked with the instruction above.
