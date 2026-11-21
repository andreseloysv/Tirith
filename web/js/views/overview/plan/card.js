// One project's plan card: the autopilot line, the project's bar, Main is red, and its steps.
import { html, useState } from '../../../../vendor/preact-htm.js';
import { ACTIVE } from '../../../common.js';
import { memo, hit } from '../bits.js';
import { Bar } from './items.js';
import { Step } from './step.js';
import { Autopilot } from './autopilot.js';
import { latestLanes, summarize, currentStep } from './steps.js';

function mainRed(red) {
  return html`<div class="ierr ovred" role="alert"><b>Main is red.</b> Merges wait until it is green again${typeof red === 'string' && red !== 'true' && red !== '1' ? html`: <span>${red}</span>` : '.'}</div>`;
}

/** One project's card; re-rendered only when its plan, search, lanesSig or pSig change (`lanes` itself is a new
    array on every store change and `p` is changed in place, so they are compared through their signatures). */
export const PlanCard = memo(function PlanCard({ p, plan, lanes, q, multi }) {
  const [modes, setModes] = useState({});
  const [adding, setAdding] = useState(null);
  const latest = latestLanes(lanes);
  const steps = summarize(plan, latest);
  const laneOf = (num) => (i) => latest[`${num}\n${i.text}`] || i.lane || null;
  const current = currentStep(steps);
  const total = steps.reduce((n, s) => n + s.total, 0), done = steps.reduce((n, s) => n + s.done, 0);
  const visible = q && !hit(q, p.name, p.id) ? steps.filter((s) => s.items.some((i) => hit(q, i.text, s.title))) : steps;
  if (q && !visible.length) return null;
  const working = lanes.filter((l) => ACTIVE.includes(l.status)).length;
  const red = plan.main_red || p.main_red;
  const modeOf = (s) => (q ? 'all' : modes[s.num] || (s.num === current ? 'quiet' : 'closed'));
  const add = (s) => (n) => { setAdding(n); if (n && modeOf(s) === 'closed') setModes((m) => ({ ...m, [s.num]: 'quiet' })); };
  const toggle = (s) => () => setModes((m) => ({ ...m, [s.num]: modeOf(s) === 'all' ? 'closed' : 'all' }));
  return html`<div class="ovplan">
    <${Autopilot} p=${p} working=${working} open=${total - done} total=${total} />
    <div class=${`card pcard${multi ? ' compact' : ''}`}>
      <div class="phead"><b>${p.name || p.id}</b><${Bar} done=${done} total=${total} cls="ptotal" /><span class="pcnt big">${done} / ${total}</span></div>
      ${red ? mainRed(red) : null}
      <ul class="psteps">${visible.map((s) => html`<${Step} key=${s.num} project=${p.id} s=${s} q=${q} laneOf=${laneOf(s.num)}
        mode=${modeOf(s)} adding=${adding === s.num} setAdding=${add(s)}
        onToggle=${toggle(s)} />`)}</ul>
    </div></div>`;
}, ['lanes']);
