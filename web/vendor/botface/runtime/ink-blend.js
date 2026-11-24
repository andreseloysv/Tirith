// The face between two drawings (ink-drawings.js), `a` of the way from the old one to the new: a part both have
// (the same key) slides point by point into its new line; a part only the old one had is rubbed out from its end
// in the first half of the change, and a part only the new one has is drawn in from its start in the second, the
// way a pen would. Each part carries `f`, how much of it is drawn, so a change that starts before the last one
// ended starts from what is on the face.
import { smoothstep } from "./math.js";

const out = (a) => 1 - smoothstep(Math.min(1, a / 0.55));                   /* an old part, while it goes */
const into = (a) => smoothstep(Math.min(1, Math.max(0, (a - 0.35) / 0.65))); /* a new part, while it comes */

function mix(p, q, t) {
  const r = new Float32Array(q.length);
  for (let i = 0; i < q.length; i += 1) r[i] = p[i] + (q[i] - p[i]) * t;
  return r;
}

const mixBlush = (b, c, t) => ({
  ...c, cx: b.cx + (c.cx - b.cx) * t, cy: b.cy + (c.cy - b.cy) * t, rx: b.rx + (c.rx - b.rx) * t,
  ry: b.ry + (c.ry - b.ry) * t, a: b.a + (c.a - b.a) * t, f: (b.f ?? 1) + (1 - (b.f ?? 1)) * t,
});

/* the parts of `from` and `to` by key, each as it is `a` of the way through */
function both(from, to, a, mixPart) {
  const old = new Map(from.map((p) => [p.key, p])), now = [];
  for (const p of to) {
    const q = old.get(p.key);
    if (q) { now.push(mixPart(q, p, a)); old.delete(p.key); }
    else now.push({ ...p, f: into(a) });
  }
  for (const q of old.values()) now.push({ ...q, f: (q.f ?? 1) * out(a) });
  return now;
}

/** The drawing `a` (0..1) of the way from `from` (a blend already, or null) to `to`. */
export function blend(from, to, a) {
  if (!from || a >= 1) return { strokes: to.strokes.map((s) => ({ ...s, f: 1 })), blush: to.blush.map((b) => ({ ...b, f: 1 })) };
  const strokes = both(from.strokes, to.strokes, a, (q, p, t) => ({ ...p, pts: mix(q.pts, p.pts, t), f: (q.f ?? 1) + (1 - (q.f ?? 1)) * t }))
    .filter((s) => s.f > 0.002);
  const blush = both(from.blush, to.blush, a, mixBlush).filter((b) => b.f > 0.002);
  return { strokes, blush };
}
