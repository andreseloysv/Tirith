// The Add project dialog's fixed parts: the colors it offers, a folder's last name, and the daemon's errors in words.
import { t } from '../../i18n.js';
export const colors = () => [['#2f7d5b', t('Green')], ['#3b6fd8', t('Blue')], ['#8a5cd0', t('Purple')], ['#d0663b', t('Orange')], ['#c2417c', t('Pink')], ['#b8860b', t('Gold')], ['#0e8aa0', t('Teal')], ['#64748b', t('Slate')]];
export const base = (p) => String(p || '').replace(/[\\/]+$/, '').split(/[\\/]/).pop() || '';
/** The daemon's message; an older daemon than the page answers "not found" to the projects API. */
export const why = (e) => {
  const m = String((e && e.message) || e).trim();
  return /^not found$/i.test(m) ? t('The running NeoBot daemon is older than this page and cannot add projects: quit NeoBot, stop the daemon (pkill -f "neobot serve"), open NeoBot again.') : m;
};
