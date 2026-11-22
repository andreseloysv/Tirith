// The tab «Enseñar»'s rehearsals: each time the bot tried a draft in the owner's Edge, newest first, with its steps and
// their screenshots in order, and two buttons: «Aprobar» (the draft loses `draft: true`, so the bot uses it from then
// on) and «Crear rutina» (the routine form with the task already written).
import { html, useState } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';

const clock = (ms) => new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
const what = (s) => (s.kind === 'open' ? `Open ${s.target}` : s.kind === 'click' ? `Click «${s.target}»` : `Type «${s.value}» in «${s.target}»`);
const title = (skill) => (skill ? skill.charAt(0).toUpperCase() + skill.slice(1).replace(/-/g, ' ') : 'Taught task');

function Step({ s, path }) {
  const src = s.shot ? `${path}/shot/${encodeURIComponent(s.shot)}` : '';
  return html`<li class=${s.ok ? '' : 'bad'}>
    <span class="tdo"><b>${what(s)}</b><span class="bmut">${s.ok ? '' : 'Failed · '}${s.result}</span></span>
    ${src ? html`<a href=${src} target="_blank" rel="noopener"><img class="tshot" src=${src} loading="lazy" alt=${`Screenshot after step ${s.n}`} /></a>` : null}
  </li>`;
}

function Rehearsal({ r, path, onRoutine }) {
  const [said, setSaid] = useState(''); /* '', 'approved', or why it could not */
  const steps = r.steps || [];
  const live = !!r.live; /* the daemon says: its clock and this one may differ */
  const worked = steps.some((s) => s.ok);
  const approve = () => api('POST', `${path}/approve`, { skill: r.skill })
    .then(() => setSaid('approved'), (e) => setSaid(/not a draft/.test(e.message) ? 'approved' : e.message));
  const routine = () => onRoutine({ title: title(r.skill), prompt: `Do the task of your skill «${r.skill || 'the one the owner taught you'}», following its steps.` });
  return html`<li class="treh">
    <p class="trehh"><b>${r.skill || 'A draft'}</b> <span class="bmut">${clock(Date.parse(r.since))} · ${steps.length} ${steps.length === 1 ? 'step' : 'steps'}${live ? ' · rehearsing now' : ''}</span></p>
    ${steps.length ? html`<ol class="tshots">${steps.map((s) => html`<${Step} key=${s.n} s=${s} path=${path} />`)}</ol>` : html`<p class="bnote">No step yet.</p>`}
    <div class="trow">
      <button type="button" class="bbtn pri" onClick=${approve} disabled=${!r.skill || live || !worked || said === 'approved'}
        title=${live ? 'The rehearsal is still running' : worked ? '' : 'No step worked yet: rehearse it first'}>${said === 'approved' ? 'Aprobada' : 'Aprobar'}</button>
      <button type="button" class="bbtn" onClick=${routine}>Crear rutina</button>
    </div>
    ${said && said !== 'approved' ? html`<p class="berr">${said}</p>` : null}
  </li>`;
}

export function Rehearsals({ path, list, onRoutine }) {
  if (!list || !list.length) return null;
  return html`<h4><span>Rehearsals</span> <span class="bn">${list.length}</span></h4>
    <p class="bnote">The bot tries its draft in your Edge, in its own tab, and stops before any step that needs your yes. Approve it, and it uses the skill from then on.</p>
    <ul class="trehs">${[...list].reverse().map((r) => html`<${Rehearsal} key=${r.since} r=${r} path=${path} onRoutine=${onRoutine} />`)}</ul>`;
}
