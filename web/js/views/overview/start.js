// The Home's first thing on a new computer: while no agent is connected, nothing can work, so the agents to set up
// come first, each with its one button (web/js/views/setup/). Gone once one is connected; Settings keeps them all.
import { html } from '../../../vendor/preact-htm.js';
import { useAgents, AgentList } from '../setup/agents.js';

export function Start() {
  const [list, reload] = useAgents();
  if (!list || !list.length || list.some((a) => a.connected && !(a.job && a.job.running))) return null;
  return html`<section class="hm-sec" aria-label="Get started"><h2 class="hm-h">Get started</h2>
    <article class="hm-ask su-start">
      <p class="hm-q">OpenBot works through an AI helper you already use. Set up at least one: it installs and signs in right here.</p>
      <${AgentList} list=${list} reload=${reload} />
    </article></section>`;
}
