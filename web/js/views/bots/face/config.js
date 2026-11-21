// The 3D bots' numbers, the reference's: the inflation P and depth of the body, pixels per box unit, the drawn
// square's margin for the halo and its largest size; and ?freeze (with yaw, pitch, mood) for still pictures.
export const P = 2.35, DEPTH = 0.62, PX = 512 / 150, MARGIN = 1.7, CAP = 820; // the drawn square is the bot's box grown for the halo
const Q3 = typeof location !== 'undefined' ? new URLSearchParams(location.search) : new URLSearchParams();
export const FREEZE = Q3.has('freeze') ? { yaw: Number(Q3.get('yaw') || 0), pitch: Number(Q3.get('pitch') || 0), mood: Q3.get('mood') } : null; // still pictures to compare against
