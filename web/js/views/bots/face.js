// The bots' faces: a live 3D character in one of SHAPES and one of COLORS — the owner-approved preview of
// 2026-11-21 (~/.tirith/logo-source/bots3d/live_template.html), ported with its own code and numbers: the body
// inflated from the shape's outline r(theta) (shapes3d.js; P 2.35, depth 0.62), its glowing body shader, glossy
// eyes with a light dot, the halo and the mouths per state (idle and watch smile, work opens its mouth with its
// tongue, sleep shuts its eyes). `state` is what the bot is doing: 'work', 'watch', 'sleep' or 'idle'.
//
// One WebGL context draws every bot that is due into a fixed slot of one sheet; each bot copies its slot into its
// own little canvas inside its element, so scrolling never lags behind them. Bots under 64 px draw 15 times a
// second, the rest (and the one under the pointer) 30; nothing draws off screen or while the page is hidden.
// Motion is the reference's: float, breathing, looks around, blinks, the halo pulsing — and with the pointer over
// it, only that bot turns whole towards the pointer, then falls back to its animation. `still` faces (the 22 px
// message avatars, the shape picker) draw once and move only under the pointer; with reduced motion every face
// holds still; without WebGL the face below it in this file shows instead: the 2D SVG face of before, with its
// own animator (its blurred layers rasterize once into will-change layers and move by transform/opacity only).
import { html, useRef, useEffect, useMemo } from '../../../vendor/preact-htm.js';
import * as THREE from '../../../vendor/three/three.module.min.js';
import { SHAPES3D } from './shapes3d.js';

const petals = (n, r, pr, cy = 52) => Array.from({ length: n }, (_, i) => {
  const a = (i / n) * Math.PI * 2 - Math.PI / 2;
  return `<circle cx="${(50 + r * Math.cos(a)).toFixed(1)}" cy="${(cy + r * Math.sin(a)).toFixed(1)}" r="${pr}"/>`;
}).join('') + `<circle cx="50" cy="${cy}" r="${r + 4}"/>`;
const star = (n, ro, ri) => '<polygon points="' + Array.from({ length: n * 2 }, (_, i) => {
  const a = (i / (n * 2)) * Math.PI * 2 - Math.PI / 2, r = i % 2 ? ri : ro;
  return `${(50 + r * Math.cos(a)).toFixed(1)},${(51 + r * Math.sin(a)).toFixed(1)}`;
}).join(' ') + '"/>';

/** name: [svg body, eyes [left x, right x, y, rx, ry], round corners] — eyes sit high and small, leaving room for the mouth */
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
export const COLORS = ['#f2f2f2', '#9a6b45', '#e5484d', '#f07a3a', '#f2a93b', '#4fb86e', '#3fb6a8', '#3b82f6', '#8b5cf6', '#ec4899', '#8c8c8c'];

const rgb = (hex) => { const n = parseInt(String(hex).slice(1, 7), 16) || 0; return [n >> 16, (n >> 8) & 255, n & 255]; };
const mix = (hex, to, t) => '#' + rgb(hex).map((c) => Math.round(c + (to - c) * t).toString(16).padStart(2, '0')).join('');

let uid = 0;
function markup(shape, color, state) {
  const [body, e, round] = SHAPES[shape] || SHAPES.round;
  const id = `bf${uid++}`, ink = '#1d1030';
  const core = mix(color, 255, 0.78), mid = mix(color, 255, 0.18), edge = mix(color, 0, 0.08), halo = mix(color, 255, 0.25);
  const soft = ` stroke="url(#${id}b)" stroke-width="${round ? 14 : 6}" stroke-linejoin="round"`; /* every shape a little puffy */
  const eyes = state === 'sleep'
    ? `<path d="M${e[0] - 6} ${e[2]}q6 5 12 0M${e[1] - 6} ${e[2]}q6 5 12 0" stroke="${ink}" stroke-width="3.2" stroke-linecap="round" fill="none"/>`
    : [e[0], e[1]].map((x) => `<ellipse cx="${x}" cy="${e[2]}" rx="${e[3]}" ry="${e[4]}" fill="${ink}"/><circle cx="${x + e[3] * 0.35}" cy="${e[2] - e[4] * 0.45}" r="${Math.max(1.4, e[3] * 0.28)}" fill="#fff"/>`).join('');
  const mx = (e[0] + e[1]) / 2, my = e[2] + e[4] + 6, w = (e[1] - e[0]) * 0.5;
  const mouth = state === 'work' /* a wide D, with the tongue inside */
    ? `<path d="M${mx - w} ${my}C${mx - w} ${my + w * 1.35} ${mx + w} ${my + w * 1.35} ${mx + w} ${my}Q${mx} ${my + w * 0.16} ${mx - w} ${my}z" fill="${ink}"/><path d="M${mx - w * 0.55} ${my + w * 0.86}C${mx - w * 0.3} ${my + w * 0.45} ${mx + w * 0.3} ${my + w * 0.45} ${mx + w * 0.55} ${my + w * 0.86}C${mx + w * 0.3} ${my + w * 1.06} ${mx - w * 0.3} ${my + w * 1.06} ${mx - w * 0.55} ${my + w * 0.86}z" fill="#d2405d"/>`
    : state === 'sleep'
      ? `<path d="M${mx - w * 0.35} ${my}q${w * 0.35} ${w * 0.3} ${w * 0.7} 0" stroke="${ink}" stroke-width="2.6" stroke-linecap="round" fill="none"/>`
      : `<path d="M${mx - w * 0.6} ${my - 1}q${w * 0.6} ${w * 0.65} ${w * 1.2} 0" stroke="${ink}" stroke-width="3" stroke-linecap="round" fill="none"/>`;
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
    eyes: `<g class="look"><g class="eyes" data-y="${e[2]}">${eyes}</g>${mouth}</g>`,
  };
}

/* ---------- the animators ---------- */
const R = (a, b) => a + Math.random() * (b - a);
const TAU = Math.PI * 2;
const ease = (x) => (x < 0.5 ? 2 * x * x : 1 - (-2 * x + 2) ** 2 / 2);
const bell = (x) => Math.sin(Math.PI * Math.min(1, Math.max(0, x)));
const hold = (x, a = 0.2) => (x < a ? ease(x / a) : x > 1 - a ? ease((1 - x) / a) : 1); /* in, stay, out */
const pick = (w) => { let n = Math.random() * Object.values(w).reduce((s, v) => s + v, 0); for (const k in w) if ((n -= w[k]) <= 0) return k; return Object.keys(w)[0]; };

/** Each state: the float's size and pace, how often the eyes blink and move (seconds, and how far), its acts and their weights. */
const MOOD = {
  idle: { bob: 2.6, sway: 0.9, tilt: 1.6, breath: 0.012, pace: 1, halo: 0.5, blink: [1.8, 6.5], look: [1.1, 4.2], reach: [4.5, 2.6], act: [4, 12], acts: { hop: 2, tilt: 3, wiggle: 2, glance: 3, stretch: 1, double: 2 } },
  watch: { bob: 2, sway: 0.6, tilt: 1, breath: 0.01, pace: 0.85, halo: 0.55, blink: [2.2, 7], look: [0.7, 2.6], reach: [5.5, 3.2], act: [3.5, 10], acts: { glance: 4, lean: 2, tilt: 2, double: 1, hop: 1 } },
  work: { bob: 2.8, sway: 0.4, tilt: 0.7, breath: 0.016, pace: 1.7, halo: 0.62, blink: [2.6, 7.5], look: [0.22, 0.75], reach: [4.5, 1.4], act: [2.5, 7], acts: { nod: 3, hop: 1, double: 1, wiggle: 1 } },
  sleep: { bob: 1.3, sway: 0.25, tilt: 0.6, breath: 0.03, pace: 0.42, halo: 0.18, blink: null, look: null, reach: [0, 0], act: [5, 14], acts: { settle: 3, twitch: 2, yawn: 1 }, z: [1.3, 3.4] },
};
const NUM = ['bob', 'sway', 'tilt', 'breath', 'pace', 'halo'];

/** An act: how long it lasts and what it adds at time x in [0, 1] (y, x, rot in degrees, sx, sy, eye offset). */
const ACTS = {
  hop: () => ({ d: R(0.55, 0.8), f: (x) => (x < 0.18 ? { sy: 1 - 0.1 * bell(x / 0.36), sx: 1 + 0.07 * bell(x / 0.36) }
    : x < 0.82 ? { y: -9 * bell((x - 0.18) / 0.64), sy: 1 + 0.05 * bell((x - 0.18) / 0.64), sx: 1 - 0.03 * bell((x - 0.18) / 0.64) }
      : { sy: 1 - 0.08 * bell((x - 0.82) / 0.36), sx: 1 + 0.05 * bell((x - 0.82) / 0.36) }) }),
  tilt: () => { const a = R(5, 9) * (Math.random() < 0.5 ? -1 : 1); return { d: R(1.3, 2.6), f: (x) => ({ rot: a * hold(x, 0.25), ex: -a * 0.15 * hold(x, 0.25) }) }; },
  wiggle: () => ({ d: R(0.55, 0.85), f: (x) => ({ rot: 5 * Math.sin(x * TAU * 2) * (1 - x) }) }),
  glance: () => { const ex = R(3.5, 5.5) * (Math.random() < 0.5 ? -1 : 1), ey = R(-2, 1); return { d: R(0.9, 1.8), eyes: true, f: (x) => ({ ex: ex * hold(x, 0.12), ey: ey * hold(x, 0.12) }) }; },
  stretch: () => ({ d: R(1, 1.5), f: (x) => ({ sy: 1 + 0.07 * hold(x, 0.35), sx: 1 - 0.04 * hold(x, 0.35), y: -1.5 * hold(x, 0.35) }) }),
  lean: () => { const s = Math.random() < 0.5 ? -1 : 1; return { d: R(1.6, 3), f: (x) => ({ x: 3 * s * hold(x, 0.3), rot: 3 * s * hold(x, 0.3), ex: 2 * s * hold(x, 0.3) }) }; },
  nod: () => ({ d: R(0.7, 1), f: (x) => ({ y: 2.2 * bell(x * 2), rot: 1.5 * bell(x * 2) }) }),
  double: () => ({ d: 0.42, blinks: [0, 0.22] }),
  settle: () => ({ d: R(1.6, 2.4), f: (x) => ({ sy: 1 - 0.05 * bell(x), sx: 1 + 0.035 * bell(x) }) }),
  twitch: () => ({ d: R(0.35, 0.5), f: (x) => ({ rot: 2.5 * Math.sin(x * TAU * 1.5) * (1 - x), y: -0.8 * bell(x) }) }),
  yawn: () => ({ d: R(1.8, 2.6), f: (x) => ({ sy: 1 + 0.08 * hold(x, 0.4), sx: 1 - 0.03 * hold(x, 0.4), rot: -2 * hold(x, 0.4) }) }),
  wake: () => ({ d: 0.7, f: (x) => ({ y: -6 * bell(x), sy: 1 + 0.04 * bell(x) }) }),
};

const live = new Set();
const still = typeof matchMedia === 'function' && matchMedia('(prefers-reduced-motion: reduce)');
let pageHidden = typeof document !== 'undefined' && document.hidden;
if (typeof document !== 'undefined') document.addEventListener('visibilitychange', () => { pageHidden = document.hidden; });
let frame = 0, lastStep = 0;
function loop(now) {
  frame = 0;
  if (now - lastStep >= 1000 / 30) { /* at most 30 steps a second, however fast the screen refreshes */
    lastStep = now;
    if (!pageHidden) for (const a of live) a.step(now / 1000);
  }
  if (live.size) frame = requestAnimationFrame(loop);
}
const seen = typeof IntersectionObserver === 'function'
  ? new IntersectionObserver((list) => list.forEach((e) => { if (e.target.animator) e.target.animator.visible = e.isIntersecting; })) : null;

class Animator {
  constructor(root, state) {
    this.mood = { ...MOOD[state] || MOOD.idle };
    this.target = MOOD[state] || MOOD.idle;
    this.state = state;
    /* incommensurate periods and random phases: this face's float never repeats and never matches another's */
    this.w = [R(3.1, 3.9), R(5.3, 7.7), R(6.5, 9.5), R(7, 11.5), R(3.6, 4.9)].map((p) => TAU / p);
    this.p = this.w.map(() => R(0, TAU));
    this.t = null; this.clock = 0;
    this.visible = true;
    this.eye = { x: 0, y: 0, fx: 0, fy: 0, tx: 0, ty: 0, at: 0, d: 1 };
    this.next = { blink: R(0.5, 4), look: R(0.3, 2), act: R(1.5, 7), z: R(0.5, 2) };
    this.blinks = []; this.act = null; this.zs = [];
    this.bind(root, state);
    live.add(this);
    if (seen) { root.animator = this; seen.observe(root); }
    if (!frame) frame = requestAnimationFrame(loop);
  }
  /** New markup (another state, shape or colour): the same motion goes on in it. */
  bind(root, state) {
    this.root = root;
    this.float = root.querySelector('.ffloat'); this.eyes = root.querySelector('.eyes'); this.look = root.querySelector('.look');
    this.halo = root.querySelector('.fhalo'); this.zg = root.querySelector('.zs');
    this.zs.forEach((z) => z.el.remove()); this.zs = [];
    this.lookT = this.eyesT = null; /* the last eye transforms written: unchanged ones are not written again */
    this.ey = Number(this.eyes && this.eyes.dataset.y) || 50;
    if (state !== this.state) {
      if (this.state === 'sleep' && state !== 'sleep') { this.act = { ...ACTS.wake(), at: this.clock }; this.blinks.push(this.clock + 0.05); }
      this.state = state; this.target = MOOD[state] || MOOD.idle;
      this.next.act = this.clock + R(1.5, 4);
    }
  }
  stop() { live.delete(this); if (seen && this.root) seen.unobserve(this.root); }
  step(t) {
    const dt = this.t == null ? 0 : Math.min(0.1, t - this.t);
    this.t = t;
    if (!this.visible || (still && still.matches)) return;
    const m = this.mood, g = this.target, c = (this.clock += dt * m.pace);
    for (const k of NUM) m[k] += (g[k] - m[k]) * Math.min(1, dt * 2.5); /* a new state blends in */
    const now = this.clock;
    /* events of this state, each at its own random moment */
    if (g.blink && now > this.next.blink) { this.blinks.push(now); if (Math.random() < 0.18) this.blinks.push(now + 0.24); this.next.blink = now + R(...g.blink); }
    if (g.look && now > this.next.look) {
      const e = this.eye, [rx, ry] = g.reach;
      const work = this.state === 'work';
      e.fx = e.x; e.fy = e.y; e.at = now; e.d = R(0.12, work ? 0.2 : 0.35);
      e.tx = work ? (e.tx > 0 ? -R(1, rx) : R(1, rx)) : Math.random() < 0.3 ? 0 : R(-rx, rx);
      e.ty = work ? R(-0.5, 1.5) : Math.random() < 0.4 ? 0 : R(-ry, ry * 0.6);
      this.next.look = now + R(...g.look);
    }
    if (!this.act && now > this.next.act) {
      const name = pick(g.acts), a = ACTS[name]();
      this.act = { ...a, at: now };
      if (a.blinks) a.blinks.forEach((o) => this.blinks.push(now + o));
      this.next.act = now + a.d + R(...g.act);
    }
    if (g.z && now > this.next.z && this.zg && this.zs.length < 3) { this.spawnZ(now); this.next.z = now + R(...g.z); }
    /* the act now playing */
    let o = { x: 0, y: 0, rot: 0, sx: 1, sy: 1, ex: 0, ey: 0 };
    if (this.act) {
      const x = (now - this.act.at) / this.act.d;
      if (x >= 1) this.act = null;
      else if (this.act.f) o = { ...o, ...this.act.f(x) };
    }
    /* the float: slow waves that never line up */
    const [w1, w2, w3, w4, w5] = this.w, [p1, p2, p3, p4, p5] = this.p;
    const y = m.bob * (0.62 * Math.sin(c * w1 + p1) + 0.38 * Math.sin(c * w2 + p2)) + o.y;
    const x = m.sway * Math.sin(c * w3 + p3) + o.x;
    const rot = m.tilt * Math.sin(c * w4 + p4) + o.rot;
    const br = 1 + m.breath * Math.sin(c * w5 + p5);
    const sx = o.sx * (2 - br), sy = o.sy * br;
    /* the float and the halo pulse move HTML layers only (CSS transform/opacity on will-change elements), never
       attributes inside the filtered SVGs: 124 viewBox units span the layer, so a unit is 100/124 % of it, and the
       pivot (50 90) is the layers' transform-origin in bots.css */
    if (this.float) this.float.style.transform = `translate(${(x * 100 / 124).toFixed(3)}%, ${(y * 100 / 124).toFixed(3)}%) rotate(${rot.toFixed(2)}deg) scale(${sx.toFixed(4)}, ${sy.toFixed(4)})`;
    if (this.halo) this.halo.style.opacity = (m.halo + 0.06 * Math.sin(c * w2 + p5)).toFixed(3);
    /* the eyes: where they look (with a quick ease, like a glance) and their blinks */
    const e = this.eye, k = ease(Math.min(1, (now - e.at) / e.d));
    e.x = e.fx + (e.tx - e.fx) * k; e.y = e.fy + (e.ty - e.fy) * k;
    if (this.look && this.state !== 'sleep') {
      const tr = `translate(${(e.x + o.ex).toFixed(2)} ${(e.y + o.ey).toFixed(2)})`;
      if (tr !== this.lookT) { this.lookT = tr; this.look.setAttribute('transform', tr); }
    }
    let lid = 1;
    this.blinks = this.blinks.filter((b) => now - b < 0.16);
    for (const b of this.blinks) if (now >= b) lid = Math.min(lid, 1 - 0.92 * bell((now - b) / 0.16));
    if (this.eyes && this.state !== 'sleep') {
      const tr = `translate(0 ${this.ey}) scale(1 ${lid.toFixed(3)}) translate(0 ${-this.ey})`;
      if (tr !== this.eyesT) { this.eyesT = tr; this.eyes.setAttribute('transform', tr); }
    }
    this.zs = this.zs.filter((z) => {
      const x2 = (now - z.at) / z.d;
      if (x2 >= 1 || this.state !== 'sleep') { z.el.remove(); return false; }
      z.el.setAttribute('transform', `translate(${(z.x + 10 * x2 + 1.5 * Math.sin(x2 * TAU)).toFixed(2)} ${(z.y - 26 * x2).toFixed(2)})`);
      z.el.setAttribute('opacity', bell(x2).toFixed(3));
      return true;
    });
  }
  spawnZ(now) {
    const el = document.createElementNS('http://www.w3.org/2000/svg', 'text');
    el.textContent = 'z'; el.setAttribute('class', 'z'); el.setAttribute('font-size', R(11, 16).toFixed(1)); el.setAttribute('opacity', '0');
    this.zg.appendChild(el);
    this.zs.push({ el, at: now, d: R(2.2, 3.2), x: R(72, 82), y: R(16, 24) });
  }
}

/* ---------- the 3D bots: the reference's code and numbers ---------- */
const P = 2.35, DEPTH = 0.62, PX = 512 / 150, MARGIN = 1.7, CAP = 820; // the drawn square is the bot's box grown for the halo
const Q3 = typeof location !== 'undefined' ? new URLSearchParams(location.search) : new URLSearchParams();
const FREEZE = Q3.has('freeze') ? { yaw: Number(Q3.get('yaw') || 0), pitch: Number(Q3.get('pitch') || 0), mood: Q3.get('mood') } : null; // still pictures to compare against

// shape maths (the same as the Blender script)
function radiusAt(sh, th) { const n = sh.r.length; const f = (((th % TAU) + TAU) % TAU) / TAU * n; const i = Math.floor(f) % n, t = f - Math.floor(f); return sh.r[i] * (1 - t) + sh.r[(i + 1) % n] * t; }
function boxPt(sh, x, y) { return [((x + 25) * PX - sh.cx) / sh.rmax, ((512 - (y + 25) * PX) - sh.cy) / sh.rmax]; }
function frontPt(sh, u, w) { const t = Math.min(Math.hypot(u, w) / radiusAt(sh, Math.atan2(w, u)), 0.995); return new THREE.Vector3(u, w, DEPTH * Math.pow(1 - Math.pow(t, P), 1 / P)); }
function normalAt(sh, u, w) { const e = 1e-3, p = frontPt(sh, u, w); return frontPt(sh, u + e, w).sub(p).cross(frontPt(sh, u, w + e).sub(p)).normalize(); }
const geoCache3 = {};
/** An indexed sphere made by hand — the same points THREE.SphereGeometry(1, seg, round(seg * 0.75)) has after
 *  mergeVertices (seam and poles welded), so the body below needs no BufferGeometryUtils addon. */
function sphere3(seg) {
  const h = Math.round(seg * 0.75), pos = [], rows = [], idx = [];
  for (let iy = 0; iy <= h; iy++) {
    const theta = (iy / h) * Math.PI, st = Math.sin(theta), ct = Math.cos(theta), row = [];
    if (iy === 0 || iy === h) { const v = pos.length / 3; pos.push(0, ct, 0); for (let ix = 0; ix < seg; ix++) row.push(v); }
    else for (let ix = 0; ix < seg; ix++) { const phi = (ix / seg) * TAU; row.push(pos.length / 3); pos.push(-Math.cos(phi) * st, ct, Math.sin(phi) * st); }
    rows.push(row);
  }
  for (let iy = 0; iy < h; iy++) for (let ix = 0; ix < seg; ix++) {
    const a = rows[iy][ix], b = rows[iy + 1][ix], c = rows[iy + 1][(ix + 1) % seg], d = rows[iy][(ix + 1) % seg];
    if (iy !== 0) idx.push(a, b, d);
    if (iy !== h - 1) idx.push(b, c, d);
  }
  return { pos, idx };
}
function bodyGeometry3(name, seg) {
  const key = name + seg; if (geoCache3[key]) return geoCache3[key];
  const sh = SHAPES3D[name] || SHAPES3D.round;
  const { pos, idx } = sphere3(seg), arr = new Float32Array(pos);
  for (let i = 0; i < arr.length; i += 3) {
    const a = arr[i], c = arr[i + 1], b = arr[i + 2];
    const th = Math.atan2(c, a), m = Math.min(1, Math.hypot(a, c)), rr = radiusAt(sh, th) * Math.pow(m, 2 / P);
    arr[i] = rr * Math.cos(th); arr[i + 1] = rr * Math.sin(th); arr[i + 2] = DEPTH * Math.sign(b) * Math.pow(Math.abs(b), 2 / P);
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.BufferAttribute(arr, 3));
  g.setIndex(idx);
  g.computeVertexNormals();
  return (geoCache3[key] = g);
}

// materials: body, eyes and halo are ShaderMaterials fed THREE.Color values — they go in linear (colour
// management) and come out unconverted, as in the reference; the mouth, tongue and dot are MeshBasicMaterial,
// whose own shader converts back, so they show as their hex.
const hex3 = (h) => new THREE.Color(h);
const LIGHT3 = new THREE.Vector3(-0.55, 0.65, 0.75).normalize(); // soft key from the upper left, in view space
const bodyVS = `varying vec3 vN; varying vec3 vV; varying vec3 vWN;
void main(){ vec4 mv = modelViewMatrix * vec4(position,1.0); vN = normalize(normalMatrix * normal); vV = -mv.xyz; vWN = normalize(mat3(modelMatrix) * normal); gl_Position = projectionMatrix * mv; }`;
const bodyFS = `uniform vec3 core; uniform vec3 edge; uniform vec3 low; uniform vec3 L; uniform float glow;
varying vec3 vN; varying vec3 vV; varying vec3 vWN;
void main(){
  vec3 N = normalize(vN), V = normalize(vV);
  float facing = clamp(dot(N, V), 0.0, 1.0), f = 1.0 - facing;
  vec3 col = mix(core, edge, smoothstep(0.05, 0.95, pow(f, 0.75)));      // light inside: a pale core, its own colour at the rim
  float under = clamp((vWN.y + 1.0) / 1.5, 0.0, 1.0);
  col = mix(low, col, 0.72 + 0.28 * under);                                 // warmer and deeper underneath
  float wrap = clamp((dot(N, L) + 0.55) / 1.55, 0.0, 1.0);
  vec3 H = normalize(L + V);
  float spec = pow(max(dot(N, H), 0.0), 48.0) * 0.32 + pow(max(dot(N, H), 0.0), 300.0) * 0.35; // a soft coat and a small hot spot
  vec3 c = col * (0.80 + 0.28 * wrap) * glow + vec3(spec);
  c += mix(edge, vec3(1.0), 0.35) * pow(f, 2.4) * 0.6;                                           // light leaking at the silhouette
  gl_FragColor = vec4(c, 1.0);
}`;
function bodyMaterial3(color) {
  const base = hex3(color);
  const core = base.clone().lerp(new THREE.Color(1, 1, 1), 0.62);
  const low = base.clone().multiplyScalar(0.78).lerp(new THREE.Color(1, 0.55, 0.1), 0.18);
  return new THREE.ShaderMaterial({ vertexShader: bodyVS, fragmentShader: bodyFS, uniforms: { core: { value: core }, edge: { value: base }, low: { value: low }, L: { value: LIGHT3 }, glow: { value: 1.12 } } });
}
const eyeMat3 = new THREE.ShaderMaterial({ vertexShader: bodyVS, uniforms: { L: { value: LIGHT3 } }, fragmentShader: `uniform vec3 L; varying vec3 vN; varying vec3 vV; varying vec3 vWN;
void main(){ vec3 N = normalize(vN), V = normalize(vV), H = normalize(L + V);
  vec3 c = vec3(0.07, 0.035, 0.09) + vec3(0.10, 0.07, 0.12) * pow(1.0 - clamp(dot(N, V), 0.0, 1.0), 2.0);
  c += vec3(1.0) * smoothstep(0.86, 0.95, pow(max(dot(N, H), 0.0), 40.0));  // the catchlight
  gl_FragColor = vec4(c, 1.0); }` });
const flat3 = (c) => new THREE.MeshBasicMaterial({ color: c, side: THREE.DoubleSide });
const EYE_GEO3 = new THREE.SphereGeometry(1, 24, 16), DOT_GEO3 = new THREE.SphereGeometry(1, 12, 8), DOT_MAT3 = new THREE.MeshBasicMaterial({ color: 0xffffff }), HALO_GEO3 = new THREE.PlaneGeometry(1, 1);
const MOUTH3 = flat3('#2a0a16'), TONGUE3 = flat3('#e2506d'), INK3 = flat3('#1d1030');
const haloVS = `varying vec2 vUv; void main(){ vUv = uv; gl_Position = projectionMatrix * modelViewMatrix * vec4(position,1.0); }`;
const haloFS = `uniform vec3 color; uniform float amount; varying vec2 vUv;
void main(){ float d = length(vUv - 0.5) * 2.0; float a = pow(clamp(1.0 - d, 0.0, 1.0), 1.7) * amount; gl_FragColor = vec4(color, a); }`;

function decal3(sh, pts2d, lift, mat) { // a flat shape laid on the face, a hair above the surface
  const c = pts2d.reduce((a, p) => [a[0] + p[0] / pts2d.length, a[1] + p[1] / pts2d.length], [0, 0]);
  const all = [c, ...pts2d], pos = [];
  const v = all.map(([u, w]) => frontPt(sh, u, w).add(normalAt(sh, u, w).multiplyScalar(lift)));
  for (let i = 1; i < all.length; i++) { const j = i === all.length - 1 ? 1 : i + 1; pos.push(...v[0].toArray(), ...v[i].toArray(), ...v[j].toArray()); }
  const g = new THREE.BufferGeometry(); g.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
  return new THREE.Mesh(g, mat);
}
function tube3(sh, pts2d, radius, mat) {
  const curve = new THREE.CatmullRomCurve3(pts2d.map(([u, w]) => frontPt(sh, u, w)));
  return new THREE.Mesh(new THREE.TubeGeometry(curve, 24, radius, 8, false), mat);
}
const cubic3 = (p0, p1, p2, p3, n) => Array.from({ length: n }, (_, i) => { const t = i / n, s = 1 - t; return [0, 1].map((k) => s * s * s * p0[k] + 3 * s * s * t * p1[k] + 3 * s * t * t * p2[k] + t * t * t * p3[k]); });
const quad3 = (p0, p1, p2, n) => Array.from({ length: n }, (_, i) => { const t = i / n, s = 1 - t; return [0, 1].map((k) => s * s * p0[k] + 2 * s * t * p1[k] + t * t * p2[k]); });

class Bot3D {
  constructor(el, name, color, mood, frozen) {
    this.el = el; this.name = SHAPES3D[name] ? name : 'round'; this.sh = SHAPES3D[this.name]; this.color = color;
    this.reduced = !!(still && still.matches);
    this.quiet = !!frozen || this.reduced; // drawn once; moves only under the pointer (never, with reduced motion)
    this.drawnOnce = false; this.dirty = true; this.settled = true; this.pointerOver = false; this.hovered = false;
    this.scene = new THREE.Scene();
    this.cam = new THREE.PerspectiveCamera(24, 1, 0.1, 50);
    const sh = this.sh, e = sh.eyes;
    // the box centre (50,50) of the 2D face is the middle of the element
    const [bx, by] = boxPt(sh, 50, 50), half = (62 * PX / sh.rmax) * MARGIN; // the element shows the -12..112 box, grown by MARGIN
    const dist = half / Math.tan(THREE.MathUtils.degToRad(12));
    this.cam.position.set(bx, by, dist); this.cam.lookAt(bx, by, 0);
    this.halo = new THREE.Mesh(HALO_GEO3, new THREE.ShaderMaterial({ vertexShader: haloVS, fragmentShader: haloFS, transparent: true, depthWrite: false,
      uniforms: { color: { value: hex3(this.color).lerp(new THREE.Color(1, 1, 1), 0.45) }, amount: { value: 0.95 } } }));
    this.halo.scale.setScalar(4.6); this.halo.position.set(0, 0, -1.2);
    this.scene.add(this.halo);
    this.rig = new THREE.Group(); this.scene.add(this.rig);
    const seg = el.offsetWidth > 90 ? 128 : 64;
    this.body = new THREE.Mesh(bodyGeometry3(this.name, seg), bodyMaterial3(this.color)); this.rig.add(this.body);
    // eyes: glossy dark lenses set into the surface, each with its dot of light
    this.eyes = [];
    for (const x of [e[0], e[1]]) {
      const [u, w] = boxPt(sh, x, e[2]), rx = e[3] * PX / sh.rmax, ry = e[4] * PX / sh.rmax;
      const n = normalAt(sh, u, w), p = frontPt(sh, u, w);
      const eye = new THREE.Mesh(EYE_GEO3, eyeMat3);
      const z = n.clone(), xv = new THREE.Vector3(0, 1, 0).cross(z).normalize(), yv = z.clone().cross(xv);
      eye.quaternion.setFromRotationMatrix(new THREE.Matrix4().makeBasis(xv, yv, z));
      eye.position.copy(p.addScaledVector(n, -0.12 * rx));
      eye.userData.s = [rx * 0.9, ry * 0.9, 0.3 * rx];
      eye.scale.set(...eye.userData.s);
      const dot = new THREE.Mesh(DOT_GEO3, DOT_MAT3);
      dot.position.set(-0.3, 0.42, 0.75); dot.scale.set(0.26, 0.28, 0.25); eye.add(dot);
      this.rig.add(eye); this.eyes.push(eye);
    }
    // mouths for each mood; sleeping eyes
    const mx = (e[0] + e[1]) / 2, my = e[2] + e[4] + 6, hw = (e[1] - e[0]) * 0.5, k = (v) => boxPt(sh, v[0], v[1]);
    const grin = [...cubic3([mx - hw, my], [mx - hw, my + hw * 1.35], [mx + hw, my + hw * 1.35], [mx + hw, my], 24), ...quad3([mx + hw, my], [mx, my + hw * 0.16], [mx - hw, my], 12)].map(k);
    const tongue = Array.from({ length: 20 }, (_, i) => { const a = i / 20 * TAU; return k([mx + hw * 0.5 * Math.cos(a), my + hw * 0.78 + hw * 0.2 * Math.sin(a)]); });
    this.grin = new THREE.Group(); this.grin.add(decal3(sh, grin, 0.004, MOUTH3), decal3(sh, tongue, 0.008, TONGUE3));
    const lineW = 1.6 * PX / sh.rmax;
    this.smile = tube3(sh, [...quad3([mx - hw * 0.6, my - 1], [mx, my - 1 + hw * 1.3], [mx + hw * 0.6, my - 1], 16), [mx + hw * 0.6, my - 1]].map(k), lineW, INK3);
    this.sleepy = new THREE.Group();
    for (const x of [e[0], e[1]]) this.sleepy.add(tube3(sh, quad3([x - 6, e[2]], [x, e[2] + 5], [x + 6, e[2]], 10).concat([[x + 6, e[2]]]).map(k), lineW * 1.1, INK3));
    this.sleepy.add(tube3(sh, quad3([mx - hw * 0.35, my], [mx, my + hw * 0.3], [mx + hw * 0.35, my], 10).concat([[mx + hw * 0.35, my]]).map(k), lineW * 0.9, INK3));
    this.rig.add(this.grin, this.smile, this.sleepy);
    this.yaw = 0; this.pitch = 0; this.ty = 0; this.tp = 0; this.nextLook = 0; this.nextBlink = 1 + Math.random() * 3; this.blinkAt = -1;
    this.phase = Math.random() * 10; this.setMood(mood);
    this.cv = el.querySelector('canvas.botcv');
    if (!this.cv) { this.cv = document.createElement('canvas'); this.cv.className = 'botcv'; el.append(this.cv); }
    this.ctx = this.cv.getContext('2d');
    this.visible = true; el.bot3d = this; if (seen3) seen3.observe(el); this.id = Bot3D.n = (Bot3D.n || 0) + 1;
    this.onEnter = () => { this.pointerOver = true; };
    this.onLeave = () => { this.pointerOver = false; };
    el.addEventListener('pointerenter', this.onEnter);
    el.addEventListener('pointerleave', this.onLeave);
  }
  setMood(m) { this.mood = m; this.grin.visible = m === 'work'; this.smile.visible = m === 'idle' || m === 'watch'; this.sleepy.visible = m === 'sleep'; this.eyes.forEach((x) => (x.visible = m !== 'sleep')); this.dirty = true; }
  update(t, dt, pointer) {
    const r = this.el.getBoundingClientRect(), cx = r.left + r.width / 2, cy = r.top + r.height / 2;
    if (FREEZE) { // no motion, no blink, the given angle: the same picture every time
      if (FREEZE.mood && this.mood !== FREEZE.mood) this.setMood(FREEZE.mood);
      this.rig.rotation.set(FREEZE.pitch, FREEZE.yaw, 0); this.rig.position.y = 0; this.rig.scale.set(1, 1, 1);
      this.halo.material.uniforms.amount.value = 0.82;
      for (const eye of this.eyes) eye.scale.set(...eye.userData.s);
      return r;
    }
    const near = !this.reduced && pointer && pointer.x >= r.left && pointer.x <= r.right && pointer.y >= r.top && pointer.y <= r.bottom; // only the bot under the pointer
    this.hovered = !!near;
    if (this.quiet && !near) { // a still face off the pointer: ease back to front and hold the pose
      const easeBack = Math.min(1, dt * 3.5);
      this.yaw += (0 - this.yaw) * easeBack; this.pitch += (0 - this.pitch) * easeBack;
      this.settled = Math.abs(this.yaw) < 0.002 && Math.abs(this.pitch) < 0.002;
      if (this.settled) { this.yaw = 0; this.pitch = 0; }
      this.rig.rotation.set(this.pitch, this.yaw, 0); this.rig.position.y = 0; this.rig.scale.set(1, 1, 1);
      this.halo.material.uniforms.amount.value = 0.82;
      for (const eye of this.eyes) eye.scale.set(...eye.userData.s);
      return r;
    }
    this.settled = false;
    const lim = this.mood === 'sleep' ? 0.3 : 1;
    if (near) { // it turns, all of it, towards the pointer
      this.ty = THREE.MathUtils.clamp((pointer.x - cx) / (r.width / 2), -1, 1) * 0.62 * lim;
      this.tp = THREE.MathUtils.clamp((pointer.y - cy) / (r.height / 2), -1, 1) * 0.42 * lim;
      this.nextLook = t + 0.8;
    } else if (t > this.nextLook) { // on its own: it looks around now and then
      const busy = this.mood === 'work';
      this.ty = (Math.random() * 2 - 1) * (busy ? 0.45 : 0.32) * lim; this.tp = (Math.random() * 2 - 1) * 0.18 * lim;
      this.nextLook = t + (busy ? 0.6 + Math.random() : 1.6 + Math.random() * 2.6);
    }
    const easeAmt = Math.min(1, dt * (near ? 9 : 3.5));
    this.yaw += (this.ty - this.yaw) * easeAmt; this.pitch += (this.tp - this.pitch) * easeAmt;
    const sp = this.mood === 'work' ? 2.2 : this.mood === 'sleep' ? 0.45 : 1, ph = this.phase + t * sp;
    this.rig.rotation.set(this.pitch + 0.04 * Math.sin(ph * 0.9), this.yaw, 0.05 * Math.sin(ph * 0.63));
    this.rig.position.y = 0.05 * Math.sin(ph * 1.1);
    const br = 1 + 0.025 * Math.sin(ph * 1.7); this.rig.scale.set(2 - br, br, 1);
    this.halo.material.uniforms.amount.value = 0.82 + 0.2 * Math.sin(ph * (this.mood === 'work' ? 2.4 : 1.2));
    // blinks
    if (t > this.nextBlink) { this.blinkAt = t; this.nextBlink = t + 2.2 + Math.random() * 4.5; }
    const b = this.blinkAt < 0 ? 1 : Math.min(1, Math.abs(t - this.blinkAt - 0.08) / 0.08);
    for (const eye of this.eyes) eye.scale.set(eye.userData.s[0], eye.userData.s[1] * Math.max(0.08, b), eye.userData.s[2]);
    return r;
  }
  dispose() {
    bots3.delete(this);
    if (seen3) seen3.unobserve(this.el);
    this.el.removeEventListener('pointerenter', this.onEnter);
    this.el.removeEventListener('pointerleave', this.onLeave);
    if (this.el.bot3d === this) this.el.bot3d = null;
    this.body.material.dispose(); this.halo.material.dispose();
    for (const g of [this.grin, this.sleepy]) g.children.forEach((m) => m.geometry.dispose());
    this.smile.geometry.dispose();
  }
}

// one GPU renderer off screen draws every bot that is due into its own corner of one sheet; then each bot copies
// its corner into its own little canvas, which scrolls with the page. One sheet means one snapshot per frame.
const bots3 = new Set();
let gl3 = null, glFailed = false, sheetW = 0, sheetH = 0, raf3 = 0, pointer3 = null;
let pack3 = { key: '', spots: new Map() };
const seen3 = typeof IntersectionObserver === 'function'
  ? new IntersectionObserver((es) => es.forEach((e) => { if (e.target.bot3d) e.target.bot3d.visible = e.isIntersecting; }), { rootMargin: '120px' }) : null;
function ensureGL() {
  if (gl3) return true;
  if (glFailed || typeof document === 'undefined') return false;
  try {
    const canvas = document.createElement('canvas');
    const renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true });
    renderer.setPixelRatio(1);
    renderer.setClearColor(0x000000, 0);
    renderer.autoClear = false;
    renderer.setScissorTest(true);
    gl3 = { canvas, renderer };
    addEventListener('pointermove', (ev) => { pointer3 = { x: ev.clientX, y: ev.clientY, t: performance.now() / 1000 }; }, { passive: true });
    addEventListener('pointerdown', (ev) => { pointer3 = { x: ev.clientX, y: ev.clientY, t: performance.now() / 1000 }; }, { passive: true });
    document.documentElement.addEventListener('pointerleave', () => { pointer3 = null; bots3.forEach((b) => { b.pointerOver = false; }); });
    addEventListener('scroll', () => { pointer3 = null; }, { passive: true });
    addEventListener('resize', () => bots3.forEach((b) => { b.dirty = true; b.drawnOnce = false; }));
    return true;
  } catch (err) { glFailed = true; return false; }
}
function place3(all) { // a fixed square on the sheet for every visible bot; recomputed only when that set or a size changes
  const key = all.map((d) => d.b.id + ':' + d.px).join(',');
  if (key === pack3.key) return pack3.spots;
  const order = [...all].sort((a, b) => b.px - a.px), spots = new Map();
  const W = 2048; let x = 0, y = 0, row = 0;
  for (const d of order) { if (x + d.px > W) { x = 0; y += row; row = 0; } spots.set(d.b, [x, y]); x += d.px; row = Math.max(row, d.px); }
  const H = y + row, w = Math.min(W, order.reduce((a, d) => a + d.px, 0));
  const nw = Math.max(256, Math.ceil(w / 256) * 256), nh = Math.max(256, Math.ceil(H / 256) * 256);
  if (nw !== sheetW || nh !== sheetH) { sheetW = nw; sheetH = nh; gl3.renderer.setSize(nw, nh, false); }
  pack3 = { key, spots };
  return spots;
}
function frame3() {
  if (!bots3.size) { raf3 = 0; return; }
  raf3 = requestAnimationFrame(frame3);
  if (document.hidden) return;
  const now = performance.now() / 1000;
  const dpr = Math.min(2, devicePixelRatio);
  const due = [], shown = [];
  for (const b of bots3) {
    if (!b.visible) continue; // off screen: nothing to draw
    shown.push({ b, px: b.cv.width || Math.round(b.el.offsetWidth * MARGIN * dpr) });
    const hoverActive = b.pointerOver && !b.reduced;
    if (FREEZE && b.drawnOnce && !b.dirty) continue; // frozen: the same picture, drawn once
    if (b.quiet && b.drawnOnce && !b.dirty && !hoverActive && b.settled) continue; // still: drawn once, moves only under the pointer
    // small bots in lists move a pixel or two: 15 pictures a second are enough; big ones and the one under the pointer get 30
    const every = b.hovered || hoverActive || b.el.offsetWidth >= 64 ? 1 / 30 : 1 / 15;
    if (now - (b.drawnAt || 0) < every - 0.004) continue;
    const dt = Math.min(0.1, now - (b.drawnAt || now)); b.drawnAt = now;
    const r = b.update(now, dt, pointer3);
    const px = Math.min(CAP, Math.round(r.width * MARGIN * dpr));
    if (b.cv.width !== px) b.cv.width = b.cv.height = px;
    due.push({ b, px });
  }
  if (due.length) {
    const { renderer, canvas: glCanvas } = gl3;
    const spots = place3(shown);
    renderer.setScissorTest(true);
    for (const { b, px } of due) {
      const [x, y] = spots.get(b);
      renderer.setViewport(x, y, px, px); renderer.setScissor(x, y, px, px); renderer.clear();
      renderer.render(b.scene, b.cam);
    }
    for (const { b, px } of due) {
      const [x, y] = spots.get(b);
      b.ctx.clearRect(0, 0, px, px);
      b.ctx.drawImage(glCanvas, x, sheetH - y - px, px, px, 0, 0, px, px);
      b.drawnOnce = true; b.dirty = false;
    }
  }
}
/** Put a live 3D bot in an element (Face uses it; the test page web/bots3d-test.html too).
 *  Returns the bot (setMood, dispose), or null without WebGL. */
export function attachBot3D(el, { shape = 'round', color = '#3b82f6', state = 'idle', still: frozen = false } = {}) {
  if (!el || !ensureGL()) return null;
  const b = new Bot3D(el, shape, color, state, frozen);
  bots3.add(b);
  if (!raf3) raf3 = requestAnimationFrame(frame3);
  return b;
}

/** One bot's face, live in 3D (see the header); without WebGL, the 2D face as stacked layers. The size travels as
 *  the --fs custom property, so the stylesheet's own `.bot` sizes still win where they apply, as the svg's did. */
export function Face({ shape = 'round', color = '#3b82f6', size = 64, state = 'idle', still: frozen = false }) {
  const ref = useRef(null), bot = useRef(null), anim = useRef(null);
  const ok3 = ensureGL();
  const parts = useMemo(() => (ok3 ? null : markup(shape, color, state)), [ok3, shape, color, state]);
  useEffect(() => {
    if (!ok3 || !ref.current) return undefined;
    bot.current = attachBot3D(ref.current, { shape, color, state, still: frozen });
    return () => { if (bot.current) bot.current.dispose(); bot.current = null; };
  }, [ok3, shape, color, frozen]);
  useEffect(() => { if (bot.current) bot.current.setMood(state); }, [state]);
  useEffect(() => {
    if (ok3 || frozen || !ref.current) return undefined;
    anim.current = new Animator(ref.current, state);
    return () => anim.current.stop();
  }, [ok3, frozen]);
  useEffect(() => { if (anim.current && ref.current) anim.current.bind(ref.current, state); }, [parts]);
  if (ok3) return html`<span ref=${ref} class=${`bot face face3d ${state}`} style=${`--fs:${size}px`} aria-hidden="true"><canvas class="botcv"></canvas></span>`;
  return html`<span ref=${ref} class=${`bot face ${state}`} style=${`--fs:${size}px`} aria-hidden="true"><span class="ffloat"><span class="fhalo"><svg class="fsvg" viewBox="-12 -12 124 124" dangerouslySetInnerHTML=${{ __html: parts.halo }} /></span><svg class="fsvg" viewBox="-12 -12 124 124" dangerouslySetInnerHTML=${{ __html: parts.body }} /><svg class="fsvg" viewBox="-12 -12 124 124" dangerouslySetInnerHTML=${{ __html: parts.eyes }} /></span><svg class="fsvg fzs" viewBox="-12 -12 124 124" dangerouslySetInnerHTML=${{ __html: '<g class="zs"></g>' }} /></span>`;
}

/** The "new bot" mark: a dashed outline that turns slowly around a plus. */
export function NewFace({ size = 44 }) {
  return html`<svg class="bot newbot" width=${size} height=${size} viewBox="-12 -12 124 124" aria-hidden="true">
    <g class="ring" fill="none" stroke="currentColor" stroke-width="2.4" stroke-dasharray="6 6"><circle cx="50" cy="50" r="42" /></g>
    <path d="M50 32v36M32 50h36" stroke="currentColor" stroke-width="3" stroke-linecap="round" /></svg>`;
}
