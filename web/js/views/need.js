// One open question for the owner with its answer box: a card the manager sent with neobot ask-owner, or a lane
// waiting on its question. The same card shows in "Needs you", in the chat where it was asked and on the lane whose
// question it carries (a card made with --lane), so the owner can answer it wherever they are looking.
import { html, useState } from '../../vendor/preact-htm.js';
import { setUi } from '../store.js';
import { agoText, go } from '../common.js';
import { useAnswer, WORDS } from './answer.js';
import { t } from '../i18n.js';

const kind = (k) => ({ credentials: t('Credentials'), payment: t('Payment'), account: t('Account'), device: t('Device'), decision: t('Decision'), approval: t('Approval'), lane: t('Agent') })[k];
const doneWord = (d) => (d === 'Answered' ? t('Answered') : d === 'Closed' ? t('Closed') : d);

/** The open questions: the daemon's cards, and the lanes waiting on a question that no card carries (read live from
    the lanes, so one appears and goes with its lane, not only when the page loads). */
export function openCards(s) {
  const cards = s.owner.filter((o) => o.kind !== 'lane').map((o) => (o.lane && s.lanes[o.lane] ? { ...o, item: s.lanes[o.lane].item } : o));
  const carried = new Set(cards.map((o) => o.lane).filter(Boolean));
  const lanes = Object.values(s.lanes).filter((l) => l.status === 'waiting' && l.question && !carried.has(l.id))
    .map((l) => ({ id: l.id, at: l.created_at, kind: 'lane', project: l.project, status: 'open', question: l.question, lane: l.id, item: l.item }));
  return cards.concat(lanes);
}

/** `p`: the project chip (only where several projects show); `full`: the whole question, not two lines; `done`: an
    answered or closed card (its label: 'Answered', 'Closed', else 'Done'), drawn without its answer box; `here`: drawn on its own lane, so without the link to it. */
export function Need({ o, p, full, done, here }) {
  const [draft, setDraft] = useState('');
  const { busy, err, send: answer, close } = useAnswer(o); /* close: a card that no longer applies; its chat says so */
  const send = async (text) => { if (await answer(text)) setDraft(''); };
  const words = WORDS.includes(o.kind) || !!o.lane; /* a lane's question wants words, not Done */
  const lane = o.lane && !here ? html`<button type="button" class="nlane" onClick=${() => { setUi('openLane', o.lane); go('agents', o.project); }}
    title=${t('Open the agent this question is for')}>${o.kind === 'lane' ? t('An agent asks the manager') : t('For an agent')}${o.item ? `: ${o.item.slice(0, 90)}${o.item.length > 90 ? '…' : ''}` : ''}</button>` : null;
  return html`<li class=${`ncard${done ? ' done' : ''}`}>
    <div class="nhead"><div class=${`ntitle${full ? ' open' : ''}`}>${o.question}</div>${p ? html`<span class="nchip nproj"><i class="pdot" style=${`background:${p.color || '#9ca3af'}`}></i>${p.name || p.id}</span>` : null}
      <span class="nchip">${done ? (typeof done === 'string' ? doneWord(done) : t('Done')) : kind(o.kind) || o.kind}</span>${done ? null : html`<span class="nage">${agoText(o.at)}</span>`}
      ${done || o.kind === 'lane' ? null : html`<button type="button" class="nclose" disabled=${busy} title=${t('Close this card: it is no longer needed')} aria-label=${t('Close this card')} onClick=${close}>×</button>`}</div>
    ${lane}
    ${done || words ? null : o.kind === 'approval' /* a yes or a no: a connector call runs once only on the yes */
      ? html`<div class="nacts"><button class="nbtn primary" disabled=${busy} onClick=${() => send('yes')}>${t('Yes')}</button><button class="nbtn" disabled=${busy} onClick=${() => send('no')}>${t('No')}</button></div>`
      : html`<div class="nacts"><button class="nbtn primary" disabled=${busy} onClick=${() => send('Done')}>${t('Done')}</button></div>`}
    ${done ? null : html`<div class="nwrite"><input class="iin" type="text" placeholder=${o.kind === 'lane' ? t('Answer it yourself…') : words ? t('Your answer…') : t('Or write what you need…')} aria-label=${t('Answer')}
      value=${draft} disabled=${busy} onInput=${(e) => setDraft(e.target.value)} onKeyDown=${(e) => { if (e.key === 'Enter') send(draft); }} />
      <button class="isend ovsend" disabled=${busy || !draft.trim()} onClick=${() => send(draft)}>${busy ? t('Sending…') : t('Answer')}</button></div>`}
    ${err ? html`<div class="ierr">${err}</div>` : null}
  </li>`;
}
