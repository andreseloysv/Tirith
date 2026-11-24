// One connector on the Connectors page: what it is, Block / Unblock for all bots, Remove (asked once), its bots and
// its real tools marked for the chosen bot.
import { html, useState } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { clockTime } from '../../common.js';
import { ConnectorBots } from './bots.js';
import { t } from '../../i18n.js';

const MARK = { allowed: 'api', approval: 'tcp', denied: 'blk' };

/** Remove, asked once: the definition and every grant go. */
function Remove({ c, busy, act }) {
  const [ask, setAsk] = useState(false);
  const key = `r:${c.id}`;
  if (busy === key) return html`<span class="mut sm">${t('Removing…')}</span>`;
  if (!ask) return html`<button class="link sm" onClick=${() => setAsk(true)}>${t('Remove')}</button>`;
  const gone = () => act(key, () => api('POST', `/api/connectors/${encodeURIComponent(c.id)}/remove`));
  return html`<span class="sm">${t('Remove {id} and every bot\'s grant of it?', { id: c.id })}</span>
    <button class="link sm" onClick=${gone}>${t('Remove')}</button> <button class="link sm" onClick=${() => setAsk(false)}>${t('Keep')}</button>`;
}

/** The tools of `c` as marked for bot `bot`. */
function Tools({ c, bot }) {
  const mine = c.bots.find((b) => b.bot === bot);
  if (!c.tools.length) return html`<p class="mut sm">${t('Its tools show once a bot\'s run has opened it.')}</p>`;
  if (!mine) return html`<p class="mut sm">${t('Tools: {tools} — bot {bot} is not granted it.', { tools: c.tools.join(', '), bot: bot || t('(none)') })}</p>`;
  return html`<p class="sm">${t('Tools for bot {bot}:', { bot })} ${mine.tools.map((tool) => html`<span key=${tool.name} class=${`chip xk ${MARK[tool.mark] || 'blk'}`} title=${t(tool.mark)}>${tool.name} · ${t(tool.mark)}</span> `)}</p>`;
}

export function ConnectorCard({ c, bot, busy, act }) {
  const block = (on) => act(`${on ? 'b' : 'u'}:${c.id}:`, () => api(on ? 'POST' : 'DELETE', '/api/connectors/blocks', { connector: c.id, bot: '' }));
  return html`<div class="ccard">
    <h3><b>${c.id}</b> <span class="chip xk">${t(c.kind)}${c.host ? ` · ${c.host}` : ''}</span>
      ${c.blocked ? html` <span class="chip xk blk">${t('Blocked for all bots')}</span>` : null}
      <span class="mut sm"> ${c.last ? t('last used {time}', { time: clockTime(c.last) }) : t('never used')}</span></h3>
    <p class="sm"><button class="link sm" onClick=${() => block(!c.blocked)}>${c.blocked ? t('Unblock for all bots') : t('Block for all bots')}</button> · <${Remove} c=${c} busy=${busy} act=${act} /></p>
    <${ConnectorBots} c=${c} busy=${busy} act=${act} />
    <${Tools} c=${c} bot=${bot} />
  </div>`;
}
