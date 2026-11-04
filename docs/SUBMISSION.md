# Tirith: submission

**Project name:** Tirith

**Track:** Build for Your World

**One-line pitch:** An autonomous manager for your coding agents: they finish the plan, rules are enforced in code, you see everything and are asked only what only you can decide.

**Repository:** https://github.com/andreseloysv/Tirith (public)

**Demo video (2:48):** https://youtu.be/LVqD1oW3KDw

## Description

When you run several coding agents, you spend the day asking "what is everyone doing?", re-explaining the rules, and answering questions the agents could settle themselves. Tirith takes that job.

- **The plan drives the agents.** An agent starts only from an open item of the project's plan, and must report an outcome. Tirith is the only writer of the plan, and ticks an item when the work is on `main`.
- **Rules are code.** A hook blocks the agent's tool call and tells it the exact command to use instead: tests and builds through Tirith, no helpers started behind its back, no skipping commit checks, no editing the plan or the agents' settings.
- **The owner gate.** Nothing reaches you unless only you can do it (credentials, payment, a personal account, a physical device, your decision). The rest goes to an on-call manager.
- **A claim check.** Before a merge, the agent's report is compared with the real change (files, tests added, tests passing). A false claim blocks the merge. It uses regexes and git, not a model.
- **Keep going until done.** A lane that ends with gaps (no report, uncommitted work, no passing tests, a branch that does not merge) is sent back to the same agent session with exactly what is missing; a round that fixes nothing goes to the manager at once.
- **Loop detection.** An agent that repeats the same tool call with the same input is reminded at 3 and stopped at 8.
- **The CEO.** A few times a day a read-only agent reviews how the manager runs the work, from numbers measured by code, never from agents' claims. It sends the manager directives and asks the owner only for decisions that are theirs, keeps its own findings and decisions so it never repeats itself, and changes nothing itself (a hook enforces it).
- **Autonomy.** An on-call manager answers in the chat and delegates; Tirith checks itself every pass and heals what it can; finished lanes go through one merge queue (tested together in trains, claim-checked, merged; no new lane starts with 3 or more waiting); a disk guard, no duplicate agents, a collector that reloads itself, and a timeout on every command.
- **Visibility.** A chat that shows the manager's work live like a CLI, Overview with "Your turn", Agents (with what the manager and the CEO are doing right now), Pipelines in GitLab style, the Plan, a lane scoreboard (`tirith lane stats`: first-try rate, rounds, merges, cost per merged lane), and a health dot. Proposals that cannot run now (machine load, a failing project check, a step the owner stopped) are held by code, and a manager's "wait" expires after 2 hours.
- **A desktop app** (Tauri), signed and notarized for macOS. Chat with photos and files, environment variables per project, model and effort per role with a fallback when an agent runs out of credits.

No servers and no ports: a local collector writes one state file; the page and the app follow it.

## How Meta's models are used

- **Muse Spark** (`muse-spark-1.3`, Meta Model API, `POST /chat/completions`) is the product's brain, in `collector/tirith/brain.py`, with seven jobs, each cached by input hash:
  1. a one-sentence summary of each agent's recent log;
  2. a detector for things that need the owner;
  3. an architecture map of a project from its file tree and key files;
  4. each agent's lasting epithet;
  5. a one-line summary of each chat room;
  6. a "needs the owner" check on room messages;
  7. a short answer to the owner's question about a plan step.
  The key is read only from the environment (`META_SI_API_KEY`) and is never logged or stored. Without a key everything still works with plain log lines.
- **Muse Code** is a first-class agent alias (`muse`). `tirith lane run --agent muse` builds its headless `exec --json` command, reads its real output format (captured from a real run, covered by a test), shows its steps live, and uses its final answer as the lane's report. Its hooks install with `tirith hook install --agent muse`, and every rule applies to it. It can be the fallback when another agent is out of credits. Muse Code agents also built part of Tirith (the first collector, CLI, brain client and 3D world); the rest was built with other coding agents after the Muse Code quota ran out.

## What's real, and what is not

- Real, on `main`, each with a contract in `docs/CONTRACT-*.md` and tests in `collector/tests/`: everything above.
- In progress, not claimed: agents with no permission prompts and hooks as the only barrier, faster tests, chat with Muse Code over `muse serve` (blocked on its protocol), and for the CEO: the watchdog, a dollar budget and the owner's ratings (`docs/CONTRACT-ceo.md`).
- The Rust port in `rust/` is a foundation, not the shipped collector.

## What was built during the hackathon

Everything in the repository. The first commit is dated 2026-10-28, the first day of building, and nothing existed before it. No code or assets were reused from earlier projects.

## How to run

```bash
git clone https://github.com/andreseloysv/Tirith && cd Tirith
./run --demo                 # the page on demo data: no agents, no collector
export META_SI_API_KEY=...   # optional: switches the Muse Spark brain on
./run                        # collector in the background + Tirith in your browser
```

Requirements: [uv](https://docs.astral.sh/uv/) and a browser. Tests: `cd collector && uv run pytest -q`.
New projects can start without a `tirith.toml`: `tirith plan add --title ...` creates the plan.
Screenshots of the product are in `web/screenshots/` and in the README.
