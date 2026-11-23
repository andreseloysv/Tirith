// The eyes as paint on the body's front: each a capsule drawn in the box's units (where the owner's example has
// them) and laid point by point on the surface, a hair above it, so that seen from the front it is exactly that
// capsule and, as the body turns, it bends with it; no part of it ever stands off the body or sinks into it.
import * as THREE from '../../../../vendor/three/three.module.min.js';
import { boxPt, frontPt, normalAt } from './geometry.js';

const N = 10;        /* points on each round end */
const LIFT = 0.004;  /* above the surface, so the body never shows through */
const INK = new THREE.MeshBasicMaterial({ color: '#161616', side: THREE.DoubleSide }); /* shows its hex as it is */

/** The rim of a capsule one unit wide and `long` long, turned `lean` radians counter-clockwise, in box terms (y down). */
export function rimOf(long, lean) {
  const r = 0.5, h = Math.max(0, long / 2 - r), c = Math.cos(lean), s = Math.sin(lean), pts = [];
  for (let i = 0; i <= N; i++) { const a = (Math.PI * i) / N; pts.push([r * Math.cos(a), h + r * Math.sin(a)]); }
  for (let i = 0; i <= N; i++) { const a = Math.PI + (Math.PI * i) / N; pts.push([r * Math.cos(a), -h + r * Math.sin(a)]); }
  return pts.map(([x, y]) => [x * c - y * s, -(x * s + y * c)]);
}

const RINGS = 4;     /* the rim and three smaller copies of it: small triangles, so none dips under a curved body */

/** A mesh for an eye: its middle, then a capsule's rim at 1/4, 2/4, 3/4 and all of its size; `lay` puts it on the body. */
export function paintMesh() {
  const g = new THREE.BufferGeometry(), n = 2 * (N + 1), at = (k, i) => 1 + (k - 1) * n + (i % n), idx = [];
  for (let i = 0; i < n; i++) idx.push(0, at(1, i), at(1, i + 1));
  for (let k = 1; k < RINGS; k++) for (let i = 0; i < n; i++) idx.push(at(k, i), at(k + 1, i), at(k + 1, i + 1), at(k, i), at(k + 1, i + 1), at(k, i + 1));
  g.setAttribute('position', new THREE.BufferAttribute(new Float32Array((1 + RINGS * n) * 3), 3));
  g.setIndex(idx);
  const m = new THREE.Mesh(g, INK);
  m.userData.key = '';
  return m;
}

/* a point of the box laid on the front, lifted along its normal */
function on(sh, x, y, arr, k) {
  const [u, w] = boxPt(sh, x, y), p = frontPt(sh, u, w).addScaledVector(normalAt(sh, u, w), LIFT);
  arr[k] = p.x; arr[k + 1] = p.y; arr[k + 2] = p.z;
}

/** Lay eye mesh `m` on shape `sh`: a capsule `long` times as long as it is `width` wide, leaning `lean`, its middle at
    (x, y) in the box, squashed to `sy` of its height (a blink). Laid again only when one of them changed. */
export function lay(m, sh, { x, y, lean, long, width, sy = 1 }) {
  const key = [x, y, lean, long, width, sy].map((v) => v.toFixed(3)).join(',');
  if (key === m.userData.key) return;
  m.userData.key = key;
  const pos = m.geometry.getAttribute('position'), arr = pos.array;
  const rim = rimOf(long, lean), n = rim.length;
  on(sh, x, y, arr, 0);
  for (let k = 1; k <= RINGS; k++) {
    const f = (k / RINGS) * width;
    rim.forEach(([px, py], i) => on(sh, x + px * f, y + py * f * sy, arr, (1 + (k - 1) * n + i) * 3));
  }
  pos.needsUpdate = true;
  m.geometry.computeBoundingSphere();
}
