# Gates contract: rules as code

No rule lives in a markdown file. Every rule below is code that blocks, with a message that tells the
agent what to do instead. Agents are named by alias (`anthropic`, `muse`); the real executable of each
alias is `[agents.<alias>] command` in the owner's `~/.tirith/config.toml`.

Every rule describes itself in one plain sentence for someone who does not code (for example raw-stage:
"Tests and builds must run through Tirith, so you can see them in Pipelines."). Built-in rules: the table
`gates.DESCRIPTIONS`; a test fails when a rule of `manifest.BUILT_IN_RULES` has none. Custom rules: the
required `description` field (`CONTRACT-setup.md`). The description is what the web page shows; the
message below is for the agent only.

Message format, one block per violation:
`tirith: <rule-id>: <what was blocked>. Do this instead: <exact command>`

## Rules

| Rule | Blocks | Enforced in | Do this instead |
|---|---|---|---|
| `subagent-tool` | the agent's internal sub-agent tool (`Task`, `Agent`) | `PreToolUse` hook | `tirith lane run --project <id> --class <class> --feature "<description>" --brief <file> -- <agent cmd> …` |
| `agent-cli` | a shell command that starts a configured agent CLI (any simple command of the line, also by absolute path, `npx`/`bunx`, or inside `sh -c`) without `tirith lane run` | `PreToolUse` hook, shims | the same `tirith lane run …` line |
| `no-verify` | the git flag that skips hooks (long form), `git commit -n` (also `-an`) | `PreToolUse` hook | run the git command without it; fix what the hook reports |
| `hooks-path` | any command naming `core.hooksPath` | `PreToolUse` hook | edit `[[hooks]]` in `tirith.toml`, run `tirith check` |
| `git-hooks` | editing/removing files under `.git/hooks` (shell mutators, redirects, Edit/Write tools) | `PreToolUse` hook | leave it; the owner uses `tirith guard install` |
| `roadmap-edit` | an edit/write tool or shell command that writes to a plan file of the project (`roadmap` and `plans` of `tirith.toml`, else `docs/TODO.md` / `TODO.md`), in a lane's worktree or in the main checkout (plain readers and git's read-only commands are allowed) | `PreToolUse` hook | `tirith plan add --project <id> --step <N> --text "<item>"` to add work, `tirith plan report --lane $TIRITH_LANE --status done\|partial\|blocked` to report; only Tirith writes the plan |
| `plan-item` | `lane run` / `tirith launch` without `--step <N> --item "<open item>"`, or with an item that is not open in the plan (exit 3) | `lane run`, `launch` | `tirith plan add --project <id> --step <N> --text "<item>"`, then start again with that item |
| `settings-edit` | an edit/write tool or shell command touching a configured agent settings file (`[agents.<alias>] settings`) or `~/.tirith/config.toml` (plain readers such as `cat`, `grep`, `ls` are allowed), and `tirith hook uninstall` | `PreToolUse` hook | leave them alone; the owner changes them (`tirith hook install --agent <alias>` restores the hooks) |
| `hooks-removed` | not a block: recorded as a violation when the collector found the hooks gone from an agent's settings and put them back | collector (every cycle) | nothing to do; the hooks are already restored |
| `raw-stage` | a stage `run` of the project's `tirith.toml` executed raw (the whole sequence, or the stage's last command with its args) | `PreToolUse` hook | `tirith run [--project <id>] [--pipeline <id>] --stage <name>` |
| `raw-stage` (runner) | in a folder that belongs to a project with a `tirith.toml`, a build or test runner not wrapped by `tirith run`: pytest (also `python -m pytest`, `uv run … pytest`), `npm`/`pnpm`/`yarn` `test` or `build`, vitest, jest, `go test`/`build`, `cargo test`/`build`, `gradle`/`gradlew` test/build/assemble, `make test`/`check`/`build`, `xcodebuild`, `swift test`/`build`, `node --check` | `PreToolUse` hook | `tirith run --project <id> --stage <stage> [-- <cmd>]` (test runners: the project's test stage; builds: its build stage, by stage kind; with no such stage: `tirith setup add-stage …`) |
| `lane-brief` | `lane run` with no `--brief`, or a missing/empty brief file (exit 3) | `lane run` | write the task into a file, pass `--brief <file>` |
| `project-check` | `lane run` while `tirith check` fails for the project (exit 3) | `lane run` | fix the listed problems, run `tirith check --project <id>` |
| (guard) | `lane run` while the commit guard is not intact (exit 3) | `lane run` | `tirith guard install <project>` |
| `workspace-write` | in a lane (`TIRITH_LANE`): Edit/Write/NotebookEdit, or a shell write (`>`, `>>`, `tee`, `rm`, `mv`, `cp` target, `mkdir`, `touch`, `chmod`, `ln`, `sed -i`, `dd of=`, `git -C <dir>` that changes a folder), whose real path (symlinks followed) is outside the lane's worktree, `$TMPDIR`, `/tmp`, `/private/tmp`, `/private/var/folders` and the project's `<repo>-lanes` folder. `git -C <main checkout> merge\|push\|pull` is the lane's closing step and is allowed | `PreToolUse` hook | write in the lane copy or a temp folder; ask the owner (`tirith wait add`) for anything else |
| `no-secrets` | in a lane: Read/Grep/Glob/Edit tools or any shell word that names `~/.ssh`, `~/.aws`, `~/.gnupg`, `~/.config/gh`, `~/.netrc`, `~/Library/Keychains`, `~/.codex/auth*`, `~/.copilot` credentials, the credentials folder of each agent CLI, `*.pem`, or a `.env` file outside the worktree (`.env.example` is fine) | `PreToolUse` hook | ask the owner for what is needed |
| `no-servers` | in a lane: `python -m http.server`, `serve`, `uvicorn`, `gunicorn`, `flask run`, `vite`/`next`/`nuxt` dev, `npm\|pnpm\|yarn run dev\|start\|serve`, `nc -l`, `php -S`, any `--port` flag | `PreToolUse` hook | test through `tirith run --stage …` |
| `own-processes` | in a lane: `pkill`/`killall`, `xargs kill`, `kill` of a pid outside the lane's process group or from a command substitution (`kill $!`, `%1` and own-group pids are fine) | `PreToolUse` hook | stop only what the lane started |

The four lane rules live in `tirith.lanegates`; compound commands (`&&`, `;`, `|`, `$(…)`, `bash -c`) are checked segment by segment, a write to an unresolvable `$VAR` path is denied, and every deny is a `violation` event with the rule and the command. Lane agents are never asked for permission (`agentrun` presets allow every tool): these rules and the ones above decide.

Custom rules: every `[[rules]]` entry of the project's `tirith.toml` is enforced by the same
`gates.check_call`, with the same message format and `violation` events (`CONTRACT-setup.md`).

`tirith run --stage …`, `tirith lane run … -- <agent>` and commands that merely mention an agent name
(`echo`, `cat`, file names) are never blocked. Here-document bodies are data and never count
(`strip_heredocs`); a runner on a later line of the same command still does. The runner check reuses
the detection of `tirith.activity` (`runner_kinds`); only one `raw-stage` message is given per call,
the exact-stage-match one first. Folders without a `tirith.toml` are not gated.

## Blocking

`tirith hook` records the event as usual, then for `PreToolUse` runs `hooks.decide`. A violation exits
**2** with the message on stderr (the anthropic CLI feeds stderr back to the model). Muse uses the same
exit 2 (its docs give no other convention). The JSON alternative (`permissionDecision: "deny"`) is not
used.

Every block is also appended to `events.jsonl` as
`{"kind": "violation", "ts", "rule", "agent", "lane", "session", "project", "tool", "message", "cwd"}`.
Pipeline code and hook readers ignore these rows.

`TIRITH_GATES=off` in the environment never disables anything (an agent could set it). The only off
switch is the owner's config:

```toml
[gates]
enabled = false
```

## Shims

`tirith shim install|uninstall|verify [--dir <folder>]` (default `~/.tirith/shims`, which `lane run` puts first in every lane's PATH, before the git wrapper in `~/.local/bin`). For every
`[agents.<alias>] command` it writes an executable `sh` wrapper named like that command. The wrapper
`exec`s the real binary (resolved at install from `PATH` without the shim folder) when stdin and stdout
are terminals, or when `TIRITH_LANE_TOKEN` equals the `token` in `lanes/$TIRITH_LANE.json`; otherwise
it prints the `agent-cli` message and exits 2. A file without the marker line (`# tirith-shim v1`) is
never overwritten or removed. `verify` exits 1 when a shim is missing, foreign, not executable, points
at a missing binary, or is not first in `PATH`.

On Windows the shim is `<name>.cmd` (second line `REM tirith-shim v1`, CRLF): it cannot detect a
terminal, so it runs the real binary only when the token matches (`findstr` on the lane file);
otherwise it prints the same message and exits 2. `verify` reads its `set "REAL=..."` line.

### Windows: commit guard and safe-git shim

**Guard.** On Windows (`sys.platform` starts with `win`) `tirith_guard` renders the same hooks for Git for
Windows' bundled `sh`: always LF (written as bytes, never translated), `git` from PATH instead of
`/usr/bin/git`, `sh -c` instead of `/bin/sh -c`, and the `tirith.toml` reader drops a trailing `\r`. The lock
is the read-only attribute instead of `chflags uchg`. `verify` accepts it (no execute bit is checked; a writable
hook is "not read-only"; a hook with CRLF is reported as such), `guard reinstall` clears the attribute first and
the upgrade steps use `attrib -r` / `del`. Elsewhere the rendering is unchanged.

**Safe-git shim.** `tirith safegit install|uninstall|verify [--dir <folder>]` (default `~/.tirith/shims`, first in
every lane's PATH) writes `git.cmd` (CRLF, `REM tirith-safegit v1` on line 2, real git path resolved at install
from PATH without the shim folder) and `tirith_safegit.py` (a copy of `tirith/safegit.py`, standard library
only). The core refuses, with the usual `tirith: <rule>: … Do this instead: …` message and exit **2**: `--no-verify`
(and its abbreviations, also inside an alias), `git commit -n` / `-an`, any `core.hooksPath` (`-c`,
`--config-env`, `git config`, `GIT_CONFIG_KEY_n`, `GIT_CONFIG_PARAMETERS`) and `commit-tree`; the value of
`-m`/`-F`/`-C` is data. Everything else runs the real git. A `git.cmd` without the marker is never overwritten
or removed; the macOS wrapper in `~/.local/bin` is never touched. `verify` exits 1 when the shim is missing,
foreign, differs from this Tirith's core, points at a missing git or Python, or is not first for `git` in PATH.

`lane run` generates the token, stores it in the lane file, passes it as `TIRITH_LANE_TOKEN` to the
child and removes it when the lane ends.

## In `state.json`

`violations`: the last 50 (`ts, rule, agent, lane, project, message`). `waiting_on_you` gets one item
(`source: "violation"`) per violation of the last hour. Each row also carries `blocked` (what was
blocked) and `instead` (the command), split from `message`.

`rules`: every active rule once (custom rules once per project):
`{id, description, source: "built-in"|"tirith.toml", project, count_24h, last_at, last_agent}`.
`count_24h` counts the violations of the last 24 hours (all of `events.jsonl`, not only the 50 kept in
`violations`); `last_agent` is the lane id (else the agent alias) of the newest one, null when none.

The web shows an Overview "Rules" section with one row per rule that fired in the last 24 hours: the
description, "11 times today · last 09:04 by <avatar> Zeludile", and a fold-out list of the blocked
commands (time, agent, what was blocked). The "Do this instead" command is not shown there (the agent
already got it). The agent panel still lists a lane's violations in red with a count on agent cards, and
Flow draws a red edge from the agent to The Guard.

## Activate

```sh
tirith hook install --agent anthropic      # and: --agent muse
tirith shim install                        # shims in ~/.tirith/shims, first in every lane's PATH
tirith shim verify
```

## Guard chaining: `tirith guard install <project> --chain`

For a repository that already has its own hooks (for example a protected-paths `pre-commit`). Plain
`install` never touches a hook without the marker and reports `present but without the
FORBIDDEN_WORDS_GUARD marker`. With `--chain`, every such hook among `pre-commit`, `commit-msg` and
`pre-push` is moved to `<hook>.local` in the repository's hooks folder (never over an existing `.local`
file, which is left for the owner), and the guard is installed in its place. The rendered guard runs
`<hook>.local` after its own checks with the hook's arguments and **fails when it fails** (the block is
recorded as `the repository's own <hook> hook failed`). `verify` accepts the chained install: it renders
the expected text with the chain block for every hook that has a `.local` file, and reports a `.local` file
that is not executable. `install` also locks (`uchg`) a guard that is exactly as expected but lost its flag.

`[[projects]] guard = false` in `config.toml` (written by the "Never for this project" button) switches
the check off for one project.

## Stop a running lane: `tirith lane stop <lane-id> [--reason ..] [--by owner|manager]`

`lane run` starts a non-interactive child as the leader of its own process group and stores `pgid` in
the lane file. `lane stop` writes `stop: {by, reason, at}` into the lane file, sends **SIGTERM** to that
group, waits up to **10 s** and then sends **SIGKILL**. The lane ends `failed` with the report
`Stopped by <owner|manager>: <reason>` (`Stopped by <who>.` without a reason); `lane run` ends it when its
child is gone, and `lane stop` ends it itself if `lane run` is gone. A stopped lane adds no
manager queue item. The worktree is kept. An interactive lane (own terminal) is signalled by
pid only. Unknown or already ended lanes exit 3. `--by` defaults to `manager` for agents and scripts and
`owner` at a terminal; the dispatcher and the manager call `tirith lane stop` (or
`lanestop.stop_lane`) the same way.

From the page: a **Stop** button on working or stalled agents (card top right, drawer / list row, next to
the avatar in the Plan step), each with a confirm popover first ("Stop <name>? It ends as failed and its
worktree is kept."). It sends only the lane id: the Tauri command `lane_stop(lane_id)` (checks the id
against the running local lanes in `state.json`, then runs `tirith lane stop <id> --by owner` in the
background), or the `tirith://stop?lane=<id>&token=<launch token>` link (`tirith url open` checks the
token and that the lane is running), or the line `tirith lane stop <id> --by owner` to copy.

## Hooks stay installed: `hooks.protect`

Every collector cycle checks, for each `[agents.<alias>]`, that all `INSTALL_EVENTS` entries are in its
settings file (`hooks.has_hooks`). If an agent whose hooks were seen intact before lost them (edited
file, deleted file), `hooks.install` puts them back, merging, never deleting another key, with the same
executable and role as before, and a `hooks-removed` violation is recorded with the time. An agent never
installed is reported but not touched. `[gates] enabled = false` switches the healing off too. The result
is kept in `~/.tirith/protect.json` and shown in `state.projects[].setup.protection`
(`[{agent, ok, checked_at, healed_at}]`); the Setup card shows "Hooks protected · last check 2 s ago".
