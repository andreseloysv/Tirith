// The hero face's front (hero.js): its mesh seen from the front as a grid of heights and of normals, so its eyes and
// mouths sit on it as the other faces' sit on their cushions (geometry.js asks it for frontPt and normalAt).
import * as THREE from '../../../../vendor/three/three.module.min.js';

const G = 160, SPAN = 1.05, CELL = (2 * SPAN) / G; /* G×G cells over -SPAN..SPAN, a cell's middle at a whole number */
const grid = (v) => (v + SPAN) / CELL - 0.5;

/** One triangle's heights over the cells whose middles it covers, the highest kept: the front. */
function lay(z, pos, a, b, c) {
  const [ax, ay, bx, by, cx, cy] = [a, a + 1, b, b + 1, c, c + 1].map((i) => grid(pos[i]));
  const d = (by - cy) * (ax - cx) + (cx - bx) * (ay - cy);
  if (Math.abs(d) < 1e-12) return;
  const y0 = Math.max(0, Math.ceil(Math.min(ay, by, cy))), y1 = Math.min(G - 1, Math.floor(Math.max(ay, by, cy)));
  const x0 = Math.max(0, Math.ceil(Math.min(ax, bx, cx))), x1 = Math.min(G - 1, Math.floor(Math.max(ax, bx, cx)));
  for (let y = y0; y <= y1; y++) {
    for (let x = x0; x <= x1; x++) {
      const l0 = ((by - cy) * (x - cx) + (cx - bx) * (y - cy)) / d, l1 = ((cy - ay) * (x - cx) + (ax - cx) * (y - cy)) / d;
      if (l0 < 0 || l1 < 0 || l0 + l1 > 1) continue;
      const h = l0 * pos[a + 2] + l1 * pos[b + 2] + (1 - l0 - l1) * pos[c + 2];
      if (h > z[y * G + x]) z[y * G + x] = h;
    }
  }
}

/** The normal of each cell, from its neighbours' heights (none at the rim: it faces the camera there). */
function normals(z) {
  const n = new Float32Array(G * G * 3);
  const h = (x, y) => z[Math.min(G - 1, Math.max(0, y)) * G + Math.min(G - 1, Math.max(0, x))];
  for (let y = 0; y < G; y++) {
    for (let x = 0; x < G; x++) {
      const dx = (h(x + 1, y) - h(x - 1, y)) / (2 * CELL), dy = (h(x, y + 1) - h(x, y - 1)) / (2 * CELL);
      const v = Number.isFinite(dx) && Number.isFinite(dy) ? new THREE.Vector3(-dx, -dy, 1).normalize() : new THREE.Vector3(0, 0, 1);
      n.set([v.x, v.y, v.z], (y * G + x) * 3);
    }
  }
  return n;
}

/** The grid's value at (u, w), between its four nearest cells (those off the body left out). */
function sample(get, u, w) {
  const gx = Math.min(G - 1.001, Math.max(0, grid(u))), gy = Math.min(G - 1.001, Math.max(0, grid(w)));
  const x = Math.floor(gx), y = Math.floor(gy), fx = gx - x, fy = gy - y;
  let sum = 0, weight = 0;
  for (const [i, j, k] of [[x, y, (1 - fx) * (1 - fy)], [x + 1, y, fx * (1 - fy)], [x, y + 1, (1 - fx) * fy], [x + 1, y + 1, fx * fy]]) {
    const v = get(j * G + i);
    if (Number.isFinite(v) && k > 0) { sum += v * k; weight += k; }
  }
  return weight ? sum / weight : 0;
}

/** {front(u, w), normal(u, w)} of the mesh `pos` (x, y, z) with triangles `idx`, as geometry.js answers for a cushion. */
export function surface(pos, idx) {
  const z = new Float32Array(G * G).fill(-Infinity);
  for (let t = 0; t < idx.length; t += 3) lay(z, pos, idx[t] * 3, idx[t + 1] * 3, idx[t + 2] * 3);
  const n = normals(z);
  return {
    front: (u, w) => new THREE.Vector3(u, w, sample((k) => z[k], u, w)),
    normal: (u, w) => new THREE.Vector3(...[0, 1, 2].map((c) => sample((k) => n[k * 3 + c], u, w))).normalize(),
  };
}
