# Contract addition: a project's setup, seen and changed by asking (v1)

The owner sees, per project, whether there are builds, tests, a deploy step, git hooks and which rules
apply, and can add a rule or a stage by asking (chat or the UI form). Everything lives in the project's
`tirith.toml` (`CONTRACT-manifest.md`); Tirith only reads it, except for the explicit `tirith setup`
edits.

```
collector: tirith.toml + runs + agent settings + shims  -> state.projects[].setup (read only)
UI form / chat -> inbox request kind "setup" -> manager: tirith setup apply <id> -> tirith.toml -> tirith check
```

## Custom rules: `[[rules]]` in `tirith.toml`

```toml
[[rules]]
id = "no-force-push"
match = "git push --force*"        # command glob, or regex = "...", or paths = ["harness/**"] for edits
action = "block"                    # or "warn"
message = "force pushes rewrite history"
instead = "git push (without --force); ask the owner if history must change"
description = "Nobody may rewrite the shared history by force-pushing."
described = "3fa9c01b27"          # written by tirith setup, see below
```

| Field | Meaning |
|---|---|
| `id` | unique in the file, not a built-in rule id |
| `match` | glob (`*` matches anything, also spaces) against each simple command of a Bash call, as typed and without env assignments or wrappers (`env`, `sudo`, …), and against the whole line |
| `regex` | Python regular expression searched in the whole Bash command line |
| `paths` | globs for files an agent may not edit (`Edit`, `Write`, `MultiEdit`, `NotebookEdit`), tried on the path as given, relative to the folder of `tirith.toml` and relative to the cwd |
| `action` | `block`: the `PreToolUse` hook exits 2. `warn`: the message is printed and the violation recorded, the call goes through |
| `description` | **required**: one plain sentence saying what the rule does, for someone who does not code. Shown by the web page; never read by the agent |
| `described` | hash of `action` + the match fields when the description was written, kept by `tirith setup add-rule` / `describe-rule`. When the match fields no longer hash to it, `tirith check` reports `rule-stale` ("description may be stale"); reread the description and run `tirith setup describe-rule` |
| `message`, `instead` | what the agent reads: `tirith: <id>: <message>: <what was matched>. Do this instead: <instead>` |

Exactly one of `match`, `regex`, `paths`. `gates.check_call` enforces them next to the built-in rules;
each hit is a `kind: "violation"` event (a `warn` one also has `"action": "warn"`), so it shows in
`state.violations`. `tirith check` reports bad entries as `<file>:<line>: <rule>: <what>. Fix: <how>` with
the rules `rule-id`, `rule-duplicate`, `rule-match`, `rule-regex`, `rule-paths`, `rule-action`,
`rule-message`, `rule-instead`, `rule-description` (missing), `rule-stale`, `rule-shape`.

## `tirith setup`

| Command | Does |
|---|---|
| `tirith setup show [--project P]` | pipelines, hooks and rules of the project |
| `tirith setup add-stage --pipeline ci --name lint --run "make lint" [--kind build\|test\|deploy] [--after build]` | adds `[[pipelines.stages]]` (after the given stage, else last); a new pipeline id creates `[[pipelines]]` at the end |
| `tirith setup remove-stage --name lint [--pipeline ci]` | cuts that stage block (and the comments right above it) |
| `tirith setup add-rule --id … (--match G \| --regex R \| --paths a,b) [--action block\|warn] --message … --instead … --description …` | appends a `[[rules]]` block; `--description` is required |
| `tirith setup describe-rule --id … --description …` | rewrites the description and refreshes `described` |
| `tirith setup remove-rule --id …` | cuts that block |
| `tirith setup apply <request-id>` | applies a `setup` inbox request, answers it and closes it (`run-stage` runs `tirith run`) |

`--project` defaults to the project of the cwd. The edits work on the text of `tirith.toml`: they insert,
append or cut whole blocks and leave comments and formatting elsewhere alone. Every edit runs
`tirith check` (manifest rules, not the commit guard) and prints its problems; exit 3 when the edit is
refused (reason and `Fix:` on stderr, file untouched), 1 when the edit introduced problems that were not
there before, else 0.

## Inbox request `kind: "setup"`

See `CONTRACT-inbox.md`. `text` is JSON, one of:

```json
{"action": "add-stage", "pipeline": "ci", "name": "lint", "run": "make lint", "kind": "test", "after": "build"}
{"action": "remove-stage", "pipeline": "ci", "name": "lint"}
{"action": "add-rule", "id": "no-force-push", "match": "git push --force*", "effect": "block", "message": "…", "instead": "…", "description": "…"}
{"action": "remove-rule", "id": "no-force-push"}
{"action": "run-stage", "pipeline": "ci", "name": "build"}
```

For `add-rule`, `description` is required, `effect` is `block` or `warn` and one of `match`, `regex`, `paths` (list or comma text)
is given.

## `state.projects[].setup`

```json
{"manifest": "/path/tirith.toml",
 "check": {"ok": true, "problems": [{"rule": "pipeline-coverage", "message": "…", "fix": "…", "line": 1}]},
 "pipelines": [{"id": "ci", "name": "CI", "stages": [
   {"name": "build", "kind": "build", "run": "make build",
    "last": {"status": "passed", "at": "…", "duration_s": 21, "run": "ci#12"}}]}],
 "hooks": [{"event": "pre-commit", "run": "make lint", "last": null}],
 "rules": [{"id": "raw-stage", "label": "Raw pipeline stage", "source": "built-in",
            "enforced_by": ["hook"], "active": true, "description": "…"},
           {"id": "no-force-push", "label": "no-force-push", "source": "tirith.toml", "action": "block",
            "enforced_by": ["hook"], "active": true, "description": "…"}],
 "gates_active": true, "gates_problems": [], "gates_fix": [], "gates_summary": "", "gates_actions": [],
 "pending": [{"id": "in-5a41c9e2", "at": "…", "action": "add-stage", "summary": "add-stage ci/lint: make lint"}]}
```

- `manifest` is `null` without a `tirith.toml`; then `check.problems` holds `manifest-missing`.
- `check` is `tirith check` without the commit guard rule, plus the commit-guard problems the collector
  already found (`rule: "commit-guard"`).
- `kind` is the stage's `kind` or the one recognised from its name (`build`, `test`, `deploy`), else null.
- `last` is the newest run of that stage that started (`status` `passed`, `failed` or `running`, `at` its
  end or start); null = never run. Hooks use the project's `git-hooks` runs.
- `rules`: the built-in rules (`CONTRACT-gates.md`, plus `commit-guard`) and the `[[rules]]` of the file.
  `enforced_by` lists `hook`, `shim`, `git`, `lane run`, `CI`; a rule is `active` when any of them works
  on this machine: `hook` = tirith hooks are in the settings of every configured agent, `shim` = shims
  intact and hooks installed, `git` = the commit guard is intact, `lane run` and `CI` always.
- `gates_active` = hooks installed for every `[agents.<alias>]` (settings file read, never written) and
  `verify_shims` clean and `[gates] enabled` not false. `gates_problems` says why not, `gates_fix` is the
  list of fix steps (`tirith hook install --agent <alias>`, `tirith shim install`, or, for an alias with
  no `settings` path, the line to add to config.toml or the section to remove).
  The shims are checked by `gates.verify_home_shims`, the same function as `tirith shim verify`:
  `<home>/shims` against the lane PATH. `gates_summary` is one plain sentence on what is missing and why
  it matters ("" when all is active); an alias with no `settings` path is named, with whether its CLI is
  on the lane PATH (it can run unguarded) or not (not installed here). `gates_actions` are Run actions
  (generated, stable ids) for the install steps; config.toml edits have no Run action, the sentence says
  what to change. Nothing is installed by collecting.
- `pending`: open `setup` requests of the project that the manager has not closed.

## Web (`web/js/v-setup.js`)

A "Setup" card per project in the Overview, under the Plan (project filter applies):

- banner (first card only, only when `gates_active` is false): the `gates_summary` sentence with the
  `gates_actions` as Run buttons (`T.actions`); no copyable commands; nothing at all when all is active;
- `tirith check`: green "Ready", or the problems with their `Fix` lines;
- Pipelines: each stage as a chip `build · make build ✓ 2 min ago` (kind icon, last result, `never run`),
  with a **Run** button that sends a `run-stage` request;
- Git hooks: event → command and its last result;
- Rules: every rule as its description (custom first, with `BLOCK`/`WARN` tag, then built-in) and its
  on/off state;
- The web has no add-stage / add-rule forms: the owner asks the manager in the chat (the CLI
  `tirith setup add-stage|add-rule …` stays). **Run** sends a `run-stage` request: Tauri `inbox_add`, else
  the `tirith://inbox` link, else the `tirith run …` command to copy. Until the collector shows a request
  closed, the card lists it as "Waiting for the manager".
- The Overview (Your turn, Plan, Setup, Rules, Pipelines) shows one project: the sidebar's, else the one
  with the most recent activity (`T.effProject`); the filter stays in the URL hash.
- Auto projects never come from temporary folders (`projects.is_temp_folder`).
