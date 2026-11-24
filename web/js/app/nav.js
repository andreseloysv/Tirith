// The ONE sidebar: the agents' faces at its far left, and next to them the menu (Overview, Connectors, Settings and
// the projects, folded). Whether the page is live is said only when it is not, over the view (shell.js).
import { html } from '../../vendor/preact-htm.js';
import { useStore } from '../store.js';
import { ICON, Icon, go } from '../common.js';
import { TOP } from './views.js';
import { useNavMemory } from './memory.js';
import { Faces } from './faces.js';
import { NavResizer } from './resize.js';
import { projectRow } from './projects.js';

function topViews(r) {
  return TOP.map(([id, label]) => html`<button class=${`nav${r.view === id && !r.project ? ' on' : ''}`} onClick=${() => go(id, '')}>
            <${Icon} d=${ICON[id] || ICON.overview} size=${16} w=${1.6} /><span>${label}</span></button>`);
}

function addProject(onAdd) {
  return html`<div class="grp grp-add"><span>Projects</span><button type="button" class="grp-plus" aria-label="Add project" title="Add project"
            onClick=${onAdd}><${Icon} d="M12 5v14 M5 12h14" size=${14} w=${2} /></button></div>`;
}

export function Nav({ r, onAdd }) {
  const s = useStore();
  const mem = useNavMemory();
  const lanes = Object.values(s.lanes);
  return html`<aside id="nav">
    <${Faces} r=${r} />
    <div class="nmenu">
      <div id="navlist">
        <div class="brand"><span>NeoBot</span></div>
        <nav>
          ${topViews(r)}
          ${addProject(onAdd)}
          ${s.projects.map((p) => projectRow(p, { r, lanes, mem }))}
        </nav>
      </div>
    </div>
    <${NavResizer} />
  </aside>`;
}
