// What moves on a 3D bot's face: its painted eyes, from its shape's rest pose (shapes3d.js) through its state's pose
// (moods3d.js) and the act now playing — a glance, a blink, a wink, bigger eyes, a cheer's "/ \" — to shut, a dash
// along the pair (asleep, or a wink held long enough to see). There is no mouth.
import { lay } from './paint.js';
import { REST } from './pose.js';
import { easeLook } from './moods3d.js';
import { outAt } from './geometry.js';

const SHUT = 0.08; /* a blinked eye is this much of itself: a line */
const ARC = 0.22;  /* an act shuts an eye for long enough to be seen: from here it is drawn as its dash */
const GAZE = 0.9;  /* box units an eye travels per unit of a glance */
const DASH = 2.6;  /* a shut eye's length, in its own widths */
const JOY = [-0.22, 0.42]; /* how far a cheer turns each eye: the left leans forward, the right back, "/ \" */
const EDGE = 0.86; /* an eye's ends stay this far inside the outline: past it, it would fold round the rim */

/* eye `e` moved towards the middle until both its ends are well inside the body */
function keepIn(sh, e) {
  const half = (e.long * e.width) / 2, dx = -Math.sin(e.lean) * half, dy = -Math.cos(e.lean) * half;
  for (let k = 0; k < 8 && Math.max(outAt(sh, e.x + dx, e.y + dy), outAt(sh, e.x - dx, e.y - dy)) > EDGE; k++) {
    e.x += (50 - e.x) * 0.15; e.y += (50 - e.y) * 0.15;
  }
  return e;
}

/** The face at pose `p`; `blink`: the opening its own blink leaves the eyes; `dt`: seconds since the last frame. */
export function showFace(b, p = REST, blink = 1, dt = 1) {
  const L = easeLook(b.look, b.mood, dt), [[x0, y0], [x1, y1]] = b.sh.eyes;
  const along = Math.atan2(y0 - y1, x1 - x0) - Math.PI / 2;
  b.sh.eyes.forEach(([x, y, lean, w, l], i) => {
    const c = Math.max(L.shut, p.lids[i] < ARC ? 1 : 0), joy = p.joy || 0; /* how shut; how glad */
    const open = lean * (1 - L.upright) + JOY[i] * joy, wide = w * p.big, pull = L.pull + (0.22 - L.pull) * joy;
    lay(b.eyes[i], b.sh, keepIn(b.sh, {
      x: x + (50 - x) * pull + p.ex * GAZE, y: y + L.down + 6 * joy + p.ey * GAZE,
      lean: open + (along - open) * c, long: l / w + (DASH - l / w) * c, width: wide + (w * 0.55 - wide) * c,
      sy: c > 0.5 ? 1 : Math.max(SHUT, Math.min(blink, p.lids[i])),
    }));
  });
}
