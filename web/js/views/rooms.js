// Rooms view: the owner's phone first — one QR that always works (it is used once, and the page asks for the next one
// by itself): scanned once, the phone has a chat with the manager of every project — then each project's encrypted
// room on agentsmessenger.com (who else is in it, the last message, Invite another Tirith, Leave), Join with a code
// from another Tirith, and the public rooms Tirith listens to: summary, participants, the last messages with the
// owner's mentions marked, and a box to say something. Tirith makes a project's room itself.
// Data: GET /api/rooms, refetched on SSE "room" / "room-mention", every 15 s and when the window comes back.
import { html, useState, useMemo, useEffect, useRef } from '../../vendor/preact-htm.js';
import { useFetch, useStore, api } from '../store.js';
import { agoText, CopyBtn, useAction } from './settings/ui.js';

const LOCK = html`<svg class="rlock" viewBox="0 0 16 16" width="13" height="13" role="img" aria-label="Encrypted" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><title>Encrypted</title><rect x="3.2" y="7" width="9.6" height="6.4" rx="1.6"/><path d="M5.4 7V5.2a2.6 2.6 0 015.2 0V7"/></svg>`;
const HASH = html`<svg class="rlock rpub" viewBox="0 0 16 16" width="13" height="13" role="img" aria-label="Public" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><title>Public</title><path d="M3 6h10M3 10h10M6.5 2.5l-1 11M10.5 2.5l-1 11"/></svg>`;
const MAX_SHOWN = 8;      // messages shown per public room
const short = (id) => String(id || '').slice(0, 8);
const enc = encodeURIComponent;
const clip = (t, n) => { t = String(t == null ? '' : t).replace(/\s+/g, ' '); return t.length > n ? `${t.slice(0, n - 1)}…` : t; };

function Status({ r }) {
  const ok = !r.error;
  return html`<span class=${`rconn ${ok ? 'on' : 'off'}`}><i class=${`dot ${ok ? 'green' : 'grey'}`}></i>${ok ? 'Connected' : 'Not connected'}</span>`;
}

/** The phones of a room (`phones`), or the other managers in it. A forgotten phone is not shown: it is out, and the
    daemon keeps its key only to ignore it. */
function Who({ r, phones, forget }) {
  const rows = phones ? r.devices || [] : (r.managers || []).filter((m) => !m.blocked);
  if (!rows.length) return phones ? html`<p class="mut sm rwho">No phone is connected yet.</p>` : null;
  return html`<ul class="rwho">${rows.map((p) => html`<li key=${p.key_id}>
    <span class="rk">${phones ? 'Your phone' : p.accepted ? 'Manager' : 'Peer, no code'}</span><b>${p.name}</b>
    <code title="Key id">${short(p.key_id)}</code>${p.last_seen ? html`<span class="mut sm">seen ${agoText(p.last_seen)}</span>` : null}
    ${phones ? html`<button class="alink rforget" onClick=${() => forget(p.key_id)}>Forget</button>` : null}</li>`)}</ul>`;
}

function Last({ r }) {
  const m = (r.messages || [])[(r.messages || []).length - 1];
  if (!m) return html`<p class="mut sm">No messages yet.</p>`;
  return html`<p class="rlast"><span class="mut sm">${m.device ? 'Your phone' : m.from} · ${agoText(m.at)}</span><br />${clip(m.text, 140)}</p>`;
}

/** One call at a time, the rooms read again after each: `[busy, error, run]`. */
function useRoomAction(reload) {
  const [busy, error, run0] = useAction();
  return [busy, error, async (key, fn) => { const v = await run0(key, fn); reload(); return v; }];
}

function Popover({ r, ask, close, act }) {
  const [busy, , run] = act;
  const closeBtn = html`<button class="alink" onClick=${close}>Close</button>`;
  if (ask === 'invite') {
    const key = `invite:${r.id}`;
    const fresh = () => run(key, () => api('POST', `/api/rooms/${enc(r.id)}/invite`));
    return html`<div class="acpop" role="dialog" aria-label="Invite another Tirith">
      <div><b>For the Tirith of another computer, not for a phone:</b> a phone scans the QR at the top of this page. The other
        Tirith joins with this code (Join with a code, or <code>tirith room join</code>) as a peer: what it writes is information, never an order.</div>
      ${r.peer_code ? html`<input class="rtitle rcode rpeer" type="text" readonly aria-label="Code for another Tirith" value=${r.peer_code} onFocus=${(e) => e.target.select()} />
        <div class="apb"><${CopyBtn} text=${r.peer_code} label="Copy code" cls="abtn" /><button class="alink" disabled=${busy === key} onClick=${fresh}>New code</button>${closeBtn}</div>`
      : html`<div class="apb rget"><button class="abtn" disabled=${busy === key} onClick=${fresh}>${busy === key ? 'Asking…' : 'Get a code'}</button>${closeBtn}</div>`}</div>`;
  }
  if (ask.startsWith('forget:')) {
    const id = ask.slice(7);
    return html`<div class="acpop" role="dialog" aria-label="Forget a phone"><div>Forget the phone <code>${short(id)}</code>? It is disconnected from every project: it reads nothing more and what it writes is ignored. To connect it again, scan the QR with it.</div>
      <div class="apb"><button class="abtn" disabled=${busy === ask} onClick=${async () => { await run(ask, () => api('POST', '/api/rooms/forget', { key_id: id })); close(); }}>${busy === ask ? 'Forgetting…' : 'Forget'}</button>${closeBtn}</div></div>`;
  }
  const key = `leave:${r.id}`;
  return html`<div class="acpop" role="dialog" aria-label="Leave"><div>Leave this room? Tirith stops listening here and it goes from your phones. They stay connected to the other rooms (to disconnect a phone, use Forget), and a project left without a room gets a new one.</div>
    <div class="apb"><button class="abtn" disabled=${busy === key} onClick=${async () => { await run(key, () => api('POST', `/api/rooms/${enc(r.id)}/leave`)); close(); }}>${busy === key ? 'Leaving…' : 'Leave'}</button>${closeBtn}</div></div>`;
}

/** The owner's phone: the QR of `r` (the selected project's room, or the room of no project) and the phones that are
    connected. The QR works once; when it is used, or none was made yet, the next one is asked for without a click. */
function PhoneCard({ r, reload, title }) {
  const act = useRoomAction(reload);
  const [busy, error, run] = act;
  const [ask, setAsk] = useState('');
  const asked = useRef(0);
  const make = () => run('qr', () => api('POST', `/api/rooms/${enc(r.id)}/qr`));
  // Looked at on every draw (the rooms are read again at least every 15 s): no QR, so one is asked for, at most every 10 s.
  useEffect(() => {
    if (r.invite) asked.current = 0;
    else if (Date.now() - asked.current > 10000) { asked.current = Date.now(); make(); }
  });
  // the url is part of the src so a new code draws a new picture
  const src = `/api/rooms/${enc(r.id)}/qr.svg?kind=invite&v=${enc(r.invite || '')}`;
  return html`<div class="card scard rphone" data-room=${r.id}>
    <div class="phead">${LOCK}<b>Your phone</b><${Status} r=${r} /></div>
    ${r.error ? html`<div class="ierr rerr">Tirith cannot reach agentsmessenger.com now, so a phone connects when it can again: ${r.error}</div>` : null}
    <div class="rscan">
      ${r.invite ? html`<img class="qr" src=${src} alt="QR code that connects a phone to Tirith" width="232" height="232" />`
        : html`<div class="qr qrwait">${busy === 'qr' || !error ? 'Making the QR…' : html`<button class="abtn" onClick=${make}>Show the QR</button>`}</div>`}
      <div class="rhow">
        <p class="rlead">Scan this QR with the phone's camera.</p>
        <p>It opens ${title} on the phone and gives it a chat with the manager of every project, also of those you add later. Encrypted, with nothing to install.</p>
        <p class="mut sm">It works once, and the next one appears here by itself. Scan it again whenever the phone says it is not connected: a browser forgets its chats after some days without use.${r.invite ? html` <${CopyBtn} text=${r.invite} label="Copy it as a link" />` : null}</p>
        <${Who} r=${r} phones=${true} forget=${(id) => setAsk(ask === id ? '' : id)} />
      </div>
    </div>
    ${ask ? html`<${Popover} r=${r} ask=${`forget:${ask}`} close=${() => setAsk('')} act=${act} />` : null}
    ${error ? html`<div class="ierr">${error}</div>` : null}
  </div>`;
}

/** Before the first room: Tirith opens rooms on the platform only when the owner asks, so the first QR takes one click. */
function NoRoom({ reload, project }) {
  const [busy, error, run] = useRoomAction(reload);
  return html`<div class="card scard rphone">
    <div class="phead">${LOCK}<b>Your phone</b></div>
    <p class="rlead">Work from your phone.</p>
    <p>Tirith opens an encrypted room on agentsmessenger.com and shows a QR here: scan it with the phone's camera and the phone has a chat with the manager of every project, with nothing to install.</p>
    <div class="sfoot"><button class="isend" disabled=${busy === 'new'} onClick=${() => run('new', () => api('POST', '/api/rooms', { title: '', project: project || '' }))}>${busy === 'new' ? 'Making the room…' : 'Show the QR'}</button></div>
    ${error ? html`<div class="ierr">${error}</div>` : null}
  </div>`;
}

function PrivateCard({ r, reload, title }) {
  const [ask, setAsk] = useState('');
  const act = useRoomAction(reload);
  const toggle = (k) => setAsk(ask === k ? '' : k);
  return html`<div class="card scard rcard" data-room=${r.id}>
    <div class="phead">${LOCK}<b>${title}</b><${Status} r=${r} />
      ${r.kind === 'peer' ? html`<span class="chip">peer</span>` : null}
      ${r.me && r.me.name ? html`<span class="mut sm rme" title="Your key id here">you: ${r.me.name} <code>${short(r.me.key_id)}</code></span>` : null}</div>
    ${r.error ? html`<div class="ierr rerr">${r.error}</div>` : null}
    ${r.braked ? html`<div class="icli rbrake">The manager stopped answering here: too many messages in a row came from agents. It answers again when you write from a paired phone.</div>` : null}
    <${Who} r=${r} /><${Last} r=${r} />
    <div class="sfoot">
      ${r.kind === 'manager' ? html`<button class=${`sadd${ask === 'invite' ? ' on' : ''}`} onClick=${() => toggle('invite')}>Invite another Tirith</button>` : null}
      <button class=${`sadd rleave${ask === 'leave' ? ' on' : ''}`} onClick=${() => toggle('leave')}>Leave</button></div>
    ${ask ? html`<${Popover} r=${r} ask=${ask} close=${() => setAsk('')} act=${act} />` : null}
    ${act[1] ? html`<div class="ierr">${act[1]}</div>` : null}
  </div>`;
}

function JoinCard({ reload, project }) {
  const [code, setCode] = useState('');
  const [busy, error, run] = useRoomAction(reload);
  const join = async () => {
    if (busy || !code.trim()) return;
    if (await run('join', () => api('POST', '/api/rooms/join', { code: code.trim(), project: project || '' }))) setCode('');
  };
  return html`<div class="card scard rnew">
    <div class="phead"><b>The room of another Tirith</b><span class="mut sm">Paste the code its owner got with Invite another Tirith. Not for a phone: a phone scans the QR at the top.</span></div>
    <div class="sfoot rjoin"><input type="text" class="rtitle rcode" placeholder="sealed-…:key id:code" aria-label="Room code from another Tirith" autocomplete="off" spellcheck="false"
        value=${code} onInput=${(e) => setCode(e.target.value)} onKeyDown=${(e) => { if (e.key === 'Enter') { e.preventDefault(); join(); } }} />
      <button class="sadd" disabled=${busy === 'join' || !code.trim()} onClick=${join}>${busy === 'join' ? 'Joining…' : 'Join with a code'}</button></div>
    ${error ? html`<div class="ierr">${error}</div>` : null}
  </div>`;
}

function PublicCard({ r, reload }) {
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
  const people = (r.participants || []).slice().sort((a, b) => String(b.last_seen).localeCompare(String(a.last_seen)));
  return html`<div class="card scard rcard rpublic" data-room=${r.id}>
    <div class="phead">${HASH}<b>${r.title || r.id}</b><${Status} r=${r} />
      ${r.project ? html`<span class="chip">${r.project}</span>` : null}
      ${needs.size ? html`<span class="ichip">${needs.size === 1 ? 'Mentions you' : `${needs.size} mentions of you`}</span>` : null}
      <a class="mut sm rme" href=${r.url} target="_blank" rel="noopener">${r.id}</a></div>
    ${r.error ? html`<div class="ierr rerr">${r.error}</div>` : null}
    ${r.summary ? html`<p class="rsum">${r.summary}</p>` : null}
    <div class="rgrid">
      <div>${msgs.length ? html`<ul class="rmsgs">${msgs.map((m) => html`<li key=${m.n} class=${needs.has(m.n) ? 'rment' : ''}>
          <span class="mut sm">${m.from} · ${agoText(m.at)}</span><span class="rtext">${clip(m.text, 400)}</span></li>`)}</ul>`
        : html`<p class="mut sm">No messages yet.</p>`}</div>
      <div class="rpeople"><div class="slabel">Participants</div>
        ${people.length ? html`<ul class="rwho">${people.map((p) => html`<li key=${p.name}><b>${p.name}</b>
          <span class="mut sm">${p.messages} ${p.messages === 1 ? 'message' : 'messages'} · ${agoText(p.last_seen)}</span></li>`)}</ul>`
          : html`<p class="mut sm">Nobody has written yet.</p>`}
        ${(r.here || []).length ? html`<p class="mut sm">Here now: ${r.here.join(', ')}</p>` : null}</div>
    </div>
    <div class="sfoot"><input type="text" class="rtitle rsay" maxlength="2000" placeholder="Say something in this room" aria-label=${`Message to ${r.title || r.id}`}
        value=${text} onInput=${(e) => setText(e.target.value)} onKeyDown=${(e) => { if (e.key === 'Enter' && !e.isComposing) { e.preventDefault(); say(); } }} />
      <button class="isend" disabled=${busy === 'say' || !text.trim()} onClick=${say}>${busy === 'say' ? 'Sending…' : 'Send'}</button></div>
    ${error ? html`<div class="ierr">${error}</div>` : null}
  </div>`;
}

/** Inside a project: that project's rooms (its room talks to its manager); with none selected: all. */
export function Rooms({ project }) {
  const s = useStore();
  const [data, reload, err] = useFetch('/api/rooms', ['room', 'room-mention']);
  // The QR on the screen must be the one that works: the rooms are also read on a clock and when the window comes
  // back, in case the stream missed the moment a QR was used.
  useEffect(() => {
    const again = () => { if (!document.hidden) reload(); };
    const timer = setInterval(again, 15000);
    addEventListener('focus', again);
    document.addEventListener('visibilitychange', again);
    return () => { clearInterval(timer); removeEventListener('focus', again); document.removeEventListener('visibilitychange', again); };
  }, []);
  const rooms = (data && data.rooms) || [];
  const [priv, pub, phone] = useMemo(() => {
    const byTitle = (a, b) => String(a.title || a.id).localeCompare(String(b.title || b.id));
    const mine = rooms.filter((r) => !project || r.project === project).sort(byTitle);
    // the phone connects through the selected project's own room, else through the room of no project
    const own = rooms.filter((r) => r.kind === 'manager');
    return [mine.filter((r) => r.private), mine.filter((r) => !r.private), own.find((r) => r.project === (project || '')) || (project ? null : own[0]) || null];
  }, [data, project]);
  /* a room without a name of its own goes by its project's, as it does on the phone (the room of no project: All projects) */
  const titleOf = (r) => r.title || (s.projects.find((p) => p.id === r.project) || {}).name || (r.kind === 'manager' && !r.project ? 'All projects' : r.id);
  return html`<div class="msettings mrooms">
    ${err && !data ? html`<div class="ierr">Could not read the rooms: ${err.message}</div>` : null}
    ${phone ? html`<${PhoneCard} key=${phone.id} r=${phone} reload=${reload} title=${titleOf(phone)} />` : data ? html`<${NoRoom} reload=${reload} project=${project} />` : null}
    ${priv.map((r) => html`<${PrivateCard} key=${r.id} r=${r} reload=${reload} title=${titleOf(r)} />`)}
    ${data ? html`<${JoinCard} reload=${reload} project=${project} />` : null}
    ${pub.length ? html`<h2 class="rsec">Public rooms <span class="n">${pub.length}</span></h2>
      ${pub.map((r) => html`<${PublicCard} key=${r.id} r=${r} reload=${reload} />`)}` : null}
  </div>`;
}
