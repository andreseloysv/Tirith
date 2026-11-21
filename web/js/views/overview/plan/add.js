// The "Add item" row at the end of a step's items, and its input (POST /api/plan/<id>).
import { html, useState, useEffect, useRef } from '../../../../vendor/preact-htm.js';
import { api } from '../../../store.js';
import { Svg, OPEN, PLUS } from '../bits.js';

export function AddItem({ project, step, editing, setEditing }) {
  const [draft, setDraft] = useState('');
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  const ref = useRef(null);
  useEffect(() => { if (editing && ref.current) ref.current.focus(); }, [editing]);
  if (!editing) {
    return html`<li class="padd"><button class="pedadd" title="Add an item to this step" onClick=${() => setEditing(true)}>
      <${Svg} s=${PLUS} /><span>Add item</span></button></li>`;
  }
  const save = async () => {
    const text = draft.trim();
    if (!text || busy) return;
    setBusy(true); setErr('');
    try {
      await api('POST', `/api/plan/${encodeURIComponent(project)}`, { step, text });
      setDraft(''); setEditing(false);
    } catch (e) { setErr(String(e.message || e)); }
    setBusy(false);
  };
  return html`<li class="pediting"><${Svg} cls="pi" s=${OPEN} />
      <input ref=${ref} class="iin pedin" type="text" maxlength="500" aria-label="New item" placeholder="What needs doing?" value=${draft}
        disabled=${busy} onInput=${(e) => setDraft(e.target.value)}
        onKeyDown=${(e) => { if (e.key === 'Enter') save(); else if (e.key === 'Escape') { setEditing(false); setErr(''); } }} />
      <span class="ihint">${busy ? 'Saving…' : 'Enter saves · Esc cancels'}</span></li>
    ${err ? html`<li class="perr"><div class="ierr">${err}</div></li>` : null}`;
}
