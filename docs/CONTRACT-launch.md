# Contract addition: launching a sub-agent from the plan (v1)

Each unfinished step of the Plan in the Overview has a "Start an agent now" button (in the expanded step). The web is a
file:// page (no servers, no open ports), so a click cannot run commands: it opens a `tirith://` link
that a small local handler turns into `tirith launch`.

```
button click -> tirith://launch?project=<id>&step=<n>[&item=<text>]&token=<t>
             -> Tirith Launcher.app (macOS) -> tirith url open "<url>"
             -> token check -> tirith launch --project <id> --step <n> --item "<open item>"
             -> git worktree + brief + `tirith lane run` (detached)
```

## The plan is the only way to start an agent

No agent starts without an open item of the plan, with no exceptions (`docs/CONTRACT-roadmap.md`, "The plan
gate"). Every start (`tirith lane run`, `tirith launch`, the dispatcher, the autopilot, the manager lanes, a start
again after running out of credits or without a plan outcome, plan refresh and new plan, verification lanes, the
desktop app's chat and launch commands) passes `--step <n> --item "<exact text of the open item>"` (or the item's id
`i-xxxxxxxx`), and code checks it against the project's plan before anything is created. Otherwise the start is
refused (exit 3):

```
tirith: plan-item: no agent starts without a plan item: that item is not an open item of step 2.
Add it to the plan first: tirith plan add --project demo --step 2 --text "<what you will do>"
```

It is also refused when the plan has no such step, the item is already done, in review, blocked, or in progress
by a lane that is still running, or the project has no plan file. What Tirith starts by itself gets its item from
Tirith before it starts (`tirith.plangate.ops_item`, in step `0` "Trabajo automático de Tirith", already marked
"(en curso: <lane>)" so the autopilot never takes it): plan refresh `Revisar el plan (<date>)`, new plan
`Plan nuevo: <title>`, verification `Verificar: <check>`, the manager lane `Manager de guardia: atender el chat`
(one standing item), the owner's chat `Chat con el dueño: <chat id>` (ticked when the chat ends), a review
`Revisar: <item>` (a sub-item under the reviewed item). The autopilot, a click on a step and the owner's "Finish
this step" request name no item: Tirith takes the first open item an agent can do in that step. `tirith lane
start` only records an agent that already runs; it is linked to an item only when `--item` is given.

When the lane starts, the item is marked `(en curso: <lane>)`. The lane stores `item` (and `item_step` when it
is not the lane's `step`, e.g. step `0` for plan refresh, whose `step` stays `plan-refresh`).

## A lane finishes with an outcome

`tirith plan report --lane <id> --status done|partial|blocked [--add "<new item>"]... [--reason "<why>"]`
(the lane is `$TIRITH_LANE` by default) records the outcome of the lane's item in `lane.outcome` and Tirith
writes it into the plan:

| Status | Plan |
|---|---|
| `done` | the item becomes `(en revisión)`; each `--add` is a new open item of the same step |
| `partial` | like `done`, and needs at least one `--add` for what is left |
| `blocked` | needs `--reason`; the item becomes `(bloqueado: <reason>)`, or `(dueño: <reason>)` when the reason starts with an owner reason key (`credentials`, `payment`, `personal-account`, `physical-device`, `owner-decision`; the owner gate rules apply); a blocked lane is not reviewed or merged |

A lane that ends `done` without a report ends as `no-outcome`: its work is not reviewed or merged, its item is
reopened (`(reabierto: <lane> terminó sin informe)`) and the dispatcher starts it once more with a note in the brief
(`retry_of` names the first lane); when that one also ends without a report, one manager queue item
`sq-no-outcome-<lane>` is opened. The owner's interactive conversations, the chat lane and the manager lane are
exempt from the report: their item is ticked (chat) or given back (manager) when they end.

An outcome the plan could not take at once (the plan file has uncommitted changes) is kept in the lane and written
by the next dispatcher pass.

**Done only when verified.** `done` never ticks anything. When the automatic review starts, a sub-item `Revisar:
<item>` is added under the item. A merge (tests passed) ticks the item and the sub-item, after the claim check
that the lane's work is on the project's branch; a failed review closes the sub-item with the reason and reopens the
item as `(reabierto: <reason>)`. Lanes without an item keep the older report matching (below, point 3).

## `tirith launch --project <id> --step <n> --item "<open item>" [--agent <alias>] [--dry-run]`

1. Reads the step (title, open items) and the roadmap intro paragraph (the first paragraph after the
   title, before the first `##`) from the project's roadmap (`docs/CONTRACT-roadmap.md`).
2. Refuses (exit 3, reason on stderr) when: the step is unknown, done or has no items; `--item` is missing or is
   not an open item of the step (see above); a working lane of
   the same project already has that item (the message names it; other items of the step are free); `tirith check` fails for the project;
   no agent command is configured. `tirith lane run --step N` (what every start ends in: autopilot,
   dispatcher, manager lanes) refuses the same way (exit 3, "<lane> is already working on item "<item>" of step N") unless it
   continues that lane (`--fallback-from`: fallback, relaunch, claims re-send).
3. Creates a git worktree `<repo>-lanes/<lane-id>` next to the repo on a new branch `lane/<lane-id>`
   from the current branch. `<lane-id>` is the slug of the step title, made unique against lanes,
   worktrees and branches.
4. Writes `<home>/briefs/<lane-id>.md`:

   | Section | Content |
   |---|---|
   | `# Lane: <step title>` | |
   | `## Why` | the step number and title, then the roadmap intro paragraph |
   | `## Your item` | the item the lane works on and a reminder to report it |
   | `## What to build` | the item |
   | `## Hard rules` | do not commit, alias-only agent names, no servers, tests must pass, report what changed, no sub-agents outside `lane run`, nothing outside the worktree, never edit `docs/TODO.md`, and the two commands `tirith plan add` and `tirith plan report` |
   | `## Where to read` | `docs/TODO.md` (read only, only Tirith writes it) and `docs/CONTRACT*.md` |

5. Starts, detached (own session, survives the handler exiting; output in
   `<home>/logs/<lane-id>.launch.log`):
   `tirith lane run --id <lane-id> --step <n> --item "<item>" --project <id> --class <class> --feature "<title>"
   --brief <file> --workspace <worktree> -- <agent command>`.
   The class comes from keywords of the step title (`warden`, `loremaster`, `glasswright`,
   `swiftrider`), else `stonewright`.
6. Prints `lane <lane-id>`.

`--dry-run` runs every check, then prints the worktree, brief and command and creates or starts nothing.

### The agent command

`config.toml`, outside the repo:

```toml
[launch]
command = "agentcmd --brief {brief} --cwd {workspace}"   # string (shell-split) or list

[agents.muse]
command = "muse"
launch = "muse run {brief}"                              # used with --agent muse
```

`{brief}` and `{workspace}` are replaced by the brief file and the worktree. `--agent <alias>` uses that
alias's `launch` template and falls back to `[launch] command`. Agents are named by alias only.
When `[models.workers]` sets a model or effort (`docs/CONTRACT-models.md`), the model and effort flags of
the command are replaced by the configured ones.

## The link handler (macOS)

`tirith url install|uninstall|verify [--apps-dir <dir>]` manages `~/Applications/Tirith Launcher.app`:
an AppleScript app (`on open location`, compiled with `osacompile`) with `CFBundleURLSchemes = [tirith]`,
no Dock icon, registered with Launch Services. It runs `tirith url open "<url>"` detached.
`verify` checks the bundle exists and declares the scheme. Other platforms: not yet (`install` says so).

`tirith url open "<url>"` accepts only `tirith://launch?project=..&step=..[&agent=..]&token=..`.

## Token

`<home>/launch.json` holds `{"launch_token": "<random>", "created_at": ...}` (mode 0600), created by the
collector on first run. The same token is written into `state.json`, so only the owner's page knows it.
A link with a wrong or missing token is refused (exit 3) and recorded as a `launch-token` violation
(`kind: "violation"` in `events.jsonl`, shown in `state.violations` and "waiting on you").

## `state.launch`

```json
{"token": "<launch_token>", "available": true}
```

`available` is true when the handler app is installed.

## Web (`web/js/v-plan.js`)

On every step that is not done and has no working agent, in the expanded step: a rocket button
"Start an agent now" (the direct path; the row's buttons are the owner's inbox, `docs/CONTRACT-inbox.md`).

- `available`: click opens the link; the button shows "Starting…" until a working agent appears on the
  step (the lane is tagged with `--step`, so its avatar shows) or 90 s pass.
- not `available`: click shows a tooltip with the one command to install the handler (`tirith url install`).

`tirith url open` also accepts `tirith://inbox?kind=..&project=..&step=..&text=..&token=..` (same token check): see `docs/CONTRACT-inbox.md`.

## Review and merge (`tirith lane review <lane-id> [--tests-only]`)

For a lane that `tirith launch` started (its workspace is `<repo>-lanes/<lane-id>`) and that ended `done`.
The agent leaves its work uncommitted in the worktree; review finishes the job:

1. Refuses (exit 3) when the lane is unknown, still running, not `done`, not a launched lane, already
   merged or already being reviewed, or when the manifest has no test stage with a `run` command.
2. Runs the first test stage of `tirith.toml` (kind `test`, else a name like `test`) in the worktree.
   Failing tests: `review.status = "tests-failed"`, nothing is merged (exit 1).
3. With `--tests-only`: stops with `ready`.
4. Otherwise: refuses (`merge-failed`) if the repo has uncommitted tracked changes; commits the worktree
   (`Lane <id>: <feature>`), merges `lane/<id>` into the branch checked out in the repo with `--no-ff`
   (a conflict is aborted: `merge-failed`), then removes the worktree and the branch: `merged`.

The result is stored on the lane file as `review` `{status, at, pid, message?, commit?}`. `state.agents[].review`
is `{status, at?, message?}` with `status` one of `pending` (launched lane, done, not reviewed), `running`,
`tests-failed`, `claims-failed`, `ready`, `merge-failed`, `merged`; `null` for lanes not started by `tirith launch`. A `running`
review whose process died shows as `merge-failed` ("interrupted").

`tirith lane drop <lane-id>` removes the worktree and the branch of an unmerged launched lane (`review.status = "dropped"`).
`state.agents[].review` also carries `run` (the pipeline run of the tests, known while they run), and after a
merge or passing tests `files`, `branch` and `passed` (the count the test runner printed, if any).

### Claim check (`tirith/claimcheck.py`)

After the tests pass and before any merge, the lane's final report (`lane.report`) is checked against facts, by
regexes and line structure only (no model). Each claim that does not hold is one sentence in
`review.claims.mismatches`; `review.claims` is `{ok, checked, mismatches}` (also on `ready`/`merged`).
- files said changed/added/created/updated: must be in `git diff <start_commit>` (commits and uncommitted)
  or untracked; lines with a negation ("did not change") are skipped;
- "added N tests" / `test_x in <file>`: the diff must add that many test functions (python `def test_`,
  rust `#[test] fn`, js `test(`/`it(`), the named ones, in the named file;
- "tests pass" / "N passed": a passing test-stage run of the lane's worktree must be recorded in
  `events.jsonl` after the last changed file, and N must be a count printed by such a run's log;
- "committed as <sha>": the commit must exist; "merged": the lane's HEAD must be in the repo's branch;
  "pushed": the lane's HEAD must be on a remote branch ("merged main into ..." and future tense are skipped);
- "looked at the screenshot <path>": the file must exist and be newer than the lane's start;
- the brief's "Fix"/"Do" bullet items against a diff that is empty or only docs (`*.md`, `*.rst`, `docs/`,
  README), unless the item itself is about docs/wording.
A lane sent back (`fallback_lane`) is judged against the first lane's `start_commit` and start time.

Any mismatch: `review.status = "claims-failed"`, `review.message` is the list, nothing is merged (exit 1).
The dispatcher settles it without bothering the owner: the lane is sent back once to its agent
(`cli.relaunch_lane(..., note=...)`: the old work is committed on its branch, the new worktree starts from it,
the brief gets the list appended under `## Claim check failed`); if the brief already has that heading, or the
agent is unknown, the manager's queue gets one item with "Retry the review" / "Drop the lane". The Agents detail
shows "Claims verified ✓" or the mismatches.

### Automatic review (`tirith/autoreview.py`, called by every dispatcher pass)

No finished lane is left unreviewed, by code. State in `<home>/dispatch.json` under `reviews` (per lane) and
`overdue`.

1. A launched lane that ended `done` (within the last 24 h) gets, once, a manager queue item
   (`kind: "review"`) and a detached `tirith lane review <lane>` (it merges when the tests pass; `[review] auto_merge`
   is ignored: merging is never a question for the owner). The tests are an ordinary pipeline run tagged with the lane.
2. When the review has a result, the dispatcher settles it once per result:
   - `merged`: the lane's `request_id` gets an answer `by: "tirith"` "Merged <lane> into <branch>: <n> files,
     tests <n> passed"; the review item is closed with that result; the plan is updated (below).
   - `tests-failed` / `merge-failed` (conflict, dirty main checkout, low disk, ...): nothing is merged; ONE
     manager queue item with the reason and the commands "Retry the review", "Start an agent to fix it",
     "Show the log", "Drop the lane". Never an owner item.
   - `ready` (a manual `--tests-only` run): a lane with no review record is merged by the next pass; a settled
     `ready` result is one manager queue item "not merged".
   A later result of the same lane (a retry) replaces the earlier queue item.
3. Plan update on merge (lanes without a plan item only; a lane with an item is ticked by the plan gate above): the brief lists the step's open items under "What to build"; items whose words
   (at least 70 %) appear in the report's "What changed" lines are ticked in the repo's roadmap file and
   committed as "Roadmap: step N progress from <lane>" (through the git hooks; a refused commit or an edited
   plan file leaves the plan untouched). Unsure items (30 to 70 %, or no "What changed" section) stay
   unticked and one `kind: "plan"` queue item "update the plan for step N" lists them.
4. Deadline: a lane that ended more than 30 minutes ago with unmerged work in its worktree and no review result
   (never started, or the process died) is a `review-overdue` violation in `events.jsonl`, one macOS
   notification, and an owner item `review-overdue-<lane>` with Run actions "Review it now" / "Drop the lane".
5. The manager `Stop` hook also blocks while review queue items older than 60 s are open (a review that is
   running right now does not count), listing `tirith lane review <lane>` and `tirith queue done <id>`; same
   limit of 3 blocks per set, then an `inbox-ignored` violation.

Web: under a Plan step whose lane has a review: "Testing <agent>…" linked to the pipeline run, "Merged <agent> ·
<time>" with files and tests, or the failure with the output tail (the owner options are in "Your turn", where
Run actions of an ask are shown). The agent drawer shows the review result.

Link: `tirith://review?lane=<lane-id>&token=<t>` (same token check as launch). `tirith url open` validates
the lane, starts `tirith lane review <lane-id>` detached (log `<home>/logs/<lane-id>.review.log`) and exits.

Web: in the expanded Plan step, a done launched lane shows "Review and merge" instead of "Start an agent
now": "Testing…" while running, a "Tests failed" / "Merge failed" chip with the output tail, "Merged" at the end.
The step's checkboxes in the roadmap are still ticked by the owner.

### Disk diet (`tirith/diskdiet.py`, called by every dispatcher pass)

- **Cleanup.** A lane that ended `done`, was started by `tirith launch` (never the manager lane or the main
  checkout) and whose branch `lane/<id>` is an ancestor of the project checkout's HEAD loses its worktree
  (`git worktree remove`, never `--force`) and its local branch (`git branch -d`). `tirith lane review` merges
  the same way. A worktree with uncommitted changes is left alone: `lane.cleanup = {status: "dirty", message, at}`
  (`blocked` when git refuses). A removed lane gets `cleanup = {status: "removed", branch_deleted, at}`. Lane
  records and logs are kept.
- **Shared cargo target.** Every stage run (`runstage.run_stage`, so `tirith run` and the review) sets
  `CARGO_TARGET_DIR` unless the environment already has it: the project's `cargo_target_dir` in
  `config.toml` (`[[projects]]` / `[projects.<id>]`), else `<repo>-lanes/.cargo-target`. Worktrees share compiled
  dependencies instead of building ~2 GB each.
- **Disk guard.** Each pass records `state.disk = {free_bytes, min_free_bytes, low, at, queue?}` (also in
  `dispatch.json`): the smallest free space of the folders holding the projects' lanes. Below `[disk] min_free_gb`
  (default 5) `low` is true: the autopilot starts no lane, `finish` / plan requests wait unanswered, and one
  manager queue item (`kind: "disk"`, not an owner item) says why; it closes when space is back.

## Real launch from the browser (owner checklist)

A lane cannot do this itself: it registers a URL scheme in the owner's Launch Services and starts an agent.

1. `tirith url install`, then `tirith url verify` prints `handler OK`.
2. `tirith launch --project <id> --step <n> --dry-run` on an unfinished step prints the worktree, brief and command
   and creates nothing (checks `tirith check`, the agent command and that no lane has the step).
3. Open the Overview in the browser, expand that step in the Plan and click "Start an agent now". The browser asks once
   to open "Tirith Launcher"; accept.
4. Expected: the button shows "Starting…", then the step shows the working agent's avatar (lane tagged with `--step`)
   within 90 s; `<home>/logs/<lane-id>.launch.log` has the output; `tirith lane list` shows the lane.
5. If nothing happens: run `tirith url open "<the link>"` by hand (a wrong token exits 3 and records a `launch-token`
   violation), and read `launch.log`.

Status: automated parts pass (see "Link-to-command check"); steps 3 and 4 are still to be done by the owner.

## Installing the handler (macOS)

`tirith url install` builds `~/Applications/Tirith Launcher.app` and registers it with Launch Services; run
it once. Verified with `--apps-dir <dir>`: the bundle compiles, `verify` passes and the bundle declares
`CFBundleURLSchemes = [tirith]`. A wrong-token link is refused with exit 3.

### Verified on macOS

`tirith url install --apps-dir <dir>` compiles the bundle with `osacompile`, writes `CFBundleURLSchemes = [tirith]`
and `LSUIElement`, registers it with `lsregister`; `tirith url verify --apps-dir <dir>` reports `handler OK`;
`tirith url uninstall` unregisters and removes it. Not done by the lane: the real install in `~/Applications`
and a real click-to-launch (they change the owner's Launch Services and start an agent). The owner runs
`tirith url install` once, then clicks "Start an agent now" in the Plan.

### Link-to-command check (automated)

`test_handler_command_runs_url_open_end_to_end` takes the shell command from the handler's AppleScript, appends the
quoted link (with its `&` separators) and runs it through `/bin/sh` with `TIRITH_HOME`, as `do shell script` does. A
wrong-token link exits 3 and records the `launch-token` violation; nothing is launched. A right-token link for a done step
(`test_handler_command_with_right_token_reaches_launch`) gets past the token check into `tirith launch`, which refuses the step. What stays manual: after
`tirith url install`, open the Plan in the browser, click "Start an agent now" and confirm `lane <id>` appears in the step.
