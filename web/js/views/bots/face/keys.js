// Keyframes of the faces' acts: a track is a list of [time 0..1, value, ease into it], or a function of the time.
const { PI, cos, exp } = Math;
const BACK = 1.9; /* how far `back` overshoots before it settles */

export const EASE = {
  soft: (x) => (x < 0.5 ? 4 * x * x * x : 1 - (-2 * x + 2) ** 3 / 2),
  sine: (x) => (1 - cos(PI * x)) / 2, /* turns: never faster than the eye can follow */
  fall: (x) => x * x,
  out: (x) => 1 - (1 - x) ** 3,
  back: (x) => 1 + (BACK + 1) * (x - 1) ** 3 + BACK * (x - 1) ** 2,
  spring: (x) => (x >= 1 ? 1 : 1 - exp(-6 * x) * cos(9 * x)),
};

/** The track's value at `x`; before its first key and after its last it holds. */
export function valueAt(track, x) {
  if (typeof track === 'function') return track(x);
  if (x <= track[0][0]) return track[0][1];
  for (let i = 1; i < track.length; i++) {
    const [t, v, ease = 'soft'] = track[i];
    if (x > t) continue;
    const [from, was] = track[i - 1];
    return was + (v - was) * EASE[ease]((x - from) / (t - from));
  }
  return track[track.length - 1][1];
}
