// The 3D bot's body: inflated from its shape's outline r(theta) (shapes3d.js), on a sphere made by hand; and the
// points and normals of its front, where the eyes and mouths sit.
import * as THREE from '../../../../vendor/three/three.module.min.js';
import { SHAPES3D } from '../shapes3d.js';
import { TAU } from './motion.js';
import { P, DEPTH, PX } from './config.js';

// shape maths (the same as the Blender script)
export function radiusAt(sh, th) { const n = sh.r.length; const f = (((th % TAU) + TAU) % TAU) / TAU * n; const i = Math.floor(f) % n, t = f - Math.floor(f); return sh.r[i] * (1 - t) + sh.r[(i + 1) % n] * t; }
export function boxPt(sh, x, y) { return [((x + 25) * PX - sh.cx) / sh.rmax, ((512 - (y + 25) * PX) - sh.cy) / sh.rmax]; }
export function frontPt(sh, u, w) { const t = Math.min(Math.hypot(u, w) / radiusAt(sh, Math.atan2(w, u)), 0.995); return new THREE.Vector3(u, w, DEPTH * Math.pow(1 - Math.pow(t, P), 1 / P)); }
export function normalAt(sh, u, w) { const e = 1e-3, p = frontPt(sh, u, w); return frontPt(sh, u + e, w).sub(p).cross(frontPt(sh, u, w + e).sub(p)).normalize(); }
const geoCache3 = {};
/** An indexed sphere made by hand — the same points THREE.SphereGeometry(1, seg, round(seg * 0.75)) has after
 *  mergeVertices (seam and poles welded), so the body below needs no BufferGeometryUtils addon. */
function sphere3(seg) {
  const h = Math.round(seg * 0.75), pos = [], rows = [], idx = [];
  for (let iy = 0; iy <= h; iy++) {
    const theta = (iy / h) * Math.PI, st = Math.sin(theta), ct = Math.cos(theta), row = [];
    if (iy === 0 || iy === h) { const v = pos.length / 3; pos.push(0, ct, 0); for (let ix = 0; ix < seg; ix++) row.push(v); }
    else for (let ix = 0; ix < seg; ix++) { const phi = (ix / seg) * TAU; row.push(pos.length / 3); pos.push(-Math.cos(phi) * st, ct, Math.sin(phi) * st); }
    rows.push(row);
  }
  for (let iy = 0; iy < h; iy++) for (let ix = 0; ix < seg; ix++) {
    const a = rows[iy][ix], b = rows[iy + 1][ix], c = rows[iy + 1][(ix + 1) % seg], d = rows[iy][(ix + 1) % seg];
    if (iy !== 0) idx.push(a, b, d);
    if (iy !== h - 1) idx.push(b, c, d);
  }
  return { pos, idx };
}
export function bodyGeometry3(name, seg) {
  const key = name + seg; if (geoCache3[key]) return geoCache3[key];
  const sh = SHAPES3D[name] || SHAPES3D.round;
  const { pos, idx } = sphere3(seg), arr = new Float32Array(pos);
  for (let i = 0; i < arr.length; i += 3) {
    const a = arr[i], c = arr[i + 1], b = arr[i + 2];
    const th = Math.atan2(c, a), m = Math.min(1, Math.hypot(a, c)), rr = radiusAt(sh, th) * Math.pow(m, 2 / P);
    arr[i] = rr * Math.cos(th); arr[i + 1] = rr * Math.sin(th); arr[i + 2] = DEPTH * Math.sign(b) * Math.pow(Math.abs(b), 2 / P);
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.BufferAttribute(arr, 3));
  g.setIndex(idx);
  g.computeVertexNormals();
  return (geoCache3[key] = g);
}
