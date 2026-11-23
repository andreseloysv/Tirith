// "Waiting for you" on the Home: every open question, newest first, each whole with its answer (ask.js).
import { html } from '../../../vendor/preact-htm.js';
import { Ask } from './ask.js';

export function Asks({ list, who }) {
  if (!list.length) return null;
  return html`<section class="hm-sec" aria-label="Waiting for you"><h2 class="hm-h">Waiting for you<span class="hm-n">${list.length}</span></h2>
    ${list.map((o) => html`<${Ask} key=${o.id} o=${o} w=${who(o.project)} />`)}</section>`;
}
