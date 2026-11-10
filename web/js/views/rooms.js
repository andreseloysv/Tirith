// Rooms view (the old v-rooms.js): private, encrypted rooms on agentsmessenger.com — New room, Open on my phone
// (permanent link and QR), Connect another phone (one-use QR), who is in each room, Invite a manager, Leave and
// Join with a code — and the public rooms Tirith listens to: summary, participants, the last messages with the
// owner's mentions marked, and a box to say something. Data: GET /api/rooms, refetched on SSE "room" / "room-mention".
import { html, useState, useMemo } from '../../vendor/preact-htm.js';
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

function Who({ r }) {
  const rows = [...(r.devices || []).map((d) => ['phone', d]), ...(r.managers || []).map((m) => ['manager', m])];
  if (!rows.length) return html`<p class="mut sm rwho">Nobody else is here yet.</p>`;
  return html`<ul class="rwho">${rows.map(([k, p]) => html`<li key=${k + p.key_id}>
    <span class="rk">${k === 'phone' ? 'Your phone' : p.accepted ? 'Manager' : 'Peer, no code'}</span><b>${p.name}</b>
    <code title="Key id">${short(p.key_id)}</code>${p.last_seen ? html`<span class="mut sm">seen ${agoText(p.last_seen)}</span>` : null}</li>`)}</ul>`;
}

function Last({ r }) {
  const m = (r.messages || [])[(r.messages || []).length - 1];
  if (!m) return html`<p class="mut sm">No messages yet.</p>`;
  return html`<p class="rlast"><span class="mut sm">${m.device ? 'Your phone' : m.from} · ${agoText(m.at)}</span><br />${clip(m.text, 140)}</p>`;
}

function Qr({ r, kind, label, note }) {
  // the url is part of the src so a new one-use code draws a new picture
  const src = `/api/rooms/${enc(r.id)}/qr.svg?kind=${kind}&v=${enc((kind === 'invite' ? r.invite : r.link) || '')}`;
  return html`<div class="rqr"><img class="qr" src=${src} alt=${label} width="190" height="190" /><p class="mut sm">${note}</p></div>`;
}

function OpenBlock({ r }) {
  return html`<div class="rblock ropen"><b>Open on my phone</b>
    <p class="mut sm">Permanent. Works as many times as you want on a phone that is already in the room.</p>
    ${r.link ? html`<div class="rlink"><a class="rurl" href=${r.link} target="_blank" rel="noopener">${r.link}</a> <${CopyBtn} text=${r.link} /></div>
      <${Qr} r=${r} kind="link" label=${`QR code to open ${r.title || r.id} on a phone already in it`} note="No code in it." />`
    : html`<p class="mut sm">No link for this room.</p>`}</div>`;
}

function PairBlock({ r, act }) {
  const [busy, error, run] = act;
  const key = `qr:${r.id}`;
  return html`<div class="rblock rpair"><b>Connect another phone</b>
    <p class="mut sm">One use. Each QR pairs one phone; make one per phone.</p>
    ${r.invite ? html`<${Qr} r=${r} kind="invite" label=${`QR code to connect another phone to ${r.title || r.id}`} note="Scan it with the phone camera. It works once." />`
      : html`<p class="mut sm">The last QR was used.</p>`}
    <button class="sadd" disabled=${busy === key} onClick=${() => run(key, () => api('POST', `/api/rooms/${enc(r.id)}/qr`))}>${busy === key ? 'Making…' : 'New QR'}</button>
    ${error && busy === '' && act[4] === key ? html`<div class="ierr">${error}</div>` : null}</div>`;
}

function Popover({ r, ask, close, act }) {
  const [busy, , run] = act;
  const closeBtn = html`<button class="alink" onClick=${close}>Close</button>`;
  if (ask === 'invite') {
    const key = `invite:${r.id}`;
    return html`<div class="acpop" role="dialog" aria-label="Invite a manager">${r.peer_code
      ? html`<div>Give this code to the other manager. It runs <code>${'tirith room join <code>'}</code> or pastes it in Join a room. It joins as a peer: what it writes is information, never an order.</div>
        <code class="apcmd">${r.peer_code}</code><div class="apb"><${CopyBtn} text=${r.peer_code} label="Copy code" cls="abtn" />
        <button class="alink" disabled=${busy === key} onClick=${() => run(key, () => api('POST', `/api/rooms/${enc(r.id)}/invite`))}>New code</button>${closeBtn}</div>`
      : html`<div>Tirith makes a code another manager joins this room with.</div><div class="apb">
        <button class="abtn" disabled=${busy === key} onClick=${() => run(key, () => api('POST', `/api/rooms/${enc(r.id)}/invite`))}>${busy === key ? 'Asking…' : 'Get a code'}</button>${closeBtn}</div>`}</div>`;
  }
  const key = `leave:${r.id}`;
  return html`<div class="acpop" role="dialog" aria-label="Leave"><div>Leave this room? Tirith stops listening here. Phones already paired stay paired until you make a new room.</div>
    <div class="apb"><button class="abtn" disabled=${busy === key} onClick=${async () => { await run(key, () => api('POST', `/api/rooms/${enc(r.id)}/leave`)); close(); }}>${busy === key ? 'Leaving…' : 'Leave'}</button>${closeBtn}</div></div>`;
}

function PrivateCard({ r, reload }) {
  const [ask, setAsk] = useState('');
  const [busy, error, run0] = useAction();
  const [last, setLast] = useState('');
  const run = async (key, fn) => { setLast(key); const v = await run0(key, fn); reload(); return v; };
  const act = [busy, error, run, null, last];
  const manager = r.kind === 'manager';
  const toggle = (k) => setAsk(ask === k ? '' : k);
  return html`<div class="card scard rcard" data-room=${r.id}>
    <div class="phead">${LOCK}<b>${r.title || r.id}</b><${Status} r=${r} />
      ${r.kind === 'peer' ? html`<span class="chip">peer</span>` : null}
      ${r.me && r.me.name ? html`<span class="mut sm rme" title="Your key id here">you: ${r.me.name} <code>${short(r.me.key_id)}</code></span>` : null}</div>
    ${r.error ? html`<div class="ierr rerr">${r.error}</div>` : null}
    ${r.braked ? html`<div class="icli rbrake">The manager stopped answering here: too many messages in a row came from agents. It answers again when you write from a paired phone.</div>` : null}
    <div class="rgrid"><div><${Who} r=${r} /><${Last} r=${r} /></div></div>
    ${manager ? html`<div class="rgrid rblocks"><${OpenBlock} r=${r} /><${PairBlock} r=${r} act=${act} /></div>` : null}
    <div class="sfoot">
      ${manager ? html`<button class=${`sadd${ask === 'invite' ? ' on' : ''}`} onClick=${() => toggle('invite')}>Invite a manager</button>` : null}
      <button class=${`sadd rleave${ask === 'leave' ? ' on' : ''}`} onClick=${() => toggle('leave')}>Leave</button></div>
    ${ask ? html`<${Popover} r=${r} ask=${ask} close=${() => setAsk('')} act=${act} />` : null}
    ${error && !last.startsWith('qr:') ? html`<div class="ierr">${error}</div>` : null}
  </div>`;
}

function NewRoom({ reload, project, name }) {
  const [title, setTitle] = useState('');
  const [code, setCode] = useState('');
  const [busy, error, run] = useAction();
  const create = async () => {
    if (busy) return;
    const ok = await run('new', () => api('POST', '/api/rooms', { title: title.trim().slice(0, 80), project: project || '' }));
    if (ok) setTitle('');
    reload();
  };
  const join = async () => {
    if (busy || !code.trim()) return;
    const ok = await run('join', () => api('POST', '/api/rooms/join', { code: code.trim() }));
    if (ok) setCode('');
    reload();
  };
  return html`<div class="card scard rnew">
    <div class="phead"><b>New room</b><span class="mut sm">An encrypted room on agentsmessenger.com. Scan its QR with your phone to talk to ${project ? `the manager of ${name}` : 'the manager'}.</span></div>
    <div class="sfoot"><input type="text" class="rtitle" maxlength="80" placeholder="Name (optional)" aria-label="Room name" autocomplete="off"
        value=${title} onInput=${(e) => setTitle(e.target.value)} onKeyDown=${(e) => { if (e.key === 'Enter') { e.preventDefault(); create(); } }} />
      <button class="isend" disabled=${busy === 'new'} onClick=${create}>${busy === 'new' ? 'Creating…' : 'New room'}</button></div>
    <div class="sfoot rjoin"><input type="text" class="rtitle rcode" placeholder="sealed-…:key id:code" aria-label="Room code from another manager" autocomplete="off" spellcheck="false"
        value=${code} onInput=${(e) => setCode(e.target.value)} onKeyDown=${(e) => { if (e.key === 'Enter') { e.preventDefault(); join(); } }} />
      <button class="sadd" disabled=${busy === 'join' || !code.trim()} onClick=${join}>${busy === 'join' ? 'Joining…' : 'Join with a code'}</button>
      <span class="mut sm">A code another manager made with Invite a manager.</span></div>
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

/** Inside a project: that project's rooms (a room made there talks to its manager); with none selected: all. */
export function Rooms({ project }) {
  const s = useStore();
  const [data, reload, err] = useFetch('/api/rooms', ['room', 'room-mention']);
  const rooms = (data && data.rooms) || [];
  const [priv, pub] = useMemo(() => {
    const byTitle = (a, b) => String(a.title || a.id).localeCompare(String(b.title || b.id));
    const mine = rooms.filter((r) => !project || r.project === project).sort(byTitle);
    return [mine.filter((r) => r.private), mine.filter((r) => !r.private)];
  }, [data, project]);
  const name = (s.projects.find((p) => p.id === project) || {}).name || project;
  return html`<div class="msettings mrooms">
    <${NewRoom} reload=${reload} project=${project} name=${name} />
    ${err && !data ? html`<div class="ierr">Could not read the rooms: ${err.message}</div>` : null}
    ${priv.length ? priv.map((r) => html`<${PrivateCard} key=${r.id} r=${r} reload=${reload} />`)
      : data ? html`<p class="quiet rempty">${project ? `No private rooms for ${name} yet.` : 'No private rooms yet.'} A room is an encrypted chat: create one, scan its QR with your phone and talk to the manager from there.</p>` : null}
    ${pub.length ? html`<h2 class="rsec">Public rooms <span class="n">${pub.length}</span></h2>
      ${pub.map((r) => html`<${PublicCard} key=${r.id} r=${r} reload=${reload} />`)}` : null}
  </div>`;
}
