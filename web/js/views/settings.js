// Settings view (the old v-models.js, v-theme.js and the setup cards): Appearance (System / Light / Dark), Access
// behind its button (#/settings/access), the factory's health, the models of the roles with no screen of their own
// (the rest pick theirs on their screen), the commit guard and the autopilot of each project. Inside a project
// its own two cards (autopilot, commit guard) come first; the rest is the same for every project, and says so.
// With no project there is no factory to set: only the agents, the look, the language and the access.
import { html, useState } from '../../vendor/preact-htm.js';
import { useStore, useFetch, api } from '../store.js';
import { RoleCard, roleList } from './settings/models.js';
import { useAction } from './settings/ui.js';
import { AccessCard, AccessPage } from './settings/access.js';
import { Health } from './settings/health.js';
import { AgentsCard } from './setup/agents.js';
import { LanguageCard } from './settings/language.js';
import { solo } from '../app/solo.js';
import { t } from '../i18n.js';

const THEME_KEY = 'tirith.theme';
const themes = () => [['system', t('System')], ['light', t('Light')], ['dark', t('Dark')]];

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
  return html`<div class="card scard themecard"><div class="phead"><b>${t('Appearance')}</b><span class="mut sm">${t('System follows the light or dark setting of your computer')}</span></div>
    <div class="seg" role="group" aria-label=${t('Theme')}>${themes().map(([k, l]) => html`<button type="button" aria-pressed=${k === cur} class=${k === cur ? 'on' : ''}
      onClick=${() => { setTheme(k); set(k); }}>${l}</button>`)}</div></div>`;
}

function GuardCard({ project }) {
  const [list, reload, err] = useFetch('/api/guard', ['event']);
  const [busy, error, run] = useAction();
  const rows = (list || []).filter((g) => !project || g.project === project);
  return html`<div class="card scard"><div class="phead"><b>${t('Commit guard')}</b><span class="mut sm">${t('Git hooks that refuse a commit with one of your forbidden words (words from config.toml)')}</span></div>
    ${err && !list ? html`<div class="ierr">${err.message}</div>` : null}
    ${rows.map((g) => {
      const none = !g.words;
      return html`<div class="srow2 scheck ${g.ok ? 'ok' : 'bad'}" key=${g.project}>
        <span class=${`dot ${none ? 'grey' : g.ok ? 'green' : 'red'}`}></span><b>${g.name || g.project}</b>
        <span class="mut sm grow">${none ? t('No forbidden words: no guard') : g.ok ? (g.words === 1 ? t('Installed · 1 word') : t('Installed · {n} words', { n: g.words })) : t(g.reason)}</span>
        ${!g.ok && !none ? html`<button class="sadd" disabled=${busy === g.project}
          onClick=${async () => { await run(g.project, () => api('POST', `/api/guard/${encodeURIComponent(g.project)}/install`)); reload(); }}>${busy === g.project ? t('Installing…') : t('Install')}</button>` : null}
      </div>`;
    })}
    ${list && !rows.length ? html`<p class="mut sm">${t('No projects.')}</p>` : null}
    ${error ? html`<div class="ierr">${error}</div>` : null}</div>`;
}

function AutopilotCard({ project }) {
  const s = useStore();
  const [busy, error, run] = useAction();
  const ps = s.projects.filter((p) => !project || p.id === project);
  return html`<div class="card scard"><div class="phead"><b>${t('Autopilot')}</b><span class="mut sm">${t('Starts the open items of the plan by itself, step after step')}</span></div>
    ${ps.map((p) => html`<div class="srow2" key=${p.id}><i class="pdot" style=${`background:${p.color || '#9ca3af'}`}></i><b>${p.name || p.id}</b>
      <span class="mut sm grow">${p.autopilot ? t('On: the plan runs by itself') : t('Paused: only you or the manager start work')}</span>
      <div class="seg" role="group" aria-label=${t('Autopilot of {name}', { name: p.name || p.id })}>${[[true, t('On')], [false, t('Off')]].map(([on, l]) => html`<button type="button"
        class=${!!p.autopilot === on ? 'on' : ''} aria-pressed=${!!p.autopilot === on} disabled=${busy === p.id}
        onClick=${() => !!p.autopilot !== on && run(p.id, () => api('POST', `/api/autopilot/${encodeURIComponent(p.id)}`, { on }))}>${l}</button>`)}</div></div>`)}
    ${error ? html`<div class="ierr">${error}</div>` : null}</div>`;
}

/* the factory's part, shown once there is a project (app/solo.js): its health and its roles' models */
function factory(M, reload, err) {
  return html`<${Health} />
    <p class="mut sm mlead">${t('Roles come from the factory; their model and effort from config.toml under [roles]. What you save here overrides it until you go back to config.toml.')}</p>
    <p class="mut sm mlead">${t('Only the roles with no screen of their own are here. The others choose their model on their own screen: the Project Manager in each project\'s chat, and each bot on its bot screen.')}</p>
    ${err && !M ? html`<div class="ierr">${t('Could not read the models: {message}', { message: err.message })}</div>` : null}
    ${M ? roleList(M).map(([id, label, note]) => html`<${RoleCard} key=${id} id=${id} label=${label} note=${note} M=${M} reload=${reload} />`) : null}`;
}

export function Settings({ project, sub }) {
  const alone = solo(useStore());
  const [M, reload, err] = useFetch(sub === 'access' || alone ? '' : '/api/models', ['models']);
  if (sub === 'access') return html`<${AccessPage} />`;
  const own = html`<${AutopilotCard} project=${project} /><${GuardCard} project=${project} />`;
  return html`<section class="msettings">
    ${project ? html`${own}<h2 class="rsec">${t('For all projects')}</h2>` : null}
    <${AgentsCard} />
    <${Appearance} />
    <${LanguageCard} />
    <${AccessCard} />
    ${alone ? null : factory(M, reload, err)}
    ${project || alone ? null : own}
  </section>`;
}
