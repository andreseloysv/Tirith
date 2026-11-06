// Overview (STUB, the overview agent fills it): the old page's .ovg grid — main column (health, plan, needs you,
// running pipelines, working agents, setup) and the manager chat in the right column.
import { html } from '../../vendor/preact-htm.js';
import { Chat } from './chat.js';

export function Overview({ project }) {
  return html`<div class="ovg"><div class="ovmain"><section><h2>Overview</h2></section></div>
    <aside class="ovside"><${Chat} project=${project} /></aside></div>`;
}
