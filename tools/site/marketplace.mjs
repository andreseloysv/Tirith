// Builds site/marketplace/ from site/marketplace/bots.json:  node tools/site/marketplace.mjs [--check]
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { index, detail } from './pages.mjs';

const dir = new URL('../../site/marketplace/', import.meta.url).pathname;
const data = JSON.parse(readFileSync(dir + 'bots.json', 'utf8'));
const pages = { 'index.html': index(data) };
for (const b of data.bots) pages[`bots/${b.id}/index.html`] = detail(b);

let stale = 0;
for (const [path, text] of Object.entries(pages)) {
  const file = dir + path;
  if (process.argv.includes('--check')) {
    if (!existsSync(file) || readFileSync(file, 'utf8') !== text) { console.log('stale: ' + path); stale++; }
    continue;
  }
  mkdirSync(file.replace(/[^/]+$/, ''), { recursive: true });
  writeFileSync(file, text);
}
process.exit(stale ? 1 : 0);
