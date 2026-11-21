// Sending: each file first (POST /api/attachments), then the message. A message sent from this page shows as
// "Sending…" until the daemon shows it, or as "Not sent" with Retry and Copy text.
import { useState, useEffect, useRef } from '../../../vendor/preact-htm.js';
import { api } from '../../store.js';

let cidN = 0;

async function transmit(o, project) {
  for (const p of o.pics) {
    if (p.saved) continue;
    const r = await fetch(`/api/attachments?name=${encodeURIComponent(p.name)}`, { method: 'POST', body: p.file });
    const t = await r.text();
    if (!r.ok) throw new Error(`${p.name}: ${t || r.statusText}`);
    p.saved = JSON.parse(t).path;
  }
  await api('POST', '/api/chat', { text: o.text, project, attachments: o.pics.map((p) => p.saved) });
}

/** The local sends ({cid, text, pics, state, err, at, after}) of the conversation `project`, whose rows are `chat`. */
export function useSend({ project, talk, chat, lastId, draft, edit, pics, setPics, setErr, ta, stick }) {
  const [out, setOut] = useState([]);
  const lastSent = useRef('');
  /* a local send leaves once the daemon shows an owner message after it with its text */
  useEffect(() => {
    const seen = out.filter((o) => o.state === 'sending' && o.done && chat.some((m) => m.id > o.after && m.role === 'owner' && m.text.startsWith(o.text)));
    if (seen.length) setOut((xs) => xs.filter((o) => !seen.includes(o)));
  }, [lastId, out]);
  const dispatch = (o) => {
    o.state = 'sending'; o.err = null; o.done = false; o.at = Date.now();
    o.after = chat.length ? chat[chat.length - 1].id : 0;
    stick.current = true;
    setOut((xs) => (xs.includes(o) ? xs.slice() : xs.concat(o)));
    transmit(o, project).then(() => { o.done = true; setOut((xs) => xs.slice()); }, (e) => {
      o.state = 'failed'; o.err = `Could not send: ${e.message || e}`;
      setOut((xs) => xs.slice());
      /* the text goes back into an empty box, like the old page */
      if (!ta.current || !ta.current.value.trim()) edit(o.text);
    });
  };
  const send = () => {
    const text = draft.trim();
    if (!text && !pics.length) return;
    /* a conversation that could not be read (a daemon older than this page) would swallow the message */
    if (talk.error && !talk.loaded) { setErr('This conversation could not be read, so nothing was sent.'); return; }
    lastSent.current = text;
    dispatch({ cid: ++cidN, text, pics: pics.slice() });
    edit(''); setPics([]); setErr(null);
  };
  const retry = (o) => {
    if (draft === o.text) edit('');
    dispatch(o);
  };
  const copy = (o) => { try { navigator.clipboard.writeText(o.text).catch(() => {}); } catch { /* the text is in the box too */ } };
  return { out, send, retry, copy, lastSent };
}
