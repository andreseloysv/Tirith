# Contract addition: launching a sub-agent from the plan (v1)

Each unfinished step of the Plan in the Overview has a "Start an agent now" button (in the expanded step). The web is a
file:// page (no servers, no open ports), so a click cannot run commands: it opens a `tirith://` link
that a small local handler turns into `tirith launch`.

```
button click -> tirith://launch?project=<id>&step=<n>&token=<t>
             -> Tirith Launcher.app (macOS) -> tirith url open "<url>"
             -> token check -> tirith launch --project <id> --step <n>
             -> git worktree + brief + `tirith lane run` (detached)
```

## `tirith launch --project <id> --step <n> [--agent <alias>] [--dry-run]`

1. Reads the step (title, open items) and the roadmap intro paragraph (the first paragraph after the
   title, before the first `##`) from the project's roadmap (`docs/CONTRACT-roadmap.md`).
2. Refuses (exit 3, reason on stderr) when: the step is unknown, done or has no items; a working lane of
   the same project already has that step (the message names it); `tirith check` fails for the project;
   no agent command is configured.
3. Creates a git worktree `<repo>-lanes/<lane-id>` next to the repo on a new branch `lane/<lane-id>`
   from the current branch. `<lane-id>` is the slug of the step title, made unique against lanes,
   worktrees and branches.
4. Writes `<home>/briefs/<lane-id>.md`:

   | Section | Content |
   |---|---|
   | `# Lane: <step title>` | |
   | `## Why` | the step number and title, then the roadmap intro paragraph |
   | `## What to build` | the open items of the step |
   | `## Hard rules` | do not commit, alias-only agent names, no servers, tests must pass, report what changed, no sub-agents outside `lane run`, nothing outside the worktree |
   | `## Where to read` | `docs/TODO.md` (read only) and `docs/CONTRACT*.md` |

5. Starts, detached (own session, survives the handler exiting; output in
   `<home>/logs/<lane-id>.launch.log`):
   `tirith lane run --id <lane-id> --step <n> --project <id> --class <class> --feature "<title>"
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
`tests-failed`, `ready`, `merge-failed`, `merged`; `null` for lanes not started by `tirith launch`. A `running`
review whose process died shows as `merge-failed` ("interrupted").

Link: `tirith://review?lane=<lane-id>&token=<t>` (same token check as launch). `tirith url open` validates
the lane, starts `tirith lane review <lane-id>` detached (log `<home>/logs/<lane-id>.review.log`) and exits.

Web: in the expanded Plan step, a done launched lane shows "Review and merge" instead of "Start an agent
now": "Testing…" while running, a "Tests failed" / "Merge failed" chip with the output tail, "Merged" at the end.
The step's checkboxes in the roadmap are still ticked by the owner.

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
