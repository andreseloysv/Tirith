// A lane's card in the kanban: who works on it, the item, what it does now (or its question), and its footer.
import { html } from '../../../vendor/preact-htm.js';
import { MdInline } from '../boards/md.js';
import { intentOf } from '../../intent.js';
import { itemText, live, forYou } from './lanes.js';
import { Avatar, Clock, StopBtn } from './stop.js';
import { t } from '../../i18n.js';

/** The card's "now" line: the question for the owner, the tool call it runs, or its note when working or failed. */
function nowLine(a, doing, q, work) {
  return q ? html`<span class="anow ask"><i class="pdot2"></i>${forYou(q) ? html`<b>${t('For you:')} </b>` : null}<${MdInline} text=${q.question} /></span>`
    : work && doing ? html`<span class=${`anow${live(a) ? ' live' : ''}`}><i class="pdot2"></i><span class="md">${intentOf(doing)}…</span></span>`
    : (work || a.status === 'failed') && a.note ? html`<span class=${`anow${live(a) ? ' live' : ''}${a.status === 'failed' ? ' bad' : ''}`}><i class="pdot2"></i><${MdInline} text=${a.note} /></span>` : null;
}

export function LaneCard({ a, sel, color, projName, onOpen, doing, q }) {
  const work = a.status === 'running';
  const head = html`<span class="ahead"><${Avatar} a=${a} color=${color} /><span class="aname"><b title=${a.agent}>${a.agent || t('agent')}</b>
    <span class="pill" title=${a.id}>${a.model || a.id}</span></span></span>`;
  const now = nowLine(a, doing, q, work);
  return html`<button class=${`card acard${work ? ' work' : ''}${sel ? ' sel' : ''}`} onClick=${() => onOpen(a.id)}>
    ${head}<span class="afeat">${itemText(a.item)}</span>${now}
    <span class="afoot">${a.attempts ? html`<span class="chip atry" title=${t('Sent back by the merge checks')}>${t('Try {n}', { n: a.attempts + 1 })}</span>` : null}
      ${a.step && work ? html`<span class="chip astep" title=${t('Plan step')}>${t('Step {n}', { n: a.step })}</span>` : null}
      <span class="aproj"><i class="pdot" style=${`background:${color};animation:none`}></i>${projName}</span><${Clock} a=${a} /></span>
    ${work ? html`<${StopBtn} a=${a} where="card" />` : null}
  </button>`;
}
