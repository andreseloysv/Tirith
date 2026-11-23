// "Working now" on the Home, and nothing more: how many helpers work and, for each boss, a tree (tree.js) of its face
// and its helpers' faces, each with the title of what it does. A bot or manager at work on its own is one row that
// says what it is doing, and opens its chat.
import { html } from '../../../vendor/preact-htm.js';
import { Face } from '../bots/face.js';
import { Tree } from './tree.js';

const many = (n, one, more) => `${n} ${n === 1 ? one : more}`;

/* a boss with no helpers: its face at work, its name and what it does */
function Alone({ g, w }) {
  return html`<div class="hm-crew">
    <button type="button" class="hm-boss" onClick=${w.open} title="Open the chat">
      <span class="hm-face"><${Face} shape=${w.face.shape} color=${w.face.color} size=${44} state="work" /></span>
      <span class="hm-bn"><b>${w.name}</b>${w.place ? html`<span>${w.place}</span>` : null}</span>
      <span class="hm-does">${g.doing}…</span></button>
  </div>`;
}

/** `groups`: crew.js's answer; `who`: who.js's. */
export function Working({ groups, who }) {
  if (!groups.length) return null;
  const n = groups.reduce((a, g) => a + g.helpers.length, 0);
  return html`<section class="hm-sec" aria-label="Working now"><h2 class="hm-h">Working now${n ? html`<span class="hm-n">${many(n, 'helper', 'helpers')}</span>` : null}</h2>
    ${groups.map((g) => (g.helpers.length ? html`<${Tree} key=${g.key} g=${g} w=${who(g.key)} />` : html`<${Alone} key=${g.key} g=${g} w=${who(g.key)} />`))}</section>`;
}
