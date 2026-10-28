// Renders the opening, closing and 'built' cards as PNG (built.png replaces the recorded built card). Usage: node cards.mjs [outDir]
import { createRequire } from 'node:module';
import { mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
const here = dirname(fileURLToPath(import.meta.url));
const out = resolve(process.argv[2] || join(here, '../../../Tirith-video/work'));
const require = createRequire(process.env.PLAYWRIGHT_CORE ? join(resolve(process.env.PLAYWRIGHT_CORE), 'x.js') : import.meta.url);
const { chromium } = require('playwright-core');
mkdirSync(out, { recursive: true });
const b = await chromium.launch({ channel: 'msedge' });
const p = await b.newPage({ viewport: { width: 1920, height: 1080 } });
for (const id of ['title', 'close', 'built']) {
  await p.goto(pathToFileURL(join(here, 'cards.html')).href + '#' + id);
  await p.reload(); await p.waitForTimeout(400);
  await p.screenshot({ path: join(out, id + '.png') });
}
await b.close();
console.log('cards written to', out);
