// The 2D face's eyes and mouth, as if they sat on a ball: turned, each slides round it and narrows, is gone behind
// it and comes back on the other side. Only their groups move, in an SVG of their own with no filter in it.
const { sin, cos, asin, max, min, abs } = Math;
const CX = 50; /* the box's middle, which a turn goes round */
const ARC = 0.22, MOUTH = 0.36; /* as the 3D face's (bot3d-face.js): a shut eye is a curve, a yawn's mouth no dot */

/** The groups markup.js made, with where each rests; none of a kind when the state has none (asleep, no eyes). */
export function partsOf(root) {
  const read = (el) => el && { el, x: Number(el.dataset.x), y: Number(el.dataset.y), tr: '', shown: '' };
  const all = (sel) => [...root.querySelectorAll(sel)].map(read);
  const eyes = all('.eye');
  const spread = eyes.length ? max(...eyes.map((e) => abs(e.x - CX))) : 12;
  return { eyes, shut: all('.shut'), mouth: read(root.querySelector('.mouth')), yawn: read(root.querySelector('.yawn')), r: min(30, max(18, spread * 2.2)) };
}

function put(part, tr, shown) {
  if (tr !== part.tr) { part.tr = tr; part.el.setAttribute('transform', tr); }
  if (shown !== part.shown) { part.shown = shown; part.el.setAttribute('opacity', shown); }
}

/** One part turned by the pose, moved by dx, dy and scaled by w, h about its own middle; `show` false hides it. */
function turned(part, r, p, { dx = 0, dy = 0, w = 1, h = 1, show = true }) {
  const rest = asin(max(-0.9, min(0.9, (part.x - CX) / r)));
  const now = rest + p.yaw, narrow = cos(now) / cos(rest);
  const x = CX + r * sin(now) + dx, y = part.y + r * 0.7 * sin(p.pitch) + dy;
  const sx = max(0.001, narrow) * w, sy = max(0.001, cos(p.pitch) * h);
  const seen = show ? max(0, min(1, (narrow - 0.1) / 0.25)) : 0; /* it fades into the rim: never a hairline */
  put(part, `translate(${x.toFixed(2)} ${y.toFixed(2)}) scale(${sx.toFixed(3)} ${sy.toFixed(3)}) translate(${-part.x} ${-part.y})`, seen.toFixed(2));
}

/** The face at pose `p`, with `gaze` ([x, y]) and the opening `blink` leaves of its own. */
export function showParts(parts, p, gaze, blink) {
  const f = parts.r / 28.6; /* a small face: the eyes travel less */
  const [gx, gy] = gaze, dx = (p.ex + gx) * f, dy = (p.ey + gy) * f;
  parts.eyes.forEach((eye, i) => {
    const shut = p.lids[i] < ARC;
    turned(eye, parts.r, p, { dx, dy, w: p.big, h: p.big * max(0.08, min(blink, p.lids[i])), show: !shut });
    if (parts.shut[i]) turned(parts.shut[i], parts.r, p, { dx, dy, show: shut });
  });
  const yawning = p.open > MOUTH;
  if (parts.mouth) turned(parts.mouth, parts.r, p, { show: !yawning });
  if (parts.yawn) turned(parts.yawn, parts.r, p, { w: p.open, h: p.open, show: yawning });
}
