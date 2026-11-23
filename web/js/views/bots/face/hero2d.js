// The hero face's outline and eyes in the box's units, for every way it is drawn (hero.js in 3D, shapes2d.js without
// WebGL, the picker's ring): its outline as its model is seen from the front, and two capsule eyes like every other
// face's (shapes3d.js), where the icon has its own painted ones, the right one a little higher.
import { HERO_MESH as M } from './hero-mesh.js';

/** Its outline as outline.js reads a shape. */
export const HERO_OUTLINE = { r: M.r, cx: 255.5, cy: 255.5, rmax: M.rmax };

const LEAN = 0.49, WIDTH = 8, LENGTH = 19, RISE = 2; /* the reference's eye, as most of its shapes have it */
/** [[x, y, lean, width, length] of the left eye, … of the right]. */
export const HERO_EYES = [[M.eyes[0], M.eyes[2] + RISE, LEAN, WIDTH, LENGTH], [M.eyes[1], M.eyes[2] - RISE, LEAN, WIDTH, LENGTH]];
