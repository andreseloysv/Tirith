// The Overview's working agents (the old agent cards): who works on what, what it does now, each opening Agents.
import { html } from '../../../vendor/preact-htm.js';
import { ICON, Icon, Span, ACTIVE, laneLabel, laneClass, go } from '../../common.js';
import { memo, Avatar, Rich, clip, hit } from './bits.js';
import { intentOf } from '../../intent.js';

const noCheck = (t) => String(t || '').replace(/\s*\(check:\s*`[^`]*`\)\s*/i, ' ').trim();
const AgentCard = memo(function AgentCard({ l, now, pname, you }) {
  const live = l.status === 'running';
  return html`<button class="card acard work" onClick=${() => go('agents', l.project)} title="Open the Agents view">
    <span class="ahead"><${Avatar} id=${l.id} size=${36} /><span class="aname"><b>${l.agent ? l.agent.charAt(0).toUpperCase() + l.agent.slice(1) : 'Worker'}</b>
      <span class="pill" title=${l.id}>${[l.id, l.model].filter(Boolean).join(' · ')}</span></span></span>
    ${l.item ? html`<span class="afeat"><${Rich} t=${noCheck(l.item)} /></span>` : null}
    ${now ? html`<span class=${`anow${live ? ' live' : ''}`}><i class="pdot2"></i><span class="md">${clip(now, 220)}</span></span>` : null}
    <span class="afoot"><span class=${`rb ${laneClass(l.status) === 'working' ? 'running' : 'pending'}`}>${you ? 'Waiting for you' : laneLabel(l.status)}</span>
      ${l.step ? html`<span class="chip astep" title="Plan step">Step ${l.step}</span>` : null}
      <span class="aproj">${pname}</span>
      <span class="aclock"><${Icon} d=${ICON.clock} size=${13} w=${1.7} /><${Span} start=${l.started_at || l.created_at} /></span></span>
  </button>`;
});

/** Each lane's latest event in words, newest first. */
function lastEvents(events) {
  const last = {};
  for (let i = events.length - 1; i >= 0; i--) { const e = events[i]; if (e.lane && !last[e.lane] && e.summary) last[e.lane] = `${intentOf(e)}…`; }
  return last;
}

export function Agents({ lanes, events, projects, project, q, needs }) {
  const act = lanes.filter((l) => ACTIVE.includes(l.status) && (!project || l.project === project) && hit(q, l.item, l.id, l.agent, l.project))
    .sort((a, b) => String(a.created_at).localeCompare(String(b.created_at)));
  const last = lastEvents(events);
  const pname = (id) => (project ? '' : (projects.find((x) => x.id === id) || {}).name || id);
  if (project && !act.length) return null;
  return html`<section><h2>Working agents <span class="n">${act.length}</span></h2>
    ${act.length ? html`<div class="grid2">${act.map((l) => html`<${AgentCard} key=${l.id} l=${l} now=${(l.status === 'waiting' && l.question) || last[l.id] || l.note} pname=${pname(l.project)} you=${needs.some((o) => o.lane === l.id && o.kind !== 'lane')} />`)}</div>`
      : html`<p class="quiet">No agents working</p>`}</section>`;
}
