# The factory

Tirith is a software factory defined as code. `factory/factory.toml` says who works (roles), on which agent, what each
role hears (routes), what it may do, how a plan item reaches main, what is measured, and how the factory improves itself.
The daemon keeps only the mechanisms: agent processes, the sandbox and its proxy, worktrees and slots, the merge train,
rooms, the store and the page. Every decision those mechanisms take (who, when, which agent and model, which prompt,
which target, which limit) is read from the factory.

Today those decisions are Rust: one manager per project plus one of no project, a CEO that answers only that one, eight
metrics with their targets, about twenty `manager::notify/inform/tell` calls that fix who hears what, prompts compiled
into the binary, each agent's flags in a `match` on its name, and plan items for a role recognised by Spanish prefixes.
This document is the target and the order of the migration (the last section).

## Where it lives

- `factory/` in this repo: `factory.toml`, `prompts/*.md`, `evals/*.md`. Nothing personal goes there: the repo is
  public and every merge pushes main. Its commit guard also refuses some vendor and model words in added lines, so the
  real agent commands and model ids are never written in it.
- `~/.tirith/config.toml` on the machine, private: projects and their folders, rooms and phones, each agent's
  `command`, model ids, keys, the port, and the owner's own roles (the bots) with their routines. It is laid over the
  factory key by key, and a key there wins. The page writes this file, keeping its comments; the improver writes
  `factory/`.

Like Terraform: the factory is the code, config.toml its variables.

## Loading

- The daemon embeds `factory/` at build time as its fallback, and at run time reads the `factory/` of the checkout it
  was built from (like the page folder; `factory` in config.toml points elsewhere). It reloads when a file changes or
  main moves: no rebuild, no restart.
- Strict: an unknown key, a route to a missing role, or a role on a missing agent or prompt makes a version invalid. The
  daemon keeps the last good version, shows the error on the page and emits `factory.invalid`.
- config.toml: unknown keys are errors shown on the page and told to the lead, never silently ignored (since the port to
  Rust, `[ceo] enabled`, `runs_per_day`, `upkeep_steps` and `upkeep_max_share` are read by nothing). Today's sections
  (`[models.<role>]`, `[lanes]`, `[merge]`, `[ceo]`, ...) are read in memory under the new names (`Config::load`); the
  file itself is not rewritten at start, since `toml::to_string` would drop the owner's comments and reorder its
  blocks. Its sections are renamed in the file when the page writes it with `toml_edit`, keeping the comments (step 6).

## The file

The default factory reproduces today's behaviour; the values below are today's.

```toml
[agents.anthropic]                 # the harness; its command lives in config.toml
protocol = "stream-json"           # stream-json: a live chat session; exec: one process per turn
model_flag = "--model"
effort_flag = "--effort"
chat_args = ["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose"]
lane_args = ["-p", "Read the brief at {brief} and do it completely in {workspace}."]
resume_args = ["--resume", "{session}", "-p", "{message}"]

[agents.muse]
protocol = "exec"
model_flag = "--model"
effort_flag = "--reasoning-effort"
lane_args = ["exec", "--json", "--workspace", "{workspace}", "--prompt-file", "{brief}"]
# [agents.codex] and [agents.copilot]: today's flags() of models.rs

[roles.manager]                    # runs one project's work and talks with the owner
agent = "anthropic"
session = "chat"                   # chat: long-lived, resumed; turn: one fresh run; lane: a plan item in a worktree
scope = "project"                  # project: one per project; global: one for all; own: a conversation of its own
prompt = ["prompts/manager.md", "prompts/one-project.md"]
idle_minutes = 30
runs_on = "local"                  # the only value for now

[roles.lead]                       # the conversation of no project: health, reviews, what spans projects
extends = "manager"
scope = "global"
prompt = ["prompts/manager.md", "prompts/no-project.md"]

[roles.worker]
agent = "anthropic"
session = "lane"
scope = "project"
prompt = ["prompts/brief.md"]

[roles.reviewer]                   # today's CEO
extends = "manager"
session = "turn"
scope = "global"
access = "read"                    # file-writing tools denied; tirith commands allowed
prompt = ["prompts/reviewer.md"]
max_per_day = 6

[[routes]]
on = "lane.failed"
to = "manager"
mode = "turn"                      # turn: wakes the role now, with the other events of the next batch_seconds; note: waits for its next turn
chat = true                        # a line in the project's chat
phone = true                       # that line, and the answer of the turn it starts, reach the owner's phones

[[routes]]
on = "metric.red"
after_minutes = 15                 # still red that long
to = "reviewer"
then = "lead"                      # where the reviewer's answer goes

[limits]                           # config.toml may override any of them for this machine
max_load = 2.0                     # per core
inactivity_minutes = 20
long_running_minutes = 30
stage_timeout_minutes = 30
merge_train = 5
merge_backlog = 3
low_disk_gb = 5
plan_commit_seconds = 120
batch_seconds = 2
agent_messages_in_a_row = 30

[plan]
role_prefixes = ["manager de guardia:", "ceo:"]  # items a role attends: never a lane
owner_wait = ["(espera:", "(espera :"]
waits_for = ["wait until", "made by the item", "created by the item", "after the item", "espera a que", "hasta que"]

[cards]
kinds = ["credentials", "payment", "account", "device", "decision"]

[metrics.merged]
label = "En main (24 h)"
target = 10
at_least = true
# median, first_try, idle, stuck, main_red, failed, owner: today's labels and targets of metrics.rs
```

A bot, in config.toml:

```toml
[roles.bot-1a2b3c]
extends = "manager"
scope = "own"
name = "Noticias"
face = { shape = "drop", color = "#22d3ee" }
prompt = ["prompts/bot.md"]

[[routes]]
on = "schedule"
days = "weekdays"
at = "08:00"
to = "bot-1a2b3c"
task = "Lo nuevo de mis proyectos, en cinco líneas."
```

## Events and routes

Code emits a closed list of events. Each one is a row of `events` (its project, its data, the factory version), is shown
on the page, and goes to every route whose `on` matches it. The default routes reproduce today:

| event | today | default route |
|---|---|---|
| `owner.message` | the owner writes in a chat or from a phone | the conversation's role: turn, its answer to the phones |
| `room.message` | another agent writes in a room | manager: turn, its answer back to the room |
| `lane.asked` | `tirith ask` | manager: turn, chat, phone |
| `lane.failed` | out of credits, no commits, too many tries | manager: turn, chat, phone |
| `lane.not_started` | the scheduler could not start it | manager: turn, chat, phone |
| `lane.long` | running longer than `long_running_minutes` | manager: turn, chat, phone |
| `merge.blocked` | no disk, a broken slot, main cannot move | manager: turn, chat, phone |
| `merge.landed` | items reached main | manager: note, chat, phone |
| `main.red` / `main.green` | a check fails / passes again on main | manager: turn / note, chat, phone |
| `plan.empty` | no open item left | manager: turn |
| `deploy.done` / `deploy.failed` | a deploy stage ran | manager: note / turn, chat, phone |
| `metric.red` | a measure turned red | lead: turn, chat, phone; after 15 min the reviewer, then the lead |
| `review.done` | a turn role answered | its route's `then` |
| `schedule` | a routine is due | its role: turn |
| `card.opened` / `card.answered` / `card.closed` | owner cards | none (measured) |
| `factory.invalid` | a version did not load | lead: turn, chat |

The answer of a turn reaches the phones only when an event that started it has `phone = true` (`owner.message`
always). Today every manager message reaches them. A routine answer that starts with ∅ still never does.

## Measures

Code computes a closed set of measures from the store; the factory gives each one its label, target and side, and
`metric.red` is routed like any event. Today's: merged in 24 h, median launch to main, first try, idle with work, stuck
lanes, main red, failed lanes, owner questions.

The owner comes first. From 2026-11-13 to 2026-11-20 Tirith wrote 227 notices to the owner's chats and phones (111 of
them merges) and the managers 260 messages; the owner wrote 62. Of 32 cards, 11 were answered, 14 closed without an
answer and 7 are still open.

- `interruptions`: per merged item in 24 h, what reached the owner without the owner asking: phone messages of turns
  that no owner message started, notices with `phone = true`, cards. It should fall toward 0.
- `needless_cards`: the share of cards closed without an answer, plus those the `card-needed` eval says the factory
  could have decided alone.
- `satisfaction`: from the `owner-mood` eval of each owner message: (approvals − corrections − repeats − noise
  complaints) / owner messages, per day. Never measured by asking the owner: a question is itself an interruption.

## Evals

```toml
[roles.judge]                      # small text jobs on a cheap, fast model
agent = "muse"
session = "turn"
scope = "global"
access = "read"

[[evals]]
id = "owner-mood"
on = "owner.message"
judge = "judge"
rubric = "evals/owner-mood.md"     # one label: approval, neutral, correction, repeat, noise
sample = 1.0

[[evals]]
id = "card-needed"
on = "card.closed"
judge = "judge"
rubric = "evals/card-needed.md"    # could the factory have decided it alone?
sample = 1.0
```

The judge gets the event with its context (the messages before it) and answers a label or a score from 0 to 1 and one
line why. A `scores` row keeps it with the event, the eval and the factory version.

## Stamps

Every lane, turn, card, event and score carries the factory version (the main commit that last changed `factory/`, plus
a short hash of config.toml's overlay) and a hash of the role setting it ran with. Comparing two versions, two models or
two prompts is then a query, with no extra runs.

## The improver closes the loop

The reviewer becomes the improver. Routed on `metric.red` (after 15 min) and once a day, it reads the measures by factory
version, the eval scores and `factory/`, and proposes at most one change to `factory/`: an item of this repo's plan with
its check, and the lane started for it. The queue checks and lands it, and the daemon reloads the factory when main
moves: no rebuild, no restart, no owner. A change whose next 10 merged items (or 24 h) are worse than the version before
it on satisfaction, interruptions or first try is reverted by the improver's next run, as one more change. Nothing in the
factory needs the owner's yes.

## What stays in code

Processes per protocol, the sandbox and its proxy, worktrees and slots, the merge train, rooms and their crypto, the store
and its schema, the page, the closed lists of events and measures, and the loader (strict, last good version kept,
embedded fallback). The loader cannot be turned off from the factory: it is what undoes a bad factory.

## Tests stay fast

- `cargo test -q -p tirith` runs 44 tests in under a second (13 s with the build). It becomes a stage of `tirith.toml`,
  so every lane runs it.
- A unit test loads `factory/`: it parses, every reference resolves, and the default routes equal a table of today's.
- Nothing slow gates a merge, and no replay benchmark is a check. Quality is measured online, on real work, with the
  stamps, the measures and the evals. Later, experiments split real traffic between two settings of a role
  (`[[experiments]] role = "worker"`, `share = 0.2`, `agent = "muse"`), compared by stamp.

## Line budget

`rust/tirith/src` had 6329 lines on 2026-11-20 (limit 6330). The migration brings it back to 6000: step 1 ends at 6200
or less, no later step ends above 6200, and step 10 ends at 6000 or less and sets the limit of `tirith.toml` back to 6000.
Configuration replaces special cases: a step that adds a mechanism removes the code it makes general. Real simplification
only: formatted code (`cargo fmt -p tirith`), no joined lines.

## Migration

One plan item and one lane per step, in this order, each starting from a main that has the step before. Each step keeps
the daemon's behaviour unless it says otherwise, ends with `cargo test -q -p tirith` green, and updates docs/DAEMON.md
and the README. Nothing goes live in the running Tirith until its daemon is rebuilt and restarted.

1. **Brain out, tests in.** Delete `brain.rs` and all that only serves it: `/api/brain`, `brain::start`, the `brain`
   role (in `ROLES`, `BRAIN_MODEL`, its defaults and checks in models.rs, `/model brain`), the brief's `{architecture}`,
   the room summary (`summarize` in rooms.rs), the page's Brain choices, and its lines in the README, docs/SUBMISSION.md
   and docs/DAEMON.md (Muse stays in through the `muse` agent). Add a stage `test` running `cargo test -q -p tirith` to
   tirith.toml. 6200 lines or less.
2. **The factory file.** `factory/factory.toml` with the agents (today's `flags()`), the roles with today's settings,
   `[limits]`, `[plan]`, `[cards]` and `[metrics.*]`; `factory/prompts/` with the prompts of `rust/tirith/assets`
   (manager, the CEO's as reviewer, bot, brief) and the two scope texts of manager.rs. The loader as above. Every value
   listed moves out of Rust. 6200 lines or less.
3. **Events and routes.** One function emits an event; `[[routes]]` replace every call to `manager::notify`,
   `notify_as`, `inform` and `tell` outside manager.rs, including the rule for answers on the phones; the default routes
   reproduce today (a table test). 6200 lines or less.
4. **Quieter phones**, in `factory/` only: `merge.landed`, `deploy.done` and `main.green` get `phone = false`.
5. **Roles.** manager, lead, worker and reviewer as roles: `session`, `scope`, `extends`, `access`, `idle_minutes`,
   `max_per_day`, `runs_on`. `ROLES` and every special case by role name go; the CEO review runs as any turn role; the
   page and `/model` list the factory's roles; role items of the plan come from `[plan] role_prefixes`. 6200 lines or less.
6. **Bots in config.** Bots become roles with `scope = "own"` and their routines `schedule` routes in config.toml; the
   page writes that file keeping its comments (`toml_edit`); the bots kept in kv move there once, at start. 6200 lines or
   less.
7. **Stamps**, as above. 6200 lines or less.
8. **Owner measures:** `interruptions` and `needless_cards`, with targets and routes in the factory. 6200 lines or less.
9. **Evals and satisfaction:** `[[evals]]`, the `judge` role, the `scores` table, the two rubrics, `satisfaction`.
   6200 lines or less.
10. **The improver, and 6000.** The reviewer becomes the improver; the daemon reloads the factory when main moves; the
    limit of tirith.toml goes back to 6000. 6000 lines or less.

Later: experiments.
