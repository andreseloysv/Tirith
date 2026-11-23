// One connector on the Connectors page: what it is, Block / Unblock for all bots, Remove (asked once), its bots and
// its real tools marked for the chosen bot.
import { html, useState } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { clockTime } from '../../common.js';
import { ConnectorBots } from './bots.js';

const MARK = { allowed: 'api', approval: 'tcp', denied: 'blk' };

/** Remove, asked once: the definition and every grant go. */
function Remove({ c, busy, act }) {
  const [ask, setAsk] = useState(false);
  const key = `r:${c.id}`;
  if (busy === key) return html`<span class="mut sm">Removing…</span>`;
  if (!ask) return html`<button class="link sm" onClick=${() => setAsk(true)}>Remove</button>`;
  const gone = () => act(key, () => api('POST', `/api/connectors/${encodeURIComponent(c.id)}/remove`));
  return html`<span class="sm">Remove ${c.id} and every bot's grant of it?</span>
    <button class="link sm" onClick=${gone}>Remove</button> <button class="link sm" onClick=${() => setAsk(false)}>Keep</button>`;
}

/** The tools of `c` as marked for bot `bot`. */
function Tools({ c, bot }) {
  const mine = c.bots.find((b) => b.bot === bot);
  if (!c.tools.length) return html`<p class="mut sm">Its tools show once a bot's run has opened it.</p>`;
  if (!mine) return html`<p class="mut sm">Tools: ${c.tools.join(', ')} — bot ${bot || '(none)'} is not granted it.</p>`;
  return html`<p class="sm">Tools for bot ${bot}: ${mine.tools.map((t) => html`<span key=${t.name} class=${`chip xk ${MARK[t.mark] || 'blk'}`} title=${t.mark}>${t.name} · ${t.mark}</span> `)}</p>`;
}

export function ConnectorCard({ c, bot, busy, act }) {
  const block = (on) => act(`${on ? 'b' : 'u'}:${c.id}:`, () => api(on ? 'POST' : 'DELETE', '/api/connectors/blocks', { connector: c.id, bot: '' }));
  return html`<div class="ccard">
    <h3><b>${c.id}</b> <span class="chip xk">${c.kind}${c.host ? ` · ${c.host}` : ''}</span>
      ${c.blocked ? html` <span class="chip xk blk">Blocked for all bots</span>` : null}
      <span class="mut sm"> ${c.last ? `last used ${clockTime(c.last)}` : 'never used'}</span></h3>
    <p class="sm"><button class="link sm" onClick=${() => block(!c.blocked)}>${c.blocked ? 'Unblock for all bots' : 'Block for all bots'}</button> · <${Remove} c=${c} busy=${busy} act=${act} /></p>
    <${ConnectorBots} c=${c} busy=${busy} act=${act} />
    <${Tools} c=${c} bot=${bot} />
  </div>`;
}
