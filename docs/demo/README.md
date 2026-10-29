# Demo video tooling

The 3-minute demo is recorded by script, against the live page, while real pipelines run. Narration script and
timings: [`script.md`](script.md). Everything below writes into `../Tirith-video/` (outside the repo).

## Reproduce

Prerequisites: the collector running (`./run`), Microsoft Edge, `uv`, `ffmpeg`, Node 20+, `playwright-core`
(install it in a scratch folder, not in the repo: `npm i playwright-core`, then `export PLAYWRIGHT_CORE=<that>/node_modules`),
a Meta Model API key in `META_SI_API_KEY` for the brain, and for the narration `TEXT_TO_SPEACH_GOOGLE_API_KEY`
(Google Cloud Text-to-Speech, voice `en-US-Studio-O`; without it the macOS `say` voice `TIRITH_VOICE` is used).

```bash
cd docs/demo
node narrate.mjs        # narration.json -> audio/*.wav and timings.json (clip lengths set the scene lengths)
node cards.mjs          # opening and closing title cards (cards.html) -> title.png, close.png
node record.mjs         # Edge 1920x1080 with recordVideo, scripted cursor, scenes timed to the narration
./assemble.sh           # ffmpeg: cards + recording + narration -> tirith-demo.mp4 (and a 720p copy)
```

All steps take an optional output folder (default `../../../Tirith-video/work`; the mp4 lands one level up).

## What the recorder does

- Opens `web/index.html#state=file://~/.tirith/state.js` (the live page; `TIRITH_HOME` is honoured) and drives it with
  eased cursor moves, clicks, typing and filters. Scenes: problem, one command, Overview, Pipelines, Agents, Flow, brain,
  filters, built-during-the-hackathon.
- `activity.sh` makes the activity real through the `tirith` CLI: starts the session "Hackathon demo", sets the Manager
  line, records flows, starts a lane (an agent process with a log), adds a request for the owner, runs the pipelines
  `tirith-checks` (collector tests, schema, web check, a live Muse Spark call) and `android-release` (the ChromaSol
  `make android-build`, `android-test`, `android-lint`, rebuilt with Gradle `--rerun-tasks`), then answers the request and
  ends the session. It never edits source, commits, or posts to a room, and the recorder puts back the previously
  selected session file.
- The brain scene reads the summaries Muse Spark wrote into `state.js` while the demo ran (the card is rendered from
  that file, not typed by hand).
- `scenes.json` records where each scene and narration clip starts so `assemble.sh` can place the audio exactly.

Cards: `cards.html#title|terminal|brain|built|close`.
