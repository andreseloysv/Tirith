// A project's encrypted room (who else is in it, the last message, Invite another Tirith, Leave), and the card
// that joins the room of another Tirith with its code.
import { html, useState } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { short, Status, AgentFace, Who, Last, useRoomAction } from './bits.js';
import { Popover } from './popover.js';

export function PrivateCard({ r, reload, title, agent }) {
  const [ask, setAsk] = useState('');
  const act = useRoomAction(reload);
  const toggle = (k) => setAsk(ask === k ? '' : k);
  return html`<div class="card scard rcard" data-room=${r.id}>
    <div class="phead"><${AgentFace} a=${agent} /><b>${title}</b><span class="chip">Encrypted</span><${Status} r=${r} />
      ${r.kind === 'peer' ? html`<span class="chip">peer</span>` : null}
      ${r.me && r.me.name ? html`<span class="mut sm rme" title="Your key id here">you: ${r.me.name} <code>${short(r.me.key_id)}</code></span>` : null}</div>
    ${r.error ? html`<div class="ierr rerr">${r.error}</div>` : null}
    ${r.braked ? html`<div class="icli rbrake">The manager stopped answering here: too many messages in a row came from agents. It answers again when you write from a paired phone.</div>` : null}
    <${Who} r=${r} /><${Last} r=${r} />
    <div class="sfoot">
      ${r.kind === 'manager' ? html`<button class=${`sadd${ask === 'invite' ? ' on' : ''}`} onClick=${() => toggle('invite')}>Invite another OpenBot</button>` : null}
      <button class=${`sadd rleave${ask === 'leave' ? ' on' : ''}`} onClick=${() => toggle('leave')}>Leave</button></div>
    ${ask ? html`<${Popover} r=${r} ask=${ask} close=${() => setAsk('')} act=${act} />` : null}
    ${act[1] ? html`<div class="ierr">${act[1]}</div>` : null}
  </div>`;
}

export function JoinCard({ reload, project }) {
  const [code, setCode] = useState('');
  const [busy, error, run] = useRoomAction(reload);
  const join = async () => {
    if (busy || !code.trim()) return;
    if (await run('join', () => api('POST', '/api/rooms/join', { code: code.trim(), project: project || '' }))) setCode('');
  };
  return html`<div class="card scard rnew">
    <div class="phead"><b>The room of another OpenBot</b><span class="mut sm">Paste the code its owner got with Invite another OpenBot. Not for a phone: a phone scans the QR at the top.</span></div>
    <div class="sfoot rjoin"><input type="text" class="rtitle rcode" placeholder="sealed-…:key id:code" aria-label="Room code from another OpenBot" autocomplete="off" spellcheck="false"
        value=${code} onInput=${(e) => setCode(e.target.value)} onKeyDown=${(e) => { if (e.key === 'Enter') { e.preventDefault(); join(); } }} />
      <button class="sadd" disabled=${busy === 'join' || !code.trim()} onClick=${join}>${busy === 'join' ? 'Joining…' : 'Join with a code'}</button></div>
    ${error ? html`<div class="ierr">${error}</div>` : null}
  </div>`;
}
