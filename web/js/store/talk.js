// The conversations the page has opened, one per project.
import { useEffect } from '../../vendor/preact-htm.js';
import { S, changed, useStore } from './state.js';
import { api } from './api.js';

/** One conversation: `{rows, loaded, error}`, read from the daemon when first asked for and after every
    reconnect, then kept current by the stream (rows that arrive while it loads are kept). */
export function useTalk(project) {
  const s = useStore();
  useEffect(() => { openTalk(project); }, [project, s.boots]);
  return S.talks[project] || { rows: [], loaded: false, error: null };
}

/** Reads one conversation into `S.talks` (and the stream keeps it current from then on), without showing it:
    the bots' unread counts need the rows of chats that are not open. */
export function openTalk(project) {
  const t = S.talks[project] || (S.talks[project] = { rows: [], loaded: false, error: null });
  return api('GET', `/api/chat?project=${encodeURIComponent(project)}`).then((rows) => {
    if (!Array.isArray(rows)) throw new Error('not found');
    const have = new Set(rows.map((r) => r.id));
    t.rows = rows.concat(t.rows.filter((r) => !have.has(r.id))).sort((a, b) => a.id - b.id);
    t.loaded = true; t.error = null;
    changed();
  }).catch((error) => { t.error = error; changed(); });
}
