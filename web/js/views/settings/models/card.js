// Settings, one role's card: agent, model, effort and the fallback chain, with Save, Cancel and Back to config.toml.
import { html } from '../../../../vendor/preact-htm.js';
import { optionsFor, ownerOf, pickModel } from '../../../model-controls.js';
import { BOLT, SOURCE } from './roles.js';
import { Options, Pick } from './pick.js';
import { Fallback } from './fallback.js';
import { useRoleDraft } from './draft.js';

function fields(c, ch, value, opts) {
  const { codex, models, efforts } = opts;
  const { draft, setDraft, free, setFree, busy, v, set, setF } = c;
  return html`<div class="sfields mfields">
      <label class="sf"><span>Agent</span><select aria-label="Agent" value=${v('agent')} disabled=${!!busy} onChange=${(e) => {
        setDraft({ ...draft, agent: e.target.value, model: '', effort: '' }); setFree({});
      }}><${Options} list=${ch.agents || []} cur=${v('agent')} blank="(choose)" /></select></label>
      <${Pick} label="Model" value=${v('model')} list=${models.map(m => m.slug)} blank="(from the agent)"
        owner=${(m) => ownerOf(ch, m)} set=${model => setDraft({ ...draft, ...pickModel(ch, value, model), ...(codex ? { effort: '' } : {}) })}
        free=${free.model} setFree=${setF('model')} strict=${codex} disabled=${!!busy} />
      <${Pick} label="Effort" value=${v('effort')} list=${efforts} blank="(from the agent)"
        set=${set('effort')} free=${free.effort} setFree=${setF('effort')} strict=${codex} disabled=${!!busy} />
    </div>`;
}

function foot(c, cur, valid) {
  const { busy, draft } = c;
  const dirty = Object.keys(draft).length > 0;
  return html`<div class="sfoot">
      <button class="isend" disabled=${!dirty || !!busy || !valid} onClick=${c.save}>${busy === 'save' ? 'Saving…' : 'Save'}</button>
      ${dirty ? html`<button class="scancel" disabled=${!!busy} onClick=${c.clear}>Cancel</button>` : null}
      ${cur.source === 'page' ? html`<button class="mapply" disabled=${!!busy} title="Forget what was saved here and use config.toml again"
        onClick=${c.reset}>${BOLT}<span>Back to config.toml</span></button>` : null}
      <span class="mnote mut sm">${cur.session === 'chat' ? 'Applies after the current turn finishes.' : cur.session === 'lane' ? 'New lanes use it; running lanes keep theirs.' : ''}</span>
    </div>`;
}

export function RoleCard({ id, label, note, M, reload }) {
  const cur = M.roles[id] || {};
  const ch = M.choices || {};
  let valid = false;
  const c = useRoleDraft(id, cur, () => valid, reload);
  const value = { agent: c.v('agent'), model: c.v('model'), effort: c.v('effort') };
  const opts = optionsFor(ch, value);
  valid = opts.valid;
  const [srcLabel, srcCls] = SOURCE[cur.source] || [cur.source || '', ''];
  return html`<div class="card scard"><div class="phead"><b>${label}</b><span class="mut sm">${note}</span>
      <span class="msrc"><span class=${`ssrc ${srcCls}`} title="Where this setting comes from">${srcLabel}</span></span></div>
    ${fields(c, ch, value, opts)}
    <${Fallback} list=${c.v('fallback')} main=${c.v('agent')} agents=${ch.agents || []} set=${c.set('fallback')} />
    ${foot(c, cur, valid)}
    ${c.error ? html`<div class="ierr">${c.error}</div>` : null}
  </div>`;
}
