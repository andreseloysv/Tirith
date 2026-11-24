// The page's words in the owner's language. English is what the code says; Spanish and German come from the
// dictionaries in i18n/<lang>/<area>.json (one per area, keyed by the English text, `{name}` for what fills in),
// loaded before the page is drawn. A text with no translation stays English. The choice lives in this browser
// (localStorage) and is told to the daemon, whose bots then write to the owner in it; changing it reloads the page.
const KEY = 'neobot.lang';
/** [code, its own name]: what the pickers offer. */
export const LANGS = [['en', 'English'], ['es', 'Español'], ['de', 'Deutsch']];
const AREAS = ['app', 'setup', 'home', 'settings', 'bots', 'chat', 'projects', 'connectors', 'daemon', 'onboarding'];

function stored() {
  try { const l = localStorage.getItem(KEY); return LANGS.some(([k]) => k === l) ? l : 'en'; } catch { return 'en'; }
}
const lang = stored();
let book = {};

export const getLang = () => lang;

/** The chosen language's dictionaries, read once before the page is drawn (nothing to read for English). */
export async function loadLanguage() {
  document.documentElement.lang = lang;
  if (lang === 'en') return;
  const read = (area) => fetch(new URL(`./i18n/${lang}/${area}.json`, import.meta.url)).then((r) => (r.ok ? r.json() : {}), () => ({}));
  book = Object.assign({}, ...(await Promise.all(AREAS.map(read))));
}

/** Chooses the language: kept here, told to the daemon, and the page drawn again in it. */
export function setLang(code) {
  const next = LANGS.some(([k]) => k === code) ? code : 'en';
  try { localStorage.setItem(KEY, next); } catch { /* private window: this visit only */ }
  const tell = fetch('/api/lang', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ lang: next }) });
  tell.catch(() => null).finally(() => location.reload());
}

/** `text` in the chosen language, with `{key}` filled from `vars`. */
export function t(text, vars) {
  const said = book[text] || text;
  return vars ? said.replace(/\{(\w+)\}/g, (m, k) => (k in vars ? String(vars[k]) : m)) : said;
}
