// The shell: sidebar (views, projects and the "+" that adds one), header (crumb, search, health) and the current view.
// Same markup and classes as the old page, so css/style.css draws it the same way.
import { html, render, useState, useEffect } from '../vendor/preact-htm.js';
import { useStore, useFetch, setUi } from './store.js';
import { ICON, Icon, ACTIVE, route, go } from './common.js';
import { Overview } from './views/overview.js';
import { Pipelines } from './views/pipelines.js';
import { Agents } from './views/agents.js';
import { Rooms } from './views/rooms.js';
import { Access } from './views/access.js';
import { Settings } from './views/settings.js';
import { AddProject } from './views/addproject.js';

const VIEWS = [
  ['overview', 'Overview', Overview],
  ['pipelines', 'Pipelines', Pipelines],
  ['agents', 'Agents', Agents],
  ['rooms', 'Rooms', Rooms],
  ['accesos', 'Access', Access],
  ['settings', 'Settings', Settings],
];

const BRAND = html`<svg width="22" height="22" viewBox="0 0 24 24" aria-hidden="true"><defs><linearGradient id="lg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#6fdccb"/><stop offset=".45" stop-color="#79a8ff"/><stop offset=".75" stop-color="#c79bf2"/><stop offset="1" stop-color="#ffb299"/></linearGradient></defs><path d="M12 2l8 8-8 12L4 10z" fill="url(#lg)" fill-opacity=".85"/><path d="M12 2l8 8H4z" fill="#fff" fill-opacity=".45"/><path d="M12 2l8 8-8 12L4 10z" fill="none" stroke="#fff" stroke-opacity=".9"/></svg>`;

function useRoute() {
  const [r, set] = useState(route());
  useEffect(() => {
    const on = () => set(route());
    addEventListener('hashchange', on);
    return () => removeEventListener('hashchange', on);
  }, []);
  return r;
}

function Nav({ r, onAdd }) {
  const s = useStore();
  const lanes = Object.values(s.lanes);
  return html`<aside id="nav">
    <div id="navlist">
      <div class="brand">${BRAND}<span>Tirith</span></div>
      <nav>
        <div class="grp">Views</div>
        ${VIEWS.map(([id, label]) => html`<button class=${`nav${r.view === id ? ' on' : ''}`} onClick=${() => go(id, r.project)}>
          <${Icon} d=${ICON[id] || ICON.overview} size=${16} w=${1.7} /><span>${label}</span></button>`)}
        <div class="grp grp-add"><span>Projects</span><button type="button" class="grp-plus" aria-label="Add project" title="Add project"
          onClick=${onAdd}><${Icon} d="M12 5v14 M5 12h14" size=${14} w=${2} /></button></div>
        ${s.projects.map((p) => {
          const n = lanes.filter((l) => l.project === p.id && ACTIVE.includes(l.status)).length;
          return html`<button class=${`nav${r.project === p.id ? ' on' : ''}`} onClick=${() => go(r.view, r.project === p.id ? '' : p.id)}>
            <i class="pdot" style=${`background:${p.color || '#9ca3af'}`}></i><span>${p.name || p.id}</span>${n ? html`<em>${n}</em>` : null}</button>`;
        })}
      </nav>
    </div>
    <div id="src" class="src">${s.connected ? html`<i class="ok"></i> live` : html`<i class="bad"></i> reconnecting…`}</div>
  </aside>`;
}

function Top({ r }) {
  const s = useStore();
  const [metrics] = useFetch('/api/metrics', ['metrics']);
  const red = (metrics && metrics.metrics || []).filter((m) => m.red);
  const label = (VIEWS.find((v) => v[0] === r.view) || VIEWS[0])[1];
  const project = s.projects.find((p) => p.id === r.project);
  const crumb = [label, project && (project.name || project.id)].filter(Boolean);
  useEffect(() => {
    const key = (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); document.getElementById('f-q').focus(); }
    };
    addEventListener('keydown', key);
    return () => removeEventListener('keydown', key);
  }, []);
  return html`<header id="top">
    <div id="crumb" class="crumb">${crumb.map((c, i) => html`${i ? html`<span class="sep">/</span>` : null}<span class=${i === crumb.length - 1 ? 'last' : ''}>${c}</span>`)}</div>
    <label class="cmd"><${Icon} d="M11 4a7 7 0 100 14 7 7 0 000-14z M20 20l-4-4" size=${14} w=${2} />
      <input id="f-q" type="search" placeholder="Search" aria-label="Search" autocomplete="off" value=${s.q || ''}
        onInput=${(e) => setUi('q', e.target.value)} /><kbd>⌘K</kbd></label>
    <button id="health-btn" class=${`health-btn ${red.length ? 'red' : 'green'}`} type="button"
      title=${red.length ? red.map((m) => m.label).join(', ') : 'Healthy'} onClick=${() => go('overview', r.project)}><i class="dot"></i></button>
  </header>`;
}

function App() {
  const r = useRoute();
  const s = useStore();
  const [adding, setAdding] = useState(false);
  const View = (VIEWS.find((v) => v[0] === r.view) || VIEWS[0])[2];
  return html`<div id="shell">
    <${Nav} r=${r} onAdd=${() => setAdding(true)} />
    <div id="content">
      <${Top} r=${r} />
      ${!s.connected && s.loaded ? html`<div id="stale" class="stale" role="status">Tirith is not updating — reconnecting…</div>` : null}
      <div id="view" class=${`view v-${r.view}`}>${s.loaded ? html`<${View} project=${r.project} />` : null}</div>
    </div>
  </div>${adding ? html`<${AddProject} onClose=${() => setAdding(false)} />` : null}`;
}

render(html`<${App} />`, document.getElementById('app'));
