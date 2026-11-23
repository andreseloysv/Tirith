// A connector's bots on the Connectors page: each one's status (needs login, ok or error, with why), its last use,
// Login (a remote connector) and Block / Unblock for it alone.
import { html } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { clockTime } from '../../common.js';
import { login } from './login.js';

const TONE = { ok: 'api', 'needs login': 'tcp', error: 'blk' };

export function ConnectorBots({ c, busy, act }) {
  if (!c.bots.length) return html`<p class="mut sm">No bot is granted it.</p>`;
  const block = (b, on) => act(`${on ? 'b' : 'u'}:${c.id}:${b}`, () => api(on ? 'POST' : 'DELETE', '/api/connectors/blocks', { connector: c.id, bot: b }));
  const signIn = (b) => act(`l:${c.id}:${b}`, () => login(c.id, b));
  return html`<table class="mtab ext"><tbody>${c.bots.map((b) => html`<tr key=${b.bot}>
      <td><b>${b.name || b.bot}</b></td>
      <td class="sm">${b.blocked ? html`<span class="chip xk blk">blocked for this bot</span>` : html`<span class=${`chip xk ${TONE[b.status] || 'blk'}`}>${b.status}</span>`}
        ${b.why ? html` <span class="mut">${b.why}</span>` : null}</td>
      <td class="sm num">${b.last ? clockTime(b.last) : html`<span class="mut">never</span>`}</td>
      <td class="sm tact">${busy === `l:${c.id}:${b.bot}` ? html`<span class="mut">Signing in…</span>`
        : c.kind === 'remote' && !b.blocked ? html`<button class="link sm" onClick=${() => signIn(b.bot)}>Login</button> ` : null}
        ${busy && busy.endsWith(`:${c.id}:${b.bot}`) && !busy.startsWith('l:') ? html`<span class="mut">…</span>`
          : html`<button class="link sm" onClick=${() => block(b.bot, !b.blocked)}>${b.blocked ? 'Unblock' : 'Block'}</button>`}</td></tr>`)}</tbody></table>`;
}
