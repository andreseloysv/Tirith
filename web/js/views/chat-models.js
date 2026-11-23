// The model picker of one chat: it chooses only the model of who speaks in this conversation — a project's chat
// the Project Manager's (`manager`, for every project, and it says so), a bot's screen that bot's. Roles with no
// screen of their own keep their picker in Settings. It offers only what can run, and a pick is saved at once.
import { html, useState, useEffect, useLayoutEffect, useRef } from '../../vendor/preact-htm.js';
import { useFetch, api } from '../store.js';
import { offered, effortName, PROVIDERS } from '../model-controls.js';
import { useAction } from './settings/ui.js';

/** The summary's words: the model and its effort, the agent's own default, or that it cannot run. */
function words(choices, v) {
  if (!v.model) return `${PROVIDERS[v.agent] || v.agent || '…'} · its default`;
  const m = offered(choices).find(x => x.slug === v.model);
  return [m ? m.name : v.model, m ? effortName(v.effort) : 'not available, choose another'].filter(Boolean).join(' · ');
}

function Selection({ role, M, reload }) {
  const [picked, setPicked] = useState(null);   /* shown while it is saved, until the new state arrives */
  const [busy, error, run] = useAction();
  const ref = useRef(null);
  useEffect(() => setPicked(null), [M]);
  useLayoutEffect(() => {
    ref.current.onChange = async (v) => {
      setPicked(v);
      if (await run('save', () => api('POST', `/api/models/${role}`, v))) reload();
      else setPicked(null);
    };
    ref.current.configure({ choices: M.choices, value: picked || M.roles[role] || {}, disabled: !!busy });
  });
  return html`<div class="mc-selection">
    <model-controls ref=${ref}></model-controls>
    ${error ? html`<p class="ierr">${error}</p>` : null}
  </div>`;
}

export function ChatModels({ role, note }) {
  const [M, reload, error] = useFetch('/api/models', ['models']);
  const current = M?.roles[role];
  if (M && !current) return null;   /* this conversation's speaker is no factory role: nothing to pick */
  return html`<details class="mc-models"><summary><span class="mc-mcap">Model</span>${M ? words(M.choices, current) : '…'}</summary>
    ${note ? html`<p class="mut sm mc-mnote">${note}</p>` : null}
    ${error ? html`<p class="ierr">${error.message}</p>` : null}
    ${M ? html`<${Selection} key=${role} role=${role} M=${M} reload=${reload} />` : null}
  </details>`;
}
