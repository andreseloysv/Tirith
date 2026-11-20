// Visual controls belong to the chat composer. Settings keeps its select-based form.
import { html, useState, useLayoutEffect, useRef } from '../../vendor/preact-htm.js';
import { useFetch, api } from '../store.js';
import { optionsFor } from '../model-controls.js';
import { roleList } from './settings/models.js';
import { useAction } from './settings/ui.js';

function Selection({ role, M, reload }) {
  const [draft, setDraft] = useState(null);
  const [busy, error, run] = useAction();
  const ref = useRef(null);
  const value = draft || M.roles[role] || {};
  const { valid } = optionsFor(M.choices, value);
  useLayoutEffect(() => {
    ref.current.onChange = setDraft;
    ref.current.configure({ choices: M.choices, value, disabled: !!busy });
  });
  const save = async () => {
    if (!draft || !valid) return;
    if (await run('save', () => api('POST', `/api/models/${role}`, draft))) { setDraft(null); reload(); }
  };
  return html`<div class="mc-selection">
    <model-controls ref=${ref}></model-controls>
    <div class="mc-model-actions">
      <button type="button" class="isend" disabled=${!draft || !!busy || !valid} onClick=${save}>${busy ? 'Saving…' : 'Save'}</button>
      ${draft ? html`<button type="button" class="scancel" disabled=${!!busy} onClick=${() => setDraft(null)}>Cancel</button>` : null}
      <span class="mut sm">Applies to all projects.</span>
    </div>
    ${error ? html`<p class="ierr">${error}</p>` : null}
  </div>`;
}

export function ChatModels() {
  const [M, reload, error] = useFetch('/api/models', ['models']);
  const [role, setRole] = useState('manager');
  const current = M?.roles[role];
  return html`<details class="mc-models"><summary>Models · ${current?.model || current?.agent || 'Manager'}</summary>
    <div class="model-roles" role="group" aria-label="Model role">
      ${roleList(M).map(([id, name]) => html`<button type="button" aria-pressed=${role === id} onClick=${() => setRole(id)}>${name}</button>`)}</div>
    ${error ? html`<p class="ierr">${error.message}</p>` : null}
    ${M ? html`<${Selection} key=${role} role=${role} M=${M} reload=${reload} />` : null}
  </details>`;
}
