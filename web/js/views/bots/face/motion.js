// The 2D face's motion: easing curves and each state's mood (float, blinks, glances), and the small acts of sleep
// and of waking (awake, both faces play the ten of acts.js).
export const R = (a, b) => a + Math.random() * (b - a);
export const TAU = Math.PI * 2;
export const ease = (x) => (x < 0.5 ? 2 * x * x : 1 - (-2 * x + 2) ** 2 / 2);
export const bell = (x) => Math.sin(Math.PI * Math.min(1, Math.max(0, x)));
const hold = (x, a = 0.2) => (x < a ? ease(x / a) : x > 1 - a ? ease((1 - x) / a) : 1); /* in, stay, out */
export const pick = (w) => { let n = Math.random() * Object.values(w).reduce((s, v) => s + v, 0); for (const k in w) if ((n -= w[k]) <= 0) return k; return Object.keys(w)[0]; };

/** Each state: the float's size and pace, how often the eyes blink and move (seconds, and how far); asleep, its acts and their weights. */
export const MOOD = {
  idle: { bob: 2.6, sway: 0.9, tilt: 1.6, breath: 0.012, pace: 1, blink: [1.8, 6.5], look: [1.1, 4.2], reach: [4.5, 2.6] },
  watch: { bob: 2, sway: 0.6, tilt: 1, breath: 0.01, pace: 0.85, blink: [2.2, 7], look: [0.7, 2.6], reach: [5.5, 3.2] },
  work: { bob: 2.8, sway: 0.4, tilt: 0.7, breath: 0.016, pace: 1.7, blink: [2.6, 7.5], look: [0.22, 0.75], reach: [4.5, 1.4] },
  sleep: { bob: 1.3, sway: 0.25, tilt: 0.6, breath: 0.03, pace: 0.42, blink: null, look: null, reach: [0, 0], act: [5, 14], acts: { settle: 3, twitch: 2, yawn: 1 }, z: [1.3, 3.4] },
};
export const NUM = ['bob', 'sway', 'tilt', 'breath', 'pace'];

/** An act: how long it lasts and what it adds at time x in [0, 1] (y, x, rot in degrees, sx, sy). */
export const ACTS = {
  settle: () => ({ d: R(1.6, 2.4), f: (x) => ({ sy: 1 - 0.05 * bell(x), sx: 1 + 0.035 * bell(x) }) }),
  twitch: () => ({ d: R(0.35, 0.5), f: (x) => ({ rot: 2.5 * Math.sin(x * TAU * 1.5) * (1 - x), y: -0.8 * bell(x) }) }),
  yawn: () => ({ d: R(1.8, 2.6), f: (x) => ({ sy: 1 + 0.08 * hold(x, 0.4), sx: 1 - 0.03 * hold(x, 0.4), rot: -2 * hold(x, 0.4) }) }),
  wake: () => ({ d: 0.7, f: (x) => ({ y: -6 * bell(x), sy: 1 + 0.04 * bell(x) }) }),
};
