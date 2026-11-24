// What the face's eyes draw for each expression, as lines of ink: the 35 of the owner's sheet exactly as traced
// (ink-data.js: eyes, brows, tears, sparkles, "?" and grey blush), and the face's older looks (a glance, a shut
// eye) as the sheet would draw them: a capsule's outline, or one line when it is a dash. While a reaction holds,
// a glance does not change its drawing: it only says where to look (`look`), and the body turns that way.
// A drawing: { strokes: [{ key, side, kind, closed, pts }], blush: [{ key, side, cx, cy, rx, ry, a }], rings }.
// side 0 left eye, 1 right eye, 2 between them; kind 0 the eye itself (it blinks), 1 a mark; pts x,y in head
// units; rings: where the engine places each eye and keeps it inside the head.
import { INK_REF, INK_POINTS as N } from "./ink-data.js";
import { centroid } from "./geometry.js";

const GLANCE = 0.5; /* how far a held reaction looks towards a glance, in head units of gaze */
const DASH = 0.42;  /* a capsule this narrow for its length is a shut eye: one line */
const MID = 114.27; /* the head's middle: blush either side of it belongs to that eye */
const cache = new Map();

const pairs = (flat) => Array.from({ length: flat.length / 2 }, (_, i) => [flat[2 * i], flat[2 * i + 1]]);

/* keys: the same part of two drawings has the same key (side, kind, rank), so it moves instead of being redrawn */
function keyed(items, group) {
  const seen = {};
  return items.map((it) => {
    const g = group(it);
    seen[g] = (seen[g] ?? -1) + 1;
    return { ...it, key: `${g}.${seen[g]}` };
  });
}

function fromSheet(index) {
  const d = INK_REF[index];
  const strokes = keyed(d.s.map(([side, kind, closed, pts]) => ({ side, kind, closed: !!closed, pts: Float32Array.from(pts) })),
    (s) => `${s.side}.${s.kind}`);
  const blush = keyed(d.b.map(([cx, cy, rx, ry, a]) => ({ side: cx < MID - 10 ? 0 : cx > MID + 10 ? 1 : 2, cx, cy, rx, ry, a })),
    (b) => `b${b.side}`);
  return { strokes, blush, rings: d.r.map(pairs) };
}

/* ring points resampled to N by length, starting at the left and going clockwise, and ending where they start,
   as the traced loops do (so a loop can open into a line point by point) */
function around(ring) {
  const [cx, cy] = centroid(ring);
  let area = 0;
  for (let i = 0; i < ring.length; i += 1) {
    const [x0, y0] = ring[i], [x1, y1] = ring[(i + 1) % ring.length];
    area += x0 * y1 - x1 * y0;
  }
  const pts = area < 0 ? [...ring].reverse() : [...ring];
  let start = 0, best = -Infinity;
  pts.forEach(([x, y], i) => { const v = Math.cos(Math.atan2(y - cy, x - cx) - Math.PI); if (v > best) { best = v; start = i; } });
  const loop = [...pts.slice(start), ...pts.slice(0, start)];
  return along([...loop, loop[0]], N);
}

/* `n` points spread evenly along the polyline `pts` */
function along(pts, n) {
  const cum = [0];
  for (let i = 1; i < pts.length; i += 1) cum.push(cum[i - 1] + Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]));
  const out = new Float32Array(2 * n), total = cum[cum.length - 1] || 1;
  for (let k = 0, j = 1; k < n; k += 1) {
    const t = (total * k) / (n - 1);
    while (j < pts.length - 1 && cum[j] < t) j += 1;
    const span = cum[j] - cum[j - 1] || 1, u = Math.min(1, Math.max(0, (t - cum[j - 1]) / span));
    out[2 * k] = pts[j - 1][0] + (pts[j][0] - pts[j - 1][0]) * u;
    out[2 * k + 1] = pts[j - 1][1] + (pts[j][1] - pts[j - 1][1]) * u;
  }
  return out;
}

/* an older look's eye as a line: its outline, or, when it is a dash (a shut eye), the line along its middle */
function line(ring, side) {
  const [cx, cy] = centroid(ring);
  let xx = 0, xy = 0, yy = 0;
  for (const [x, y] of ring) { xx += (x - cx) ** 2; xy += (x - cx) * (y - cy); yy += (y - cy) ** 2; }
  const a = 0.5 * Math.atan2(2 * xy, xx - yy), ux = Math.cos(a), uy = Math.sin(a);
  let lo = Infinity, hi = -Infinity, wlo = Infinity, whi = -Infinity;
  for (const [x, y] of ring) {
    const t = (x - cx) * ux + (y - cy) * uy, w = (y - cy) * ux - (x - cx) * uy;
    lo = Math.min(lo, t); hi = Math.max(hi, t); wlo = Math.min(wlo, w); whi = Math.max(whi, w);
  }
  if (whi - wlo >= DASH * (hi - lo)) return { side, kind: 0, closed: true, pts: around(ring) };
  const p = [cx + ux * lo * 0.9, cy + uy * lo * 0.9], q = [cx + ux * hi * 0.9, cy + uy * hi * 0.9];
  return { side, kind: 0, closed: false, pts: along(p[0] <= q[0] ? [p, q] : [q, p], N) };
}

function older(rings) {
  return { strokes: keyed(rings.map((r, side) => line(r, side)), (s) => `${s.side}.${s.kind}`), blush: [], rings };
}

function remember(key, make) {
  if (!cache.has(key)) cache.set(key, make());
  return cache.get(key);
}

/** The face's everyday states shown with the sheet's nearest expression (their own looks are glances it follows):
    at work Serious, thinking Pensive (its "..."), searching Curious (its "??"), listening Calm, sending Pleased,
    celebrating Excited, proud Confident (its sparkle), shy Shame (its blush), playful Flirty, suspicious Incredulous. */
export const SHEET_FOR = Object.freeze({
  working: 32, writing: 32, thinking: 57, searching: 45, listening: 54, dictating: 54, sending: 29, receiving: 29,
  celebrate: 58, proud: 42, shy: 49, playful: 34, suspicious: 50,
});

/** Whether expression `index`, in a state whose first one is `primary`, shows a drawing traced from the sheet. */
export const traced = (index, primary) => Boolean(INK_REF[index] || INK_REF[primary]);

/** The drawing for expression `index` of `expressions` (the engine's), while the state's own first one is `primary`. */
export function drawingFor(index, primary, expressions) {
  if (INK_REF[index]) return remember(index, () => fromSheet(index));
  if (INK_REF[primary] && expressions[primary] && expressions[index]) {
    return remember(`${index}@${primary}`, () => {
      const d = [0, 1].map((i) => {
        const [ax, ay] = centroid(expressions[index][i]), [bx, by] = centroid(expressions[primary][i]);
        return [ax - bx, ay - by];
      });
      const look = [(GLANCE * (d[0][0] + d[1][0])) / 2, (GLANCE * (d[0][1] + d[1][1])) / 2];
      return { ...drawingFor(primary, primary, expressions), look };
    });
  }
  return remember(index, () => older(expressions[index] || expressions[0]));
}
