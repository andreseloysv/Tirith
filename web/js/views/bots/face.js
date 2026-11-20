// The bots' faces: an SVG character in one of SHAPES and one of COLORS (the look of the "Agents, die aufpassen" deck:
// a soft gradient body with a glow, white eyes that blink, a slow float). `state` is what the bot is doing:
// 'work' (a turn runs: the eyes scan), 'watch' (a routine is on: it looks around), 'sleep' (nothing to do: eyes shut, z z),
// 'idle' (just there). Shapes live in a 100x100 box; `round` shapes get a thick stroke of their own fill, which rounds their corners.
import { html } from '../../../vendor/preact-htm.js';

const petals = (n, r, pr, cy = 52) => Array.from({ length: n }, (_, i) => {
  const a = (i / n) * Math.PI * 2 - Math.PI / 2;
  return `<circle cx="${(50 + r * Math.cos(a)).toFixed(1)}" cy="${(cy + r * Math.sin(a)).toFixed(1)}" r="${pr}"/>`;
}).join('') + `<circle cx="50" cy="${cy}" r="${r + 4}"/>`;
const star = (n, ro, ri) => '<polygon points="' + Array.from({ length: n * 2 }, (_, i) => {
  const a = (i / (n * 2)) * Math.PI * 2 - Math.PI / 2, r = i % 2 ? ri : ro;
  return `${(50 + r * Math.cos(a)).toFixed(1)},${(51 + r * Math.sin(a)).toFixed(1)}`;
}).join(' ') + '"/>';

/** name: [svg body, eyes [left x, right x, y, rx, ry], round corners] */
export const SHAPES = {
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
export const COLORS = ['#f2f2f2', '#9a6b45', '#e5484d', '#f07a3a', '#f2a93b', '#4fb86e', '#3fb6a8', '#3b82f6', '#8b5cf6', '#ec4899', '#8c8c8c'];

const rgb = (hex) => { const n = parseInt(String(hex).slice(1, 7), 16) || 0; return [n >> 16, (n >> 8) & 255, n & 255]; };
const mix = (hex, to, t) => '#' + rgb(hex).map((c) => Math.round(c + (to - c) * t).toString(16).padStart(2, '0')).join('');
const light = (hex) => { const [r, g, b] = rgb(hex); return 0.2126 * r + 0.7152 * g + 0.0722 * b > 190; };

let uid = 0;
/** One bot's face. */
export function Face({ shape = 'round', color = '#3b82f6', size = 64, state = 'idle', delay = 0 }) {
  const [body, e, round] = SHAPES[shape] || SHAPES.round;
  const id = `bf${uid++}`, eye = light(color) ? '#1b1b1b' : '#fff';
  const fill = round ? ` stroke="url(#${id}g)" stroke-width="12" stroke-linejoin="round"` : '';
  const eyes = state === 'sleep'
    ? `<g class="look" stroke="${eye}" stroke-width="3" stroke-linecap="round" fill="none"><path d="M${e[0] - 6} ${e[2]}q6 5 12 0M${e[1] - 6} ${e[2]}q6 5 12 0"/></g>`
    : `<g class="eyes"><g class="look"><ellipse cx="${e[0]}" cy="${e[2]}" rx="${e[3]}" ry="${e[4]}" fill="${eye}"/><ellipse cx="${e[1]}" cy="${e[2]}" rx="${e[3]}" ry="${e[4]}" fill="${eye}"/></g></g>`;
  const svg = `<defs><linearGradient id="${id}g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${mix(color, 255, 0.28)}"/><stop offset="1" stop-color="${mix(color, 0, 0.12)}"/></linearGradient>`
    + `<filter id="${id}f" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="9"/></filter></defs>`
    + `<g class="float" style="animation-delay:${delay}s"><g class="halo" fill="${mix(color, 255, 0.2)}"${fill.replace(`url(#${id}g)`, mix(color, 255, 0.2))} filter="url(#${id}f)">${body}</g>`
    + `<g fill="url(#${id}g)"${fill} class="body">${body}</g>`
    + (shape === 'cloud' ? '' : '<ellipse cx="34" cy="28" rx="9" ry="4" fill="#fff" opacity=".22" transform="rotate(-30 34 28)"/>')
    + eyes + '</g>'
    + (state === 'sleep' ? '<text class="z" x="80" y="20">z</text><text class="z" x="88" y="8">z</text>' : '');
  return html`<svg class=${`bot ${state}`} width=${size} height=${size} viewBox="-12 -12 124 124" aria-hidden="true" dangerouslySetInnerHTML=${{ __html: svg }} />`;
}

/** The "new bot" mark: a dashed outline that turns slowly around a plus. */
export function NewFace({ size = 44 }) {
  return html`<svg class="bot newbot" width=${size} height=${size} viewBox="-12 -12 124 124" aria-hidden="true">
    <g class="ring" fill="none" stroke="currentColor" stroke-width="2.4" stroke-dasharray="6 6"><circle cx="50" cy="50" r="42" /></g>
    <path d="M50 32v36M32 50h36" stroke="currentColor" stroke-width="3" stroke-linecap="round" /></svg>`;
}
