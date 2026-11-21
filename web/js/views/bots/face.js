// The bots' faces: an SVG character in one of SHAPES and one of COLORS, in the luminous style (the body emits light:
// a radial gradient with a light core, a strong bloom of its own colour, dark eyes with a catchlight, a smile — the
// reference is ~/.tirith/logo-source/glowface.mjs). `state` is what the bot is doing: 'work' (a turn runs: a wide open
// grin), 'watch' (a routine is on), 'sleep' (nothing to do: eyes shut, z z) or 'idle'. Shapes live in a 100x100 box;
// `round` shapes get a thicker stroke of their own fill, which rounds their corners.
//
// Motion: every face has an animator of its own, a small state machine. Its float is a sum of slow waves whose periods
// never line up, so no loop can be seen; on top of it, at random moments, it plays one act of its state's repertoire
// (blink, double blink, glances, a hop with squash and stretch, a tilt, a wiggle, a stretch, a lean, a nod; asleep:
// settling, a twitch, a yawn, drifting z's). Phases and schedules are random per face, so faces on one screen never
// move in step. A new state blends in over a moment (and waking up plays a little hop). Faces off screen do not move;
// with reduced motion none do. `still` faces (the shape picker) are drawn without an animator.
import { html, useRef, useEffect, useMemo } from '../../../vendor/preact-htm.js';

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
  return `<defs><radialGradient id="${id}b" cx="46%" cy="42%" r="62%"><stop offset="0" stop-color="${core}"/><stop offset=".5" stop-color="${mid}"/><stop offset="1" stop-color="${edge}"/></radialGradient>`
    + `<filter id="${id}w" x="-80%" y="-80%" width="260%" height="260%"><feGaussianBlur stdDeviation="14"/></filter>`
    + `<filter id="${id}n" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="4"/></filter>`
    + `<filter id="${id}s" x="-10%" y="-10%" width="120%" height="120%"><feGaussianBlur stdDeviation="0.7"/></filter></defs>`
    + `<g class="float"><g class="halo"><g opacity=".85" fill="${halo}" stroke="${halo}" stroke-width="16" stroke-linejoin="round" filter="url(#${id}w)">${body}</g>`
    + `<g opacity=".9" fill="${halo}" stroke="${halo}" stroke-width="8" stroke-linejoin="round" filter="url(#${id}n)">${body}</g></g>`
    + `<g fill="url(#${id}b)"${soft} filter="url(#${id}s)">${body}</g>`
    + `<g opacity=".35" fill="none" stroke="#fff" stroke-width="2" filter="url(#${id}n)">${body}</g>` /* a faint inner rim of light */
    + `<g class="look"><g class="eyes" data-y="${e[2]}">${eyes}</g>${mouth}</g></g><g class="zs"></g>`;
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
let frame = 0;
function loop(now) {
  frame = 0;
  for (const a of live) a.step(now / 1000);
  if (live.size) frame = requestAnimationFrame(loop);
}
const seen = typeof IntersectionObserver === 'function'
  ? new IntersectionObserver((list) => list.forEach((e) => { if (e.target.animator) e.target.animator.visible = e.isIntersecting; })) : null;

class Animator {
  constructor(svg, state) {
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
    this.bind(svg, state);
    live.add(this);
    if (seen) { svg.animator = this; seen.observe(svg); }
    if (!frame) frame = requestAnimationFrame(loop);
  }
  /** New markup (another state, shape or colour): the same motion goes on in it. */
  bind(svg, state) {
    this.svg = svg;
    this.float = svg.querySelector('.float'); this.eyes = svg.querySelector('.eyes'); this.look = svg.querySelector('.look');
    this.halo = svg.querySelector('.halo'); this.zg = svg.querySelector('.zs'); this.zs = [];
    this.ey = Number(this.eyes && this.eyes.dataset.y) || 50;
    if (state !== this.state) {
      if (this.state === 'sleep' && state !== 'sleep') { this.act = { ...ACTS.wake(), at: this.clock }; this.blinks.push(this.clock + 0.05); }
      this.state = state; this.target = MOOD[state] || MOOD.idle;
      this.next.act = this.clock + R(1.5, 4);
    }
  }
  stop() { live.delete(this); if (seen && this.svg) seen.unobserve(this.svg); }
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
    if (this.float) this.float.setAttribute('transform', `translate(${x.toFixed(2)} ${y.toFixed(2)}) translate(50 90) rotate(${rot.toFixed(2)}) scale(${sx.toFixed(4)} ${sy.toFixed(4)}) translate(-50 -90)`);
    if (this.halo) this.halo.setAttribute('opacity', (m.halo + 0.06 * Math.sin(c * w2 + p5)).toFixed(3));
    /* the eyes: where they look (with a quick ease, like a glance) and their blinks */
    const e = this.eye, k = ease(Math.min(1, (now - e.at) / e.d));
    e.x = e.fx + (e.tx - e.fx) * k; e.y = e.fy + (e.ty - e.fy) * k;
    if (this.look && this.state !== 'sleep') this.look.setAttribute('transform', `translate(${(e.x + o.ex).toFixed(2)} ${(e.y + o.ey).toFixed(2)})`);
    let lid = 1;
    this.blinks = this.blinks.filter((b) => now - b < 0.16);
    for (const b of this.blinks) if (now >= b) lid = Math.min(lid, 1 - 0.92 * bell((now - b) / 0.16));
    if (this.eyes && this.state !== 'sleep') this.eyes.setAttribute('transform', `translate(0 ${this.ey}) scale(1 ${lid.toFixed(3)}) translate(0 ${-this.ey})`);
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

/** One bot's face. */
export function Face({ shape = 'round', color = '#3b82f6', size = 64, state = 'idle', still: frozen = false }) {
  const ref = useRef(null), anim = useRef(null);
  const svg = useMemo(() => markup(shape, color, state), [shape, color, state]);
  useEffect(() => {
    if (frozen || !ref.current) return undefined;
    anim.current = new Animator(ref.current, state);
    return () => anim.current.stop();
  }, [frozen]);
  useEffect(() => { if (anim.current && ref.current) anim.current.bind(ref.current, state); }, [svg]);
  return html`<svg ref=${ref} class=${`bot ${state}`} width=${size} height=${size} viewBox="-12 -12 124 124" aria-hidden="true" dangerouslySetInnerHTML=${{ __html: svg }} />`;
}

/** The "new bot" mark: a dashed outline that turns slowly around a plus. */
export function NewFace({ size = 44 }) {
  return html`<svg class="bot newbot" width=${size} height=${size} viewBox="-12 -12 124 124" aria-hidden="true">
    <g class="ring" fill="none" stroke="currentColor" stroke-width="2.4" stroke-dasharray="6 6"><circle cx="50" cy="50" r="42" /></g>
    <path d="M50 32v36M32 50h36" stroke="currentColor" stroke-width="3" stroke-linecap="round" /></svg>`;
}
