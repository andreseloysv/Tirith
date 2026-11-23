// The Add project dialog's fixed parts: the colors it offers, a folder's last name, and the daemon's errors in words.
export const COLORS = [['#2f7d5b', 'Green'], ['#3b6fd8', 'Blue'], ['#8a5cd0', 'Purple'], ['#d0663b', 'Orange'], ['#c2417c', 'Pink'], ['#b8860b', 'Gold'], ['#0e8aa0', 'Teal'], ['#64748b', 'Slate']];
export const base = (p) => String(p || '').replace(/[\\/]+$/, '').split(/[\\/]/).pop() || '';
/** The daemon's message; an older daemon than the page answers "not found" to the projects API. */
export const why = (e) => {
  const m = String((e && e.message) || e).trim();
  return /^not found$/i.test(m) ? 'The running OpenBot daemon is older than this page and cannot add projects: quit OpenBot, stop the daemon (pkill -f "tirith serve"), open OpenBot again.' : m;
};
