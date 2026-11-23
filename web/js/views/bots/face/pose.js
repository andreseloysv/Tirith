// What an act adds to a face at one moment, in channels both faces draw:
//   x, y      moved, in units of the 100-unit box (y down);      sx, sy  squash and stretch, about the feet
//   yaw       turned (radians, the front to the right);          pitch   nodded (the front down)
//   roll      rolled about its middle (clockwise);               puff    rounder front to back, while it turns
//   ex, ey    the eyes moved on the face (box units);            big     the eyes' size
//   lids      each eye's opening, [left, right];                 open    a yawn's mouth
//   joy       a cheer's happy eyes, "/ \" (0..1)
import { valueAt } from './keys.js';

const ZERO = { x: 0, y: 0, yaw: 0, pitch: 0, roll: 0, ex: 0, ey: 0, open: 0, joy: 0 };
const ONE = { sy: 1, puff: 1, big: 1, lid: 1, wink: 1 };
export const REST = Object.freeze({ ...ZERO, sx: 1, sy: 1, puff: 1, big: 1, lids: Object.freeze([1, 1]) });

/** The act at `x` (0..1). `flip`: mirrored left to right. `reach`: how much of its moving and stretching there is
    room for (an avatar in a list keeps inside its box). */
export function poseOf(act, x, { flip = false, reach = 1 } = {}) {
  const p = { ...ZERO, ...ONE };
  for (const k in act.keys) p[k] = valueAt(act.keys[k], x);
  const side = flip ? -1 : 1;
  const sy = 1 + (p.sy - 1) * reach;
  return {
    x: p.x * reach * side,
    y: p.y * reach,
    yaw: p.yaw * side,
    pitch: p.pitch,
    roll: p.roll * side,
    ex: p.ex * side,
    ey: p.ey,
    open: p.open,
    joy: p.joy,
    puff: p.puff,
    big: p.big,
    sy,
    sx: 1 + (1 - sy) * 0.7,
    lids: flip ? [p.lid * p.wink, p.lid] : [p.lid, p.lid * p.wink],
  };
}
