// The keys of the chat: Esc stops the running turn (like the CLI), Enter sends, ArrowUp in an empty box brings
// back the last message, and pasted files are attached.
import { useEffect } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';
import { ownerParts } from './owner.js';

/** Stop the running turn of `project`; Esc anywhere outside a field does it too. */
export function useStop(project, busy, zoom, setErr) {
  const interrupt = () => {
    api('POST', '/api/manager/interrupt', { project }).then(() => setErr(null), (e) => setErr(`Could not stop: ${e.message}`));
  };
  /* Esc anywhere outside a field stops the running turn, like the CLI */
  useEffect(() => {
    const key = (e) => {
      if (e.key !== 'Escape' || e.defaultPrevented || !busy || zoom) return;
      const tag = e.target && e.target.tagName;
      if (tag === 'TEXTAREA' || tag === 'INPUT' || tag === 'SELECT') return;
      e.preventDefault(); interrupt();
    };
    addEventListener('keydown', key);
    return () => removeEventListener('keydown', key);
  }, [busy, zoom]);
  return interrupt;
}

/** onKey and onPaste of the box. */
export function boxKeys({ chat, busy, pics, lastSent, send, interrupt, edit, addFiles }) {
  const lastOwnerText = () => {
    if (lastSent.current) return lastSent.current;
    for (let i = chat.length - 1; i >= 0; i--) if (chat[i].role === 'owner') return ownerParts(chat[i].text).text;
    return '';
  };
  const onKey = (e) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); send(); }
    else if (e.key === 'Escape' && busy) { e.preventDefault(); e.stopPropagation(); interrupt(); }
    else if (e.key === 'ArrowUp' && !e.target.value && !pics.length && lastOwnerText()) { e.preventDefault(); edit(lastOwnerText()); }
  };
  const onPaste = (e) => {
    const files = (e.clipboardData && e.clipboardData.files) || [];
    if (files.length) { e.preventDefault(); addFiles(files); }
  };
  return { onKey, onPaste };
}
