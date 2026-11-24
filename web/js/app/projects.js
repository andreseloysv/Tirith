// One project in the menu: its arrow folds or opens it, its name opens its overview, and when open its own
// views (Pipelines, Agents, Rooms, Access) hang under it.
import { html } from '../../vendor/preact-htm.js';
import { Icon, ACTIVE, go } from '../common.js';
import { SUB, labelOf } from './views.js';
import { t } from '../i18n.js';

const CHEV = 'M9 6l6 6-6 6';

/* A name: an open project you are already in folds (same view); a closed one opens and goes to its overview;
   an open one you are not in just goes to its overview. */
function onName(remember, p, here, isOpen) {
  if (here && isOpen) { remember(p.id, false); return; }
  if (!isOpen) remember(p.id, true);
  go('overview', p.id);
}

function subViews(p, r, here) {
  return html`<div class="nsub">${SUB.map(([id, label]) => html`<button type="button" class=${`nav nsubi${here && r.view === id ? ' on' : ''}`}
                onClick=${() => go(id, p.id)}><span>${labelOf(label)}</span></button>`)}</div>`;
}

/** The menu row of project `p`, drawn inline (no component of its own, so the page's tree stays as it was). */
export function projectRow(p, { r, lanes, mem }) {
  const { open, folded, remember } = mem;
  const n = lanes.filter((l) => l.project === p.id && ACTIVE.includes(l.status)).length;
  const here = r.view !== 'bots' && r.project === p.id, isOpen = open.has(p.id) || (here && !folded.has(p.id));
  return html`<div class=${`nproj${isOpen ? ' open' : ''}`} key=${p.id}>
              <div class=${`nav nprow${here && r.view === 'overview' ? ' on' : here ? ' here' : ''}`}>
                <button type="button" class="nchev" aria-expanded=${isOpen} aria-label=${isOpen ? t('Fold {name}', { name: p.name || p.id }) : t('Open {name}', { name: p.name || p.id })}
                  onClick=${() => remember(p.id, !isOpen)}><${Icon} d=${CHEV} size=${12} w=${1.8} /></button>
                <button type="button" class="nname" onClick=${() => onName(remember, p, here, isOpen)}><i class="pdot" style=${`background:${p.color || '#9ca3af'}`}></i>
                  <span>${p.name || p.id}</span>${n ? html`<em title=${t('Agents working')}>${n}</em>` : null}</button>
              </div>
              ${isOpen ? subViews(p, r, here) : null}
            </div>`;
}
