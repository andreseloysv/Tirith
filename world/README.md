# Tirith — the 3D citadel (`world/`)

A Godot 4.7 (Forward+) scene that turns `state.json` into a living citadel:
projects are ring districts whose buildings are the code architecture, agents
are heroes at workbenches, pipelines are caravans on the outer road, and a
beacon on the Steward's tower burns while something is waiting for you.
The data contract is `../docs/CONTRACT.md`; classes and ranks are in
`../docs/LORE.md`. The world only reads files — it never writes outside
`user://`, opens no ports and needs no collector.

## Run it

With the demo fixture (no collector needed):

```sh
/Applications/Godot.app/Contents/MacOS/Godot --path world -- --state "$PWD/fixtures/state.example.json"
```

Or open `world/` in the Godot editor and press Play. Which state file is read,
in order:

1. `--state <file>` (user argument, after `--`)
2. the `TIRITH_STATE` environment variable
3. `~/.tirith/state.json` (written by the collector in `../collector/`)

The file's modification time is polled every second; changes animate in
(heroes walk to their benches, caravan carts roll to the next station,
station lanterns blend to their new colour). A missing or broken file leaves
the last good world standing and shows the problem in the HUD.

## Controls

- Default is an orthographic 2.5D board (35 deg down, 45 deg around). Drag or two-finger scroll: pan; pinch, Cmd+two-finger scroll, mouse wheel or +/-: zoom; Q/E: rotate in 90 degree steps; WASD/arrows: pan; double-click a hero, building or cart to centre on it
- `V`: toggle free 3D (left-drag orbit, right-drag pan) · `T`: demo tour · `H`: next hero · `B`: beacon
- Click a hero, building, district banner, cart, station, beacon or the
  notice board: parchment panel with summary, log tail, files, report or
  stage logs
- `1`–`9`: fly to that project district · `R`: reset camera · `Esc`: close panel

## What you are looking at

- **Tower**: the Steward on the balcony with a speech bubble (current
  activity), the seven-pointed star, the beacon and "Waiting for you" scroll
  (lit and pulsing while `waiting_on_you` is not empty), and the notice board
  with recent notes at the tower foot.
- **Districts** (one per project, banner in the project colour): architecture
  nodes as buildings by `kind` — input = gate, service = workshop, core =
  crystal tower, data = domed vault, ui = columned hall — joined by roads
  with light flowing along each edge's direction. No architecture yet means
  an empty fenced plot with a "Not mapped yet" sign.
- **Heroes**: one per agent at a workbench, with class silhouette/gear and
  rank cues (trim, chest stars, cape, aura). Working heroes hammer away,
  done heroes celebrate then rest, failed heroes slump under a red glow,
  stalled heroes grey out under a "…" bubble. Nameplates show name, epithet,
  rank, class, status and elapsed time.
- **Caravans**: one cart per pipeline run on the outer ring road; stations
  glow green (passed), amber (running, cart waits there), red (failed) or
  grey (pending/skipped).

## Character models

Drop a GLB per class into `assets/characters/` (`stonewright.glb`, …; full
list and spec in that folder's README) and it replaces the procedural
placeholder automatically — it is scaled to ~1.85 m, grounded, and if it has
an `AnimationPlayer`, clips named like `idle`, `work`, `celebrate`, `fail`,
`walk` (substring, case-insensitive) are used; a static mesh gets whole-body
motion instead. Three CC0 models are already in place; see
`assets/characters/CREDITS.md`.

## Tests and screenshots

Headless test (parses the fixture, builds the world, mutates a temp state
file and checks the world follows):

```sh
/Applications/Godot.app/Contents/MacOS/Godot --headless --path world \
  --script res://tests/test_world.gd -- --state "$PWD/fixtures/state.example.json"
```

Regenerate the demo stills in `screenshots/` (overview, district, hero panel,
caravan):

```sh
/Applications/Godot.app/Contents/MacOS/Godot --path world \
  -- --state "$PWD/fixtures/state.example.json" --capture "$PWD/world/screenshots"
```

## Code map

- `scripts/tirith_state.gd` — `state.json` parsing/normalising, shared
  vocabulary (classes, ranks, status colours). No scene code.
- `scripts/mats.gd` — shared materials and procedural mesh helpers (star
  emblem, banners, labels, click targets).
- `scripts/hero.gd` — one agent as a hero: procedural build per class/rank,
  GLB loading, status animation, nameplate, workbench.
- `scripts/world_builder.gd` — the citadel itself: static hill/tower/roads,
  diffed districts/heroes/caravans, beacon, steward, ambient animation.
- `scripts/ui.gd` — HUD counters and the parchment detail panel.
- `scripts/main.gd` — environment/light/post, orbit camera, polling,
  picking, capture mode.
- `visual-plan/` — the art-direction reference and the visual targets the
  screenshots are checked against.
