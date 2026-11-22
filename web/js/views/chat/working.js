// While a turn runs: the bot's own face, alive, beside what it is doing in plain words with a soft light sweeping
// through them (intent.js), or beside the phone's three dots until there is anything to say. It is one row for the
// whole turn, so the face never starts again as the steps change; the written message takes its place.
import { html } from '../../../vendor/preact-htm.js';
import { intentOf } from '../../intent.js';
import { Face } from '../bots/face.js';

const SIZE = 56; /* the box: the body in it is near 40 px, the rest is for its glow and its hop */

/* the lit copy of the words slides inside a soft window that slides the other way: only transforms move */
function doing(text) {
  return html`<span class="mc-doing"><span class="mc-words">${text}</span><span class="mc-sweep" aria-hidden="true"><span>${text}</span></span></span>`;
}

const DOTS = html`<span class="mc-dots" aria-hidden="true"><i></i><i></i><i></i></span>`;

/** `step`: the turn's newest row; none before the first. */
export function Working({ who, face, step }) {
  const text = step ? intentOf(step) : '';
  return html`<div class="mc-work" role="status" aria-live="polite">
    ${face && face.shape ? html`<span class="mc-wface"><${Face} shape=${face.shape} color=${face.color || '#3b82f6'} size=${SIZE} state="work" lively /></span>` : null}
    <span class="mc-sr">${text ? `${who}: ` : `${who} is typing`}</span>
    ${text ? doing(text) : DOTS}
  </div>`;
}
