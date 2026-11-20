// Unread bot messages: how many answers each bot gave since the owner last looked at its chat (kept per browser).
// Every bot's conversation is read once (openTalk) and the stream keeps it current; quiet answers (∅) never count.
// A bot seen here for the first time starts with nothing unread, so an old history is not shown as news.
import { useEffect } from '../vendor/preact-htm.js';
import { S, useStore, useFetch, openTalk, setUi } from './store.js';
import { shownAnswers } from './views/chat.js';

const KEY = 'tirith.bots.seen';
let seen = {};
try { seen = JSON.parse(localStorage.getItem(KEY) || '{}') || {}; } catch { /* private window: counts start over */ }
const save = () => { try { localStorage.setItem(KEY, JSON.stringify(seen)); } catch { /* the counts just do not survive a reload */ } };
const opened = new Set();
const lastId = (rows) => (rows.length ? rows[rows.length - 1].id : 0);

/** `{bots, by: {id: n}, total}`: the bots and their unread answers. */
export function useUnread() {
  const s = useStore();
  const [bots] = useFetch('/api/bots', ['bots']);
  useEffect(() => {
    for (const b of bots || []) {
      const key = `bot-${b.id}:${s.boots}`;
      if (!opened.has(key)) { opened.add(key); openTalk(`bot-${b.id}`); }
    }
  }, [bots, s.boots]);
  const by = {};
  let total = 0;
  for (const b of bots || []) {
    const k = `bot-${b.id}`, t = S.talks[k];
    if (!t || !t.loaded) continue;
    if (seen[b.id] == null) { seen[b.id] = lastId(t.rows); save(); }
    by[b.id] = shownAnswers(t.rows, !!S.busy[k]).filter((r) => r.id > seen[b.id]).length;
    total += by[b.id];
  }
  return { bots: bots || [], by, total };
}

/** The owner is looking at this bot's chat: all of it so far is read. */
export function markSeen(botId, rows) {
  const last = lastId(rows);
  if (last && last !== seen[botId]) { seen = { ...seen, [botId]: last }; save(); setUi('seenAt', Date.now()); }
}
