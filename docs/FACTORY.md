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
  factory key by key, and a key there wins. The page writes this file, keeping its comments; the CEO writes
  `factory/`.

Like Terraform: the factory is the code, config.toml its variables.

## Loading

- The daemon embeds `factory/` at build time as its fallback, and at run time reads the `factory/` of the checkout it
  was built from (like the page folder; `factory` in config.toml points elsewhere). It reloads when a file changes or
  main moves: no rebuild, no restart.
- Strict: an unknown key, a route to a missing role, or a role on a missing agent or prompt makes a version invalid. The
  daemon keeps the last good version, shows the error on the page and emits `factory.invalid`.
- config.toml: unknown keys are errors shown on the page and told to the manager of no project, never silently ignored (since the port to
  Rust, `[ceo] enabled`, `runs_per_day`, `upkeep_steps` and `upkeep_max_share` are read by nothing). Today's sections
  (`[models.<role>]`, `[lanes]`, `[merge]`, `[ceo]`, ...) are read in memory under the new names (`Config::load`); the
  file itself is not rewritten at start, since `toml::to_string` would drop the owner's comments and reorder its
  blocks. The page writes that file with `toml_edit`, keeping its comments (the bots' roles and schedule routes,
  step 6; and, from each bot's card, its routes, its `session` and its `access`); yesterday's sections stay as they
  are in the file and are read under the new names.

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
name = "Project Manager"           # its face and name head each project's chat and sit beside its messages
agent = "anthropic"
session = "chat"                   # chat: long-lived, resumed; turn: one fresh run; lane: a plan item in a worktree
scope = "project"                  # only two scopes: project (one per project) and own (one bot, a conversation of its own)
prompt = ["prompts/manager.md", "prompts/one-project.md"]
idle_minutes = 30
runs_on = "local"                  # the only value for now
face = { shape = "hex", color = "#8e7cf0" }   # its own, distinct from the sidebar bots' (they override it)

[roles.manager-general]            # a system bot: the manager of no project (health, reviews, what spans projects)
name = "Manager"
extends = "manager"
scope = "own"
prompt = ["prompts/manager.md", "prompts/no-project.md"]
show = true                        # the sidebar paints every own role with show = true
face = { shape = "dome", color = "#3fb6a8" }

[roles.worker]
agent = "anthropic"
session = "lane"
scope = "project"
prompt = ["prompts/brief.md"]

[roles.ceo]                        # a system bot too, like the judge below
name = "CEO"
extends = "manager"
session = "turn"
scope = "own"
access = "read"                    # file-writing tools denied; tirith commands allowed
prompt = ["prompts/ceo.md"]
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
to = "ceo"
then = "manager-general"           # where the CEO's answer goes (it also stands in the CEO's own conversation)

[limits]                           # config.toml may override any of them for this machine
max_load = 2.0                     # per core
inactivity_minutes = 20
long_running_minutes = 30
stage_timeout_minutes = 30
merge_train = 5
merge_backlog = 3
low_disk_gb = 5
build_idle_minutes = 30            # with no lane, stage or manager working that long, a project's build cache and merge folder go
plan_commit_seconds = 120
batch_seconds = 2
agent_messages_in_a_row = 30

[plan]
role_prefixes = ["manager de guardia:", "ceo:"]  # items a role attends: never a lane
owner_wait = ["(espera:", "(espera :"]
waits_for = ["wait until", "made by the item", "created by the item", "after the item", "espera a que", "hasta que"]

[cards]
kinds = ["credentials", "payment", "account", "device", "decision", "approval"]

[metrics.merged]
label = "En main (24 h)"
target = 10
at_least = true
# median, first_try, idle, stuck, main_red, failed, owner: today's labels and targets of metrics.rs
```

A bot, in config.toml:

The factory's own system bots — the CEO (`ceo`, name CEO), the Coach (`coach`, name Coach), the judge (`judge`, name
Judge, `show = false`) and the manager of no project (`manager-general`, name Manager) — are `own` roles of `factory/factory.toml` with a `name`, a
`face` and `show`, exactly like the owner's bots below. The left bar paints every `own` role with `show = true`; on the
page a system bot is seen and edited like any of the owner's bots, and what the owner changes of it (its name, face,
show, routes, `session` and `access`) is saved as a layer in `~/.tirith/config.toml` over its factory role. Every bot has its folder in
`~/.tirith/manager/<key>`, its `memory.md` and its skills. The conversation of no project is manager-general's (its
chat rows, its phone room and its sessions moved there at start, history intact), and the CEO's reviews stand in the
CEO's own conversation. A list of renames (workers → worker, reviewer and improver → ceo, lead → manager-general)
copies, at start, each page model choice `models:<old>` to `models:<new>` when the new one has none, so a renamed
role never loses the model the owner chose for it. Each role with a screen picks its model there — the Project
Manager in each project's chat (one choice for every project) and each bot on its bot screen; Settings keeps only
the roles with no screen of their own (today worker and judge), and `/api/models` gives every role's `face` and
`show` with its setting so the page can tell them apart.

```toml
[roles.bot-1a2b3c]
extends = "manager"
scope = "own"
name = "Noticias"
face = { shape = "drop", color = "#22d3ee" }
prompt = ["prompts/bot.md"]
session = "chat"
access = "read"

[[routes]]
on = "schedule"
days = "weekdays"
at = "08:00"
to = "bot-1a2b3c"
task = "Lo nuevo de mis proyectos, en cinco líneas."

[[routes]]
on = "lane.failed"
to = "bot-1a2b3c"
mode = "note"
phone = false
```

Any bot enters the factory from the page: what makes a bot a CEO or a judge is only its configuration — its
`[[routes]]` (which events it hears, as a `turn` or a `note`, with a chat line and on the phones or not), its
`session` and its `access`. Each bot's card on the page, a system bot's exactly like one of the owner's, shows that
configuration and changes it: its routes are chosen from the closed list of events below (but `schedule` and `webhook`:
its routines are its schedule and webhook routes, edited under Routines), its session is `chat` or `turn`, and its access is read-only or
writing. The card is saved in `~/.tirith/config.toml` with `toml_edit`, as above — an owner's bot as its role and its
routes, a system bot as a layer over its factory role — and takes effect on the reload, with no restart. Routes
written in config.toml to a role replace the factory's routes to that role, as a set (so the page can also take a
factory route away from a bot, the CEO's included); `schedule` and `webhook` routes stand beside them, as a bot's
routines always have. A `webhook` route carries `when` (`<source>:<kind>`, a pattern with `*`, or `*`). An event outside the closed list, a mode that is neither `turn` nor `note`, a session that is neither `chat`
nor `turn` and an access that is neither read nor write are refused by the card's API and nothing is written — and
the strict factory loader remains the check of every key behind it. The `bot_routes_from_the_page` test gives a new
bot the `lane.failed` route through that API and checks that it receives the event, and that changing its routes
replaces the old one.

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
| `merge.landed` | items reached main (its note carries each item's beginning only: up to its first `: `, ` (` or `. `, 120 chars at most) | manager: note, chat, phone |
| `main.red` / `main.green` | a check fails / passes again on main | manager: turn / note, chat, phone |
| `plan.empty` | no open item left | manager: turn |
| `deploy.done` / `deploy.failed` | a deploy stage ran | manager: note / turn, chat, phone |
| `metric.red` | a measure turned red | manager-general: turn, chat, phone; after 15 min the CEO, then manager-general |
| `review.done` | a turn role answered | its route's `then` |
| `schedule` | a routine is due | its role: turn |
| `card.opened` / `card.answered` / `card.closed` | owner cards | none (measured) |
| `factory.invalid` | a version did not load | manager-general: turn, chat |
| `webhook` | a bot's webhook brought an event from outside (agentsmessenger.com's sealed webhook): its source, kind, title, url, text and data | its routines on `webhook` whose `when` holds: turn, the event as data from outside |
| `routine.worse` | a bot's routine got worse: its last 5 runs cost 30 % more (tokens or $) than the 10 before, or 2 of them failed; once per episode, with the bot, the routine and those numbers | coach: turn (its pass, with the `bots` brief) |
| `turn.done` / `routine.done` | a bot's turn ended (`routine.done` when a routine started it): the bot, the trigger, the routine and its task, its tools, skills and the beginning of its answer (`routine.done`: its answer and the «Cómo validar» of the skills it read, 2000 characters at most, for the `turn-quality` eval); its row in `turns` keeps the event's id | none |
| `lab.compare` | a B run of a live experiment ended: the routine's task and the B and A answers as «Answer a» and «Answer b», in an order drawn at random | none (judged by `ab-compare`) |
| `lab.adopted` / `lab.discarded` | Tirith decided an experiment by `[lab]`: one line, as «Adopté gmail-safari v3: −38 % tokens, misma calidad (8/8)» | manager: note, the bot's chat; `lab.adopted` on the phones too |

The answer of a turn reaches the phones only when an event that started it has `phone = true` (`owner.message`
always). Today every manager message reaches them. A routine answer that starts with ∅ still never does.
`tirith say` run inside a Tirith session (with `TIRITH_ROLE` or `TIRITH_LANE`) is that agent's, not the owner's:
it lands in the chat under its role or lane and reaches the manager marked as the agent's, with no
`owner.message`, no owner-mood judging and no `/model` command (that one is the owner's alone).

## Measures

Code computes a closed set of measures from the store; the factory gives each one its label, target and side, and
`metric.red` is routed like any event. Today's: merged in 24 h, median launch to main, first try, idle with work, stuck
lanes, main red, failed lanes, owner questions. A red episode (when it started and whether its CEO pass already
ran) is kept in `kv` and cleared when the measure is green again, so a daemon restart neither announces
`metric.red` again nor repeats the pass for a measure that was already red.

The owner comes first. From 2026-11-13 to 2026-11-20 Tirith wrote 227 notices to the owner's chats and phones (111 of
them merges) and the managers 260 messages; the owner wrote 62. Of 32 cards, 11 were answered, 14 closed without an
answer and 7 are still open.

- `interruptions`: per merged item in 24 h, what reached the owner without the owner asking: phone messages of turns
  that no owner message started, notices with `phone = true`, cards. It should fall toward 0.
- `needless_cards`: the share of cards closed without an answer, plus those the `card-needed` eval says the factory
  could have decided alone.
- `satisfaction`: from the `owner-mood` eval of each owner message: (approvals − corrections − repeats − noise
  complaints) / owner messages, per day. Never measured by asking the owner: a question is itself an interruption.
- `unapproved`: the bots' turns and routines of 24 h the `unapproved` eval labelled `unapproved` (target 0,
  `at_least = false`). A bot asks before it sends, buys, deletes, publishes or touches production, and before any
  step its skill lists under «Qué requiere aprobación»: `tirith ask-owner --kind approval "<exactly what, to whom>"`,
  then it ends its turn (`factory/prompts/bot.md`). Its turn row stays `waiting` while that card is open; the owner's
  answer starts its next turn, like every answer to a card.

## Evals

```toml
[roles.judge]                      # a system bot: small text jobs on a cheap, fast model
name = "Judge"
agent = "muse"
session = "turn"
scope = "own"
access = "read"
show = false                       # not painted in the sidebar

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

[[evals]]
id = "turn-quality"
on = "routine.done"
judge = "judge"
rubric = "evals/turn-quality.md"   # a score from 0 to 1: did the routine's answer do its task?
sample = 1.0

[[evals]]
id = "unapproved"
on = "turn.done"                   # and a second one on routine.done
judge = "judge"
rubric = "evals/unapproved.md"     # ok, or unapproved: done without an answered approval card
sample = 1.0

[[evals]]
id = "ab-compare"
on = "lab.compare"
judge = "judge"
rubric = "evals/ab-compare.md"     # a, b or tie: which of two answers does the task better
sample = 1.0
```

The `ab-compare` eval reads a live experiment's B run and the A run before it, blind: the two answers come as «Answer
a» and «Answer b» in an order Tirith drew at random and kept, and its label is mapped back to B winning, losing or
tying (see the lab below).

The `unapproved` eval reads the event line of a bot's turn or routine, which carries its tool calls (with what each
ran) and the «Qué requiere aprobación» of the skills it read, with the conversation before it (the approval cards
and their answers are in it).

The judge gets the event with its context (the messages before it) and answers a label or a score from 0 to 1 and one
line why. A `scores` row keeps it with the event, the eval and the factory version. A `turn-quality` score is also
copied to the `score` of the bot's turn whose event it judged: `routine.done` carries the routine's task, the answer
and the «Cómo validar» of the skills the run read (2000 characters at most), and the bot's «Números» tab averages it.

## Stamps

Every lane, turn, card, event and score carries the factory version (the main commit that last changed `factory/`, plus
a short hash of config.toml's overlay) and a hash of the role setting it ran with. Comparing two versions, two models or
two prompts is then a query, with no extra runs.

## The CEO closes the loop

The reviewer becomes the CEO. Routed on `metric.red` (after 15 min) and on a `schedule` route (every day at 07:00),
it reads the measures by factory
version, the eval scores and `factory/`, and proposes at most one change to `factory/`: an item of this repo's plan with
its check, and the lane started for it. The queue checks and lands it, and the daemon reloads the factory when main
moves: no rebuild, no restart, no owner. A change whose next 10 merged items (or 24 h) are worse than the version before
it on satisfaction, interruptions or first try is reverted by the CEO's next run, as one more change. Nothing in the
factory needs the owner's yes.

## The Coach makes the bots better

The Coach is a system bot of the factory and a turn role like the CEO: `[roles.coach]` with name Coach, its own face
(a green star), `session = "turn"`, `scope = "own"`, `show = true`, `access = "write"`, `prompt = ["prompts/coach.md"]`,
`brief = "bots"` and `writes = ["bots", "lab"]` (the hooks' rules let it write only in the bots' folders, in
`<home>/bots` and in `<home>/lab`). Its routes are a weekly `schedule` (`days = [1]`, `at = "09:00"`) and
`routine.worse`. Which brief a turn role reads is its `brief` key: `ceo` (the default: the measures by factory version)
or `bots` (each bot's numbers, routines, skills with the sections they lack and memory sizes). Each runs within its
`max_per_day`, its answer in its own conversation. The Coach reads the run log and proposes one change per experiment,
with its hypothesis and the gain it expects, and starts it: a branch of the bot's repository with that one commit (made
in a draft worktree under `<home>/lab` it removes), then `tirith lab start <bot> <branch> --routine <id> --mode live
--hypothesis "<text>"`; its answer says so in 3 lines at most, or «Sin novedad.» when nothing changed since its last
pass. The bots' folders and
`<home>/bots` are git repositories Tirith commits after each turn that changed them and each Library change, so every
run's `versions` are in their history.

## The lab decides by a rule fixed before the test

`[lab]` is the rule, the factory's and never the Coach's, read by the strict loader (`deny_unknown_fields`, shares from
0 to 1):

```toml
[lab]
inputs = 8              # comparisons before Tirith decides
not_worse_share = 0.5   # B must win or tie this share of them (and win at least as often as it loses)
min_saving = 0.15       # and save this share of the mean cost (tokens or $) or time
max_days = 7            # decided with what it has after this many days
```

A live experiment alternates the routine's real runs between A (the bot's main) and B (the branch, in a worktree of
the lab that is the bot's folder for that turn); each B is compared with the A before it by `ab-compare`. With
`inputs` comparisons or after `max_days`, B is adopted when it is not worse, has no new failure and no 👎, and saves
at least `min_saving`; adopted, its branch is merged into the bot's main, else deleted, and the bot's chat gets the
line of `lab.adopted` or `lab.discarded`. The bot's «Mejoras» tab lists every experiment with its numbers and decision
(docs/DAEMON.md, `lab/`).

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

## Size

Configuration replaces special cases: a step that adds a mechanism removes the code it makes general. The migration
below ran under a limit on the whole of `rust/tirith/src` (6000 lines). Since 2026-11-21 the limit is per file instead:
every code file that is not a test has 100 lines or less, as `cargo fmt` writes it with rustfmt's defaults
(`tools/size.sh`, docs/BOTS.md part 0). The figures of the steps below are history.

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
   (manager, the CEO's, bot, brief) and the two scope texts of manager.rs. The loader as above. Every value
   listed moves out of Rust. 6200 lines or less.
3. **Events and routes.** One function emits an event; `[[routes]]` replace every call to `manager::notify`,
   `notify_as`, `inform` and `tell` outside manager.rs, including the rule for answers on the phones; the default routes
   reproduce today (a table test). 6200 lines or less.
4. **Quieter phones**, in `factory/` only: `merge.landed`, `deploy.done` and `main.green` get `phone = false`.
5. **Roles.** manager, manager-general, worker and ceo as roles: `session`, `scope`, `extends`, `access`, `idle_minutes`,
   `max_per_day`, `runs_on`. `ROLES` and every special case by role name go; the CEO review runs as any turn role; the
   page and `/model` list the factory's roles; role items of the plan come from `[plan] role_prefixes`. 6200 lines or less.
6. **Bots in config.** Bots become roles with `scope = "own"` and their routines `schedule` routes in config.toml; the
   page writes that file keeping its comments (`toml_edit`); the bots kept in kv move there once, at start. 6200 lines or
   less.
7. **Stamps**, as above. 6200 lines or less.
8. **Owner measures:** `interruptions` and `needless_cards`, with targets and routes in the factory. 6200 lines or less.
9. **Evals and satisfaction:** `[[evals]]`, the `judge` role, the `scores` table, the rubrics, `satisfaction`.
   6200 lines or less.
10. **The CEO, and 6000.** The reviewer becomes the CEO; the daemon reloads the factory when main moves; the
    limit of tirith.toml goes back to 6000. 6000 lines or less.

Later: experiments.
