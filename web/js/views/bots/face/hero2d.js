// The hero face without WebGL (shapes2d.js): its outline seen from the front as a polygon in the box's units, its eyes
// and its own mouth at rest, the icon's small "o" (middle x, y and half width, height), where they are painted.
import { HERO_MESH as M } from './hero-mesh.js';

const PX = 512 / 150; /* pixels of the mask per box unit, as geometry.js */
const at = (r, th) => [((r * Math.cos(th) * M.rmax + 255.5) / PX - 25).toFixed(1), ((512 - (r * Math.sin(th) * M.rmax + 255.5)) / PX - 25).toFixed(1)];
const points = M.r.filter((_, i) => i % 3 === 0).map((r, i, all) => at(r, (i / all.length) * 2 * Math.PI).join(','));

export const HERO_BODY = `<polygon points="${points.join(' ')}"/>`;
export const HERO_EYES = M.eyes;
export const HERO_O = M.o;
