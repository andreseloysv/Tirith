// The model picker of one chat: it chooses only the model of who speaks in this conversation — a project's chat
// the Project Manager's (`manager`, for every project, and it says so), a bot's screen that bot's. Roles with no
// screen of their own keep their picker in Settings. Visual controls belong to the chat composer; Settings keeps
// its select-based form.
import { html, useState, useLayoutEffect, useRef } from '../../vendor/preact-htm.js';
import { useFetch, api } from '../store.js';
import { optionsFor } from '../model-controls.js';
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
    </div>
    ${error ? html`<p class="ierr">${error}</p>` : null}
  </div>`;
}

export function ChatModels({ role, label, note }) {
  const [M, reload, error] = useFetch('/api/models', ['models']);
  const current = M?.roles[role];
  if (M && !current) return null;   /* this conversation's speaker is no factory role: nothing to pick */
  const name = label || current?.name || role;
  return html`<details class="mc-models"><summary>Model · ${name} · ${current?.model || current?.agent || '…'}</summary>
    ${note ? html`<p class="mut sm mc-mnote">${note}</p>` : null}
    ${error ? html`<p class="ierr">${error.message}</p>` : null}
    ${M ? html`<${Selection} key=${role} role=${role} M=${M} reload=${reload} />` : null}
  </details>`;
}
