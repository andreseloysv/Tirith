// How far the welcome got, kept in this browser for the bot that owes it (the daemon says which, GET /api/me, and
// forgets it once the welcome is said or skipped): whether the owner already said what to call it.
const KEY = 'neobot.welcome';

/** The progress kept for bot `id`: `{bot, was, named}`. Another bot's progress is nobody's. */
export function loadWelcome(id) {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) || 'null');
    return v && v.bot === id ? v : { bot: id };
  } catch { return { bot: id }; }
}

export function saveWelcome(v) {
  try { localStorage.setItem(KEY, JSON.stringify(v)); } catch { /* private window: it asks the name again next time */ }
}
