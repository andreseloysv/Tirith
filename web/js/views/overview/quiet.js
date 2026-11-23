// When nothing waits for the owner and nobody works: the agents asleep in a row, and two calm lines.
import { html } from '../../../vendor/preact-htm.js';
import { Face } from '../bots/face.js';

/** `faces`: [{id, shape, color}] — the agents of the sidebar. */
export function Quiet({ faces }) {
  return html`<div class="hm-quiet">
    <div class="hm-sleep" aria-hidden="true">${faces.slice(0, 5).map((f) => html`<${Face} key=${f.id} shape=${f.shape} color=${f.color} size=${56} state="sleep" />`)}</div>
    <h2>All quiet</h2>
    <p>Nothing needs you, and nobody is working right now. New questions and work show up here as soon as they come.</p>
  </div>`;
}
