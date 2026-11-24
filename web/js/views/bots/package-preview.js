// What a package will do and need, shown before anything is made: the creator, the routines, the skills, the connectors it asks for and what NeoBot enforces.
import { html } from '../../../vendor/preact-htm.js';
import { Face } from './face.js';
import { daysLabel, timeLabel } from './when.js';

const when = (r) => (r.trigger === 'webhook' ? `On an event · ${r.when}` : `${daysLabel(r.days)} around ${timeLabel(r.at)}`);
const tools = (g) => (g.tools ? g.tools.join(', ') : 'every tool');

export function PackagePreview({ p }) {
  return html`<div class="bpkg">
    <div class="bpkgtop"><${Face} shape=${p.shape} color=${p.color} size=${56} still /><div><b>${p.name}</b>
      <span class="bmut">signed by ${p.creator.slice(0, 12)}…</span></div></div>
    ${p.description ? html`<p>${p.description}</p>` : null}
    <h4>Routines</h4>${p.routines.length ? p.routines.map((r) => html`<p key=${r.title}><b>${r.title}</b><span class="bmut"> ${when(r)}</span><br />${r.prompt}</p>`) : html`<p class="bmut">None.</p>`}
    <h4>Skills</h4>${p.skills.length ? html`<ul>${p.skills.map((k) => html`<li key=${k.name}>${k.name}</li>`)}</ul>` : html`<p class="bmut">None.</p>`}
    ${p.instructions ? html`<h4>Instructions</h4><p>${p.instructions}</p>` : null}
    <h4>Asks for these connectors</h4>${p.integrations.length ? html`<ul>${p.integrations.map((g) => html`<li key=${g.connector}>${g.connector}: ${tools(g)}</li>`)}</ul>` : html`<p class="bmut">None.</p>`}
    <p class="bmut">${p.grants_nothing}.</p>
    <h4>Limits</h4><ul>${p.permissions.map((x) => html`<li key=${x.what}><b>${x.what}</b> · ${x.enforced ? 'enforced by NeoBot' : 'declared only'}: ${x.note}</li>`)}</ul>
  </div>`;
}
