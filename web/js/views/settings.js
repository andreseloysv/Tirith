// Settings view (the old v-models.js, v-theme.js and the setup cards): Appearance (System / Light / Dark), the
// models of each role, the commit guard and the autopilot of each project, and the brain's status.
import { html, useState } from '../../vendor/preact-htm.js';
import { useStore, useFetch, api } from '../store.js';
import { RoleCard, ROLES } from './settings/models.js';
import { useAction } from './settings/ui.js';

const THEME_KEY = 'tirith.theme';
const THEMES = [['system', 'System'], ['light', 'Light'], ['dark', 'Dark']];

function getTheme() {
  try { const v = localStorage.getItem(THEME_KEY); return v === 'light' || v === 'dark' ? v : 'system'; } catch { return 'system'; }
}
function setTheme(mode) {
  try { if (mode === 'system') localStorage.removeItem(THEME_KEY); else localStorage.setItem(THEME_KEY, mode); } catch { /* private window */ }
  if (mode === 'light' || mode === 'dark') document.documentElement.dataset.theme = mode;
  else delete document.documentElement.dataset.theme;
}

function Appearance() {
  const [cur, set] = useState(getTheme());
  return html`<div class="card scard themecard"><div class="phead"><b>Appearance</b><span class="mut sm">System follows the light or dark setting of your computer</span></div>
    <div class="seg" role="group" aria-label="Theme">${THEMES.map(([k, l]) => html`<button type="button" aria-pressed=${k === cur} class=${k === cur ? 'on' : ''}
      onClick=${() => { setTheme(k); set(k); }}>${l}</button>`)}</div></div>`;
}

function GuardCard({ project }) {
  const [list, reload, err] = useFetch('/api/guard', ['event']);
  const [busy, error, run] = useAction();
  const rows = (list || []).filter((g) => !project || g.project === project);
  return html`<div class="card scard"><div class="phead"><b>Commit guard</b><span class="mut sm">Git hooks that refuse a commit with one of your forbidden words (words from config.toml)</span></div>
    ${err && !list ? html`<div class="ierr">${err.message}</div>` : null}
    ${rows.map((g) => {
      const none = !g.words;
      return html`<div class="srow2 scheck ${g.ok ? 'ok' : 'bad'}" key=${g.project}>
        <span class=${`dot ${none ? 'grey' : g.ok ? 'green' : 'red'}`}></span><b>${g.name || g.project}</b>
        <span class="mut sm grow">${none ? 'No forbidden words: no guard' : g.ok ? `Installed · ${g.words} ${g.words === 1 ? 'word' : 'words'}` : g.reason}</span>
        ${!g.ok && !none ? html`<button class="sadd" disabled=${busy === g.project}
          onClick=${async () => { await run(g.project, () => api('POST', `/api/guard/${encodeURIComponent(g.project)}/install`)); reload(); }}>${busy === g.project ? 'Installing…' : 'Install'}</button>` : null}
      </div>`;
    })}
    ${list && !rows.length ? html`<p class="mut sm">No projects.</p>` : null}
    ${error ? html`<div class="ierr">${error}</div>` : null}</div>`;
}

function AutopilotCard({ project }) {
  const s = useStore();
  const [busy, error, run] = useAction();
  const ps = s.projects.filter((p) => !project || p.id === project);
  return html`<div class="card scard"><div class="phead"><b>Autopilot</b><span class="mut sm">Starts the open items of the plan by itself, step after step</span></div>
    ${ps.map((p) => html`<div class="srow2" key=${p.id}><i class="pdot" style=${`background:${p.color || '#9ca3af'}`}></i><b>${p.name || p.id}</b>
      <span class="mut sm grow">${p.autopilot ? 'On: the plan runs by itself' : 'Paused: only you or the manager start work'}</span>
      <div class="seg" role="group" aria-label=${`Autopilot of ${p.name || p.id}`}>${[[true, 'On'], [false, 'Off']].map(([on, l]) => html`<button type="button"
        class=${!!p.autopilot === on ? 'on' : ''} aria-pressed=${!!p.autopilot === on} disabled=${busy === p.id}
        onClick=${() => !!p.autopilot !== on && run(p.id, () => api('POST', `/api/autopilot/${encodeURIComponent(p.id)}`, { on }))}>${l}</button>`)}</div></div>`)}
    ${error ? html`<div class="ierr">${error}</div>` : null}</div>`;
}

function BrainCard() {
  const [b] = useFetch('/api/brain', ['models']);
  if (!b) return null;
  return html`<div class="card scard"><div class="phead"><b>Brain status</b><span class="mut sm">Summaries of rooms and projects, through the model API</span></div>
    <div class="scheck ${b.enabled ? 'ok' : 'bad'}"><span class=${`dot ${b.enabled ? 'green' : 'grey'}`}></span><b>${b.enabled ? 'On' : 'Off'}</b>
      <span class="mut sm">${b.enabled ? `· ${b.model}` : `· ${b.reason || ''}`}</span></div>
    <div class="bkv mut"><span>Model <b>${b.model}</b></span><span>API <b>${b.url}</b></span><span>Calls <b>${b.calls}</b></span><span>Cached answers <b>${b.cached}</b></span></div>
    ${b.last_error ? html`<div class="ierr">Last error: ${b.last_error}</div>` : null}</div>`;
}

export function Settings({ project }) {
  const [M, reload, err] = useFetch('/api/models', ['models']);
  return html`<section class="msettings">
    <${Appearance} />
    <p class="mut sm mlead">Model and effort come from <code>config.toml</code> under <code>[models]</code>; what you save here overrides it until you go back to config.toml.</p>
    ${err && !M ? html`<div class="ierr">Could not read the models: ${err.message}</div>` : null}
    ${M ? ROLES.map(([id, label, note]) => html`<${RoleCard} key=${id} id=${id} label=${label} note=${note} M=${M} reload=${reload} />`) : null}
    <${GuardCard} project=${project} />
    <${AutopilotCard} project=${project} />
    <${BrainCard} />
  </section>`;
}
