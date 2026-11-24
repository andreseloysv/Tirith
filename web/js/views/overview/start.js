// The Home's first thing on a new computer: while no agent is connected, nothing can work, so the agents to set up
// come first, each with its one button (web/js/views/setup/). Gone once one is connected; Settings keeps them all.
import { html } from '../../../vendor/preact-htm.js';
import { AgentList } from '../setup/agents.js';

/** Whether the Home starts with the agents: some are known and none works yet. */
export const starting = (list) => !!list && list.length > 0 && !list.some((a) => a.connected && !(a.job && a.job.running));

export function Start({ list, reload }) {
  if (!starting(list)) return null;
  return html`<section class="hm-sec" aria-label="Get started"><h2 class="hm-h">Get started</h2>
    <article class="hm-ask su-start">
      <p class="hm-q">OpenBot works through an AI helper you already use. Set up at least one: it installs and signs in right here.</p>
      <${AgentList} list=${list} reload=${reload} />
    </article></section>`;
}
