// Flat shapes and tubes laid on the bot's face (the mouths, the sleeping eyes), and the curves they follow.
import * as THREE from '../../../../vendor/three/three.module.min.js';
import { frontPt, normalAt } from './geometry.js';

export function decal3(sh, pts2d, lift, mat) { // a flat shape laid on the face, a hair above the surface
  const c = pts2d.reduce((a, p) => [a[0] + p[0] / pts2d.length, a[1] + p[1] / pts2d.length], [0, 0]);
  const all = [c, ...pts2d], pos = [];
  const v = all.map(([u, w]) => frontPt(sh, u, w).add(normalAt(sh, u, w).multiplyScalar(lift)));
  for (let i = 1; i < all.length; i++) { const j = i === all.length - 1 ? 1 : i + 1; pos.push(...v[0].toArray(), ...v[i].toArray(), ...v[j].toArray()); }
  const g = new THREE.BufferGeometry(); g.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
  return new THREE.Mesh(g, mat);
}
export function tube3(sh, pts2d, radius, mat) {
  const curve = new THREE.CatmullRomCurve3(pts2d.map(([u, w]) => frontPt(sh, u, w)));
  return new THREE.Mesh(new THREE.TubeGeometry(curve, 24, radius, 8, false), mat);
}
export const cubic3 = (p0, p1, p2, p3, n) => Array.from({ length: n }, (_, i) => { const t = i / n, s = 1 - t; return [0, 1].map((k) => s * s * s * p0[k] + 3 * s * s * t * p1[k] + 3 * s * t * t * p2[k] + t * t * t * p3[k]); });
export const quad3 = (p0, p1, p2, n) => Array.from({ length: n }, (_, i) => { const t = i / n, s = 1 - t; return [0, 1].map((k) => s * s * p0[k] + 2 * s * t * p1[k] + t * t * p2[k]); });
