// The connectors on the «Access» page, beside the network destinations: what the bots called through their gateway,
// newest first as time, bot, connector.tool and verdict, narrowed by bot and connector; and Block / Unblock of a
// connector for all bots or for one. Data: GET /api/connectors/calls and /api/connectors/blocks, refetched on SSE
// "connectors"; POST / DELETE /api/connectors/blocks change them (the next call is refused, no restart).
import { html, useState } from '../../vendor/preact-htm.js';
import { useFetch, api } from '../store.js';
import { clockTime } from '../common.js';
import { useAction } from './settings/ui.js';
import { t } from '../i18n.js';

const MAX_ROWS = 200;
const tone = (v) => (v === 'ran' || v === 'approved and ran' ? 'api' : v.startsWith('refused') ? 'blk' : 'tcp');

function Blocks({ blocks, busy, change }) {
  const btn = (c, bot, on) => {
    const key = `${on ? 'b' : 'u'}:${c}:${bot}`;
    if (busy === key) return html`<span class="mut sm">${on ? t('Blocking…') : t('Unblocking…')}</span>`;
    const title = bot ? t('Bot {bot} may not call {c} (its grant comes back exactly on Unblock)', { bot, c }) : t('No bot may call {c}', { c });
    const word = bot ? (on ? t('Block') : t('Unblock')) : on ? t('Block for all bots') : t('Unblock for all bots');
    return html`<button class="link sm" title=${on ? title : ''} onClick=${() => change(c, bot, on)}>${word}</button>`;
  };
  return html`<table class="mtab ext"><tbody>${blocks.map((c) => html`<tr key=${c.id}>
      <td><b>${c.id}</b></td>
      <td class="sm">${c.blocked ? html`<span class="chip xk blk">${t('Blocked for all bots')}</span> ` : null}${btn(c.id, '', !c.blocked)}</td>
      <td class="sm">${c.bots.length ? c.bots.map((b) => html`<span key=${b.bot} class="cbot">
          ${b.blocked ? html`<span class="chip xk blk">${t('bot {bot} blocked', { bot: b.bot })}</span>` : html`<span>${t('bot {bot}', { bot: b.bot })}</span>`} ${btn(c.id, b.bot, !b.blocked)}</span>`)
        : html`<span class="mut">${t('no bot is granted it')}</span>`}</td></tr>`)}</tbody></table>`;
}

export function Connectors() {
  const [bot, setBot] = useState('');
  const [connector, setConnector] = useState('');
  const query = new URLSearchParams({ bot, connector }).toString();
  const [calls, , errC] = useFetch(`/api/connectors/calls?${query}`, ['connectors']);
  const [blocks, reloadB] = useFetch('/api/connectors/blocks', ['connectors']);
  const [busy, error, run] = useAction();
  const change = async (c, b, on) => {
    await run(`${on ? 'b' : 'u'}:${c}:${b}`, () => api(on ? 'POST' : 'DELETE', '/api/connectors/blocks', { connector: c, bot: b }));
    reloadB();
  };
  const list = blocks || [];
  if (!list.length && !(calls || []).length) return null;
  const bots = [...new Set(list.flatMap((c) => c.bots.map((b) => b.bot)).concat((calls || []).map((r) => r.bot)))].sort();
  const pick = (label, value, set, options) => html`<select aria-label=${label} value=${value} onChange=${(e) => set(e.target.value)}>
      <option value="">${label}</option>${options.map((o) => html`<option key=${o} value=${o}>${o}</option>`)}</select>`;
  const rows = calls || [];
  return html`<section class="pad extsec">
    <h2>${t('Connectors')} <span class="mut">${rows.length === 1 ? t('1 call') : t('{n} calls', { n: rows.length })}</span></h2>
    <${Blocks} blocks=${list} busy=${busy} change=${change} />
    ${error ? html`<p class="ierr">${error}</p>` : null}
    <div class="netadd">${pick(t('Every bot'), bot, setBot, bots)} ${pick(t('Every connector'), connector, setConnector, list.map((c) => c.id))}</div>
    ${errC && !calls ? html`<p class="ierr">${t('Could not read the calls: {message}', { message: errC.message })}</p>` : null}
    ${rows.length ? html`<table class="mtab ext"><tbody>${rows.slice(0, MAX_ROWS).map((r, i) => html`<tr key=${i}>
        <td class="sm num">${clockTime(r.at)}</td>
        <td class="sm">${r.bot}</td>
        <td><b>${r.call}</b></td>
        <td class="sm tact"><span class=${`chip xk ${tone(r.verdict)}`}>${t(r.verdict)}</span></td></tr>`)}</tbody></table>
        ${rows.length > MAX_ROWS ? html`<p class="mut sm">${t('Showing the newest {max} of {n}.', { max: MAX_ROWS, n: rows.length })}</p>` : null}`
      : calls ? html`<p class="mut sm">${bot || connector ? t('No call matches.') : t('No bot called a connector yet.')}</p>` : null}
  </section>`;
}
