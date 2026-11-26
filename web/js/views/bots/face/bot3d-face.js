// What moves on a 3D bot's face: its painted eyes, from its shape's rest pose (shapes3d.js) through its state's pose
// (moods3d.js), the pointer it looks at and the act now playing — a glance, a blink, a wink, bigger eyes, a cheer's
// "/ \" — to shut, a dash along the pair (asleep, or a wink held long enough to see). The two eyes move as a pair:
// they keep their spacing. There is no mouth.
import { lay } from './paint.js';
import { REST } from './pose.js';
import { easeLook } from './moods3d.js';
import { outAt } from './geometry.js';

const SHUT = 0.08; /* a blinked eye is this much of itself: a line */
const ARC = 0.22;  /* an act shuts an eye for long enough to be seen: from here it is drawn as its dash */
const GAZE = 0.9;  /* box units the pair travels per unit of an act's glance */
const DASH = 2.6;  /* a shut eye's length, in its own widths */
const JOY = [-0.22, 0.42]; /* how far a cheer turns each eye: the left leans forward, the right back, "/ \" */
const LOOK = [0.7, 0.85, 12, 10]; /* eyes on the pointer: the pair drawn this far to the middle, this upright, then this many box units towards it (x, y) */
const EDGE = 0.86; /* an eye's ends stay this far inside the outline: past it, it would fold round the rim */

/* the ends of eye `e`, for keeping it inside the body */
const ends = (e) => { const h = (e.long * e.width) / 2, dx = -Math.sin(e.lean) * h, dy = -Math.cos(e.lean) * h; return [[e.x + dx, e.y + dy], [e.x - dx, e.y - dy]]; };

/* both eyes moved together towards the middle until all their ends are well inside the body (`edge`: how far out they may go) */
function keepIn(sh, pair, edge = EDGE) {
  for (let k = 0; k < 10 && Math.max(...pair.flatMap(ends).map(([x, y]) => outAt(sh, x, y))) > edge; k++) {
    const mx = (pair[0].x + pair[1].x) / 2, my = (pair[0].y + pair[1].y) / 2;
    for (const e of pair) { e.x += (50 - mx) * 0.15; e.y += (50 - my) * 0.15; }
  }
  return pair;
}

/** The face at pose `p`; `blink`: the opening its own blink leaves the eyes; `dt`: seconds since the last frame. */
export function showFace(b, p = REST, blink = 1, dt = 1) {
  const L = easeLook(b.look, b.mood, dt), [[x0, y0], [x1, y1]] = b.sh.eyes, h = b.hover || 0, joy = p.joy || 0;
  const along = Math.atan2(y0 - y1, x1 - x0) - Math.PI / 2, mx = (x0 + x1) / 2, my = (y0 + y1) / 2;
  const pull = L.pull + (LOOK[0] - L.pull) * h + (0.22 - L.pull) * joy, up = L.upright + (LOOK[1] - L.upright) * h;
  const dx = (50 - mx) * pull + p.ex * GAZE + b.gx * LOOK[2] * h;
  const dy = (50 - my) * LOOK[0] * h + L.down * (1 - h) + 6 * joy + p.ey * GAZE + b.gy * LOOK[3] * h; /* on the pointer: from the middle, every way */
  const pair = keepIn(b.sh, b.sh.eyes.map(([x, y, lean, w, l], i) => {
    const c = Math.max(L.shut, p.lids[i] < ARC ? 1 : 0), open = lean * (1 - up) + JOY[i] * joy, wide = w * p.big; /* how shut */
    return { x: x + dx, y: y + dy, lean: open + (along - open) * c, long: l / w + (DASH - l / w) * c, width: wide + (w * 0.55 - wide) * c,
      sy: c > 0.5 ? 1 : Math.max(SHUT, Math.min(blink, p.lids[i])) };
  }), EDGE - 0.08 * h); /* looking at the pointer the body turns: the eyes keep further from its rim */
  pair.forEach((e, i) => lay(b.eyes[i], b.sh, e));
}
