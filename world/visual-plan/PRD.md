# Tirith world — visual plan

Reference: `refs/hero.jpg` (generated for this project, art direction only).

## B2 art pass target (hero-game look, checked against the reference)

The B1 screenshots read pale, flat and toy-like beside `refs/hero.jpg`: flat pastel
bands, empty terraces, tiny heroes, overlapping white labels, cone hills on a mint
plain. B2 keeps every behaviour and changes only the look:

- **Light**: golden hour. Sun elevation ~24 deg, warm `#ffb26b` at 2.1 energy, long
  soft shadows (blur 1.4, opacity 0.9); cool blue fill 0.18 from the north-west.
  Shader sky: deep blue zenith `#2e62b8`, warm horizon `#ffd9a3`, sun glow and an
  fbm cloud band tinted peach on the sun side. Fog: exponential depth fog plus
  height fog below y=14, colour `#cfd8e8` warm-shifted, so far hills fade blue.
  ACES, exposure 1.05, saturation 1.28, contrast 1.07. Glow threshold 0.85 so only
  the star, beacon, crystals and lanterns bloom. SSAO 1.6 intensity, SSIL on.
- **Materials**: procedural textures made in code (original, no downloads): white
  brick with mortar + weathering, cobbles, grass noise, rock strata, wood planks,
  roof tiles; matching normal maps from the same height data. Stone stays near
  `#e9dfc8` lit and cool in shadow; grass rich `#4d8a3d` with yellow-green patches.
- **Terrain**: terraces become a hill — sloped rock skirts (4 m batter), grass tops,
  stone plaza only near the tower, crenellated ring walls, scattered rocks,
  flower specks and grass tufts (instanced), layered blue hills and forest at the
  horizon instead of bare cones.
- **Heroes**: heroic scale, 2.5 m against 2.2–4.2 m buildings (hero head near the
  eaves), glowing class-colour ring underfoot, dark nameplate panel that always
  faces the camera and scales with distance: name, epithet, rank stars + class,
  status bar in the status colour. Working = sparks at the bench, done = sparkles,
  failed = red smoke wisp.
- **Districts**: cobbled plaza with a star medallion, rim lanterns, market stall,
  crates; buildings read by kind: gate, crystal tower, domed vault, columned
  library hall, forge with chimney. Road motes larger and brighter.
- **Caravans**: cart pulled by a pack animal with turning spoked wheels; stations
  as stone checkpoints (posts, arch, caged lantern, ground ring) in status colour.
- **Camera**: slow intro fly-in to the tower, default 3/4 overview (distance 64,
  pitch 33 deg, fov 44), district keys 1–9, hero/pipeline cycling, beacon key,
  demo tour on T. Final stills at 1920x1080.

Acceptance for B2, on the final stills, judged against `refs/hero.jpg`: warm/cool
contrast with no washed-out pastel; textured stone and grass with visible variation
at district distance; every hero identifiable by silhouette, ring and nameplate
from the overview; the caravan road readable as a ring with coloured checkpoints;
beacon and star visibly blooming; no label overlaps a hero it does not belong to.

## Target in numbers

- Hill: grass disc r=70; three white-stone ring terraces r=46/32/20, each 3 m high; tower on top.
- Tower: ~27 m above its terrace, white stone + silver spires, gold seven-pointed star (3 m) glowing on top.
- Palette: white stone #f2ede2, silver #b9c2cc, gold #e8b83a, grass #6fae5a, road #a89a84;
  district accents come from `state.json` project colours; class accents from `docs/LORE.md`.
- Light: warm sun (1.15 energy, warm white) from the south-east, soft shadows, light sky-blue ambient,
  gentle distance fog, ACES tonemapping, glow on emissives, SSAO on.
- Hero proportions: ~1.9 m tall, head ~22% of height, chunky gear readable at 40 m.
- Buildings: 2.5–5 m, white walls, project-colour roofs/banners, distinct silhouette per `kind`.
- Default camera: target tower mid-height, distance 78, pitch 38°, whole citadel in frame at 16:9.

## Acceptance criteria (checked on screenshots)

1. Overview reads at a glance: star tower centre, ring walls, coloured districts, caravan road outside.
2. District close-up: building kinds distinguishable, roads with flowing light, banner in project colour.
3. Hero close-up: class gear + accent colour readable, nameplate legible, status obvious from colour/pose.
4. Beacon visibly lit (fixture has 2 waiting items); steward bubble shows the current activity.
5. Caravan: cart at the running station, station colours green/amber/grey correct.
6. Parchment panel readable over the 3D view; HUD counts match the fixture (2 working, 1 done, 1 failed,
   1 stalled, 1 pipeline running, 2 waiting).
