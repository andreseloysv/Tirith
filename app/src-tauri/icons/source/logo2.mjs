// The luminous logo: a starry sky, one big bot in front and many more bursting out from behind it with trails of light.
// node logo2.mjs <out.svg> [seed]
import { writeFileSync } from 'node:fs';
import { SHAPES, COLORS, face, mix, FILTERS } from './glowface.mjs';
let seed = Number(process.argv[3] || 9);
const rnd = () => ((seed = (seed * 16807) % 2147483647) - 1) / 2147483646;
const names = Object.keys(SHAPES).filter((n) => n !== 'blob');
const C = [430, 470]; // where the trails meet, behind the big bot
const place = (shape, color, x, y, size, rot, opts) => {
  const [g, b] = face(shape, color, opts);
  const t = `translate(${(x - size / 2).toFixed(1)} ${(y - size / 2).toFixed(1)}) scale(${(size / 100).toFixed(3)}) rotate(${rot.toFixed(1)} 50 50)`;
  return [`<g transform="${t}">${g}</g>`, `<g transform="${t}">${b}</g>`];
};
let sky = '', trails = '', glows = '', bodies = '';
for (let i = 0; i < 260; i++) {
  const r = rnd() < 0.9 ? 0.6 + rnd() * 1.4 : 2 + rnd() * 1.5;
  const c = ['#ffffff', '#ffd6f5', '#bfe6ff', '#fff3c4'][Math.floor(rnd() * 4)];
  sky += `<circle cx="${(rnd() * 1024).toFixed(0)}" cy="${(rnd() * 1024).toFixed(0)}" r="${r.toFixed(1)}" fill="${c}" opacity="${(0.35 + rnd() * 0.6).toFixed(2)}"/>`;
}
for (let i = 0; i < 9; i++) { // a few sparkles
  const x = rnd() * 1024, y = rnd() * 1024, s = 6 + rnd() * 9;
  sky += `<path d="M${x} ${y - s}Q${x} ${y} ${x + s} ${y}Q${x} ${y} ${x} ${y + s}Q${x} ${y} ${x - s} ${y}Q${x} ${y} ${x} ${y - s}z" fill="#fff" opacity=".8"/>`;
}
// flyers: big ones close to where they burst out, smaller and smaller further away; all inside the icon.
const flyers = [];
const ring = (n, from, to, r0, r1, s0, s1) => { for (let i = 0; i < n; i++) { const t = (i + 0.5) / n; flyers.push([from + (to - from) * t + (rnd() - 0.5) * 8, r0 + (r1 - r0) * rnd(), s0 + (s1 - s0) * rnd()]); } };
ring(7, -205, -12, 250, 320, 104, 140);
ring(9, -212, -8, 360, 430, 62, 84);
ring(12, -215, -5, 450, 560, 34, 50);
const palette = [...COLORS].sort(() => rnd() - 0.5);
let last = '';
flyers.forEach(([deg, dist, size], i) => {
  const a = (deg * Math.PI) / 180;
  const x = Math.min(990 - size / 2, Math.max(34 + size / 2, C[0] + Math.cos(a) * dist));
  const y = Math.min(990 - size / 2, Math.max(34 + size / 2, C[1] + Math.sin(a) * dist));
  if ((x - 640) ** 2 + (y - 690) ** 2 < (290 + size / 2) ** 2) return; // never behind the big one
  let shape = names[Math.floor(rnd() * names.length)];
  while (shape === last) shape = names[Math.floor(rnd() * names.length)];
  last = shape;
  const color = palette[i % palette.length];
  const nx = -Math.sin(a), ny = Math.cos(a), wdt = size * 0.18;
  trails += `<defs><linearGradient id="t${i}" gradientUnits="userSpaceOnUse" x1="${C[0]}" y1="${C[1]}" x2="${x.toFixed(0)}" y2="${y.toFixed(0)}"><stop offset="0" stop-color="${color}" stop-opacity="0"/><stop offset="1" stop-color="${mix(color, 255, 0.3)}" stop-opacity=".5"/></linearGradient></defs>`
    + `<polygon points="${C[0]},${C[1]} ${(x + nx * wdt).toFixed(1)},${(y + ny * wdt).toFixed(1)} ${(x - nx * wdt).toFixed(1)},${(y - ny * wdt).toFixed(1)}" fill="url(#t${i})" filter="url(#bloomNear)"/>`;
  for (let k = 0; k < 22; k++) {
    const t = rnd(), j = (rnd() - 0.5) * wdt * 1.6 * t;
    trails += `<circle cx="${(C[0] + (x - C[0]) * t + nx * j).toFixed(1)}" cy="${(C[1] + (y - C[1]) * t + ny * j).toFixed(1)}" r="${(0.6 + rnd() * 1.6).toFixed(1)}" fill="${mix(color, 255, 0.6)}" opacity="${(0.25 + 0.6 * t).toFixed(2)}"/>`;
  }
  const mood = size > 90 ? ['grin', 'smile'][i % 2] : 'smile';
  const [g, b] = place(shape, color, x, y, size, (rnd() - 0.5) * 24, { mood, look: [(512 - x) / 512 * 3, 0] });
  glows += g; bodies += b;
});
const [hg, hb] = place('blob', '#ffd23f', 640, 690, 560, -6, { mood: 'grin', look: [-2, 0], bloom: 1.2 });
const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" width="1024" height="1024">
<defs>${FILTERS}
  <clipPath id="icon"><rect width="1024" height="1024" rx="230"/></clipPath>
  <radialGradient id="sky" cx="${C[0] / 10.24}%" cy="${C[1] / 10.24}%" r="85%"><stop offset="0" stop-color="#6b2f8f"/><stop offset=".35" stop-color="#2b1a5e"/><stop offset=".75" stop-color="#0f1640"/><stop offset="1" stop-color="#070a22"/></radialGradient>
  <radialGradient id="heroLight" cx="62%" cy="66%" r="45%"><stop offset="0" stop-color="#ffd23f" stop-opacity=".35"/><stop offset="1" stop-color="#ffd23f" stop-opacity="0"/></radialGradient>
</defs>
<g clip-path="url(#icon)">
  <rect width="1024" height="1024" fill="url(#sky)"/>
  ${sky}
  <rect width="1024" height="1024" fill="url(#heroLight)"/>
  ${trails}
  ${glows}${bodies}
  ${hg}${hb}
</g>
</svg>`;
writeFileSync(process.argv[2], svg);
console.log('wrote', process.argv[2]);
