// The face without WebGL, drawn as the 3D one is: the body's outline in one flat colour, exactly the bot's, and two
// dark capsule eyes where its shape has them — each a dash when shut — and no mouth. Two layers: the body, and on
// top, alone in its own SVG, the eyes that the animator moves (features.js).
import { SHAPES } from './shapes2d.js';

const INK = '#161616';
const DEG = 180 / Math.PI;

/* a capsule `w` wide and `l` long about (x, y), turned `lean` radians counter-clockwise */
const capsule = (x, y, lean, w, l) => `<rect x="${(x - w / 2).toFixed(2)}" y="${(y - l / 2).toFixed(2)}" width="${w}" height="${l}" rx="${(w / 2).toFixed(2)}" fill="${INK}" transform="rotate(${(-lean * DEG).toFixed(1)} ${x} ${y})"/>`;

/* the dash an eye is when shut: half its width thick, along the line between the two eyes */
function dash([x, y, , w], along) {
  return capsule(x, y, along, w * 0.55, w * 1.43);
}

export function markup(shape, color, state) {
  const [body, eyes] = SHAPES[shape] || SHAPES.round;
  const [[x0, y0], [x1, y1]] = eyes, along = Math.atan2(y0 - y1, x1 - x0) - Math.PI / 2;
  const all = state === 'sleep'
    ? eyes.map((e) => dash(e, along)).join('')
    : eyes.map((e) => `<g class="eye" data-x="${e[0]}" data-y="${e[1]}">${capsule(...e)}</g>`
      + `<g class="shut" data-x="${e[0]}" data-y="${e[1]}" opacity="0">${dash(e, along)}</g>`).join('');
  return { body: `<path d="${body}" fill="${color}"/>`, eyes: `<g class="look"><g class="eyes">${all}</g></g>` };
}
