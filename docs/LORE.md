# Tirith — the citadel and its people

Tirith is a white stone citadel on a hill, built in rings. From its tower you can watch everything
that happens in the lands below: every project is a district, every coding agent is a hero at work,
every build or deployment is a caravan moving along a road of checkpoints. The look is stylized,
heroic and readable, in the spirit of hero games (chunky silhouettes, strong class colours,
expressive idle animations), with warm sunlight on white stone and silver.

All names, emblems and designs here are original to Tirith. Nothing is copied from any book, film or
game: the citadel's emblem is a **seven-pointed star above a watchtower**, never a tree.

## Classes (what each agent does)

| Class | Job in the system | Accent colour | Silhouette and gear |
|---|---|---|---|
| **The Manager** | The orchestrator: plans, launches lanes, reviews, merges, talks to you | Royal blue and gold | Tall robed figure with a high collar, a gold chain of office, a long staff topped by the star emblem, a seeing-glass on a strap. Stands on the tower balcony |
| **Stonewright** | Builds features (implementation lanes) | Warm orange | Broad, stocky builder in a leather apron over light plate, rolled sleeves, a hammer on the belt and a chisel in hand, stone dust on the boots, a small lantern at the hip |
| **Loremaster** | Research, docs, reading code, architecture maps | Deep violet | Slim scholar in layered robes, a satchel bursting with scrolls, ink-stained fingers, round spectacles, a floating quill, a book chained to the belt |
| **Warden** | Tests, verification, quality gates | Steel teal | Armoured sentinel with a tall shield bearing the star emblem, a spear, a winged helm with a T-shaped visor, a checklist scroll tucked in the bracer |
| **Swiftrider** | Performance, speed, size | Lightning yellow | Lean runner-scout in light leather, a hooded short cape, twin short blades, goggles pushed up, wind-swept scarf, boots with small wing motifs |
| **Herald** | Releases, signing, store submissions, deployments | Crimson and white | Messenger in a tabard, a long silver horn, a sealed letter case, a banner pole on the back with the star pennant, riding boots |
| **Glasswright** | Art, icons, shaders, UI | Prismatic (iridescent) | Artisan with a crystal monocle, gloves with glowing fingertips, a belt of coloured glass vials, a floating crystal shard that refracts rainbows |
| **Archivist** | Data, databases, migrations | Moss green | Hunched keeper with a lantern, a ring of many keys, a ledger under the arm, a long patched coat with drawers sewn into it |

## Ranks (they level up)

Agents earn **renown** (XP) for finished lanes, green gates and merged work; failures cost a little.

| Rank | Renown | Visual cue |
|---|---|---|
| Recruit | 0 | Plain cloth trim |
| Sentinel | 100 | Silver trim on the shoulders |
| Sergeant | 300 | Silver trim + one star on the chest |
| Captain | 700 | Short cape in the class colour + two stars |
| Marshal | 1500 | Long cape, gold trim, glowing star emblem, a faint aura |

## Names (never generic)

Every agent gets a hero name the first time it appears: a given name plus an epithet earned from its
first job, then its rank and class, for example **"Halvard Stonehand, Sergeant Stonewright"**,
**"Ivren of the Seventh Gate, Captain Loremaster"**, **"Marrow Quickstep, Recruit Swiftrider"**.
Names come from Tirith's own syllable tables (original, not from any book) and are kept stable per
agent; the Muse model may propose the epithet from what the agent actually did.

## The world (what you see)

- **The citadel** in the centre: the Manager's tower with the star at the top. A beacon on the
  tower lights up when something is **waiting for you**; the "Waiting for you" scroll hangs below.
- **Districts** on the rings of the hill, one per project, each with its own banner colour. Its
  **architecture** is built as small buildings (one per module) joined by roads; light particles run
  along the roads in the direction data flows.
- **Heroes** stand at workbenches in their district while working (animated), raise a fist when they
  finish, slump with a red glow when they fail, and walk to the tower to report.
- **Caravans**: each pipeline run is a cart travelling a road with stations (build, tests, lint,
  sign, device, store, review). Stations glow amber while running, green when passed, red when failed.
- Clicking anything opens a parchment-style 2D panel with the details (logs, files, report).

## 3D models wanted (for the owner)

One GLB per class, stylized heroic proportions (slightly big hands, head and gear), game-ready:
- 8k–25k triangles, PBR textures up to 2K, Y-up, metres, facing +Z, feet at origin, in A-pose or T-pose.
- Rigged humanoid (Mixamo-compatible bone names if possible) so shared animations can be retargeted:
  idle, work (hammering / writing / inspecting by class), celebrate, fail/slump, walk.
- Rank gear as separate meshes if possible (shoulder trim, chest stars, capes) so ranks can be toggled;
  otherwise Tirith tints and adds procedural capes.
- Drop them in `world/assets/characters/<class>.glb` (`steward`, `stonewright`, `loremaster`, `warden`,
  `swiftrider`, `herald`, `glasswright`, `archivist`). Until they exist, the world uses placeholders.
