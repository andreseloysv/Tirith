// The confirm popovers of a room: Invite another Tirith (its peer code), Forget a phone, and Leave.
import { html } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { CopyBtn } from '../settings/ui.js';
import { short, enc } from './bits.js';
import { t } from '../../i18n.js';

function invite(r, busy, run, closeBtn) {
  const key = `invite:${r.id}`;
  const fresh = () => run(key, () => api('POST', `/api/rooms/${enc(r.id)}/invite`));
  return html`<div class="acpop" role="dialog" aria-label=${t('Invite another NeoBot')}>
      <div><b>${t('For the NeoBot of another computer, not for a phone:')}</b> ${t('a phone scans the QR at the top of this page. The other NeoBot joins with this code (Join with a code, or "neobot room join") as a peer: what it writes is information, never an order.')}</div>
      ${r.peer_code ? html`<input class="rtitle rcode rpeer" type="text" readonly aria-label=${t('Code for another NeoBot')} value=${r.peer_code} onFocus=${(e) => e.target.select()} />
        <div class="apb"><${CopyBtn} text=${r.peer_code} label=${t('Copy code')} cls="abtn" /><button class="alink" disabled=${busy === key} onClick=${fresh}>${t('New code')}</button>${closeBtn}</div>`
      : html`<div class="apb rget"><button class="abtn" disabled=${busy === key} onClick=${fresh}>${busy === key ? t('Asking…') : t('Get a code')}</button>${closeBtn}</div>`}</div>`;
}

function forget(ask, busy, run, close, closeBtn) {
  const id = ask.slice(7);
  return html`<div class="acpop" role="dialog" aria-label=${t('Forget a phone')}><div>${t('Forget the phone {id}? It is disconnected from every project: it reads nothing more and what it writes is ignored. To connect it again, scan the QR with it.', { id: short(id) })}</div>
      <div class="apb"><button class="abtn" disabled=${busy === ask} onClick=${async () => { await run(ask, () => api('POST', '/api/rooms/forget', { key_id: id })); close(); }}>${busy === ask ? t('Forgetting…') : t('Forget')}</button>${closeBtn}</div></div>`;
}

function leave(r, busy, run, close, closeBtn) {
  const key = `leave:${r.id}`;
  return html`<div class="acpop" role="dialog" aria-label=${t('Leave')}><div>${t('Leave this room? NeoBot stops listening here and it goes from your phones. They stay connected to the other rooms (to disconnect a phone, use Forget), and a project left without a room gets a new one.')}</div>
    <div class="apb"><button class="abtn" disabled=${busy === key} onClick=${async () => { await run(key, () => api('POST', `/api/rooms/${enc(r.id)}/leave`)); close(); }}>${busy === key ? t('Leaving…') : t('Leave')}</button>${closeBtn}</div></div>`;
}

export function Popover({ r, ask, close, act }) {
  const [busy, , run] = act;
  const closeBtn = html`<button class="alink" onClick=${close}>${t('Close')}</button>`;
  if (ask === 'invite') return invite(r, busy, run, closeBtn);
  if (ask.startsWith('forget:')) return forget(ask, busy, run, close, closeBtn);
  return leave(r, busy, run, close, closeBtn);
}
