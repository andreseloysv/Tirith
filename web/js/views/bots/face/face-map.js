// How a bot of this house is said to the vendored face (vendor/botface/): each shape the picker knows has one
// of the face's own outlines, nearest in kind and each its own so the picker never shows two alike; each state
// the page already speaks (work, watch, sleep, idle, from state.js and speaker.js) has its face state, and the
// face's own states pass through untouched. What a running turn is doing, in intent.js's plain words, and how
// a finished turn ended (turns `status`: ok, failed, waiting) have theirs here too.
import { SHAPE_IDS, STATE_IDS } from '../../../../vendor/botface/catalog.js';

const SHAPE_OF = {
  round: 'blob', cloud: 'cloud', tile: 'squircle', star: 'crystal', clover: 'leaf', blob: 'pebble',
  flower: 'egg', drop: 'teardrop', pill: 'capsule', tri: 'wedge', dome: 'dome', burst: 'gem', hex: 'hex', hero: 'shield',
};
const STATE_OF = { work: 'working', watch: 'listening', sleep: 'sleeping', idle: 'idle' };

/* intent.js's words for a step of a running turn */
const DOING = {
  Thinking: 'thinking', 'Planning the work': 'thinking',
  Reading: 'searching', 'Looking on the web': 'searching', 'Looking at the code': 'searching', 'Checking on the agents': 'searching',
  Writing: 'writing', 'Writing a question for you': 'writing', 'Writing in a room': 'writing',
  'Running checks': 'working', 'Setting up a routine': 'working', 'Using a skill': 'working', Working: 'working',
  'Asking a helper': 'sending',
};

/** The face's outline for a shape: its own shapes pass through exact; one of this house's older shapes (a bot
    saved before the face's shapes were the picker's) lands on its nearest; an unknown one is the blob. */
export const faceShape = (shape) => (SHAPE_IDS.includes(shape) ? shape : SHAPE_OF[shape] || 'blob');

/** A bot's (or a role's face's) whole look as Face takes it, defaults filled: solid in its colour, ink eyes. */
export const lookOf = (x = {}) => ({
  shape: x.shape || 'round', color: x.color || '#3b82f6', material: x.material || 'solid',
  eye: x.eye || '#161616', gradient: x.gradient || '', gstart: x.gstart || '', gend: x.gend || '',
  gangle: typeof x.gangle === 'number' ? x.gangle : null, glass: x.glass || '',
});

/** The face's state for a state of this house, or the state itself when the face already knows it. */
export const faceState = (state) => STATE_OF[state] || (STATE_IDS.includes(state) ? state : 'idle');

/** The face's state while a turn does `words` (intent.js); anything unlisted is plain work. */
export const stateForIntent = (words) => DOING[words] || 'working';

/** How a face answers the end of a turn: celebrate a good one, mourn a failed one, nothing yet for the rest. */
export const reactionFor = (status) => (status === 'ok' ? 'celebrate' : status === 'failed' ? 'sad' : null);

/** Both catalogues are the vendor's own, re-exported so callers learn them from one place. */
export { SHAPE_IDS, STATE_IDS };
