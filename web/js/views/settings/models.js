// Settings, models per role (the old v-models.js): agent, model, effort and the fallback chain of the manager,
// the workers, the CEO and the brain. Save: POST /api/models/{role}; back to config.toml: DELETE.
import { html, useState } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { useAction } from './ui.js';

const BOLT = html`<svg viewBox="0 0 12 12" width="11" height="11" aria-hidden="true"><path d="M7 1.2L2.6 6.8h2.7L4.8 10.8 9.4 5H6.6z" fill="currentColor"/></svg>`;
const OTHER = '__other__';
export const ROLES = [
  ['manager', 'Manager', 'The orchestrator session'],
  ['workers', 'Workers', 'Agents the manager, autopilot or Plan button starts'],
  ['ceo', 'CEO', 'The one-shot review of the whole company'],
  ['brain', 'Brain', 'Small text jobs through the model API: only its model matters'],
];
const SOURCE = { page: ['Saved here', 'page'], config: ['From config.toml', ''], default: ['Default', ''] };

function Options({ list, cur, blank, other }) {
  const all = cur && !list.includes(cur) ? [cur, ...list] : list;
  return html`${blank != null ? html`<option value="">${blank}</option>` : null}
    ${all.map((o) => html`<option value=${o} selected=${o === cur}>${o}</option>`)}
    ${other ? html`<option value=${OTHER}>Other…</option>` : null}`;
}

/** A select of `list` with "Other…", which turns it into a text box (with "List" to go back). */
function Pick({ label, value, list, blank, set, free, setFree }) {
  if (free || !list.length) {
    return html`<label class="sf"><span>${label}</span><input type="text" value=${value} placeholder=${blank || label} spellcheck="false"
      aria-label=${label} onInput=${(e) => set(e.target.value)} />
      ${list.length ? html`<button type="button" class="alink" onClick=${() => { setFree(false); set(''); }}>List</button>` : null}</label>`;
  }
  return html`<label class="sf"><span>${label}</span><select aria-label=${label} onChange=${(e) => {
    if (e.target.value === OTHER) { setFree(true); set(''); } else set(e.target.value);
  }}><${Options} list=${list} cur=${value} blank=${blank} other=${true} /></select></label>`;
}

function Fallback({ list, main, agents, set }) {
  const left = agents.filter((a) => !list.includes(a) && a !== main);
  const move = (i) => { const l = list.slice(); [l[i - 1], l[i]] = [l[i], l[i - 1]]; set(l); };
  return html`<div class="sfallback"><span class="slabel">Fallback</span><span class="mut sm">Used in this order when an agent is out of credits</span>
    <div class="fbrow">${list.length ? list.map((a, i) => html`<span class="fbchip" key=${a}><b>${i + 1}</b> ${a}
        ${i ? html`<button class="alink" aria-label=${`Move ${a} earlier`} onClick=${() => move(i)}>↑</button>` : null}
        <button class="alink" aria-label=${`Remove ${a}`} onClick=${() => set(list.filter((x) => x !== a))}>✕</button></span>`)
      : html`<span class="mut sm">None</span>`}
      ${left.length ? html`<select aria-label="Add a fallback agent" onChange=${(e) => { if (e.target.value) set([...list, e.target.value]); e.target.value = ''; }}>
        <${Options} list=${left} cur="" blank="Add…" /></select>` : null}</div></div>`;
}

export function RoleCard({ id, label, note, M, reload }) {
  const cur = M.roles[id] || {};
  const ch = M.choices || {};
  const [draft, setDraft] = useState({});
  const [free, setFree] = useState({});
  const [busy, error, run] = useAction();
  const v = (k) => (draft[k] !== undefined ? draft[k] : k === 'fallback' ? cur.fallback || [] : cur[k] || '');
  const set = (k) => (val) => setDraft({ ...draft, [k]: val });
  const setF = (k) => (on) => setFree({ ...free, [k]: on });
  const dirty = Object.keys(draft).length > 0;
  const brain = id === 'brain';
  const save = async () => {
    const body = { model: v('model').trim(), effort: v('effort') };
    if (!brain) { body.agent = v('agent'); body.fallback = v('fallback'); }
    if (await run('save', () => api('POST', `/api/models/${id}`, body))) { setDraft({}); setFree({}); }
    reload();
  };
  const reset = async () => {
    if (await run('reset', () => api('DELETE', `/api/models/${id}`))) { setDraft({}); setFree({}); }
    reload();
  };
  const [srcLabel, srcCls] = SOURCE[cur.source] || [cur.source || '', ''];
  return html`<div class="card scard"><div class="phead"><b>${label}</b><span class="mut sm">${note}</span>
      <span class="msrc"><span class=${`ssrc ${srcCls}`} title="Where this setting comes from">${srcLabel}</span></span></div>
    <div class="sfields mfields">
      ${brain ? null : html`<label class="sf"><span>Agent</span><select aria-label="Agent" onChange=${(e) => set('agent')(e.target.value)}>
        <${Options} list=${ch.agents || []} cur=${v('agent')} blank="(choose)" /></select></label>`}
      <${Pick} label="Model" value=${v('model')} list=${ch.models || []} blank=${brain ? '(default)' : '(from the agent)'} set=${set('model')} free=${free.model} setFree=${setF('model')} />
      ${brain ? null : html`<${Pick} label="Effort" value=${v('effort')} list=${ch.efforts || []} blank="(from the agent)" set=${set('effort')} free=${free.effort} setFree=${setF('effort')} />`}
    </div>
    ${brain ? null : html`<${Fallback} list=${v('fallback')} main=${v('agent')} agents=${ch.agents || []} set=${set('fallback')} />`}
    <div class="sfoot">
      <button class="isend" disabled=${!dirty || busy === 'save'} onClick=${save}>${busy === 'save' ? 'Saving…' : 'Save'}</button>
      ${dirty ? html`<button class="scancel" onClick=${() => { setDraft({}); setFree({}); }}>Cancel</button>` : null}
      ${cur.source === 'page' ? html`<button class="mapply" disabled=${busy === 'reset'} title="Forget what was saved here and use config.toml again"
        onClick=${reset}>${BOLT}<span>Back to config.toml</span></button>` : null}
      <span class="mnote mut sm">${id === 'manager' ? 'Saving restarts the manager session.' : id === 'workers' ? 'New lanes use it; running lanes keep theirs.' : ''}</span>
    </div>
    ${error ? html`<div class="ierr">${error}</div>` : null}
  </div>`;
}
