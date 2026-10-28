# Tirith: submission

**Project name:** Tirith

**Track:** Build for Your World

**One-line pitch:** See your coding agents work: one browser page that shows every agent, pipeline and message live, with a Muse Spark brain that says in one sentence what each agent is doing.

**Repository:** https://github.com/andreseloysv/Tirith (public)

**Demo video (3 min):** `<PASTE VIDEO LINK HERE>`

## Description

When you run several coding agents in parallel, you spend the day asking "what is everyone doing?" and keeping the whole codebase in your head. Tirith turns that into a live page.

Start it with `./run`. A small collector watches your agents, pipelines and messages and writes one state file; the page, a plain local HTML file with no server, follows it every two seconds.

- **Overview:** what needs you, what is running now, and the activity feed.
- **Pipelines:** runs grouped Running, Failed and Passed; the selected run is a node graph with per-step durations and logs.
- **Agents:** a board of heroes, each coding agent with a name, a rank and a live timer; click a card for its summary and log.
- **Flow:** a graph of how information passes: You, the Steward (orchestrator), the agents, pipeline reports and agentsmessenger.com rooms. Every edge counts its messages; click one to read them.
- **Filters:** by session and by project (a project is any folder), plus search over heroes, features, runs, steps and messages.

A commit guard is installed in every managed repository, and the Overview raises a request if it breaks. Agents, pipelines, notes and flows are reported through one small command, `tirith`.

The brain is Muse Spark on the Meta Model API. It writes the one-line summaries, finds what needs the owner, summarizes chat rooms and names the heroes. Without a key everything still works with plain log lines.

The demo video was recorded by a script in this repository (`docs/demo/`), against the live page, while real pipelines ran: Tirith's own checks and a real Android build, test and lint of a separate app.

## How Meta's models are used

- **Muse Spark** (`muse-spark-1.3`, Meta Model API, `POST /chat/completions`) is the product's brain, in `collector/tirith/brain.py`, with six jobs, each cached by input hash:
  1. a one-sentence summary of each agent's last ~200 log lines;
  2. a detector for things that need the owner (they appear under "Needs you");
  3. an architecture map of a project from its file tree and key files;
  4. each hero's lasting epithet, from its first summary;
  5. a one-line summary of each agentsmessenger.com room;
  6. a "needs the owner" check on room messages.
  The key is read only from the environment (`META_SI_API_KEY`) and is never logged or stored. One pipeline stage in the demo (`muse-spark-live`) makes a real Muse Spark call.
- **Muse Code**: Tirith launches coding agents with `tirith lane run` (commit guard checked, the owner's git wrapper in PATH) and shows them as heroes. Muse Code agents also built part of Tirith during the hackathon (the first collector, CLI, brain client and 3D world lanes); the rest was built with other coding agents after the Muse Code quota ran out.

## What was built during the hackathon

Everything in the repository. The first commit is dated 2026-10-28, the first day of building, and nothing existed before it: the `tirith` CLI and collector (lanes, pipelines, notes, waits, flows, rooms, projects as folders, sessions), the Muse Spark brain client, the commit guard, the browser interface (Overview, Pipelines, Agents, Flow, search and filters), the optional Godot 3D citadel, the `./run` command, the tests (70+) and the demo tooling in `docs/demo/`. No code or assets were reused from earlier projects.

## How to run

```bash
git clone https://github.com/andreseloysv/Tirith && cd Tirith
export META_SI_API_KEY=...   # optional: switches the Muse Spark brain on
./run                        # collector in the background + Tirith in your browser
./run --demo                 # the page on demo data, no collector
```

Requirements: [uv](https://docs.astral.sh/uv/) and a browser. Tests: `cd collector && uv run pytest -q`.
To report from your own agents: `tirith session start "My session"`, `tirith run --project p --pipeline build --stage test -- make test`, `tirith lane run ...` (see `collector/README.md`).
