// A body's outline as an SVG path in the box's units (the 2D face's viewBox -12 -12 124 124), from the same r(theta)
// the 3D body is inflated from: the face without WebGL draws it, and the shape picker rings the chosen one with it,
// `grow` box units outside the body, as the owner's example does.
const PX = 512 / 150; /* pixels of the 512 frame per box unit, as geometry.js */
const STEP = 2;       /* every second of the 360 samples: smooth at any size the page draws */

/** The outline's points [x, y] (y down) of shape `sh` ({r, cx, cy, rmax}). */
export function outlinePoints(sh) {
  const n = sh.r.length, out = [];
  for (let i = 0; i < n; i += STEP) {
    const t = (i / n) * Math.PI * 2, k = sh.r[i] * sh.rmax;
    out.push([(sh.cx + k * Math.cos(t)) / PX - 25, (512 - (sh.cy + k * Math.sin(t))) / PX - 25]);
  }
  return out;
}

/** Each point moved `d` box units along its outward normal (the outline runs counter-clockwise on screen). */
function grown(pts, d) {
  return pts.map((p, i) => {
    const a = pts[(i + pts.length - 1) % pts.length], b = pts[(i + 1) % pts.length];
    const tx = b[0] - a[0], ty = b[1] - a[1], len = Math.hypot(tx, ty) || 1;
    return [p[0] - (ty / len) * d, p[1] + (tx / len) * d];
  });
}

/** The path `d` of shape `sh`'s outline, `grow` box units outside it. */
export function outlineD(sh, grow = 0) {
  const pts = grow ? grown(outlinePoints(sh), grow) : outlinePoints(sh);
  return `M${pts.map(([x, y]) => `${x.toFixed(2)} ${y.toFixed(2)}`).join('L')}Z`;
}
