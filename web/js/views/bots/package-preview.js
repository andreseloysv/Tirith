// What a package will do and need, shown before anything is made: the creator, the routines, the skills, the connectors it asks for and what NeoBot enforces.
import { html } from '../../../vendor/preact-htm.js';
import { Face } from './face.js';
import { daysLabel, timeLabel } from './when.js';
import { t } from '../../i18n.js';

const when = (r) => (r.trigger === 'webhook' ? t('On an event · {when}', { when: r.when }) : t('{days} around {time}', { days: daysLabel(r.days), time: timeLabel(r.at) }));
const tools = (g) => (g.tools ? g.tools.join(', ') : t('every tool'));

export function PackagePreview({ p }) {
  return html`<div class="bpkg">
    <div class="bpkgtop"><${Face} shape=${p.shape} color=${p.color} size=${56} still /><div><b>${p.name}</b>
      <span class="bmut">${t('signed by {creator}…', { creator: p.creator.slice(0, 12) })}</span></div></div>
    ${p.description ? html`<p>${p.description}</p>` : null}
    <h4>${t('Routines')}</h4>${p.routines.length ? p.routines.map((r) => html`<p key=${r.title}><b>${r.title}</b><span class="bmut"> ${when(r)}</span><br />${r.prompt}</p>`) : html`<p class="bmut">${t('None.')}</p>`}
    <h4>${t('Skills')}</h4>${p.skills.length ? html`<ul>${p.skills.map((k) => html`<li key=${k.name}>${k.name}</li>`)}</ul>` : html`<p class="bmut">${t('None.')}</p>`}
    ${p.instructions ? html`<h4>${t('Instructions')}</h4><p>${p.instructions}</p>` : null}
    <h4>${t('Asks for these connectors')}</h4>${p.integrations.length ? html`<ul>${p.integrations.map((g) => html`<li key=${g.connector}>${g.connector}: ${tools(g)}</li>`)}</ul>` : html`<p class="bmut">${t('None.')}</p>`}
    <p class="bmut">${p.grants_nothing}.</p>
    <h4>${t('Limits')}</h4><ul>${p.permissions.map((x) => html`<li key=${x.what}><b>${x.what}</b> · ${x.enforced ? t('enforced by NeoBot') : t('declared only')}: ${x.note}</li>`)}</ul>
  </div>`;
}
