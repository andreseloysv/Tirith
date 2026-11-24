// The Plan section of the Overview: one card per project (or only the selected one), each read on its own.
import { html } from '../../../../vendor/preact-htm.js';
import { usePlan } from './items.js';
import { PlanCard } from './card.js';
import { t } from '../../../i18n.js';

function PlanProject({ p, lanes, q, multi }) {
  const { data } = usePlan(p.id);
  if (!data) return null;
  if (!(data.steps || []).length) return multi ? null : html`<p class="quiet">${t('This project has no plan yet: ask its manager, in the chat, to write one.')}</p>`;
  const lanesSig = lanes.map((l) => `${l.id}:${l.status}`).join(',');
  const pSig = `${p.name}|${p.autopilot}|${p.main_red}`;
  return html`<${PlanCard} p=${p} plan=${data} lanes=${lanes} lanesSig=${lanesSig} pSig=${pSig} q=${q} multi=${multi} />`;
}

/** The Plan section: every project, or only `project`. */
export function Plan({ projects, lanes, q }) {
  if (!projects.length) return null;
  return html`<section class="plan"><h2>${t('Plan')}</h2>
    ${projects.map((p) => html`<${PlanProject} key=${p.id} p=${p} lanes=${lanes.filter((l) => l.project === p.id)} q=${q} multi=${projects.length > 1} />`)}
  </section>`;
}
