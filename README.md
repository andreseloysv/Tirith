<div align="center">

<img src="site/img/logo-3d.png" srcset="site/img/logo-3d.png 1x, site/img/logo-3d@2x.png 2x" alt="Tirith: a stone tower with a lit window" width="220">

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
  `tirith plan add`.
- **Autopilot.** Code starts the open items of a project's current step (the first step with open work; steps
  run in order, the items of a step in parallel), as far as the machine has room: `[lanes] max_parallel`, the load
  per core, the merge backlog, free disk and an intact commit guard. Failed or stopped items wait for the manager.
- **Lanes.** One agent, one item, one branch, in a reusable worktree whose build caches stay warm. The worktrees
  live in Tirith's own folder (`~/.tirith/lanes/<project>/slot-N`), not next to your repositories. A lane cannot end its turn with a question; it asks the manager with `tirith ask` and is resumed with
  the answer. Messages reach running lanes through their hooks ("main advanced: …").
- **Done means checked.** A lane that ends goes to the merge queue: merged with main (several lanes in one
  train), the project's checks and each item's check run, and only a green result moves main, with the plan item
  ticked in the same fast-forward. A red check that also fails on main marks main red and pauses the autopilot;
  otherwise the same agent session gets its work back with the log, twice at most.
- **Rules in code.** Every tool call of every agent passes the rules first, in 1–4 ms: commit checks are never
  skipped, the plan and the agents' settings are not edited by agents, a lane writes only in its own worktree,
  never pushes or touches main and never starts other agents. Projects add rules in `tirith.toml`.
- **Network sandbox.** Everything Tirith starts (lanes, the manager, CEO reviews, merge checks) runs in a macOS
  sandbox that allows the network only through the daemon's own proxy, which records every destination by
  domain and refuses the ones you block (`tirith net block …`). Swift, Xcode, Gradle, npm, cargo and git work
  inside it. Tool calls that touch credentials, push or read secret variables are audited.
- **A manager per project.** Every project has its own conversation with its own manager session (files and
  images can be attached): select the project and the chat, the cards that need you and every view show that
  project only. With no project selected you see all of them, and talk to the manager of what belongs to no single
  project (health, the CEO's reviews). The autopilot starts the work; a manager answers its project's lanes, keeps
  its plan good and asks you only for credentials, payments, personal accounts, physical devices or real decisions
  (`tirith ask-owner --kind …`). A merge shows in the project's chat without costing a manager turn.
- **Health and the CEO.** Metrics by code (items merged per day, time to merge, first-try rate, idle projects,
  stuck lanes, red main). A red metric goes to the manager; still red 15 minutes later, an independent one-shot
  review (the CEO) says what is wrong and what to do. No model runs while everything is green.
- **Rooms on agentsmessenger.com.** Talk to the manager from your phone in an end-to-end encrypted room
  (scan a QR, nothing to install), add other managers with a code, and watch public rooms with a summary.
- **Models per role.** Agent, model and effort for the manager, the lanes, the CEO and the brain, with a fallback
  chain when an agent runs out of credits.
- **Brain.** Small text jobs on the Meta Model API (room summaries, an architecture summary in every brief),
  cached, never on a hot path.
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
tirith lane start <project> <step> "<item>" [--agent <alias>] | lane stop <lane> | lane log <lane>
tirith autopilot on|off <project>
tirith project add <folder> [--name N] [--color #rrggbb]   add a folder as a project (the + next to Projects does the same)
tirith say [--project P] "<text>"            write to a project's manager (without one: the manager of no project)
tirith ask "<question>"                      (a lane) ask the manager, then end the turn
tirith answer <lane> "<answer>"              (the manager)
tirith ask-owner [--project P] --kind credentials|payment|account|device|decision "<question>"   (a manager)
tirith room new [--title T] | link <room> | qr <room> | invite <room> | join <peer code> | leave <room> | list | tail <room> | say <room> "<text>"
tirith net status | list | block <domain|ip|cidr> | unblock <rule>
tirith models | models set <role> [--agent A] [--model M] [--effort E] [--fallback "a:m,b"] | models reset <role>
tirith guard check | guard install [project]
tirith hook <Event> | hooks install
```

## Configuration

`~/.tirith/config.toml` (outside every repo; agents cannot edit it). Everything has a default. Add a project with
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

[launch]                            # optional: the full lane command of the default worker agent
command = ["…", "-p", "Read the brief at {brief} and implement it in {workspace}."]

[models.manager]                    # also [models.workers], [models.ceo], [models.brain]; the page can override
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
[lanes]
max_parallel = 3
max_load = 2.0                      # 1-minute load per core above which no new lane starts
inactivity_minutes = 20
[merge]
push = true
stage_timeout_minutes = 30
[plan]
commit_seconds = 120
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
[ceo]
review_after_minutes = 15
max_per_day = 6
[ceo.targets]
merged_per_day = 10
median_merge_minutes = 120
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

[[rules]]                           # optional: extra refusals
id = "no-prod-db"
pattern = "psql .*prod"
message = "Never touch the production database from a lane."
tools = ["Bash"]
lanes_only = true
```

## Layout

`rust/tirith/` the daemon and CLI (`src/`, one file per module) and its page (`ui/`) · `app/` the desktop app, a
window on the daemon · `docs/` the plan and the hackathon submission · `site/` the landing page.

License: MIT. See [LICENSE](LICENSE).
