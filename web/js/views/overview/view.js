// Overview: the old page's .ovg grid. With no project selected the main column shows everything: status (metrics
// and the CEO's last pass), every plan, what needs you, running pipelines, working agents. Inside a project it
// shows that project only: its plan, its cards, and its setup, pipelines and agents when there is something to see;
// the health of the whole and the CEO stay out. The chat on the right is the selected project's conversation.
import { html } from '../../../vendor/preact-htm.js';
import { useStore, useFetch } from '../../store.js';
import { ACTIVE } from '../../common.js';
import { Chat } from '../chat.js';
import { Plan } from './plan.js';
import { openCards } from '../need.js';
import { Status } from './status.js';
import { Needs, Setup } from './setup.js';
import { Pipelines } from './pipes.js';
import { Agents } from './agents.js';

export function Overview({ project }) {
  const s = useStore();
  const [metrics] = useFetch(project ? '' : '/api/metrics', ['metrics', 'review']);
  const q = String(s.q || '').trim().toLowerCase();
  const lanes = Object.values(s.lanes);
  const projects = project ? s.projects.filter((p) => p.id === project) : s.projects;
  const working = lanes.filter((l) => ACTIVE.includes(l.status) && (!project || l.project === project)).length;
  const p = s.projects.find((x) => x.id === project);
  const needs = openCards(s);
  return html`<div class="ovg"><div class="ovmain">
      ${project ? null : html`<${Status} data=${metrics} working=${working} />`}
      <${Plan} projects=${projects} lanes=${lanes} q=${q} />
      <${Needs} owner=${needs} project=${project} projects=${s.projects} q=${q} />
      ${p ? html`<${Setup} project=${p.id} name=${p.name} />` : null}
      <${Pipelines} project=${project} projects=${s.projects} q=${q} />
      <${Agents} lanes=${lanes} events=${s.events} projects=${s.projects} project=${project} q=${q} needs=${needs} />
    </div>
    <aside class="ovside"><${Chat} key=${project || ''} project=${project || ''} /></aside></div>`;
}
