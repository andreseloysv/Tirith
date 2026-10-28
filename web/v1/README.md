# Tirith web UI

A static page: no server, no build, no network. Open `web/index.html` in a browser (double click, or
`open web/index.html`).

## Data

The page loads a `state.js` file (`window.TIRITH_STATE = {...};`) with a `<script>` tag and re-injects
it every 2 s with a cache-busting query. Choose the file in the URL hash:

    index.html#state=file:///Users/me/.tirith/state.js

Without a hash it loads `../fixtures/state.demo.js`, then `demo/state.demo.js` (a copy). Demo data
older than 5 minutes is treated as a snapshot: clocks run from its `generated_at` and the flows replay.
`demo/state.sessions.js` is the demo plus sessions and project folders, to try the filters.

Hash parameters (all optional, joined with `&`; encode `&` inside values): `state=`, `session=<id>`,
`project=<id>`, `q=<text>`, `tab=pipelines|agents|messages|waiting`. Filters are written back to the hash,
so a view can be bookmarked.

## Filters

Session and Project dropdowns plus a search box (hero names, epithets, features, pipeline and run names,
stage names, flow and room-message text, waits). They combine with AND and apply to the map, the counters
and every tab. Items without a `session` only show when no session is selected.

## Files

`index.html`, `css/style.css`, `js/util.js` (helpers), `js/panel.js` (tabs), `js/map.js` (SVG map and flows),
`js/main.js` (loader, filters, counters). `screenshots/` holds headless captures.
