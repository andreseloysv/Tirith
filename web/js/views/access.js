// «Access» view (the old v-accesos.js): what the agents reached outside the machine through Tirith's proxy,
// newest first, with Block / Unblock per destination; the block list with a box to add a domain, address or
// range; and the sandbox status. Data: GET /api/net/destinations and /api/net/status, refetched on SSE "net"/"event".
// Inside a project the list is what that project's lanes reached (a lane's project comes from the store, not from its
// name); the block list and the guard are the machine's, and say so.
import { html, useState, useMemo } from '../../vendor/preact-htm.js';
import { useFetch, useStore, api } from '../store.js';
import { clockTime } from '../common.js';
import { useAction } from './settings/ui.js';

const MAX_ROWS = 200;
const IP = /^[0-9.]+$|:/;
/** Who reached a destination, by role as the old page showed it: "Manager, CEO, 48 lanes" (hover lists them all). */
const whoText = (who) => {
  const roles = [...new Set(who.filter((w) => w === 'manager' || w === 'ceo' || w.startsWith('ceo-')).map((w) => (w === 'manager' ? 'Manager' : 'CEO')))];
  const lanes = who.length - who.filter((w) => w === 'manager' || w === 'ceo' || w.startsWith('ceo-')).length;
  return [...roles, ...(lanes ? [`${lanes} ${lanes === 1 ? 'lane' : 'lanes'}`] : [])].join(', ');
};

function Guard({ st }) {
  if (!st) return null;
  return html`<p class="sm netline">${st.sandbox
    ? html`<span class="chip xk api">Network guard on</span><span class="mut">Lanes run in a sandbox: they reach the outside only through Tirith's proxy (127.0.0.1:${st.proxy_port}), which sees each domain and refuses the blocked ones.</span>`
    : html`<span class="chip xk blk">Network guard off</span><span class="mut">${st.why || ''}</span>`}
    ${st.proxy && st.proxy !== 'listening' ? html`<span class="chip xk blk" title=${st.proxy}>Proxy: ${st.proxy}</span>` : null}</p>`;
}

export function Access({ project }) {
  const s = useStore();
  const [dests, reloadD, errD] = useFetch('/api/net/destinations', ['net', 'event']);
  const [st, reloadS] = useFetch('/api/net/status', ['net']);
  const [busy, error, run] = useAction();
  const [rule, setRule] = useState('');
  const blocks = (st && st.blocks) || [];
  const change = async (r, on) => {
    await run(`${on ? 'b' : 'u'}:${r}`, () => api(on ? 'POST' : 'DELETE', '/api/net/blocks', { rule: r }));
    reloadD(); reloadS();
  };
  const add = async () => {
    const r = rule.trim();
    if (!r) return;
    await change(r, true);
    setRule('');
  };
  const q = String(s.q || '').trim().toLowerCase();
  const mine = (l) => (s.lanes[l] || {}).project === project;
  const rows = useMemo(() => (dests || []).map((d) => (project ? { ...d, lanes: (d.lanes || []).filter(mine) } : d)).filter((d) => {
    if (project && !d.lanes.length) return false;
    if (!q) return true;
    const text = [d.host, d.rule, ...(d.lanes || [])].join(' ').toLowerCase();
    return q.split(/\s+/).every((w) => text.includes(w));
  }), [dests, q, project, s.updated]);
  const shown = rows.slice(0, MAX_ROWS);

  const btn = (d) => {
    if (busy === `b:${d.host}` || (d.rule && busy === `u:${d.rule}`)) return html`<span class="mut sm">${d.rule ? 'Unblocking…' : 'Blocking…'}</span>`;
    return d.rule
      ? html`<span class="chip xk blk" title=${`Blocked by ${d.rule}`}>Blocked</span> <button class="link sm" onClick=${() => change(d.rule, false)}>Unblock</button>`
      : html`<button class="link sm" title=${`No agent may reach ${d.host} or its subdomains`} onClick=${() => change(d.host, true)}>Block</button>`;
  };

  return html`<div class="listwrap" data-scroll="accesos">
    <section class="pad extsec">
      <h2>Network${project ? html` <span class="mut">all projects</span>` : null}</h2>
      <${Guard} st=${st} />
      <p class="sm netblocks"><b>Blocked:</b>${blocks.length ? blocks.map((b) => html`<span key=${b}><span class="chip xk blk">${b}</span>
        <button class="link sm" disabled=${busy === `u:${b}`} onClick=${() => change(b, false)}>${busy === `u:${b}` ? 'Unblocking…' : 'Unblock'}</button></span>`)
        : html`<span class="mut">nothing</span>`}</p>
      <div class="netadd"><input type="text" class="rtitle" placeholder="domain, IP or range (10.0.0.0/8)" aria-label="Domain, address or range to block"
          spellcheck="false" autocomplete="off" value=${rule} onInput=${(e) => setRule(e.target.value)}
          onKeyDown=${(e) => { if (e.key === 'Enter') { e.preventDefault(); add(); } }} />
        <button class="sadd" disabled=${!rule.trim() || busy === `b:${rule.trim()}`} onClick=${add}>Block</button>
        <span class="mut sm">A domain also blocks its subdomains.</span></div>
      ${error ? html`<p class="ierr">${error}</p>` : null}
    </section>
    <section class="pad extsec">
      <h2>External <span class="mut">${rows.length} ${rows.length === 1 ? 'destination' : 'destinations'}</span></h2>
      ${errD && !dests ? html`<p class="ierr">Could not read the destinations: ${errD.message}</p>` : null}
      ${shown.length ? html`<table class="mtab ext"><tbody>${shown.map((d) => html`<tr key=${d.host}>
          <td class="sm num">${clockTime(d.last)}</td>
          <td><span class=${`chip xk ${d.rule ? 'blk' : IP.test(d.host) ? 'tcp' : 'https'}`}>${IP.test(d.host) ? 'Address' : 'Web'}</span></td>
          <td><b>${d.host}</b></td>
          <td class="sm" title=${(d.lanes || []).join(', ')}>${whoText(d.lanes || [])}</td>
          <td class="sm mut num">${project ? null : html`${d.count} ${d.count === 1 ? 'connection' : 'connections'}${d.refused ? html` · <span class="nref">${d.refused} refused</span>` : null}`}</td>
          <td class="sm tact">${btn(d)}</td></tr>`)}</tbody></table>
          ${rows.length > MAX_ROWS ? html`<p class="mut sm">Showing the newest ${MAX_ROWS} of ${rows.length}.</p>` : null}`
        : dests ? html`<p class="mut sm">${q ? 'No destination matches.' : project ? 'This project\'s agents reached nothing outside the machine yet.' : 'Nothing reached outside the machine yet.'}</p>` : null}
    </section>
  </div>`;
}
