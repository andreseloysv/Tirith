// What moves on a 3D bot's face: each eye's lid and size and where it sits on the surface, and which mouth shows.
import { seatEye } from './bot3d-parts.js';
import { REST } from './pose.js';

const SHUT = 0.08; /* a blinked eye is this much of itself: a line */
const ARC = 0.22; /* an act shuts an eye for long enough to be seen: from here it is drawn as the sleeping eyes' curve */
const MOUTH = 0.36; /* narrower than this, a yawn's mouth would be a dot */

/** The face at pose `p`; `blink`: the opening its own blink leaves the eyes. */
export function showFace(b, p = REST, blink = 1) {
  const [was, wasDown] = b.gaze;
  if (p.ex !== was || p.ey !== wasDown) {
    const f = b.sh.eyes[3] / 7; /* small eyes (star, triangle) have less face to travel */
    b.eyes.forEach((eye, i) => seatEye(eye, b.sh, i, p.ex * f, p.ey * f));
    b.gaze = [p.ex, p.ey];
  }
  b.eyes.forEach((eye, i) => {
    const [sx, sy, sz] = eye.userData.s;
    eye.scale.set(sx * p.big, sy * p.big * Math.max(SHUT, Math.min(blink, p.lids[i])), sz);
    const awake = b.mood !== 'sleep', shut = awake && p.lids[i] < ARC;
    eye.visible = awake && !shut;
    b.shut[i].visible = shut;
  });
  const yawning = p.open > MOUTH;
  b.yawn.visible = yawning;
  if (yawning) b.yawn.scale.setScalar(p.open);
  b.grin.visible = !yawning && b.mood === 'work';
  b.smile.visible = !yawning && (b.mood === 'idle' || b.mood === 'watch');
}
