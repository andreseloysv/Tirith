// The agents NeoBot works through, set up from the page with one tap each (no terminal, no window): the daemon
// installs one with its maker's installer, signs it in through the browser, points its hooks here and checks it with
// one short real turn (/api/setup/agents). Shown on the Home while none is connected, and always in Settings.
import { html, useState, useEffect } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { AgentRow } from './row.js';

/** `[agents, reload]`: asked again every 1.5 s while a setup runs. */
export function useAgents() {
  const [list, setList] = useState(null);
  const load = () => api('GET', '/api/setup/agents').then((r) => setList(r.agents || []), () => setList((l) => l || []));
  useEffect(() => { load(); }, []);
  const running = (list || []).some((a) => a.job && a.job.running);
  useEffect(() => {
    if (!running) return undefined;
    const t = setInterval(load, 1500);
    return () => clearInterval(t);
  }, [running]);
  return [list, load];
}

export function AgentList({ list, reload }) {
  const [err, setErr] = useState('');
  const act = async (path, body) => {
    setErr('');
    try { await api('POST', path, body); } catch (e) { setErr(String((e && e.message) || e)); }
    reload();
  };
  return html`<div class="su-list">
    ${list.map((a) => html`<${AgentRow} key=${a.alias} a=${a} act=${act} />`)}
    ${err ? html`<p class="hm-err" role="alert">${err}</p>` : null}
  </div>`;
}

/** Settings: every agent, connected or not. */
export function AgentsCard() {
  const [list, reload] = useAgents();
  return html`<div class="card scard"><div class="phead"><b>Agents</b><span class="mut sm">The AI helpers NeoBot works through. Set one up here: it installs and signs in on this computer.</span></div>
    ${list ? html`<${AgentList} list=${list} reload=${reload} />` : null}</div>`;
}
