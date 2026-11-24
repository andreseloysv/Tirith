// How far the welcome got, kept in this browser: it shows while under way, or while the owner has no bot of their
// own and never finished it (a system bot does not count). Done once a choice is made or it is skipped.
const KEY = 'neobot.welcome';

export function loadWelcome() {
  try { return JSON.parse(localStorage.getItem(KEY) || 'null'); } catch { return null; }
}

export function saveWelcome(v) {
  try { localStorage.setItem(KEY, JSON.stringify(v)); } catch { /* private window: it shows again next time */ }
}

/** `bots` as GET /api/bots gives them; empty until it answered (a loaded list always has the system bots). */
export const welcoming = (bots) => {
  const st = loadWelcome();
  return st ? st.step !== 'done' : Array.isArray(bots) && bots.length > 0 && !bots.some((b) => !b.system);
};
