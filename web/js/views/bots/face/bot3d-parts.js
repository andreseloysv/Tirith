// The pieces of one 3D bot (bot3d.js): its camera, and its eyes as the owner's example draws them — dark capsules
// painted on the body's front (paint.js), where its shape and its state have them (bot3d-face.js).
import * as THREE from '../../../../vendor/three/three.module.min.js';
import { PX, MARGIN } from './config.js';
import { boxPt } from './geometry.js';
import { paintMesh } from './paint.js';

/** The camera on the box centre (50,50) of the 2D face, the middle of the element. */
export function camera3(sh) {
  const cam = new THREE.PerspectiveCamera(24, 1, 0.1, 50);
  const [bx, by] = boxPt(sh, 50, 50), half = (62 * PX / sh.rmax) * MARGIN; // the element shows the -12..112 box, grown by MARGIN
  const dist = half / Math.tan(THREE.MathUtils.degToRad(12));
  cam.position.set(bx, by, dist); cam.lookAt(bx, by, 0);
  return cam;
}

/** The two eyes: painted meshes that showFace (bot3d-face.js) lays on the body each time they change. */
export const eyes3 = () => [paintMesh(), paintMesh()];
