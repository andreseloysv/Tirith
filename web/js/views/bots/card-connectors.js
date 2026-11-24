// The bot card's Permissions: its Connectors. Each connector defined in ~/.tirith/config.toml, granted to this bot
// or not, and when granted narrowed by its tools (empty: all the connector allows), deny and approval globs. Each
// change is saved at once through the bot store (POST /api/bots/<id>/connectors), which writes
// [roles.bot-<id>.connectors.<connector>]; it can only narrow what the connector itself allows.
import { html, useState } from '../../../vendor/preact-htm.js';
import { useFetch, api } from '../../store.js';
import { t } from '../../i18n.js';

const list = (s) => s.split(',').map((x) => x.trim()).filter(Boolean);
const text = (a) => (a || []).join(', ');

/** One granted connector's narrowing, saved with its own button. */
function Narrow({ c, save }) {
  const [tools, setTools] = useState(text(c.tools));
  const [deny, setDeny] = useState(text(c.deny));
  const [approval, setApproval] = useState(text(c.approval));
  const field = (label, value, set, hint) => html`<label class="bfrow"><span class="bcap">${label}</span>
    <input class="bsel" aria-label=${`${c.id} ${label}`} placeholder=${hint} value=${value} onInput=${(e) => set(e.target.value)} /></label>`;
  const grant = { tools: list(tools).length ? list(tools) : null, deny: list(deny), approval: list(approval) };
  return html`${field(t('Tools'), tools, setTools, t('all it allows, or read_*, search'))}
    ${field(t('Deny'), deny, setDeny, t('none, or delete_*'))}
    ${field(t('Asks first'), approval, setApproval, t('none, or send_*'))}
    <button type="button" class="blink" onClick=${() => save(c.id, grant)}>${t("Save {id}'s limits", { id: c.id })}</button>`;
}

export function connectorsSection(bot) {
  return bot.id && !bot.system ? html`<${Connectors} bot=${bot} />` : null;
}

function Connectors({ bot }) {
  const path = `/api/bots/${encodeURIComponent(bot.id)}/connectors`;
  const [rows, reload, err] = useFetch(path, ['connectors', 'bots']);
  const [why, setWhy] = useState('');
  const save = (connector, grant) => api('POST', path, { connector, grant }).then(() => { setWhy(''); reload(); }, (e) => setWhy(e.message));
  const all = rows || [];
  return html`<details class="bmodel bfactory"><summary><span class="bcap">${t('Permissions')}</span> ${t('{n} of {total} connectors', { n: all.filter((c) => c.granted).length, total: all.length })}</summary>
    <p class="bnote">${t('The connectors (MCP servers) this bot may use. A change is saved at once; its limits only narrow what the connector allows. Status, Login and Block are on the Connectors page.')}</p>
    ${err && !rows ? html`<p class="berr">${err.message}</p>` : null}
    ${all.map((c) => html`<div key=${c.id} class="bconn">
      <label class="bchk"><input type="checkbox" checked=${c.granted} disabled=${c.blocked && !c.granted}
        onChange=${(e) => save(c.id, e.target.checked ? {} : null)} /><b>${c.id}</b>${c.blocked ? ` (${t('blocked')})` : ''}</label>
      ${c.granted ? html`<${Narrow} key=${JSON.stringify(c)} c=${c} save=${save} />` : null}</div>`)}
    ${rows && !all.length ? html`<p class="bnote">${t('No connector is defined in ~/.tirith/config.toml.')}</p>` : null}
    ${why ? html`<p class="berr">${why}</p>` : null}
  </details>`;
}
