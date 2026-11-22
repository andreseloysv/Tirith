// The pieces of one 3D bot (bot3d.js): its camera and halo, its eyes, and the mouths of each mood with the
// sleeping eyes — the reference's code and numbers.
import * as THREE from '../../../../vendor/three/three.module.min.js';
import { TAU } from './motion.js';
import { PX, MARGIN } from './config.js';
import { boxPt, frontPt, normalAt } from './geometry.js';
import { hex3, EYE_GEO3, DOT_GEO3, DOT_MAT3, HALO_GEO3, MOUTH3, TONGUE3, INK3, eyeMat3, haloVS, haloFS } from './materials.js';
import { decal3, tube3, cubic3, quad3 } from './decals.js';

/** The camera on the box centre (50,50) of the 2D face, the middle of the element. */
export function camera3(sh) {
  const cam = new THREE.PerspectiveCamera(24, 1, 0.1, 50);
  const [bx, by] = boxPt(sh, 50, 50), half = (62 * PX / sh.rmax) * MARGIN; // the element shows the -12..112 box, grown by MARGIN
  const dist = half / Math.tan(THREE.MathUtils.degToRad(12));
  cam.position.set(bx, by, dist); cam.lookAt(bx, by, 0);
  return cam;
}

export function halo3(color) {
  const halo = new THREE.Mesh(HALO_GEO3, new THREE.ShaderMaterial({ vertexShader: haloVS, fragmentShader: haloFS, transparent: true, depthWrite: false,
    uniforms: { color: { value: hex3(color).lerp(new THREE.Color(1, 1, 1), 0.45) }, amount: { value: 0.95 } } }));
  halo.scale.setScalar(4.6); halo.position.set(0, 0, -1.2);
  return halo;
}

/** Eye `i` set into the surface, `dx` and `dy` box units from where it rests (y down). */
export function seatEye(eye, sh, i, dx = 0, dy = 0) {
  const e = sh.eyes, [u, w] = boxPt(sh, e[i] + dx, e[2] + dy), rx = e[3] * PX / sh.rmax;
  const n = normalAt(sh, u, w), p = frontPt(sh, u, w);
  const z = n.clone(), xv = new THREE.Vector3(0, 1, 0).cross(z).normalize(), yv = z.clone().cross(xv);
  eye.quaternion.setFromRotationMatrix(new THREE.Matrix4().makeBasis(xv, yv, z));
  eye.position.copy(p.addScaledVector(n, -0.12 * rx));
}

/** Eyes: glossy dark lenses set into the surface, each with its dot of light. */
export function eyes3(sh) {
  const e = sh.eyes, out = [];
  for (let i = 0; i < 2; i++) {
    const rx = e[3] * PX / sh.rmax, ry = e[4] * PX / sh.rmax;
    const eye = new THREE.Mesh(EYE_GEO3, eyeMat3);
    seatEye(eye, sh, i);
    eye.userData.s = [rx * 0.9, ry * 0.9, 0.3 * rx];
    eye.scale.set(...eye.userData.s);
    const dot = new THREE.Mesh(DOT_GEO3, DOT_MAT3);
    dot.position.set(-0.3, 0.42, 0.75); dot.scale.set(0.26, 0.28, 0.25); eye.add(dot);
    out.push(eye);
  }
  return out;
}

/** Mouths for each mood, the sleeping eyes and a yawn's mouth: {grin, smile, sleepy, yawn, shut}. */
export function mouths3(sh) {
  const e = sh.eyes;
  const mx = (e[0] + e[1]) / 2, my = e[2] + e[4] + 6, hw = (e[1] - e[0]) * 0.5, k = (v) => boxPt(sh, v[0], v[1]);
  const grinPts = [...cubic3([mx - hw, my], [mx - hw, my + hw * 1.35], [mx + hw, my + hw * 1.35], [mx + hw, my], 24), ...quad3([mx + hw, my], [mx, my + hw * 0.16], [mx - hw, my], 12)].map(k);
  const tongue = Array.from({ length: 20 }, (_, i) => { const a = i / 20 * TAU; return k([mx + hw * 0.5 * Math.cos(a), my + hw * 0.78 + hw * 0.2 * Math.sin(a)]); });
  const grin = new THREE.Group(); grin.add(decal3(sh, grinPts, 0.004, MOUTH3), decal3(sh, tongue, 0.008, TONGUE3));
  const lineW = 1.6 * PX / sh.rmax;
  const smile = tube3(sh, [...quad3([mx - hw * 0.6, my - 1], [mx, my - 1 + hw * 1.3], [mx + hw * 0.6, my - 1], 16), [mx + hw * 0.6, my - 1]].map(k), lineW, INK3);
  const sleepy = new THREE.Group();
  for (const x of [e[0], e[1]]) sleepy.add(tube3(sh, quad3([x - 6, e[2]], [x, e[2] + 5], [x + 6, e[2]], 10).concat([[x + 6, e[2]]]).map(k), lineW * 1.1, INK3));
  sleepy.add(tube3(sh, quad3([mx - hw * 0.35, my], [mx, my + hw * 0.3], [mx + hw * 0.35, my], 10).concat([[mx + hw * 0.35, my]]).map(k), lineW * 0.9, INK3));
  const shut = [e[0], e[1]].map((x) => Object.assign(tube3(sh, quad3([x - e[3], e[2]], [x, e[2] + e[4] * 0.8], [x + e[3], e[2]], 10).concat([[x + e[3], e[2]]]).map(k), lineW * 1.35, INK3), { visible: false }));
  return { grin, smile, sleepy, shut, yawn: yawn3(sh, mx, my + hw * 0.5, hw) };
}

/** A yawn's round mouth with its tongue, made about its own middle so that it opens from there. */
function yawn3(sh, cx, cy, hw) {
  const oval = (rx, ry, down = 0) => Array.from({ length: 24 }, (_, i) => { const a = i / 24 * TAU; return boxPt(sh, cx + rx * Math.cos(a), cy + down + ry * Math.sin(a)); });
  const [u, w] = boxPt(sh, cx, cy), at = frontPt(sh, u, w);
  const yawn = new THREE.Group();
  yawn.add(decal3(sh, oval(hw * 0.6, hw * 0.78), 0.024, MOUTH3), decal3(sh, oval(hw * 0.34, hw * 0.22, hw * 0.4), 0.03, TONGUE3));
  yawn.children.forEach((mesh) => mesh.geometry.translate(-at.x, -at.y, -at.z));
  yawn.position.copy(at); yawn.visible = false;
  return yawn;
}
