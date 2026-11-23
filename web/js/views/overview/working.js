// "Working now" on the Home, and nothing more: how many helpers work, each boss with its face at work, and under it
// the title of what each of its helpers does (its state in words when it is not simply working). A bot or manager
// at work on its own says what it is doing. A helper opens its details in Agents; a boss opens its chat.
import { html } from '../../../vendor/preact-htm.js';
import { setUi } from '../../store.js';
import { go } from '../../common.js';
import { Face } from '../bots/face.js';
import { SAY } from './crew.js';

const many = (n, one, more) => `${n} ${n === 1 ? one : more}`;

function Helper({ h }) {
  const open = () => { setUi('openLane', h.id); go('agents', h.project); };
  return html`<li><button type="button" class=${`hm-help s-${h.state}`} onClick=${open} title="See what it does">
    <i aria-hidden="true"></i><span>${h.title}</span>${SAY[h.state] ? html`<em>${SAY[h.state]}</em>` : null}</button></li>`;
}

function Crew({ g, w }) {
  const n = g.helpers.length;
  return html`<div class="hm-crew">
    <button type="button" class="hm-boss" onClick=${w.open} title="Open the chat">
      <span class="hm-face"><${Face} shape=${w.face.shape} color=${w.face.color} size=${44} state="work" /></span>
      <span class="hm-bn"><b>${w.name}</b>${w.place ? html`<span>${w.place}</span>` : null}</span>
      <span class="hm-does">${n ? many(n, 'helper', 'helpers') : `${g.doing}…`}</span></button>
    ${n ? html`<ul class="hm-helpers">${g.helpers.map((h) => html`<${Helper} key=${h.id} h=${h} />`)}</ul>` : null}
  </div>`;
}

/** `groups`: crew.js's answer; `who`: who.js's. */
export function Working({ groups, who }) {
  if (!groups.length) return null;
  const n = groups.reduce((a, g) => a + g.helpers.length, 0);
  return html`<section class="hm-sec" aria-label="Working now"><h2 class="hm-h">Working now${n ? html`<span class="hm-n">${many(n, 'helper', 'helpers')}</span>` : null}</h2>
    ${groups.map((g) => html`<${Crew} key=${g.key} g=${g} w=${who(g.key)} />`)}</section>`;
}
