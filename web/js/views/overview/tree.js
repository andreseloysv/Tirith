// A boss and its helpers as a tree on the Home, as in the owner's reference of 2026-11-23: the boss's face on top with
// its name, lines down (lines.js) to its helpers, each with a face of its own (kin.js), the title of what it does and,
// only when it is not simply working, its state in words. A helper opens its details in Agents; the boss its chat.
import { html, useRef } from '../../../vendor/preact-htm.js';
import { setUi } from '../../store.js';
import { go } from '../../common.js';
import { Face, SHAPES, COLORS } from '../bots/face.js';
import { say } from './crew.js';
import { helperFaces } from './kin.js';
import { useLines } from './lines.js';
import { t } from '../../i18n.js';

const AT_WORK = ['running', 'merging']; /* the rest wait: their face only watches */

function Kid({ h, f }) {
  const open = () => { setUi('openLane', h.id); go('agents', h.project); };
  return html`<button type="button" class=${`hm-kid s-${h.state}`} onClick=${open} title=${h.title}>
    <span class="hm-face"><${Face} shape=${f.shape} color=${f.color} size=${40} state=${AT_WORK.includes(h.state) ? 'work' : 'watch'} /></span>
    <span class="hm-ktx"><span class="hm-kt">${h.title}</span>${say(h.state) ? html`<em>${say(h.state)}</em>` : null}</span></button>`;
}

/** `g`: one group of crew.js (with helpers); `w`: its boss, as who.js tells it. */
export function Tree({ g, w }) {
  const ref = useRef(null);
  const ids = g.helpers.map((h) => h.id);
  const faces = helperFaces(ids, w.face, Object.keys(SHAPES), COLORS);
  const d = useLines(ref, ids.join(','));
  return html`<div class="hm-crew hm-tree" ref=${ref}>
    <svg class="hm-lines" aria-hidden="true"><path d=${d} /></svg>
    <button type="button" class="hm-tboss" onClick=${w.open} title=${t('Open the chat')}>
      <span class="hm-face"><${Face} shape=${w.face.shape} color=${w.face.color} size=${52} state="work" /></span>
      <span class="hm-tname"><b>${w.name}</b>${w.place ? html` <span>${w.place}</span>` : null}</span></button>
    <div class="hm-kids">${g.helpers.map((h, i) => html`<${Kid} key=${h.id} h=${h} f=${faces[i]} />`)}</div>
  </div>`;
}
