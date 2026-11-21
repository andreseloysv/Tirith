// A role card's draft: the values being edited over the saved ones, and Save (POST /api/models/{role}) and Back to
// config.toml (DELETE), each reading the models again after.
import { useState } from '../../../../vendor/preact-htm.js';
import { api } from '../../../store.js';
import { useAction } from '../ui.js';

export function useRoleDraft(id, cur, valid, reload) {
  const [draft, setDraft] = useState({});
  const [free, setFree] = useState({});
  const [busy, error, run] = useAction();
  const v = (k) => (draft[k] !== undefined ? draft[k] : k === 'fallback' ? cur.fallback || [] : cur[k] || '');
  const set = (k) => (val) => setDraft({ ...draft, [k]: val });
  const setF = (k) => (on) => setFree({ ...free, [k]: on });
  const clear = () => { setDraft({}); setFree({}); };
  const save = async () => {
    if (!valid()) return;
    const body = { agent: v('agent'), model: v('model').trim(), effort: v('effort'), fallback: v('fallback') };
    if (await run('save', () => api('POST', `/api/models/${id}`, body))) clear();
    reload();
  };
  const reset = async () => {
    if (await run('reset', () => api('DELETE', `/api/models/${id}`))) clear();
    reload();
  };
  return { draft, setDraft, free, setFree, busy, error, v, set, setF, clear, save, reset };
}
