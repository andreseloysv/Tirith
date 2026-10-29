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
