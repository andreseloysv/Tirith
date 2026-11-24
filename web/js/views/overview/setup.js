// A project's commit guard, on its Overview only when it needs a fix: one card that says what is wrong and offers
// Install. (What waits for the owner otherwise is in asks.js; every project's guard is in Settings.)
import { html, useState } from '../../../vendor/preact-htm.js';
import { useFetch, api } from '../../store.js';
import { ICON, Icon } from '../../common.js';
import { t } from '../../i18n.js';

export function Setup({ project, name }) {
  const [guard, reload] = useFetch('/api/guard', []);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  const g = (guard || []).find((x) => x.project === project);
  if (!g || !g.words || g.ok) return null;
  const install = async () => {
    setBusy(true); setErr('');
    try { await api('POST', `/api/guard/${encodeURIComponent(project)}/install`); reload(); } catch (e) { setErr(String(e.message || e)); }
    setBusy(false);
  };
  return html`<section class="hm-sec" aria-label=${t('Needs a fix')}><h2 class="hm-h">${t('Needs a fix')}</h2>
    <article class="hm-ask hm-fix">
      <header class="hm-who"><span class="hm-face hm-shield"><${Icon} d=${ICON.guard} size=${18} w=${1.8} /></span><b>${t('The commit guard of {name} is not intact', { name: g.name || name })}</b></header>
      <p class="hm-q">${t('Agents could commit words you forbade here.')}${g.reason ? html` <span class="hm-why">${t(g.reason)}</span>` : null}</p>
      <div class="hm-reply"><button type="button" class="hm-btn hm-yes" disabled=${busy} onClick=${install}>${busy ? t('Installing…') : t('Install')}</button></div>
      ${err ? html`<p class="hm-err" role="alert">${err}</p>` : null}
    </article></section>`;
}
