# Contract addition: projects are folders, and sessions (v1)

The owner wants to search by session and by project, and "a project is any folder I select".

## Projects = folders

- `tirith project add <folder> [--name NAME] [--color #hex] [--guard-words w1,w2]` registers any folder
  (absolute path stored; `.` works) in `~/.tirith/config.toml` as a `[[projects]]` entry. The id is a
  slug of the name (default: the folder name), unique. `tirith project list`, `tirith project remove <id>`.
- **Auto-discovery**: when a lane's `workspace`, a pipeline stage's working directory (`tirith run`
  records `cwd`) or a room's `project` points to a folder that is not inside any registered project,
  the collector derives a project from that folder (its git top-level if it is in a repo, else the
  folder itself), with `auto: true`, and persists it so it stays stable. The owner can rename or
  remove it with the CLI.
- Matching: an item belongs to the registered project whose folder is the **longest prefix** of the
  item's folder.
- `state.projects[]` gains `folder` (absolute path), `auto` (bool) and `counts`
  (`{agents_working, agents_total, pipelines_running, flows_24h}`).

## Sessions

- A session groups everything that happens during one orchestration session.
  `tirith session start "<title>" [--id ID]` creates it and makes it **current** (written to
  `~/.tirith/session`, printed as `export TIRITH_SESSION=<id>` too); `tirith session end [<id>]` closes it;
  `tirith session list`. The id defaults to a short time-based slug (`2026-10-28-tirith-web`).
- Every event written by the CLI (lane start/end, run stage events, flow, note, wait add/done) records
  `session` = env `TIRITH_SESSION`, else the current session file, else `null`.
- `state.sessions[]`: `{id, title, started_at, ended_at, projects: [ids], counts: {agents, pipelines,
  flows, notes}}`, newest first. Agents, pipeline runs, flows, notes and waits carry `session` (nullable)
  and `project`.

## Web filters

A filter bar under the header: **Session** dropdown (All, then sessions newest first with date and
title), **Project** dropdown (All, then projects with their colour; auto-discovered ones marked), and a
**search box** matching hero name/epithet, feature, pipeline name/run id, stage names, flow text, room
messages and waits. Filters apply to the map and every tab, combine with AND, show result counts, and
live in the URL hash so a view can be bookmarked (e.g. `#state=…&session=…&project=…&q=…`).
