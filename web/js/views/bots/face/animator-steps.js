// The steps of one animator frame (animator.js): what happens now, the pose of the act playing, the float, the
// eyes and the z's.
import { R, TAU, ease, bell, pick, ACTS } from './motion.js';

/** Events of this state, each at its own random moment: blinks, glances, acts and z's. */
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
  if (!a.act && now > a.next.act) {
    const name = pick(g.acts), act = ACTS[name]();
    a.act = { ...act, at: now };
    if (act.blinks) act.blinks.forEach((o) => a.blinks.push(now + o));
    a.next.act = now + act.d + R(...g.act);
  }
  if (g.z && now > a.next.z && a.zg && a.zs.length < 3) { a.spawnZ(now); a.next.z = now + R(...g.z); }
}

/** The act now playing: what it adds to the pose. */
export function actPose(a, now) {
  let o = { x: 0, y: 0, rot: 0, sx: 1, sy: 1, ex: 0, ey: 0 };
  if (a.act) {
    const x = (now - a.act.at) / a.act.d;
    if (x >= 1) a.act = null;
    else if (a.act.f) o = { ...o, ...a.act.f(x) };
  }
  return o;
}

/** The float: slow waves that never line up, plus the act; and the halo's pulse. */
export function floatPose(a, m, c, o) {
  const [w1, w2, w3, w4, w5] = a.w, [p1, p2, p3, p4, p5] = a.p;
  const y = m.bob * (0.62 * Math.sin(c * w1 + p1) + 0.38 * Math.sin(c * w2 + p2)) + o.y;
  const x = m.sway * Math.sin(c * w3 + p3) + o.x;
  const rot = m.tilt * Math.sin(c * w4 + p4) + o.rot;
  const br = 1 + m.breath * Math.sin(c * w5 + p5);
  const sx = o.sx * (2 - br), sy = o.sy * br;
  return { x, y, rot, sx, sy, halo: (m.halo + 0.06 * Math.sin(c * w2 + p5)).toFixed(3) };
}

/** The eyes: where they look (with a quick ease, like a glance) and their blinks. */
export function moveEyes(a, now, o) {
  const e = a.eye, k = ease(Math.min(1, (now - e.at) / e.d));
  e.x = e.fx + (e.tx - e.fx) * k; e.y = e.fy + (e.ty - e.fy) * k;
  if (a.look && a.state !== 'sleep') {
    const tr = `translate(${(e.x + o.ex).toFixed(2)} ${(e.y + o.ey).toFixed(2)})`;
    if (tr !== a.lookT) { a.lookT = tr; a.look.setAttribute('transform', tr); }
  }
  let lid = 1;
  a.blinks = a.blinks.filter((b) => now - b < 0.16);
  for (const b of a.blinks) if (now >= b) lid = Math.min(lid, 1 - 0.92 * bell((now - b) / 0.16));
  if (a.eyes && a.state !== 'sleep') {
    const tr = `translate(0 ${a.ey}) scale(1 ${lid.toFixed(3)}) translate(0 ${-a.ey})`;
    if (tr !== a.eyesT) { a.eyesT = tr; a.eyes.setAttribute('transform', tr); }
  }
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
