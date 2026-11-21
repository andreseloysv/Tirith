// The Rooms view's small parts: a bot's key and face state, the public mark, short texts, the connection state,
// the face of the agent that runs a room, who is in a room, its last message, and one call at a time.
import { html } from '../../../vendor/preact-htm.js';
import { ICON, Icon } from '../../common.js';
import { Face } from '../bots/face.js';
import { agoText, useAction } from '../settings/ui.js';

/* a bot's conversation and what its face shows, as in bots.js (kept local so this view does not pull the Bots view in) */
export const keyOf = (b) => b.key || `bot-${b.id}`;
export const stateOf = (b, busy) => (busy ? 'work' : (b.routines || []).some((r) => r.on !== false) || (b.routes || []).length ? 'watch' : 'sleep');

export const HASH = html`<svg class="rhash rpub" viewBox="0 0 16 16" width="13" height="13" role="img" aria-label="Public" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><title>Public</title><path d="M3 6h10M3 10h10M6.5 2.5l-1 11M10.5 2.5l-1 11"/></svg>`;
export const MAX_SHOWN = 8;      // messages shown per public room
export const short = (id) => String(id || '').slice(0, 8);
export const enc = encodeURIComponent;
export const clip = (t, n) => { t = String(t == null ? '' : t).replace(/\s+/g, ' '); return t.length > n ? `${t.slice(0, n - 1)}…` : t; };

export function Status({ r }) {
  const ok = !r.error;
  return html`<span class=${`rconn ${ok ? 'on' : 'off'}`}><i class=${`dot ${ok ? 'green' : 'grey'}`}></i>${ok ? 'Connected' : 'Not connected'}</span>`;
}

/** The face of the agent that runs a project (its manager, or the bot whose room this is), in its state; the agent
    icon when it has no face of its own. Touching it opens that agent's chat. */
export function AgentFace({ a }) {
  if (!a) return null;
  const inner = a.shape
    ? html`<${Face} shape=${a.shape} color=${a.color || '#3b82f6'} size=${40} state=${a.state || 'watch'} />`
    : html`<${Icon} d=${ICON.agents} size=${22} w=${1.6} />`;
  return a.open
    ? html`<button type="button" class="rface" title=${a.label} aria-label=${a.label} onClick=${a.open}>${inner}</button>`
    : html`<span class="rface" title=${a.label}>${inner}</span>`;
}

/** The phones of a room (`phones`), or the other managers in it. A forgotten phone is not shown: it is out, and the
    daemon keeps its key only to ignore it. */
export function Who({ r, phones, forget }) {
  const rows = phones ? r.devices || [] : (r.managers || []).filter((m) => !m.blocked);
  if (!rows.length) return phones ? html`<p class="mut sm rwho">No phone is connected yet.</p>` : null;
  return html`<ul class="rwho">${rows.map((p) => html`<li key=${p.key_id}>
    <span class="rk">${phones ? 'Your phone' : p.accepted ? 'Manager' : 'Peer, no code'}</span><b>${p.name}</b>
    <code title="Key id">${short(p.key_id)}</code>${p.last_seen ? html`<span class="mut sm">seen ${agoText(p.last_seen)}</span>` : null}
    ${phones ? html`<button class="alink rforget" onClick=${() => forget(p.key_id)}>Forget</button>` : null}</li>`)}</ul>`;
}

export function Last({ r }) {
  const m = (r.messages || [])[(r.messages || []).length - 1];
  if (!m) return html`<p class="mut sm">No messages yet.</p>`;
  return html`<p class="rlast"><span class="mut sm">${m.device ? 'Your phone' : m.from} · ${agoText(m.at)}</span><br />${clip(m.text, 140)}</p>`;
}

/** One call at a time, the rooms read again after each: `[busy, error, run]`. */
export function useRoomAction(reload) {
  const [busy, error, run0] = useAction();
  return [busy, error, async (key, fn) => { const v = await run0(key, fn); reload(); return v; }];
}
