# Demo video tooling

The 2.5-minute demo is recorded by script from the real page on a demo story. Script and narration: [`script.md`](script.md).
Output (committed): `out/tirith-demo.mp4` and `out/tirith-demo.gif` (the README loop). Scratch files go to `out/work/` (ignored).

## Reproduce

Needs Microsoft Edge, `playwright-core` (`PLAYWRIGHT_CORE` = a `node_modules` folder that has it), `ffmpeg`, Node 20+ and,
for the voice, macOS `say` (or `TEXT_TO_SPEACH_GOOGLE_API_KEY` for Google Cloud Text-to-Speech). No collector, server,
port or network; nothing is written to `~/.tirith`.

```bash
cd docs/demo
node narrate.mjs     # narration.json -> out/work/audio/*.wav and timings.json (clip lengths set the scene lengths)
node record.mjs      # Edge 1920x1080 with recordVideo -> out/work/raw/*.webm and scenes.json
node assemble.mjs    # ffmpeg: video + narration -> out/tirith-demo.mp4 and out/tirith-demo.gif
node cards.mjs       # optional: the problem and closing cards as PNG, to look at them
```

## How it works

- `story.mjs` builds the state the page shows (from `fixtures/state.demo.json`): the owner's request, the manager's
  live steps, a plan that grows from three done steps to five, two agents with their own steps, a blocked raw test run,
  the review with "Claims verified", the health checks and one decision. `record.mjs` rewrites `state.js` (and the
  steps files) as the story goes on, and the page picks each change up like a live collector. `node story.mjs --static`
  writes the last beat to `web/demo/state.video.js`.
- `record.mjs` moves an eased cursor, clicks and types in the real UI, and adds a caption bar per scene.
- `cards.html` holds the problem, brain and closing cards (`#problem`, `#brain`, `#close`).
- `assemble.mjs` finds the two hard cuts in the recording (card to app, app to card) to map the recorder's clock onto the
  video's, places each narration clip, normalises loudness (-16 LUFS) and cuts a 10 s GIF of the rule scene (set
  `GIF_OFFSET` to start it elsewhere).
