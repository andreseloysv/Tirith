// The page's words that have no Spanish or German yet: every literal t('…') in web/js, looked up in
// web/js/i18n/<lang>/*.json. `node tools/i18n-check.mjs [path…]` checks only files under those paths.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const ROOT = 'web/js';
const only = process.argv.slice(2);
const files = (dir) => readdirSync(dir).flatMap((n) => {
  const p = join(dir, n);
  if (n === 'vendor' || n === 'i18n') return [];
  return statSync(p).isDirectory() ? files(p) : p.endsWith('.js') && !p.includes('.test.') ? [p] : [];
});
const book = (lang) => Object.assign({}, ...readdirSync(join(ROOT, 'i18n', lang))
  .filter((n) => n.endsWith('.json')).map((n) => JSON.parse(readFileSync(join(ROOT, 'i18n', lang, n), 'utf8'))));
const books = { es: book('es'), de: book('de') };
const CALL = /\bt\(\s*(?:'((?:[^'\\]|\\.)*)'|"((?:[^"\\]|\\.)*)"|`([^`$]*)`)/g;
let missing = 0;
for (const f of files(ROOT).filter((f) => !only.length || only.some((o) => f.startsWith(o)))) {
  for (const m of readFileSync(f, 'utf8').matchAll(CALL)) {
    const key = (m[1] ?? m[2] ?? m[3]).replace(/\\(['"\\])/g, '$1');
    const lacks = Object.keys(books).filter((l) => !(key in books[l]));
    if (lacks.length) { missing++; console.log(`${f}: ${lacks.join(',')} lack ${JSON.stringify(key)}`); }
  }
}
console.log(missing ? `${missing} texts lack a translation` : 'every t() text has its Spanish and German');
process.exit(missing ? 1 : 0);
