// The 2D face's SVG (the face without WebGL): three layers — the glow, the body and the eyes with the mouth of
// each state.
import { SHAPES } from './shapes2d.js';

const rgb = (hex) => { const n = parseInt(String(hex).slice(1, 7), 16) || 0; return [n >> 16, (n >> 8) & 255, n & 255]; };
const mix = (hex, to, t) => '#' + rgb(hex).map((c) => Math.round(c + (to - c) * t).toString(16).padStart(2, '0')).join('');

let uid = 0;
export function markup(shape, color, state) {
  const [body, e, round] = SHAPES[shape] || SHAPES.round;
  const id = `bf${uid++}`, ink = '#1d1030';
  const core = mix(color, 255, 0.78), mid = mix(color, 255, 0.18), edge = mix(color, 0, 0.08), halo = mix(color, 255, 0.25);
  const soft = ` stroke="url(#${id}b)" stroke-width="${round ? 14 : 6}" stroke-linejoin="round"`; /* every shape a little puffy */
  const eyes = state === 'sleep'
    ? `<path d="M${e[0] - 6} ${e[2]}q6 5 12 0M${e[1] - 6} ${e[2]}q6 5 12 0" stroke="${ink}" stroke-width="3.2" stroke-linecap="round" fill="none"/>`
    : [e[0], e[1]].map((x) => `<g class="eye" data-x="${x}" data-y="${e[2]}"><ellipse cx="${x}" cy="${e[2]}" rx="${e[3]}" ry="${e[4]}" fill="${ink}"/><circle cx="${x + e[3] * 0.35}" cy="${e[2] - e[4] * 0.45}" r="${Math.max(1.4, e[3] * 0.28)}" fill="#fff"/></g>`
      + `<g class="shut" data-x="${x}" data-y="${e[2]}" opacity="0"><path d="M${x - e[3]} ${e[2]}q${e[3]} ${e[4] * 0.8} ${2 * e[3]} 0" stroke="${ink}" stroke-width="3.6" stroke-linecap="round" fill="none"/></g>`).join('');
  const mx = (e[0] + e[1]) / 2, my = e[2] + e[4] + 6, w = (e[1] - e[0]) * 0.5;
  const mouth = state === 'work' /* a wide D, with the tongue inside */
    ? `<path d="M${mx - w} ${my}C${mx - w} ${my + w * 1.35} ${mx + w} ${my + w * 1.35} ${mx + w} ${my}Q${mx} ${my + w * 0.16} ${mx - w} ${my}z" fill="${ink}"/><path d="M${mx - w * 0.55} ${my + w * 0.86}C${mx - w * 0.3} ${my + w * 0.45} ${mx + w * 0.3} ${my + w * 0.45} ${mx + w * 0.55} ${my + w * 0.86}C${mx + w * 0.3} ${my + w * 1.06} ${mx - w * 0.3} ${my + w * 1.06} ${mx - w * 0.55} ${my + w * 0.86}z" fill="#d2405d"/>`
    : state === 'sleep'
      ? `<path d="M${mx - w * 0.35} ${my}q${w * 0.35} ${w * 0.3} ${w * 0.7} 0" stroke="${ink}" stroke-width="2.6" stroke-linecap="round" fill="none"/>`
      : `<path d="M${mx - w * 0.6} ${my - 1}q${w * 0.6} ${w * 0.65} ${w * 1.2} 0" stroke="${ink}" stroke-width="3" stroke-linecap="round" fill="none"/>`;
  const oy = my + w * 0.5; /* a yawn's mouth, unseen until one comes */
  const yawn = `<g class="yawn" data-x="${mx}" data-y="${oy}" opacity="0"><ellipse cx="${mx}" cy="${oy}" rx="${w * 0.6}" ry="${w * 0.78}" fill="${ink}"/><ellipse cx="${mx}" cy="${oy + w * 0.4}" rx="${w * 0.34}" ry="${w * 0.22}" fill="#d2405d"/></g>`;
  /* three layers, each its own SVG with its own defs: the glow (both blurs of the halo), the body (gradient, its
     soft edge and the rim of light) and, filter-less on top, the eyes and mouth */
  const blur4 = (f) => `<filter id="${f}" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="4"/></filter>`;
  return {
    halo: `<defs><filter id="${id}w" x="-80%" y="-80%" width="260%" height="260%"><feGaussianBlur stdDeviation="14"/></filter>${blur4(`${id}n`)}</defs>`
      + `<g opacity=".85" fill="${halo}" stroke="${halo}" stroke-width="16" stroke-linejoin="round" filter="url(#${id}w)">${body}</g>`
      + `<g opacity=".9" fill="${halo}" stroke="${halo}" stroke-width="8" stroke-linejoin="round" filter="url(#${id}n)">${body}</g>`,
    body: `<defs><radialGradient id="${id}b" cx="46%" cy="42%" r="62%"><stop offset="0" stop-color="${core}"/><stop offset=".5" stop-color="${mid}"/><stop offset="1" stop-color="${edge}"/></radialGradient>`
      + `<filter id="${id}s" x="-10%" y="-10%" width="120%" height="120%"><feGaussianBlur stdDeviation="0.7"/></filter>${blur4(`${id}m`)}</defs>`
      + `<g fill="url(#${id}b)"${soft} filter="url(#${id}s)">${body}</g>`
      + `<g opacity=".35" fill="none" stroke="#fff" stroke-width="2" filter="url(#${id}m)">${body}</g>`, /* a faint inner rim of light */
    eyes: `<g class="look"><g class="eyes">${eyes}</g><g class="mouth" data-x="${mx}" data-y="${my}">${mouth}</g>${yawn}</g>`,
  };
}
