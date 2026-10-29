# Tirith web UI

A static page: no server, no build, no network. Open `web/index.html` in a browser (double click, or
`open web/index.html`). Plain HTML, CSS and classic scripts; it works from `file://`.
The previous design is kept untouched in `web/v1/`.

## Layout

App shell: a left sidebar (wordmark; Views; Projects with colour dot and the number of running or working
items; recent Sessions; data status at the bottom) and a top bar (breadcrumb, search styled as a command
box; `/` or Cmd/Ctrl+K focuses it, Esc clears it). Clicking a project or session sets that filter, click
again to clear. The selected view lives in the hash (`#view=`).

- **Overview**: Your turn (asks with one-click answers), running pipelines as small graphs, working agents as cards, and the chat with the
  Manager on the right (`js/v-manager.js`). Only what is live or actionable. From any other view the header's
  **Chat** button opens the same chat as a drawer.
- **Pipelines**: runs grouped Running / Failed / Passed as cards (progress bar, current stage, or the error
  line); the selected run is a stage graph on a zoomable dotted canvas (gradient icon per stage type, ring
  colour = status, duration pill, one card per stage). Click a stage for its log in a side drawer. The run's
  total time is only in the floating bar.
- **Agents**: Kanban (Working, Stalled, Done, Failed; count in each header) or List (`#mode=list`). Card click
  opens summary, files changed and log in a drawer. Finished agents older than 24 h sit behind "Show N earlier".
- The project name is left off rows and cards when every visible item shares one project; it then appears once
  in the breadcrumb (or is the selected filter).
- The Overview no longer has an Activity column: flow and room messages are no longer shown as a view.

## Data

The page loads a `state.js` file (`window.TIRITH_STATE = {...};`) with a `<script>` tag and re-injects it
every 2 s with a cache-busting query. Choose the file in the URL hash:

    index.html#state=file:///Users/me/.tirith/state.js

Without a hash it loads `../fixtures/state.demo.js`, then `demo/state.demo.js` (a copy), labelled "Demo
data". Demo data older than 5 minutes is a snapshot: clocks run from its `generated_at`.
A live file never falls back to the demo: while it is missing the status reads "Waiting for the collector...
run ./run"; once loaded, the status is a dot plus "updated N s ago" (green, amber over 10 s, red over 60 s).
`demo/state.sessions.js` is the demo plus sessions, to try the filters.

Hash parameters (all optional, joined with `&`; encode `&` inside values): `state=`, `session=<id>`,
`project=<id>`, `q=<text>`, `view=overview|pipelines|agents|settings`, `mode=list`, `open=<run or agent id>`
(select it and open its drawer; `#` as `%23`).
Filters are written back to the hash, so a view can be bookmarked.

## Filters

Sidebar session and project, plus search (hero names, epithets, features, summaries, pipeline, run and stage names, flow
and room-message text, waits). They combine with AND and apply to every section. Items without a `session`
only show when no session is selected.

## Files

`index.html`, `css/style.css`, `js/util.js` (time formatting, icons, helpers), `js/ui.js` (shell, shared
components, Overview, merged messages), `js/v-pipelines.js`, `js/v-agents.js`, `js/main.js`
(loader, hash, filters, status). `screenshots/` holds headless captures (Edge, 1600x900 and 1920x1080).
