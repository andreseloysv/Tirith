// One open question for the owner with its answer box: a card the manager sent with tirith ask-owner, or a lane
// waiting on its question. The same card shows in "Needs you", in the chat where it was asked and on the lane whose
// question it carries (a card made with --lane), so the owner can answer it wherever they are looking.
import { html, useState } from '../../vendor/preact-htm.js';
import { api, setUi } from '../store.js';
import { ago, go } from '../common.js';

export const KIND = { credentials: 'Credentials', payment: 'Payment', account: 'Account', device: 'Device', decision: 'Decision', lane: 'Agent' };
const WORDS = ['decision', 'lane']; /* answered in words only: no Done button */

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
    answered card, drawn without its answer box; `here`: drawn on its own lane, so without the link to it. */
export function Need({ o, p, full, done, here }) {
  const [draft, setDraft] = useState('');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  const send = async (answer) => {
    answer = String(answer || '').trim();
    if (!answer || busy) return;
    setBusy(true); setErr('');
    try { await api('POST', `/api/owner/${encodeURIComponent(o.id)}`, { answer }); setDraft(''); } catch (e) { setErr(String(e.message || e)); }
    setBusy(false);
  };
  const words = WORDS.includes(o.kind) || !!o.lane; /* a lane's question wants words, not Done */
  const lane = o.lane && !here ? html`<button type="button" class="nlane" onClick=${() => { setUi('openLane', o.lane); go('agents', o.project); }}
    title="Open the agent this question is for">${o.kind === 'lane' ? 'An agent asks the manager' : 'For an agent'}${o.item ? `: ${o.item.slice(0, 90)}${o.item.length > 90 ? '…' : ''}` : ''}</button>` : null;
  return html`<li class=${`ncard${done ? ' done' : ''}`}>
    <div class="nhead"><div class=${`ntitle${full ? ' open' : ''}`}>${o.question}</div>${p ? html`<span class="nchip nproj"><i class="pdot" style=${`background:${p.color || '#9ca3af'}`}></i>${p.name || p.id}</span>` : null}
      <span class="nchip">${done ? 'Answered' : KIND[o.kind] || o.kind}</span>${done ? null : html`<span class="nage">${ago(o.at)}</span>`}</div>
    ${lane}
    ${done || words ? null : html`<div class="nacts"><button class="nbtn primary" disabled=${busy} onClick=${() => send('Done')}>Done</button></div>`}
    ${done ? null : html`<div class="nwrite"><input class="iin" type="text" placeholder=${o.kind === 'lane' ? 'Answer it yourself…' : words ? 'Your answer…' : 'Or write what you need…'} aria-label="Answer"
      value=${draft} disabled=${busy} onInput=${(e) => setDraft(e.target.value)} onKeyDown=${(e) => { if (e.key === 'Enter') send(draft); }} />
      <button class="isend ovsend" disabled=${busy || !draft.trim()} onClick=${() => send(draft)}>${busy ? 'Sending…' : 'Answer'}</button></div>`}
    ${err ? html`<div class="ierr">${err}</div>` : null}
  </li>`;
}
