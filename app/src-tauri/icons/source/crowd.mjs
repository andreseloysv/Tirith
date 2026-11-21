// A crowd of Tirith's bot faces as an app icon (1024x1024 SVG). node crowd.mjs <out.svg> [seed]
import { writeFileSync } from 'node:fs';

const petals = (n, r, pr, cy = 52) => Array.from({ length: n }, (_, i) => {
  const a = (i / n) * Math.PI * 2 - Math.PI / 2;
  return `<circle cx="${(50 + r * Math.cos(a)).toFixed(1)}" cy="${(cy + r * Math.sin(a)).toFixed(1)}" r="${pr}"/>`;
}).join('') + `<circle cx="50" cy="${cy}" r="${r + 4}"/>`;
const star = (n, ro, ri) => '<polygon points="' + Array.from({ length: n * 2 }, (_, i) => {
  const a = (i / (n * 2)) * Math.PI * 2 - Math.PI / 2, r = i % 2 ? ri : ro;
  return `${(50 + r * Math.cos(a)).toFixed(1)},${(51 + r * Math.sin(a)).toFixed(1)}`;
}).join(' ') + '"/>';
// The same shapes as web/js/views/bots/face.js: [body, eyes [lx, rx, y, rx, ry], round corners]
const SHAPES = {
  round: ['<circle cx="50" cy="50" r="42"/>', [36, 62, 46, 8, 13]],
  dome: ['<path d="M50 10c26 0 42 18 42 44v16c0 11-9 20-20 20H28C17 90 8 81 8 70V54C8 28 24 10 50 10z"/>', [37, 63, 56, 8, 13]],
  cloud: ['<circle cx="33" cy="44" r="23"/><circle cx="60" cy="36" r="24"/><circle cx="76" cy="58" r="20"/><circle cx="50" cy="66" r="24"/><circle cx="24" cy="64" r="17"/>', [40, 64, 50, 8, 12]],
  tile: ['<rect x="16" y="16" width="68" height="68" rx="20" transform="rotate(-12 50 50)"/>', [38, 62, 50, 8, 12]],
  star: ['<path d="M50 8Q62 38 92 50Q62 62 50 92Q38 62 8 50Q38 38 50 8z"/>', [41, 59, 50, 5, 9], true],
  flower: [petals(5, 26, 19), [39, 61, 52, 7, 11]],
  drop: ['<path d="M22 14c18-6 46-4 60 12 14 16 12 44-4 56-16 12-44 12-58-2C6 66 4 30 22 14z"/>', [37, 61, 50, 8, 12]],
  pill: ['<rect x="4" y="24" width="92" height="60" rx="30"/>', [37, 63, 54, 9, 14]],
  tri: ['<polygon points="50,14 90,82 10,82"/>', [41, 59, 62, 6, 10], true],
  hex: ['<polygon points="50,9 86,30 86,70 50,91 14,70 14,30"/>', [38, 62, 50, 8, 12], true],
  burst: [star(9, 44, 34), [39, 61, 51, 7, 11], true],
  blob: ['<path d="M50 8c22 0 42 16 42 40s-14 44-42 44S8 74 8 50 28 8 50 8z"/>', [36, 62, 48, 9, 14]],
};
const COLORS = ['#f2f2f2', '#e5484d', '#f07a3a', '#f2a93b', '#4fb86e', '#3fb6a8', '#3b82f6', '#8b5cf6', '#ec4899', '#8e7cf0'];
const rgb = (h) => { const n = parseInt(h.slice(1, 7), 16); return [n >> 16, (n >> 8) & 255, n & 255]; };
const mix = (h, to, t) => '#' + rgb(h).map((c) => Math.round(c + (to - c) * t).toString(16).padStart(2, '0')).join('');
const light = (h) => { const [r, g, b] = rgb(h); return 0.2126 * r + 0.7152 * g + 0.0722 * b > 190; };

let seed = Number(process.argv[3] || 7);
const rnd = () => ((seed = (seed * 16807) % 2147483647) - 1) / 2147483646;
const pick = (a) => a[Math.floor(rnd() * a.length)];

let uid = 0;
const HALO = Number(process.env.HALO || 0.9); // the app's faces: 0.5 idle, 0.55 on watch, 0.62 working
function face(shape, color, x, y, size, rot, look) {
  const [body, e, round] = SHAPES[shape];
  const id = `f${uid++}`, eye = light(color) ? '#16171c' : '#fff';
  const fill = round ? ` stroke="url(#${id})" stroke-width="12" stroke-linejoin="round"` : '';
  const [lx, ly] = look;
  const eyes = `<ellipse cx="${e[0] + lx}" cy="${e[2] + ly}" rx="${e[3]}" ry="${e[4]}" fill="${eye}"/><ellipse cx="${e[1] + lx}" cy="${e[2] + ly}" rx="${e[3]}" ry="${e[4]}" fill="${eye}"/>`;
  const s = size / 100;
  const at = `<g transform="translate(${(x - size / 2).toFixed(1)} ${(y - size / 2).toFixed(1)}) scale(${s.toFixed(3)}) rotate(${rot.toFixed(1)} 50 50)">`;
  const glow = mix(color, 255, 0.2);
  const halo = at + `<g fill="${glow}" opacity="${HALO}"${round ? ` stroke="${glow}" stroke-width="12" stroke-linejoin="round"` : ''} filter="url(#halo)">${body}</g></g>`;
  return [halo, at
    + `<defs><linearGradient id="${id}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${mix(color, 255, 0.28)}"/><stop offset="1" stop-color="${mix(color, 0, 0.12)}"/></linearGradient></defs>`
    + `<g fill="url(#${id})"${fill}>${body}</g>`
    + (shape === 'cloud' ? '' : '<ellipse cx="34" cy="28" rx="9" ry="4" fill="#fff" opacity=".22" transform="rotate(-30 34 28)"/>')
    + eyes + '</g>'];
}

// A crowd in depth: tiny far away at the top, big in front and cut by the icon's edge; rows staggered and overlapping.
const names = Object.keys(SHAPES);
const BG = '#14151b';
const R = Number(process.argv[4] || 8);
let bots = '';
let allHalos = '';
let last = '';
const recent = [];
let y = 70, size = 58;
for (let k = 0; k < R; k++) {
  const t = k / (R - 1);
  const n = Math.ceil(1024 / (size * 0.86)) + 1;
  const step = 1024 / (n - 1);
  const off = (k % 2) * step * 0.5 - step * 0.25;
  let halos = '', bodies = '';
  for (let i = 0; i < n; i++) {
    let shape = pick(names);
    while (shape === last) shape = pick(names);
    last = shape;
    const x = off + step * i + (rnd() - 0.5) * step * 0.3;
    const yy = y + (rnd() - 0.5) * size * 0.16;
    const look = [Math.max(-5, Math.min(5, (512 - x) / 512 * 5)), -3 + rnd() * 3];
    let base = pick(COLORS);
    while (recent.includes(base)) base = pick(COLORS);
    recent.push(base); if (recent.length > 3) recent.shift();
    const color = mix(base, 0x14, 0.5 * (1 - t) ** 1.4);
    const [h, b] = face(shape, color, x, yy, size * (0.9 + rnd() * 0.2), (rnd() - 0.5) * 18, look);
    halos += h; bodies += b;
  }
  allHalos += halos; bots += bodies; // every halo under every body: the glow shows in the dark gaps, as around the app's faces, and never over a face
  y += size * 0.62;
  size *= 1.36;
}

const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
<defs>
  <filter id="halo" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="9"/></filter>
  <clipPath id="icon"><rect x="0" y="0" width="1024" height="1024" rx="230"/></clipPath>
  <radialGradient id="bg" cx="50%" cy="30%" r="80%"><stop offset="0" stop-color="#2a2d38"/><stop offset="1" stop-color="#0d0e12"/></radialGradient>
</defs>
<g clip-path="url(#icon)">
  <rect width="1024" height="1024" fill="url(#bg)"/>
  ${allHalos}
  ${bots}
</g>
</svg>`;
writeFileSync(process.argv[2], svg);
console.log('wrote', process.argv[2], (svg.length / 1024).toFixed(0), 'KB');
