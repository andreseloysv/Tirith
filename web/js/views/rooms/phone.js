// The owner's phone card: the QR of a room and the phones connected, or, before the first room, Show the QR.
import { html, useState, useEffect, useRef } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { CopyBtn } from '../settings/ui.js';
import { enc, Status, AgentFace, Who, useRoomAction } from './bits.js';
import { Popover } from './popover.js';

function scan(r, shown, busy, error, make, setBad, forget, title) {
  // the url is part of the src so a new code draws a new picture
  const src = `/api/rooms/${enc(r.id)}/qr.svg?kind=invite&v=${enc(r.invite || '')}`;
  return html`<div class="rscan">
      ${shown ? html`<img class="qr" src=${src} alt="QR code that connects a phone to OpenBot" width="232" height="232" onError=${() => setBad(r.invite)} />`
        : html`<div class="qr qrwait">${busy === 'qr' || !error ? 'Making the QR…' : html`<button class="abtn" onClick=${make}>Show the QR</button>`}</div>`}
      <div class="rhow">
        <p class="rlead">Scan this QR with the phone's camera.</p>
        <p>It opens ${title} on the phone and gives it a chat with the manager of every project, also of those you add later. Encrypted, with nothing to install.</p>
        <p class="mut sm">It works once, and the next one appears here by itself. Scan it again whenever the phone says it is not connected: a browser forgets its chats after some days without use.${r.invite ? html` <${CopyBtn} text=${r.invite} label="Copy it as a link" />` : null}</p>
        <${Who} r=${r} phones=${true} forget=${forget} />
      </div>
    </div>`;
}

/** The owner's phone: the QR of `r` (the selected project's room, or the room of no project) and the phones that are
    connected. The QR works once; when it is used, or none was made yet, the next one is asked for without a click. */
export function PhoneCard({ r, reload, title, agent }) {
  const act = useRoomAction(reload);
  const [busy, error, run] = act;
  const [ask, setAsk] = useState('');
  const asked = useRef(0);
  const [bad, setBad] = useState('');   // the code whose picture did not load
  const make = () => run('qr', () => api('POST', `/api/rooms/${enc(r.id)}/qr`));
  const shown = r.invite && bad !== r.invite ? r.invite : '';
  // Looked at on every draw (the rooms are read again at least every 15 s): no QR to show, so a new one is asked for, at most every 10 s.
  useEffect(() => {
    if (shown) asked.current = 0;
    else if (Date.now() - asked.current > 10000) { asked.current = Date.now(); make(); }
  });
  return html`<div class="card scard rphone" data-room=${r.id}>
    <div class="phead"><${AgentFace} a=${agent} /><b>Your phone</b><span class="chip">Encrypted</span><${Status} r=${r} /></div>
    ${r.error ? html`<div class="ierr rerr">OpenBot cannot reach agentsmessenger.com now, so a phone connects when it can again: ${r.error}</div>` : null}
    ${scan(r, shown, busy, error, make, setBad, (id) => setAsk(ask === id ? '' : id), title)}
    ${ask ? html`<${Popover} r=${r} ask=${`forget:${ask}`} close=${() => setAsk('')} act=${act} />` : null}
    ${error ? html`<div class="ierr">${error}</div>` : null}
  </div>`;
}

/** Before the first room: Tirith opens rooms on the platform only when the owner asks, so the first QR takes one click. */
export function NoRoom({ reload, project, agent }) {
  const [busy, error, run] = useRoomAction(reload);
  return html`<div class="card scard rphone">
    <div class="phead"><${AgentFace} a=${agent} /><b>Your phone</b><span class="chip">Encrypted</span></div>
    <p class="rlead">Work from your phone.</p>
    <p>OpenBot opens an encrypted room on agentsmessenger.com and shows a QR here: scan it with the phone's camera and the phone has a chat with the manager of every project, with nothing to install.</p>
    <div class="sfoot"><button class="isend" disabled=${busy === 'new'} onClick=${() => run('new', () => api('POST', '/api/rooms', { title: '', project: project || '' }))}>${busy === 'new' ? 'Making the room…' : 'Show the QR'}</button></div>
    ${error ? html`<div class="ierr">${error}</div>` : null}
  </div>`;
}
