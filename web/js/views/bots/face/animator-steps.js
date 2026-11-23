// The steps of one animator frame (animator.js): what happens now, the pose of the sleeping act playing, the float
// with the act of acts.js on it, the eyes and the z's.
import { R, TAU, ease, bell, pick, ACTS } from './motion.js';
import { showParts } from './features.js';

const FEET = 40; /* the float turns about the feet, this far below the middle (bots.css) */

/** Events of this state, each at its own random moment: blinks, glances, the sleeping acts and z's. */
export function happen(a, now, g) {
  if (g.blink && now > a.next.blink) { a.blinks.push(now); if (Math.random() < 0.18) a.blinks.push(now + 0.24); a.next.blink = now + R(...g.blink); }
  if (g.look && now > a.next.look) {
    const e = a.eye, [rx, ry] = g.reach;
    const work = a.state === 'work';
    e.fx = e.x; e.fy = e.y; e.at = now; e.d = R(0.12, work ? 0.2 : 0.35);
    e.tx = work ? (e.tx > 0 ? -R(1, rx) : R(1, rx)) : Math.random() < 0.3 ? 0 : R(-rx, rx);
    e.ty = work ? R(-0.5, 1.5) : Math.random() < 0.4 ? 0 : R(-ry, ry * 0.6);
    a.next.look = now + R(...g.look);
  }
  if (g.acts && !a.act && now > a.next.act) {
    const act = ACTS[pick(g.acts)]();
    a.act = { ...act, at: now };
    a.next.act = now + act.d + R(...g.act);
  }
  if (g.z && now > a.next.z && a.zg && a.zs.length < 3) { a.spawnZ(now); a.next.z = now + R(...g.z); }
}

/** The sleeping (or waking) act now playing: what it adds to the pose. */
export function actPose(a, now) {
  let o = { x: 0, y: 0, rot: 0, sx: 1, sy: 1 };
  if (a.act) {
    const x = (now - a.act.at) / a.act.d;
    if (x >= 1) a.act = null;
    else if (a.act.f) o = { ...o, ...a.act.f(x) };
  }
  return o;
}

/** The float: slow waves that never line up, plus the acts. */
export function floatPose(a, m, c, o, p) {
  const [w1, w2, w3, w4, w5] = a.w, [p1, p2, p3, p4, p5] = a.p;
  const br = 1 + m.breath * Math.sin(c * w5 + p5);
  const sy = o.sy * p.sy * br;
  const sx = o.sx * p.sx * (2 - br) * (1 - 0.08 * Math.abs(Math.sin(p.yaw)));
  const up = FEET * sy; /* a roll goes round the middle: the feet make way for it */
  const y = m.bob * (0.62 * Math.sin(c * w1 + p1) + 0.38 * Math.sin(c * w2 + p2)) + o.y + p.y + up * (Math.cos(p.roll) - 1);
  const x = m.sway * Math.sin(c * w3 + p3) + o.x + p.x - up * Math.sin(p.roll);
  const rot = m.tilt * Math.sin(c * w4 + p4) + o.rot + (p.roll * 360) / TAU;
  return { x, y, rot, sx, sy };
}

/** The eyes: where they look (with a quick ease, like a glance; an act has them to itself), and their blinks. */
export function moveEyes(a, now, p) {
  if (a.state === 'sleep') return;
  const e = a.eye, k = ease(Math.min(1, (now - e.at) / e.d));
  e.x = e.fx + (e.tx - e.fx) * k; e.y = e.fy + (e.ty - e.fy) * k;
  e.free += ((a.acts.playing ? 0 : 1) - e.free) * 0.2;
  let lid = 1;
  a.blinks = a.blinks.filter((b) => now - b < 0.16);
  for (const b of a.blinks) if (now >= b) lid = Math.min(lid, 1 - 0.92 * bell((now - b) / 0.16));
  showParts(a.parts, p, [e.x * e.free, e.y * e.free], lid);
}

/** The sleeping z's rise and fade. */
export function moveZs(a, now) {
  a.zs = a.zs.filter((z) => {
    const x2 = (now - z.at) / z.d;
    if (x2 >= 1 || a.state !== 'sleep') { z.el.remove(); return false; }
    z.el.setAttribute('transform', `translate(${(z.x + 10 * x2 + 1.5 * Math.sin(x2 * TAU)).toFixed(2)} ${(z.y - 26 * x2).toFixed(2)})`);
    z.el.setAttribute('opacity', bell(x2).toFixed(3));
    return true;
  });
}
