// The agents NeoBot works through, set up from the page with one tap each (no terminal, no window): the daemon
// installs one with its maker's installer, signs it in through the browser, points its hooks here and checks it with
// one short real turn (/api/setup/agents). The list lives in the store (`setupAgents`): until one agent has really
// worked here the whole page is the setup (welcome.js, app/shell.js); after that Settings keeps them all.
import { html, useState, useEffect } from '../../../vendor/preact-htm.js';
import { api, S, setUi, useStore } from '../../store.js';
import { AgentRow } from './row.js';

/** Asks the daemon again; what it says goes to the store, the last answer when it cannot say. */
export function reloadAgents() {
  const got = api('GET', '/api/setup/agents').then((r) => r.agents || [], () => S.setupAgents || []);
  return got.then((list) => setUi('setupAgents', list));
}

/** The shell's: the list once, then every 1.5 s while a setup runs. Undefined until the daemon has said. */
export function useAgents() {
  const list = useStore().setupAgents;
  useEffect(() => { reloadAgents(); }, []);
  const running = (list || []).some((a) => a.job && a.job.running);
  useEffect(() => {
    if (!running) return undefined;
    const t = setInterval(reloadAgents, 1500);
    return () => clearInterval(t);
  }, [running]);
  return list;
}

/** No agent has worked here yet: nothing else can be used, so the page is the setup alone. */
export const firstRun = (list) => Array.isArray(list) && list.length > 0 && !list.some((a) => a.connected);

export function AgentList({ list }) {
  const [err, setErr] = useState('');
  const act = async (path, body) => {
    setErr('');
    try { await api('POST', path, body); } catch (e) { setErr(String((e && e.message) || e)); }
    reloadAgents();
  };
  return html`<div class="su-list">
    ${list.map((a) => html`<${AgentRow} key=${a.alias} a=${a} act=${act} />`)}
    ${err ? html`<p class="hm-err" role="alert">${err}</p>` : null}
    <p class="su-note">NeoBot's rules then watch that helper's other sessions on this computer too.</p>
  </div>`;
}

/** Settings: every agent, connected or not. */
export function AgentsCard() {
  const list = useStore().setupAgents;
  return html`<div class="card scard"><div class="phead"><b>Agents</b><span class="mut sm">The AI helpers NeoBot works through. Set one up here: it installs and signs in on this computer.</span></div>
    ${list ? html`<${AgentList} list=${list} />` : null}</div>`;
}
