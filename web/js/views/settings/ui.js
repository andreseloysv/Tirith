// Small bits shared by the Rooms, Access and Settings views.
import { html, useState } from '../../../vendor/preact-htm.js';
import { ago } from '../../common.js';

/** "just now", "5 min ago", "2 h ago" (the old rooms view's wording). */
export function agoText(iso) {
  const a = ago(iso);
  return !a ? '' : a === 'now' ? 'just now' : `${a} ago`;
}

/** Copies `text`; the button says "Copied" for a moment. */
export function CopyBtn({ text, label = 'Copy', cls = 'alink' }) {
  const [done, setDone] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      const t = document.createElement('textarea');
      t.value = text; document.body.appendChild(t); t.select();
      try { document.execCommand('copy'); } catch { /* nothing more to try */ }
      t.remove();
    }
    setDone(true);
    setTimeout(() => setDone(false), 1500);
  };
  return html`<button type="button" class=${cls} onClick=${copy}>${done ? 'Copied' : label}</button>`;
}

/** One call at a time per key: `[busy, error, run(fn)]`; `run` keeps the error message of a failed call. */
export function useAction() {
  const [state, set] = useState({ busy: '', error: '' });
  const run = async (key, fn) => {
    set({ busy: key, error: '' });
    try {
      const r = await fn();
      set({ busy: '', error: '' });
      return r;
    } catch (e) {
      set({ busy: '', error: String((e && e.message) || e) });
      return undefined;
    }
  };
  return [state.busy, state.error, run, () => set({ busy: '', error: '' })];
}
