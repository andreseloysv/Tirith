// The 2D faces' shapes and the colours a bot can have: each shape's SVG body and where its eyes sit.
import { HERO_BODY, HERO_EYES, HERO_O } from './hero2d.js';

const petals = (n, r, pr, cy = 52) => Array.from({ length: n }, (_, i) => {
  const a = (i / n) * Math.PI * 2 - Math.PI / 2;
  return `<circle cx="${(50 + r * Math.cos(a)).toFixed(1)}" cy="${(cy + r * Math.sin(a)).toFixed(1)}" r="${pr}"/>`;
}).join('') + `<circle cx="50" cy="${cy}" r="${r + 4}"/>`;
const star = (n, ro, ri) => '<polygon points="' + Array.from({ length: n * 2 }, (_, i) => {
  const a = (i / (n * 2)) * Math.PI * 2 - Math.PI / 2, r = i % 2 ? ri : ro;
  return `${(50 + r * Math.cos(a)).toFixed(1)},${(51 + r * Math.sin(a)).toFixed(1)}`;
}).join(' ') + '"/>';

/** name: [svg body, eyes [left x, right x, y, rx, ry], round corners, a mouth of its own at rest [x, y, rx, ry]] — eyes sit
    high and small, leaving room for the mouth */
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
  hero: [HERO_BODY, HERO_EYES, false, HERO_O],
};
export const COLORS = ['#f2f2f2', '#9a6b45', '#e5484d', '#f07a3a', '#f2a93b', '#4fb86e', '#3fb6a8', '#3b82f6', '#8b5cf6', '#ec4899', '#8c8c8c'];
