// The header: the crumb (where you are) and the search (⌘K). The factory's health is in Settings.
import { html, useEffect } from '../../vendor/preact-htm.js';
import { useStore, useFetch, setUi } from '../store.js';
import { Icon } from '../common.js';
import { recall } from '../views/bots.js';
import { SUB, viewOf } from './views.js';

/* ⌘K or Ctrl+K focuses the search; the hint says the one this computer has */
const KEY = /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent) ? '⌘K' : 'Ctrl K';
function useSearchKey() {
  useEffect(() => {
    const key = (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); document.getElementById('f-q').focus(); }
    };
    addEventListener('keydown', key);
    return () => removeEventListener('keydown', key);
  }, []);
}

/* the crumb's parts: the view and the bot, or the project and its view, or just the view */
function useCrumb(r, s) {
  const label = viewOf(r.view)[1];
  const [bots] = useFetch(r.view === 'bots' ? '/api/bots' : '', ['bots']);
  const bot = r.view === 'bots' && bots && (bots.find((b) => b.id === (r.project || recall())) || bots[0]);
  const project = SUB.some((v) => v[0] === r.view) || r.view === 'overview' ? s.projects.find((p) => p.id === r.project) : null;
  if (r.view === 'settings' && r.project === 'access') return [label, 'Access'];
  return bot ? [label, bot.name] : project ? [project.name || project.id, r.view === 'overview' ? '' : label].filter(Boolean) : [label];
}

export function Top({ r }) {
  const s = useStore();
  const crumb = useCrumb(r, s);
  useSearchKey();
  return html`<header id="top">
    <div id="crumb" class="crumb">${r.view === 'overview' ? null /* the Home's heading says it */ : crumb.map((c, i) => html`${i ? html`<span class="sep">/</span>` : null}<span class=${i === crumb.length - 1 ? 'last' : ''}>${c}</span>`)}</div>
    <label class="cmd"><${Icon} d="M11 4a7 7 0 100 14 7 7 0 000-14z M20 20l-4-4" size=${14} w=${2} />
      <input id="f-q" type="search" placeholder="Search" aria-label="Search" autocomplete="off" value=${s.q || ''}
        onInput=${(e) => setUi('q', e.target.value)} /><kbd>${KEY}</kbd></label>
  </header>`;
}
