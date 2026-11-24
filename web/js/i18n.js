// The page's words in the owner's language. English is what the code says; Spanish and German come from the
// dictionaries in i18n/<lang>/ (one file per area, keyed by the English text, `{name}` for what fills in). A text with
// no translation stays English. The choice lives in this browser (localStorage) and is told to the daemon, whose bots
// then write to the owner in it; changing it reloads the page.
import es from './i18n/es.js';
import de from './i18n/de.js';

const KEY = 'neobot.lang';
const BOOKS = { es, de };
/** [code, its own name]: what the pickers offer. */
export const LANGS = [['en', 'English'], ['es', 'Español'], ['de', 'Deutsch']];

function stored() {
  try { const l = localStorage.getItem(KEY); return BOOKS[l] ? l : 'en'; } catch { return 'en'; }
}
let lang = stored();
document.documentElement.lang = lang;

export const getLang = () => lang;

/** Chooses the language: kept here, told to the daemon, and the page drawn again in it. */
export function setLang(code) {
  const next = BOOKS[code] ? code : 'en';
  try { localStorage.setItem(KEY, next); } catch { /* private window: this visit only */ }
  const tell = fetch('/api/lang', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ lang: next }) });
  tell.catch(() => null).finally(() => location.reload());
}

/** `text` in the chosen language, with `{key}` filled from `vars`. */
export function t(text, vars) {
  const book = BOOKS[lang];
  const said = (book && book[text]) || text;
  return vars ? said.replace(/\{(\w+)\}/g, (m, k) => (k in vars ? String(vars[k]) : m)) : said;
}
