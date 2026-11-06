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

Needs Rust (`cargo`) and git. `./run` builds the `tirith` binary, starts the daemon if nothing answers on its
port and opens its page (`http://127.0.0.1:7717/`). The daemon keeps running; `tirith status` shows what it does.

## What it does

- **Plan-driven.** Each project's plan is a markdown checklist (`docs/TODO.md`: `## N. Title` steps,
  `- [ ] item` lines). An agent starts only from an open item; Tirith is the only writer of the plan, and
  agents ask for new items with `tirith plan add`.
- **Lanes.** One agent, one item, one git worktree on its own branch (`<repo>-lanes/<lane>`). Lanes start when
  the machine has room (`[lanes] max_parallel` and the load per core), and stop after a stretch without activity.
- **Rules in code.** Every tool call of every agent passes the rules before it runs. A refusal tells the agent
  what to do instead: commit checks are never skipped; the plan, Tirith's home and the agents' settings are not
  edited by agents; a lane writes only in its own copy, never pushes or touches main, and never starts other
  agents. Projects add their own rules in `tirith.toml`.
- **Outcome or no stop.** A lane cannot end its turn without `tirith report --status done|partial|blocked`.
- **Merge queue.** A `done` lane is merged with main in a throwaway worktree, the project's checks run there,
  and only a green result moves main (fast-forward), ticks the plan item and is pushed. A conflict or a red
  check goes back to the same agent with the log, twice at most.
- **Manager chat.** One long-lived manager session. The owner's messages go straight to it and its answers
  stream back to the page. It turns requests into plan items and lanes, and Tirith reports every lane outcome
  to it.
- **See everything.** The page shows the chat, the working lanes, the plan, the checks and every tool call of
  every agent, updated as it happens.

## How it works

One resident process, `tirith serve`, holds everything in memory and in one SQLite file
(`~/.tirith/tirith.sqlite3`). Everything else talks to it on `127.0.0.1`:

- **Agent hooks over HTTP.** `tirith hooks install` points the Anthropic CLI's hooks at the daemon
  (`type: "http"`, the lane in a header), so a tool call costs no new process: the rules answer in 1-4 ms.
  Agents without HTTP hooks use the command bridge `tirith hook <Event>` (about 10 ms; also accepts the old
  `tirith hook --agent A --event E` form).
- **A thin CLI.** Every `tirith` command is one HTTP call to the daemon (about 20 ms).
- **Changes, not snapshots.** The page loads the state once and then receives only what changed (SSE); each
  change repaints only its own element.
- **Plan commits in batches.** Who works on an item lives in the daemon; the plan file changes only when an item
  is added or ticked, and those changes are committed together every `[plan] commit_seconds` (120 s).

The daemon is small on purpose: `rust/tirith/src` must stay under 2000 lines (a check in `tirith.toml` and CI).

## Commands

```
tirith serve                                   run the daemon
tirith status                                  lanes at work and the latest outcomes
tirith plan show <project>                     open items and what works on each
tirith plan add <project> <step> "<text>"      add an item
tirith lane start <project> <step> "<item>" [--agent <alias>]
tirith lane stop <lane>  |  tirith lane log <lane>
tirith report --status done|partial|blocked --note "<text>"     (inside a lane)
tirith say "<text>"                            write to the manager
tirith hook <Event>                            command bridge for agent hooks
tirith hooks install                           switch the Anthropic CLI's hooks to HTTP
```

## Configuration

`~/.tirith/config.toml` (outside every repo, agents cannot edit it):

```toml
[[projects]]
id = "zoo"
name = "Zoo"
repo = "~/DEVELOP/zoo"

[agents.anthropic]                  # agents are aliases; the real command lives only here
command = "…"
settings = "~/…/settings.json"      # protected from agents; `tirith hooks install` edits it
chat_args = ["-p", "--input-format", "stream-json", "--output-format", "stream-json", "--verbose"]

[agents.muse]
command = "muse"
args = ["exec", "--prompt-file", "{brief}", "--workspace", "{workspace}"]   # how a lane runs this agent

[launch]                            # optional: the full lane command of the default worker agent
command = ["…", "-p", "Read the brief at {brief} and implement it in {workspace}."]

[models.manager]                    # the manager session
agent = "anthropic"
model = "…"
[models.workers]                    # default agent of a lane
agent = "anthropic"

[server]
port = 7717
[lanes]
max_parallel = 3
max_load = 2.0                      # 1-minute load per core above which no new lane starts
inactivity_minutes = 20
[merge]
push = true
stage_timeout_minutes = 30
[plan]
commit_seconds = 120
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

`rust/tirith/` the daemon and CLI (`src/`) and its page (`ui/index.html`) · `app/` the desktop app, a window
on the daemon · `docs/` the plan and the hackathon submission · `site/` the landing page.

License: MIT. See [LICENSE](LICENSE).
