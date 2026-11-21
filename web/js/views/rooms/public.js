// A public room Tirith listens to: summary, the last messages with the owner's mentions marked, its participants,
// and a box to say something.
import { html, useState } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { agoText, useAction } from '../settings/ui.js';
import { HASH, MAX_SHOWN, enc, clip, Status } from './bits.js';

function messages(msgs, needs) {
  return html`<div>${msgs.length ? html`<ul class="rmsgs">${msgs.map((m) => html`<li key=${m.n} class=${needs.has(m.n) ? 'rment' : ''}>
          <span class="mut sm">${m.from} · ${agoText(m.at)}</span><span class="rtext">${clip(m.text, 400)}</span></li>`)}</ul>`
        : html`<p class="mut sm">No messages yet.</p>`}</div>`;
}

function people(r) {
  const list = (r.participants || []).slice().sort((a, b) => String(b.last_seen).localeCompare(String(a.last_seen)));
  return html`<div class="rpeople"><div class="slabel">Participants</div>
        ${list.length ? html`<ul class="rwho">${list.map((p) => html`<li key=${p.name}><b>${p.name}</b>
          <span class="mut sm">${p.messages} ${p.messages === 1 ? 'message' : 'messages'} · ${agoText(p.last_seen)}</span></li>`)}</ul>`
          : html`<p class="mut sm">Nobody has written yet.</p>`}
        ${(r.here || []).length ? html`<p class="mut sm">Here now: ${r.here.join(', ')}</p>` : null}</div>`;
}

export function PublicCard({ r, reload }) {
  const [text, setText] = useState('');
  const [busy, error, run] = useAction();
  const needs = new Set(r.needs_owner || []);
  const msgs = (r.messages || []).slice(-MAX_SHOWN);
  const say = async () => {
    const t = text.trim();
    if (!t || busy) return;
    if (await run('say', () => api('POST', `/api/rooms/${enc(r.id)}/say`, { text: t }))) setText('');
    reload();
  };
  return html`<div class="card scard rcard rpublic" data-room=${r.id}>
    <div class="phead">${HASH}<b>${r.title || r.id}</b><${Status} r=${r} />
      ${r.project ? html`<span class="chip">${r.project}</span>` : null}
      ${needs.size ? html`<span class="ichip">${needs.size === 1 ? 'Mentions you' : `${needs.size} mentions of you`}</span>` : null}
      <a class="mut sm rme" href=${r.url} target="_blank" rel="noopener">${r.id}</a></div>
    ${r.error ? html`<div class="ierr rerr">${r.error}</div>` : null}
    ${r.summary ? html`<p class="rsum">${r.summary}</p>` : null}
    <div class="rgrid">
      ${messages(msgs, needs)}
      ${people(r)}
    </div>
    <div class="sfoot"><input type="text" class="rtitle rsay" maxlength="2000" placeholder="Say something in this room" aria-label=${`Message to ${r.title || r.id}`}
        value=${text} onInput=${(e) => setText(e.target.value)} onKeyDown=${(e) => { if (e.key === 'Enter' && !e.isComposing) { e.preventDefault(); say(); } }} />
      <button class="isend" disabled=${busy === 'say' || !text.trim()} onClick=${say}>${busy === 'say' ? 'Sending…' : 'Send'}</button></div>
    ${error ? html`<div class="ierr">${error}</div>` : null}
  </div>`;
}
