// A message as the phone's chat draws it: a filled bubble with a tail at its top corner, who said it above the words
// (never the owner's own), and the time at the bottom right inside it. No face: a face only shows while it works.
import { html } from '../../../vendor/preact-htm.js';

/** `cls`: `mgr` (left) or `me` (right), and what kind; `meta`: what else goes beside the time. */
export function bubble({ key, cls, from, time, meta, body }) {
  return html`<div class=${`mc-m ${cls}`} key=${key}><div class="mc-b">
      ${from ? html`<span class="mc-from">${from}</span>` : null}
      ${body}
      <div class="mc-meta">${meta || null}<span class="mc-time">${time}</span></div>
    </div></div>`;
}

/** A message's drawn Markdown inside its bubble. */
export const words = (htmlText) => (htmlText ? html`<div class="mc-text" dangerouslySetInnerHTML=${{ __html: htmlText }} />` : null);
