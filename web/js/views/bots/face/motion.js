// The 2D face's motion: easing curves, each state's mood (float, blinks, glances) and the acts it plays now
// and then.
export const R = (a, b) => a + Math.random() * (b - a);
export const TAU = Math.PI * 2;
export const ease = (x) => (x < 0.5 ? 2 * x * x : 1 - (-2 * x + 2) ** 2 / 2);
export const bell = (x) => Math.sin(Math.PI * Math.min(1, Math.max(0, x)));
const hold = (x, a = 0.2) => (x < a ? ease(x / a) : x > 1 - a ? ease((1 - x) / a) : 1); /* in, stay, out */
export const pick = (w) => { let n = Math.random() * Object.values(w).reduce((s, v) => s + v, 0); for (const k in w) if ((n -= w[k]) <= 0) return k; return Object.keys(w)[0]; };

/** Each state: the float's size and pace, how often the eyes blink and move (seconds, and how far), its acts and their weights. */
export const MOOD = {
  idle: { bob: 2.6, sway: 0.9, tilt: 1.6, breath: 0.012, pace: 1, halo: 0.5, blink: [1.8, 6.5], look: [1.1, 4.2], reach: [4.5, 2.6], act: [4, 12], acts: { hop: 2, tilt: 3, wiggle: 2, glance: 3, stretch: 1, double: 2 } },
  watch: { bob: 2, sway: 0.6, tilt: 1, breath: 0.01, pace: 0.85, halo: 0.55, blink: [2.2, 7], look: [0.7, 2.6], reach: [5.5, 3.2], act: [3.5, 10], acts: { glance: 4, lean: 2, tilt: 2, double: 1, hop: 1 } },
  work: { bob: 2.8, sway: 0.4, tilt: 0.7, breath: 0.016, pace: 1.7, halo: 0.62, blink: [2.6, 7.5], look: [0.22, 0.75], reach: [4.5, 1.4], act: [2.5, 7], acts: { nod: 3, hop: 1, double: 1, wiggle: 1 } },
  sleep: { bob: 1.3, sway: 0.25, tilt: 0.6, breath: 0.03, pace: 0.42, halo: 0.18, blink: null, look: null, reach: [0, 0], act: [5, 14], acts: { settle: 3, twitch: 2, yawn: 1 }, z: [1.3, 3.4] },
};
export const NUM = ['bob', 'sway', 'tilt', 'breath', 'pace', 'halo'];

/** An act: how long it lasts and what it adds at time x in [0, 1] (y, x, rot in degrees, sx, sy, eye offset). */
export const ACTS = {
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
