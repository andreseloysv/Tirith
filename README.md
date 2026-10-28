# Tirith

**See your AI coding agents work, from a tower.**

When you run several coding agents in parallel, you end up asking "what is everyone doing?" all day
and keeping the whole codebase in your head. Tirith turns that into a living 3D citadel: every
project is a district whose buildings are its architecture, every agent is a hero with a name, a class
and a rank, working at its bench, and every build, test or deployment is a caravan moving through
checkpoints. A beacon lights up when something is waiting for you.

Meta's models do the real work: **Muse Spark** (Meta Model API) is Tirith's brain: it reads each
agent's log and says in one sentence what it is doing, spots when an agent needs you, summarizes the
agents' chat rooms, draws each project's architecture from its code, and names the heroes. Tirith
launches and watches coding agents such as **Muse Code** (`tirith lane run`), with the commit guard in place.

Built for the Meta Global AI Developer Hackathon (track: Build for Your World).
**Everything in this repository was created during the hackathon, starting 2026-10-28.**

## Run

```bash
./run          # collector in the background + Tirith in your browser (live data from ~/.tirith)
./run --demo   # the page on the demo state, no collector
./run --3d     # the optional Godot 3D citadel
```

The page is a local file (no server, no ports). Requirements: [uv](https://docs.astral.sh/uv/); Godot 4.7
only for `--3d`. Set `META_SI_API_KEY` (or `META_API_KEY`) to switch the Muse brain on.

## Layout

- `collector/` — the `tirith` CLI and collector (Python, uv): lanes, pipelines, notes, the brain.
- `web/` — the Tirith page (plain HTML/JS, opened from disk) that follows `~/.tirith/state.js`.
- `world/` — the optional Godot 4.7 3D citadel that reads `~/.tirith/state.json`.
- `fixtures/` — demo state used by the world and the tests.
- `docs/CONTRACT.md` — the data contract between both halves. `docs/LORE.md` — classes, ranks, names.

Status: under construction.
