// The new bot style (luminous: the body emits light, a light core, a strong bloom of its own colour, dark eyes with a
// catchlight, a smile), as a function the logo and a sample sheet share. Shapes are Tirith's (web/js/views/bots/face.js).
const petals = (n, r, pr, cy = 52) => Array.from({ length: n }, (_, i) => {
  const a = (i / n) * Math.PI * 2 - Math.PI / 2;
  return `<circle cx="${(50 + r * Math.cos(a)).toFixed(1)}" cy="${(cy + r * Math.sin(a)).toFixed(1)}" r="${pr}"/>`;
}).join('') + `<circle cx="50" cy="${cy}" r="${r + 4}"/>`;
const star = (n, ro, ri) => '<polygon points="' + Array.from({ length: n * 2 }, (_, i) => {
  const a = (i / (n * 2)) * Math.PI * 2 - Math.PI / 2, r = i % 2 ? ri : ro;
  return `${(50 + r * Math.cos(a)).toFixed(1)},${(51 + r * Math.sin(a)).toFixed(1)}`;
}).join(' ') + '"/>';
export const SHAPES = {
  round: ['<circle cx="50" cy="50" r="42"/>', [36, 62, 44, 7, 10]],
  dome: ['<path d="M50 10c26 0 42 18 42 44v16c0 11-9 20-20 20H28C17 90 8 81 8 70V54C8 28 24 10 50 10z"/>', [37, 63, 52, 7, 10]],
  cloud: ['<circle cx="33" cy="44" r="23"/><circle cx="60" cy="36" r="24"/><circle cx="76" cy="58" r="20"/><circle cx="50" cy="66" r="24"/><circle cx="24" cy="64" r="17"/>', [40, 64, 48, 7, 9.5]],
  tile: ['<rect x="16" y="16" width="68" height="68" rx="20" transform="rotate(-12 50 50)"/>', [38, 62, 46, 7, 9.5]],
  star: ['<path d="M50 8Q62 38 92 50Q62 62 50 92Q38 62 8 50Q38 38 50 8z"/>', [42, 58, 46, 4.5, 7], true],
  flower: [petals(5, 26, 19), [39, 61, 48, 6, 9]],
  drop: ['<path d="M22 14c18-6 46-4 60 12 14 16 12 44-4 56-16 12-44 12-58-2C6 66 4 30 22 14z"/>', [37, 61, 46, 7, 9.5]],
  pill: ['<rect x="4" y="24" width="92" height="60" rx="30"/>', [37, 63, 49, 7.5, 10.5]],
  tri: ['<polygon points="50,14 90,82 10,82"/>', [42, 58, 58, 5, 7.5], true],
  hex: ['<polygon points="50,9 86,30 86,70 50,91 14,70 14,30"/>', [38, 62, 46, 7, 9.5], true],
  burst: [star(9, 44, 34), [39, 61, 47, 6, 9], true],
  blob: ['<path d="M50 8c22 0 42 16 42 40s-14 44-42 44S8 74 8 50 28 8 50 8z"/>', [36, 62, 45, 7.5, 10.5]],
};
export const COLORS = ['#ffd23f', '#ff5d73', '#ff8a3d', '#3ee08f', '#2fd3c4', '#4aa8ff', '#9b6bff', '#ff5fd2', '#f4f1ff'];
const rgb = (h) => { const n = parseInt(h.slice(1, 7), 16); return [n >> 16, (n >> 8) & 255, n & 255]; };
export const mix = (h, to, t) => '#' + rgb(h).map((c) => Math.round(c + (to - c) * t).toString(16).padStart(2, '0')).join('');

let uid = 0;
/** One luminous bot in a 100x100 box. mood: 'grin' (open smile), 'smile', 'sleep'; look: eye offset [x, y]. Returns [glow, body]. */
export function face(shape, color, { mood = 'smile', look = [0, 0], bloom = 1 } = {}) {
  const [body, e, round] = SHAPES[shape];
  const id = `g${uid++}`;
  const soft = ` stroke="url(#${id}b)" stroke-width="${round ? 14 : 6}" stroke-linejoin="round"`; // every shape a little puffy
  const ink = '#1d1030';
  const [lx, ly] = look;
  const eyes = mood === 'sleep'
    ? `<path d="M${e[0] - 6 + lx} ${e[2] + ly}q6 5 12 0M${e[1] - 6 + lx} ${e[2] + ly}q6 5 12 0" stroke="${ink}" stroke-width="3.2" stroke-linecap="round" fill="none"/>`
    : [e[0], e[1]].map((x) => `<ellipse cx="${x + lx}" cy="${e[2] + ly}" rx="${e[3]}" ry="${e[4]}" fill="${ink}"/><circle cx="${x + lx + e[3] * 0.35}" cy="${e[2] + ly - e[4] * 0.45}" r="${Math.max(1.4, e[3] * 0.28)}" fill="#fff"/>`).join('');
  const mx = (e[0] + e[1]) / 2 + lx, my = e[2] + e[4] + 6 + ly, w = (e[1] - e[0]) * 0.5;
  const mouth = mood === 'grin' // a wide D, with the tongue inside
    ? `<path d="M${mx - w} ${my}C${mx - w} ${my + w * 1.35} ${mx + w} ${my + w * 1.35} ${mx + w} ${my}Q${mx} ${my + w * 0.16} ${mx - w} ${my}z" fill="${ink}"/><path d="M${mx - w * 0.55} ${my + w * 0.86}C${mx - w * 0.3} ${my + w * 0.45} ${mx + w * 0.3} ${my + w * 0.45} ${mx + w * 0.55} ${my + w * 0.86}C${mx + w * 0.3} ${my + w * 1.06} ${mx - w * 0.3} ${my + w * 1.06} ${mx - w * 0.55} ${my + w * 0.86}z" fill="#d2405d"/>`
    : mood === 'sleep'
      ? `<path d="M${mx - w * 0.35} ${my}q${w * 0.35} ${w * 0.3} ${w * 0.7} 0" stroke="${ink}" stroke-width="2.6" stroke-linecap="round" fill="none"/>`
      : `<path d="M${mx - w * 0.6} ${my - 1}q${w * 0.6} ${w * 0.65} ${w * 1.2} 0" stroke="${ink}" stroke-width="3" stroke-linecap="round" fill="none"/>`;
  const core = mix(color, 255, 0.78), mid = mix(color, 255, 0.18), edge = mix(color, 0, 0.08), halo = mix(color, 255, 0.25);
  const glow = `<g opacity="${(0.85 * bloom).toFixed(2)}" fill="${halo}" stroke="${halo}" stroke-width="16" stroke-linejoin="round" filter="url(#bloomWide)">${body}</g>`
    + `<g opacity="${(0.9 * bloom).toFixed(2)}" fill="${halo}" stroke="${halo}" stroke-width="8" stroke-linejoin="round" filter="url(#bloomNear)">${body}</g>`;
  const fill = `<defs><radialGradient id="${id}b" cx="46%" cy="42%" r="62%"><stop offset="0" stop-color="${core}"/><stop offset=".5" stop-color="${mid}"/><stop offset="1" stop-color="${edge}"/></radialGradient></defs>`
    + `<g fill="url(#${id}b)"${soft} filter="url(#softEdge)">${body}</g>`
    + `<g opacity=".35" fill="none" stroke="#fff" stroke-width="2" filter="url(#bloomNear)">${body}</g>` // a faint inner rim of light
    + eyes + mouth;
  return [glow, fill];
}

/** The shared filters a page of these faces needs once. */
export const FILTERS = `<filter id="bloomWide" x="-80%" y="-80%" width="260%" height="260%"><feGaussianBlur stdDeviation="14"/></filter>
<filter id="bloomNear" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="4"/></filter>
<filter id="softEdge" x="-10%" y="-10%" width="120%" height="120%"><feGaussianBlur stdDeviation="0.7"/></filter>`;
