// The hero face: the app icon's star (hero-mesh.js, from its own model) as a 3D body, with the frame the faces' code
// reads of a shape (shapes3d.js: its size in the mask, its eyes in the box's units, its outline) and its
// front, where the eyes sit (hero-front.js). Its body is its own mesh: no outline to inflate, no ball to puff into.
import * as THREE from '../../../../vendor/three/three.module.min.js';
import { HERO_MESH as M } from './hero-mesh.js';
import { surface } from './hero-front.js';
import { HERO_EYES } from './hero2d.js';

const bytes = (s) => Uint8Array.from(atob(s), (c) => c.charCodeAt(0)).buffer;
let geo = null, front = null;

/** Its body, made once and shared by every hero; the shader's ball is the body itself. */
export function heroGeometry() {
  if (geo) return geo;
  const q = new Int16Array(bytes(M.pos)), pos = new Float32Array(q.length), idx = new Uint16Array(bytes(M.idx));
  for (let i = 0; i < q.length; i++) pos[i] = (q[i] / 32767) * M.s;
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.BufferAttribute(pos, 3));
  g.setIndex(new THREE.BufferAttribute(idx, 1));
  g.computeVertexNormals();
  g.setAttribute('ballPosition', g.getAttribute('position'));
  g.setAttribute('ballNormal', g.getAttribute('normal'));
  front = surface(pos, idx);
  return (geo = g);
}

const face = () => front || (heroGeometry(), front);

/** The hero as the 3D faces read a shape, with its own body (`mesh`) and its front. */
export const HERO3D = {
  cx: 255.5, cy: 255.5, rmax: M.rmax, r: M.r,
  eyes: HERO_EYES, /* capsules like every face's, where the icon has its painted eyes */
  mesh: heroGeometry,
  front: (u, w) => face().front(u, w),
  normal: (u, w) => face().normal(u, w),
};
