// «Connectors» page, the owner's: each connector (local or remote, blocked for all or not, its last use), each bot
// granted it (or blocked for it alone) with its status — needs login, ok or error, the why never holding a secret —
// and its last use, and the connector's real tools (what its last open listed) marked allowed, approval or denied for
// the bot chosen at the top. Actions: Login (a remote one's OAuth sign-in for a bot), Block / Unblock (for one bot or
// all) and Remove (the definition and every grant, asked once like a bot's Delete). Grants are given and narrowed on
// each bot's card (Permissions). Data: GET /api/connectors/page, refetched on SSE "connectors" and "bots".
//
// The parts live in connectors/: card.js (one connector), bots.js (its bots and their actions), login.js (a sign-in).
import { html, useState } from '../../vendor/preact-htm.js';
import { useFetch } from '../store.js';
import { useAction } from './settings/ui.js';
import { ConnectorCard } from './connectors/card.js';

export function Connectors() {
  const [page, reload, err] = useFetch('/api/connectors/page', ['connectors', 'bots']);
  const [bot, setBot] = useState('');
  const [busy, error, run] = useAction();
  const list = (page && page.connectors) || [];
  const granted = [...new Set(list.flatMap((c) => c.bots.map((b) => b.bot)))].sort();
  const chosen = granted.includes(bot) ? bot : granted[0] || '';
  const act = async (key, fn) => { await run(key, fn); reload(); };
  return html`<section class="pad extsec">
    <h2>Connectors <span class="mut">${list.length}</span></h2>
    <p class="mut sm">MCP servers the bots may use, defined in ~/.tirith/config.toml. Grant one to a bot, or narrow it, on that bot's card under Permissions.</p>
    ${err && !page ? html`<p class="ierr">Could not read the connectors: ${err.message}</p>` : null}
    ${error ? html`<p class="ierr">${error}</p>` : null}
    ${granted.length ? html`<div class="netadd"><label class="sm">Tools for <select aria-label="Bot" value=${chosen} onChange=${(e) => setBot(e.target.value)}>
      ${granted.map((b) => html`<option key=${b} value=${b}>${b}</option>`)}</select></label></div>` : null}
    ${list.map((c) => html`<${ConnectorCard} key=${c.id} c=${c} bot=${chosen} busy=${busy} act=${act} />`)}
    ${page && !list.length ? html`<p class="mut sm">No connector is defined: add a [[connectors]] row to ~/.tirith/config.toml.</p>` : null}
  </section>`;
}
