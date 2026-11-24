// "Up next" in a project's Overview: what its plan still holds that nobody works on yet, as short titles in the
// plan's order, and whether the autopilot starts them by itself. The whole plan (steps, adding items) is one click
// away; what is done is never listed here.
import { html, useState } from '../../../vendor/preact-htm.js';
import { ACTIVE } from '../../common.js';
import { usePlan } from './plan/items.js';
import { latestLanes, summarize } from './plan/steps.js';
import { Autopilot } from './plan/autopilot.js';
import { Plan } from './plan.js';
import { titleOf } from './crew.js';
import { t } from '../../i18n.js';

const SHOW = 5;

export function Next({ p, lanes, q }) {
  const { data } = usePlan(p.id);
  const [whole, setWhole] = useState(false);
  if (!data) return null;
  const latest = latestLanes(lanes);
  const steps = summarize(data, latest);
  const open = steps.flatMap((s) => s.items.filter((i) => !i.done).map((i) => ({ text: i.text, lane: latest[`${s.num}\n${i.text}`] || i.lane })));
  const todo = open.filter((i) => !(i.lane && ACTIVE.includes(i.lane.status))); /* at work: in "Working now" */
  const total = steps.reduce((n, s) => n + s.total, 0);
  const working = lanes.filter((l) => ACTIVE.includes(l.status)).length;
  const more = todo.length > SHOW ? t('See all {n} in the plan', { n: todo.length }) : t('See the whole plan');
  return html`<section class="hm-sec" aria-label=${t('Up next')}><h2 class="hm-h">${t('Up next')}${todo.length ? html`<span class="hm-n">${todo.length}</span>` : null}</h2>
    <${Autopilot} p=${p} working=${working} open=${open.length} total=${total} />
    ${todo.length ? html`<ol class="hm-next">${todo.slice(0, SHOW).map((i) => html`<li key=${i.text}>${titleOf(i.text)}</li>`)}</ol>`
      : html`<p class="hm-none">${total ? t('Nothing left to start in the plan.') : t('Nothing planned yet: ask the Project Manager in the chat.')}</p>`}
    <button type="button" class="hm-more" aria-expanded=${whole} onClick=${() => setWhole(!whole)}>${whole ? t('Hide the plan') : more}</button>
    ${whole ? html`<${Plan} projects=${[p]} lanes=${lanes} q=${q} />` : null}
  </section>`;
}
