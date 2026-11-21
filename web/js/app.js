// The shell: ONE sidebar, the header (crumb, search, health) and the current view. The sidebar is one surface: at its far
// left the bots' big faces (each in its state, with its unread answers; the "+" adds a bot), and next to them the menu:
// Overview, Settings and the projects, folded. A face opens its bot (#/bots/<id>); a project opens into its own Pipelines,
// Agents, Rooms and Access (#/<view>/<project>), and its name opens its overview (plan and manager). Which projects are
// open is remembered; the selected one always is. The bot's card (new or edit) is drawn here, over any view.
import { html, render, useState, useEffect } from '../vendor/preact-htm.js';
import { useStore, useFetch, setUi } from './store.js';
import { useUnread } from './unread.js';
import { ICON, Icon, ACTIVE, route, go } from './common.js';
import { Overview } from './views/overview.js';
import { Pipelines } from './views/pipelines.js';
import { Agents } from './views/agents.js';
import { Bots, BotCard, stateOf, keyOf, recall } from './views/bots.js';
import { Face, NewFace } from './views/bots/face.js';
import { Rooms } from './views/rooms.js';
import { Access } from './views/access.js';
import { Settings } from './views/settings.js';
import { AddProject } from './views/addproject.js';

/** Views of everything, at the top of the menu (the bots are their faces); views of one project, under it. */
const TOP = [['overview', 'Overview', Overview], ['settings', 'Settings', Settings]];
const BOTS = ['bots', 'Bots', Bots];
const SUB = [['pipelines', 'Pipelines', Pipelines], ['agents', 'Agents', Agents], ['rooms', 'Rooms', Rooms], ['accesos', 'Access', Access]];
const VIEWS = [...TOP, BOTS, ...SUB];
const OPEN_KEY = 'tirith.nav.open';
const CHEV = 'M9 6l6 6-6 6';
function loadOpen() { try { return new Set(JSON.parse(localStorage.getItem(OPEN_KEY) || '[]')); } catch { return new Set(); } }
function saveOpen(s) { try { localStorage.setItem(OPEN_KEY, JSON.stringify([...s])); } catch { /* private window */ } }

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

/** The bots' faces, at the far left of the sidebar. */
function Faces({ r }) {
  const s = useStore();
  const unread = useUnread();
  const sel = r.view === 'bots' ? r.project || recall() || (unread.bots[0] || {}).id : '';
  return html`<nav class="nbots" aria-label="Bots">
    <div class="nlogo" title="Tirith">${BRAND}</div>
    ${unread.bots.filter((b) => b.show !== false).map((b) => {
      const n = b.id === sel ? 0 : unread.by[b.id] || 0;
      return html`<button type="button" key=${b.id} class=${`nbot${b.id === sel ? ' on' : ''}`} title=${b.name} aria-label=${n ? `${b.name}, ${n} new` : b.name}
        aria-current=${b.id === sel} onClick=${() => go('bots', b.id)}><${Face} shape=${b.shape} color=${b.color} size=${64} state=${stateOf(b, !!s.busy[keyOf(b)])} />
        ${n ? html`<em class="nunread">${n > 9 ? '9+' : n}</em>` : null}</button>`;
    })}
    <button type="button" class="nbot nnew" title="New bot" aria-label="New bot" onClick=${() => setUi('botCard', {})}><${NewFace} size=${46} /></button>
  </nav>`;
}

function Nav({ r, onAdd }) {
  const s = useStore();
  const [open, setOpen] = useState(loadOpen);
  const lanes = Object.values(s.lanes);
  const toggle = (id) => { const n = new Set(open); if (n.has(id)) n.delete(id); else n.add(id); setOpen(n); saveOpen(n); };
  return html`<aside id="nav">
    <${Faces} r=${r} />
    <div class="nmenu">
      <div id="navlist">
        <div class="brand"><span>Tirith</span></div>
        <nav>
          ${TOP.map(([id, label]) => html`<button class=${`nav${r.view === id && !r.project ? ' on' : ''}`} onClick=${() => go(id, '')}>
            <${Icon} d=${ICON[id] || ICON.overview} size=${16} w=${1.6} /><span>${label}</span></button>`)}
          <div class="grp grp-add"><span>Projects</span><button type="button" class="grp-plus" aria-label="Add project" title="Add project"
            onClick=${onAdd}><${Icon} d="M12 5v14 M5 12h14" size=${14} w=${2} /></button></div>
          ${s.projects.map((p) => {
            const n = lanes.filter((l) => l.project === p.id && ACTIVE.includes(l.status)).length;
            const here = r.view !== 'bots' && r.project === p.id, isOpen = here || open.has(p.id);
            return html`<div class=${`nproj${isOpen ? ' open' : ''}`} key=${p.id}>
              <div class=${`nav nprow${here && r.view === 'overview' ? ' on' : here ? ' here' : ''}`}>
                <button type="button" class="nchev" aria-expanded=${isOpen} aria-label=${isOpen ? `Fold ${p.name || p.id}` : `Open ${p.name || p.id}`}
                  onClick=${() => toggle(p.id)}><${Icon} d=${CHEV} size=${12} w=${1.8} /></button>
                <button type="button" class="nname" onClick=${() => go('overview', p.id)}><i class="pdot" style=${`background:${p.color || '#9ca3af'}`}></i>
                  <span>${p.name || p.id}</span>${n ? html`<em title="Agents working">${n}</em>` : null}</button>
              </div>
              ${isOpen ? html`<div class="nsub">${SUB.map(([id, label]) => html`<button type="button" class=${`nav nsubi${here && r.view === id ? ' on' : ''}`}
                onClick=${() => go(id, p.id)}><span>${label}</span></button>`)}</div>` : null}
            </div>`;
          })}
        </nav>
      </div>
      <div id="src" class="src">${s.connected ? html`<i class="ok"></i> live` : html`<i class="bad"></i> reconnecting…`}</div>
    </div>
  </aside>`;
}

function Top({ r }) {
  const s = useStore();
  const [metrics] = useFetch('/api/metrics', ['metrics']);
  const red = (metrics && metrics.metrics || []).filter((m) => m.red);
  const label = (VIEWS.find((v) => v[0] === r.view) || VIEWS[0])[1];
  const [bots] = useFetch(r.view === 'bots' ? '/api/bots' : '', ['bots']);
  const bot = r.view === 'bots' && bots && (bots.find((b) => b.id === (r.project || recall())) || bots[0]);
  const project = SUB.some((v) => v[0] === r.view) || r.view === 'overview' ? s.projects.find((p) => p.id === r.project) : null;
  const crumb = bot ? [label, bot.name] : project ? [project.name || project.id, r.view === 'overview' ? '' : label].filter(Boolean) : [label];
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
      title=${`All projects: ${red.length ? red.map((m) => m.label).join(', ') : 'healthy'}`} onClick=${() => go('overview', '')}><i class="dot"></i></button>
  </header>`;
}

function App() {
  const r = useRoute();
  const s = useStore();
  const [adding, setAdding] = useState(false);
  useEffect(() => { setUi('q', ''); }, [r.project]); /* a search belongs to the project it was typed in */
  const View = (VIEWS.find((v) => v[0] === r.view) || VIEWS[0])[2];
  const project = r.view === 'settings' ? '' : r.project; /* for the bots view: the bot's id */
  const card = s.botCard;
  const saved = (b) => { setUi('botCard', null); go('bots', b ? b.id : ''); };
  return html`<div id="shell">
    <${Nav} r=${r} onAdd=${() => setAdding(true)} />
    <div id="content">
      <${Top} r=${r} />
      ${!s.connected && s.loaded ? html`<div id="stale" class="stale" role="status">Tirith is not updating — reconnecting…</div>` : null}
      <div id="view" class=${`view v-${r.view}`}>${s.loaded ? html`<${View} project=${project} />` : null}</div>
    </div>
  </div>${adding ? html`<${AddProject} onClose=${() => setAdding(false)} />` : null}
  ${card ? html`<${BotCard} key=${card.id || 'new'} bot=${card} onClose=${() => setUi('botCard', null)} onSaved=${saved} />` : null}`;
}

render(html`<${App} />`, document.getElementById('app'));
