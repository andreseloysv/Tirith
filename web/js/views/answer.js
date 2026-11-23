// Answering an open question (a card): the answer sent (POST /api/owner/<id>), or the card closed as no longer
// needed; one call at a time, its error kept. The card in the chat and on its lane (need.js) and the Home's
// (overview/ask.js) answer through it.
import { useState } from '../../vendor/preact-htm.js';
import { api } from '../store.js';

export const WORDS = ['decision', 'lane']; /* answered in words only: no Done button */

/** `{busy, err, send(answer), close()}` for card `o`; each resolves true once the daemon took it. */
export function useAnswer(o) {
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState('');
  const call = async (path, body) => {
    if (busy) return false;
    setBusy(true); setErr('');
    try { await api('POST', path, body); return true; } catch (e) { setErr(String(e.message || e)); return false; } finally { setBusy(false); }
  };
  const send = (answer) => {
    const text = String(answer || '').trim();
    return text ? call(`/api/owner/${encodeURIComponent(o.id)}`, { answer: text }) : Promise.resolve(false);
  };
  const close = () => call(`/api/owner/${encodeURIComponent(o.id)}/close`, { why: 'no longer needed' });
  return { busy, err, send, close };
}
