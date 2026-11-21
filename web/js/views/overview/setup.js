// What needs the owner (the open cards), and the setup (commit guard) of the routed project, drawn only when it
// needs attention.
import { html, useState } from '../../../vendor/preact-htm.js';
import { useFetch, api } from '../../store.js';
import { ICON, Icon } from '../../common.js';
import { hit } from './bits.js';
import { Need } from '../need.js';

/** The open cards: inside a project its own, and those about the machine itself (a full disk stops every project's
    agents); with none selected all of them, each named by its project. */
export function Needs({ owner, project, projects, q }) {
  const list = owner.filter((o) => (!project || o.project === project || (!o.project && o.kind === 'device')) && hit(q, o.question, o.kind));
  if (!list.length) return null;
  return html`<section class="needs-sec"><h2>Needs you <span class="icount">${list.length}</span></h2>
    <ul class="nlist">${list.map((o) => html`<${Need} key=${o.id} o=${o} p=${project ? null : projects.find((x) => x.id === o.project)} />`)}</ul></section>`;
}

/** The guard's line: its dot, title and words, from whether words are set and whether the guard is intact. */
const stateOf = (g, none, on) => (none ? ['grey', 'No commit guard', 'no forbidden words are set for this project in config.toml']
  : on ? ['green', 'Commit guard installed', `${g.words} forbidden ${g.words === 1 ? 'word' : 'words'} blocked in commits`]
  : ['red', 'Commit guard needs attention', g.reason]);

export function Setup({ project, name }) {
  const [guard, reload] = useFetch('/api/guard', []);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  const g = (guard || []).find((x) => x.project === project);
  if (!g || !g.words || g.ok) return null;
  const none = !g.words, on = g.ok && !none;
  const install = async () => {
    setBusy(true); setErr('');
    try { await api('POST', `/api/guard/${encodeURIComponent(project)}/install`); reload(); } catch (e) { setErr(String(e.message || e)); }
    setBusy(false);
  };
  const state = stateOf(g, none, on);
  return html`<section><h2>Setup</h2><div class="card scard"><div class="phead"><b>${g.name || name}</b><span class="mut sm">Setup</span></div>
    ${none || on ? null : html`<div class="sban" role="alert"><span>Agents could commit forbidden words here, because the commit guard is not intact.</span></div>`}
    <div class="srow"><span class="slabel">Guard</span><div class="sbody">
      <div class=${`scheck ${on ? 'ok' : none ? '' : 'bad'}`}><span class=${`dot ${state[0]}`}></span><b>${state[1]}</b><span class="mut sm"> · ${state[2]}</span>
        ${none || on ? null : html`<button class="srun ovinst" disabled=${busy} onClick=${install}><${Icon} d=${ICON.guard} size=${11} w=${2} /><span>${busy ? 'Installing…' : 'Install'}</span></button>`}
      </div></div></div>
    ${err ? html`<div class="ierr">${err}</div>` : null}
  </div></section>`;
}
