// Where a 3D face's eyes sit in each state of its bot, after the lifecycle of the article the owner pointed to
// (x.ai, «Designing Grok Bot», 2026-11-23: calm and slightly curious at rest, turned to the task at work, its eyes
// dashes while it waits): at rest its shape's own pose, up and to the right; at work drawn towards the middle, more
// upright and a little lower; asleep shut into dashes along the pair. A new state's pose blends in over a moment.
export const MOODS = {
  idle: { pull: 0, down: 0, upright: 0, shut: 0 },
  watch: { pull: 0, down: 0, upright: 0, shut: 0 },
  work: { pull: 0.35, down: 5, upright: 0.55, shut: 0 },
  sleep: { pull: 0.12, down: 4, upright: 0, shut: 1 },
};
const KEYS = Object.keys(MOODS.idle);

/** A pose to start from: `mood`'s own. */
export const lookOf = (mood) => ({ ...(MOODS[mood] || MOODS.idle) });

/** `look` eased `dt` seconds towards `mood`'s pose (dt 1 or more: there at once). */
export function easeLook(look, mood, dt) {
  const to = MOODS[mood] || MOODS.idle, k = Math.min(1, dt * 5);
  for (const key of KEYS) look[key] += (to[key] - look[key]) * k;
  return look;
}
