// One step of a plan card: its row (icon, title, bar, count, Add item) and, unfolded, its items and the Add row.
import { html } from '../../../../vendor/preact-htm.js';
import { ACTIVE } from '../../../common.js';
import { Svg, IC, PLUS, hit } from '../bits.js';
import { Bar, Item } from './items.js';
import { AddItem } from './add.js';
import { t } from '../../../i18n.js';

function row(s, open, live, onToggle, setAdding) {
  return html`<div class="prow">
      <button class="ptog" aria-expanded=${open} onClick=${onToggle}>
        <${Svg} cls=${`pico${live ? ' pulse' : ''}`} s=${IC[s.status] || IC.todo} />
        <span class="ptit"><span class="pn">${s.num}.</span> ${s.title}</span>
        ${s.total ? html`<${Bar} done=${s.done} total=${s.total} cls="pbar" />` : html`<span class="pbar"></span>`}
        <span class="pcnt">${s.total ? `${s.done}/${s.total}` : '–'}</span>
      </button>
      <span class="pedtools pstit"><button class="pedbtn" title=${t('Add item')} aria-label=${t('Add item')} onClick=${() => setAdding(s.num)}><${Svg} s=${PLUS} /></button></span>
    </div>`;
}

export function Step({ project, s, mode, onToggle, laneOf, q, adding, setAdding }) {
  const items = q ? s.items.filter((i) => hit(q, i.text, s.title)) : s.items;
  const live = s.items.some((i) => !i.done && laneOf(i) && ACTIVE.includes(laneOf(i).status));
  const shown = mode === 'all' ? items : mode === 'quiet' ? items.filter((i) => !i.done) : [];
  const open = mode !== 'closed';
  const add = html`<${AddItem} project=${project} step=${s.num} editing=${adding} setEditing=${(on) => setAdding(on ? s.num : null)} />`;
  return html`<li class=${`pstep ${s.status}${mode === 'all' ? ' open' : ''}`}>
    ${row(s, open, live, onToggle, setAdding)}
    ${shown.length || adding ? html`<ul class=${`pitems${mode === 'quiet' ? ' pquiet' : ''}`}>
      ${shown.map((i) => html`<${Item} key=${i.text} i=${i} lane=${laneOf(i)} />`)}
      ${open || adding ? add : null}
    </ul>` : open && !s.total ? html`<ul class="pitems"><li class="none">${t('No items yet')}</li>
      ${add}</ul>` : null}
  </li>`;
}
