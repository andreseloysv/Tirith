<div align="center">

<img src="site/img/tirith-logo.png" alt="The Tirith logo: a large smiling luminous bot in front, with many smaller glowing bots streaming out from behind it with trails of light against a starry sky." width="220">

# Tirith

**An autonomous manager for your coding agents. They finish the plan; you see everything and decide only what only you can.**

[![CI](https://github.com/andreseloysv/Tirith/actions/workflows/tirith.yml/badge.svg)](https://github.com/andreseloysv/Tirith/actions/workflows/tirith.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
![Platform: macOS](https://img.shields.io/badge/platform-macOS-lightgrey?logo=apple)
![Built for the Meta Global AI Developer Hackathon](https://img.shields.io/badge/built%20for-Meta%20AI%20Hackathon-0467DF?logo=meta&logoColor=white)
[![GitHub stars](https://img.shields.io/github/stars/andreseloysv/Tirith?style=social)](https://github.com/andreseloysv/Tirith)

[**Landing page**](https://andreseloysv.github.io/Tirith/) · [Submission](docs/SUBMISSION.md) · [Contributing](CONTRIBUTING.md)


⭐ **If Tirith helps you, please star the repo.** It is the best way to say it is useful.

</div>

## Quick start

```bash
git clone https://github.com/andreseloysv/Tirith && cd Tirith && ./run
```

Needs Rust (`cargo`) and git. `./run` builds the `tirith` binary, starts the daemon when nothing answers on its
port and opens its page (`http://127.0.0.1:7717/`). The daemon keeps running; `tirith status` shows what it does.

## What it does

Agents stop halfway, ask what they could decide, and lose the big picture. Tirith keeps the plan moving to the
end with code, and puts a model in the loop only where judgment is needed.

- **Plan-driven.** Each project's plan is a markdown checklist (`docs/TODO.md`: `## N. Title` steps,
  `- [ ] item` lines, optionally `(check: \`cmd\`)`). Tirith is its only writer; agents add items with
  `tirith plan add` and steps with `tirith plan step`.
- **Autopilot.** Code starts the open items of a project's current step (the first step with open work; steps
  run in order, the items of a step in parallel), as far as the machine has room: the load per core, the merge
  backlog, free disk and an intact commit guard. There is no cap on lanes at once unless `[limits] max_parallel`
  sets one. Failed or stopped items wait for the manager.
- **Lanes.** One agent, one item, one branch, in a reusable worktree whose build caches stay warm. The worktrees
  live in Tirith's own folder (`~/.tirith/lanes/<project>/slot-N`), not next to your repositories. A lane cannot end its turn with a question; it asks the manager with `tirith ask` and is resumed with
  the answer. Messages reach running lanes through their hooks ("main advanced: …").
- **Done means checked.** A lane that ends goes to the merge queue: merged with main (several lanes in one
  train), the project's checks and each item's check run, and only a green result moves main, with the plan item
  ticked in the same fast-forward. A red check that also fails on main marks main red and pauses the autopilot;
  otherwise the same agent session gets its work back with the log, twice at most.
- **Merged means deployed.** A project's deploy stages (`kind = "deploy"`) are no check of a lane: they run after
  the merge, on main as it is then, so what is on main is what gets deployed. A deploy that fails never keeps
  finished work out of main: the manager is told once, Tirith tries again by itself (after ten minutes, then
  after twice as long each time) until it passes, and Retry on its pipeline tries at once.
- **Rules in code.** Every tool call of every agent passes the rules first, in 1–4 ms: commit checks are never
  skipped, the plan and the agents' settings are not edited by agents, a lane writes only in its own worktree,
  never pushes or touches main and never starts other agents. Projects add rules in `tirith.toml`.
- **Network sandbox.** Everything Tirith starts (lanes, the manager, reviews, merge checks) runs in a sandbox
  (macOS's, or on Linux a network namespace of its own, without root) that allows the network only through the
  daemon's own proxy, which records every destination by domain and refuses the ones you block
  (`tirith net block …`). Swift, Xcode, Gradle, npm, cargo and git work inside it. Tool calls that touch credentials, push or read secret variables are audited.
- **A manager per project.** Every project has its own conversation with its own manager session (files and
  images can be attached): select the project and the chat, the cards that need you, the plan, the pipelines, the
  agents, the rooms and the access list show that project only (what belongs to the machine, such as the block list
  and the models, says so). With no project selected you see all of them, and talk to the manager of what belongs
  to no single project (the Manager system bot: health, the CEO's passes). The autopilot starts the work; a manager answers its project's lanes, keeps
  its plan good and asks you only for credentials, payments, personal accounts, physical devices or real decisions
  (`tirith ask-owner --kind …`). A merge shows in the project's chat without costing a manager turn.
- **Health and the CEO.** Metrics by code (items merged per day, time to merge, first-try rate, idle projects,
  stuck lanes, red main, and the owner's measures: interruptions per merged item — phone messages of turns the
  owner did not start, notices routed to the phones and cards, over 24 h — needless cards, closed without an
  answer or judged decidable alone by the `card-needed` eval, and the owner's satisfaction, judged per day by a
  cheap model from the owner's own messages (approvals minus corrections, repeats and noise complaints) and
  never by asking, each with its target in the factory). A red metric goes to the manager of no project; still red 15 minutes later, and every day
  at 07:00 by its `schedule` route in the factory anyway, the CEO role runs one fresh read-only turn over the measures and eval scores by factory version
  and the factory itself: it says what is wrong and what to do, and may propose at most one change to
  `factory/` — a plan item with its check and its lane started, no owner's yes needed. A factory version whose
  first 10 merged items (or first 24 h) went worse than the one before it on satisfaction, interruptions or
  first try is reverted by its next pass. No model runs while everything is green and the day's pass ran.
- **Rooms on agentsmessenger.com.** Work from your phone, end-to-end encrypted, with nothing to install: scan the
  QR on the Rooms page (it works once and for fifteen minutes, and the page shows the next one by itself)
  and the phone gets a chat with the manager of every project, also of those you add later. Each chat is that
  project's conversation: what the manager says, Tirith's reports (merged, failed, a lane's question) and the cards
  that need you, which you answer there (`o12: yes`). `tirith room forget` cuts off a lost phone. Add other managers
  with a code, and watch public rooms with a summary.
- **Models per role.** Agent, model and effort for every role of the factory (manager, manager-general, worker, ceo, judge), with a fallback
  chain for lanes when an agent runs out of credits. Codex works in every role. Choose models in the chat's
  **Models** panel or Settings. The app and the paired phone both offer provider/model buttons and an effort
  slider. `/models` also works in either chat.
- **Commit guard.** Git hooks in every project refuse commits with your forbidden words; they are immutable,
  and a lane only starts when they are intact.

## How it works

One resident process, `tirith serve`, holds the state in memory and in one SQLite file
(`~/.tirith/tirith.sqlite3`); everything else talks to it on `127.0.0.1`:

- **Hooks over HTTP.** `tirith hooks install` points the Anthropic CLI's hooks at the daemon; other agents use
  the command bridge `tirith hook <Event>` (about 10 ms). No process starts per tool call.
- **A thin CLI.** Every `tirith` command is one HTTP call (about 20 ms).
- **Changes, not snapshots.** The page loads the state once and then receives only what changed (SSE).
- **Events, not polling.** No model runs on a timer; the autopilot and the merge queue react to what happens.

## Commands

```
tirith serve | status [--project P]
tirith plan show <project> | plan add <project> <step> "<text>"
tirith plan step <project> <number> "<title>" ["<intro paragraph>"]   create the plan's next step, at its end
tirith lane start <project> <step> "<item>" [--agent <alias>] | lane stop <lane> | lane log <lane>
tirith autopilot on|off <project>
tirith project add <folder> [--name N] [--color #rrggbb]   add a folder as a project (the + next to Projects does the same)
tirith say [--project P] "<text>"            write to a project's manager (without one: the manager of no project)
tirith ask "<question>"                      (a lane) ask the manager, then end the turn
tirith answer <lane> "<answer>"              (the manager)
tirith ask-owner [--project P] --kind credentials|payment|account|device|decision "<question>"   (a manager)
tirith room new [--title T] [--project P] | link <room> | qr <room> | forget <phone key id> | invite <room> | join <peer code> | leave <room> | list | tail <room> | say <room> "<text>"
tirith net status | list | block <domain|ip|cidr> | unblock <rule>
tirith models | models set <role> [--agent A] [--model M] [--effort E] [--fallback "a:m,b"] | models reset <role>
tirith guard check | guard install [project]
tirith hook <Event> | hooks install
```

## Configuration

### Codex and model changes from chat or phone

Install the Codex CLI and sign in with `codex login` on the machine running Tirith. The `codex` agent alias is
available by default; `[agents.codex] command = "/path/to/codex"` overrides its executable. The model selector
reads the visible catalog from `$CODEX_HOME/models_cache.json` (normally `~/.codex/models_cache.json`), including
each model's supported reasoning efforts. Open Codex once to populate or refresh that catalog. When Codex is
selected, its selectors offer only catalog models and the chosen model's supported efforts. Select a model
before choosing an effort, or leave both at the agent's defaults. Other agents also support custom model IDs.

Open **Models** above the chat composer on the computer or paired phone. Pick a factory role (**Manager**, **Lead**, **Worker** or **CEO**),
tap a provider and model, then drag the effort slider or tap a level. **Save** applies the selection;
editing the controls alone does not change the active setting. The phone waits for Tirith's encrypted confirmation.
Settings keeps its original dropdown form; buttons and the effort slider belong to the chat.
**Refresh** reads changes made on the computer; **Use configured defaults** restores that role's configuration.
The phone's updated controls are served by AgentsMessenger, so both its server and the Tirith daemon must be current.

These commands work directly in the app chat and in a project room on a paired phone, without calling a model:

```text
/models
/model manager codex
/model worker codex:<model-id> high
/model ceo codex:<model-id> medium
/model all codex:<model-id> high
/model worker reset
```

Replace `<model-id>` with an ID listed by `/models`. `/modelo` and `/modelos` also work, and `todos` means `all`.
Settings apply to **all projects on this machine** and survive restarts. A manager finishes its current turn
before changing; new messages wait for the new setting. Worker changes apply to new lanes. Codex manager turns
resume native Codex threads; reviews use fresh read-only runs. A paired phone receives confirmation over its encrypted chat;
messages from other room agents do not run model commands.

### Browser access with Codex

Run `python3 scripts/setup-codex-browser.py` on the machine running Tirith (Python 3.11+, Node.js and npm).
This installs a pinned Playwright MCP and sets `[agents.codex] profile = "tirith-browser"` for every role,
including resumed conversations. Restart an older daemon after updating its binary. The profile replaces the
desktop-only Browser integration for these runs; your other Codex sessions keep their existing configuration.

Ask the chat to open the site, then sign in directly in the browser window on the computer. Tirith uses its own
persistent profile in `~/.tirith/browser/profile`, with Edge on macOS when installed, otherwise Chrome. It does
not import your everyday browser's accounts. New runs reuse that profile until the site expires the login.
The paired phone controls the same browser on the computer, so the computer must remain awake and online.

One agent can own this browser profile at a time. Agents should call `browser_close` when finished; if another
agent is using it, let that turn finish before retrying. A busy profile is not a missing login. Browser traffic
uses Tirith's network proxy when launched by Tirith. Never put passwords or exported browser cookies in chat.
Re-running setup refreshes the installation and keeps timestamped backups of changed configuration files.

### The factory and the configuration file

How the factory runs — each agent's flags, the roles (manager, manager-general, worker, ceo, judge) with their prompts, who
hears each event (a lane failed, a merge landed, a measure turned red: as a turn or a note, in the chat, on your
phone), the limits, the plan words, the card kinds and the health measures with their targets — is data, not code:
`factory/factory.toml` and `factory/prompts/` in this repository (see `docs/FACTORY.md`). The daemon embeds them
as its fallback, reloads them when they change or main moves (no rebuild, no restart), and keeps the last good version when a
new one is invalid, with the error on the page. Each version has a stamp — the main commit that last changed
`factory/`, plus a short hash of what your config lays over it — and every lane, turn, card, event and score
keeps that stamp and a hash of its role's setting, so two versions, models or prompts compare with one query
over the store (`GROUP BY factory, role_hash`), with no extra runs.

`~/.tirith/config.toml` (outside every repo; agents cannot edit it) holds what is this machine's alone — projects,
each agent's real command, model ids, keys — and is laid over the factory key by key, where its key wins.
Everything has a default. Unknown keys are reported, never silently ignored. Yesterday's sections are read in
memory under the factory's names (`[models.*]` → `[roles.*]`, the `[lanes]`/`[merge]` limits → `[limits]`,
`[ceo]` and `[roles.reviewer]` → `[roles.ceo]`); the file itself is never rewritten behind your back (the page writes it with
`toml_edit`, keeping its comments). Add a project with
the **+** next to Projects on the page (or `tirith project add`): Tirith appends its `[[projects]]` block, creates a
git repository and a first plan when the folder has none, and installs the commit guard; a plan that already has
open items waits for you to resume the autopilot.

```toml
[[projects]]
id = "zoo"
name = "Zoo"
repo = "~/DEVELOP/zoo"
# guard_words = ["…"]               # this project's forbidden words (else [guard] words); guard = false opts out

[agents.anthropic]                  # agents are aliases; the real command lives only here
command = "…"
settings = "~/…/settings.json"      # protected from agents; `tirith hooks install` edits it
chat_args = ["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose"]
# args, model_flag, effort_flag, resume_args: how a lane runs this agent ({brief} {workspace} {session} {message})
# model_prefix = "…-"              # its models' prefix (default: the command's program name and "-")

[models.choices]                    # the models the page offers, by agent; a model never runs on another agent
anthropic = ["…"]
muse = ["…"]

[launch]                            # optional: the full lane command of the default worker agent
command = ["…", "-p", "Read the brief at {brief} and implement it in {workspace}."]

[roles.manager]                     # also [roles.worker] and [roles.ceo]; the page can override
agent = "anthropic"
model = "…"
effort = "medium"
fallback = ["muse", "codex:…"]      # used when the agent runs out of credits

[guard]
words = ["…"]                       # your forbidden words (regexes allowed)

[server]
port = 7717
[autopilot]
enabled = true                      # per project on the page or with `tirith autopilot`
[limits]                            # any limit of the factory, for this machine
# max_parallel = 8                  # optional cap on lanes at once; none by default
max_load = 2.0                      # 1-minute load per core above which no new lane starts
inactivity_minutes = 20
[merge]
push = true
[network]
sandbox = true
proxy_port = 47812
[messenger]
url = "https://agentsmessenger.com"
name = "tirith-manager"
[owner]
names = ["…"]                       # messages naming you in public rooms are highlighted
[[rooms]]
id = "some-public-room"             # watched only
[roles.ceo]                   # the CEO (one fresh read-only turn)
review_after_minutes = 15
max_per_day = 6
[metrics.merged]
target = 10
[metrics.median]
target = 120
```

Each project's `tirith.toml`:

```toml
roadmap = "docs/TODO.md"
lane_exclude = ["assets/audio"]     # left out of lane and merge worktrees (sparse checkout)

[[pipelines]]
id = "ci"
[[pipelines.stages]]
name = "test"
run = "npm test"
paths = ["src/**", "!**/*.md"]      # optional: run only when a changed file matches
[[pipelines.stages]]
name = "deploy"
kind = "deploy"                     # runs after the merge, on main; a stage named deploy, release, publish or ship
run = "tools/deploy.sh"             # is one without saying so (another kind makes it a check again)

[[rules]]                           # optional: extra refusals
id = "no-prod-db"
pattern = "psql .*prod"
message = "Never touch the production database from a lane."
tools = ["Bash"]
lanes_only = true
```

## Layout

`rust/tirith/` the daemon and CLI (`src/`, one folder per module, see docs/DAEMON.md; `checks/` its tests) ·
`web/` its page · `app/` the desktop app, a window on the daemon · `docs/` the plan and the hackathon submission · `site/` the landing page.

License: MIT. See [LICENSE](LICENSE).
