// Each colour's personality (the owner, 2026-11-23: «darle a cada uno una personalidad diferente por color»): how fast it
// moves (tempo), how big (energy), how often it plays an act (gap, a factor on the time between two), which acts it
// likes (weights; one it does not name has 1, 0 never comes), and how often it glances (look) and blinks (blink), both
// factors on the time between two. A colour that is not one of the page's own takes its nearest one's.
const P = (tempo, energy, gap, look, blink, likes) => ({ tempo, energy, gap, look, blink, likes });

export const PERSONAS = {
  '#e5484d': P(1.35, 1.3, 0.6, 0.8, 1, { hop: 3, spin: 2, roll: 2, reading: 0, yawn: 0 }),          /* red: energetic */
  '#f07a3a': P(1.2, 1.2, 0.75, 0.9, 1, { roll: 3, spin: 2, wink: 2, hop: 2, yawn: 0 }),             /* orange: playful */
  '#f2a93b': P(1.1, 1.15, 0.85, 0.9, 0.9, { hop: 3, doubleBlink: 2, curious: 2 }),                  /* amber: cheerful */
  '#4fb86e': P(1, 1, 0.9, 0.65, 1, { curious: 3, lookAround: 3, orbit: 2, spin: 0 }),              /* green: curious */
  '#3fb6a8': P(0.9, 0.85, 1.2, 1.1, 1.1, { reading: 3, doubleBlink: 2, spin: 0, roll: 0 }),        /* teal: focused */
  '#3b82f6': P(0.8, 0.8, 1.4, 1.2, 1.2, { lookAround: 2, reading: 2, orbit: 2, hop: 0 }),          /* blue: thoughtful */
  '#8b5cf6': P(0.7, 0.9, 1.3, 1.3, 1.3, { orbit: 3, yawn: 2, roll: 1, hop: 0 }),                   /* purple: dreamy */
  '#ec4899': P(1.25, 1.25, 0.7, 0.85, 0.75, { wink: 3, hop: 2, doubleBlink: 2, reading: 0 }),     /* pink: bubbly */
  '#f2f2f2': P(0.75, 0.7, 1.5, 1.3, 1.2, { doubleBlink: 2, lookAround: 2, spin: 0, roll: 0 }),     /* white: serene */
  '#9a6b45': P(0.85, 0.75, 1.3, 1.2, 1.2, { reading: 2, yawn: 2, spin: 0 }),                       /* brown: steady */
  '#8c8c8c': P(0.9, 0.7, 1.6, 1.5, 1, { curious: 2, doubleBlink: 2, spin: 0, roll: 0, hop: 0 }),  /* grey: shy */
};

const rgb = (hex) => { const n = parseInt(String(hex).slice(1, 7), 16) || 0; return [n >> 16, (n >> 8) & 255, n & 255]; };
const far = (a, b) => rgb(a).reduce((s, v, i) => s + (v - rgb(b)[i]) ** 2, 0);

/** The personality of colour `hex`: its own, or the nearest colour's. */
export function personaOf(hex) {
  const key = String(hex || '').toLowerCase();
  if (PERSONAS[key]) return PERSONAS[key];
  return PERSONAS[Object.keys(PERSONAS).reduce((best, k) => (far(k, key) < far(best, key) ? k : best))];
}
