// Renders the cards as PNG (for checking them by eye). Usage: node cards.mjs [outDir]   (default: ../../../Tirith-video/work)
import { createRequire } from 'node:module';
import { mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
const here = dirname(fileURLToPath(import.meta.url));
const out = resolve(process.argv[2] || join(here, 'out/work'));
const require = createRequire(join(resolve(process.env.PLAYWRIGHT_CORE || '/Users/andreseloyasanchezva/DEVELOP/propuestavenezuela/node_modules'), 'x.js'));
const { chromium } = require('playwright-core');
mkdirSync(out, { recursive: true });
const b = await chromium.launch({ channel: 'msedge' });
const p = await b.newPage({ viewport: { width: 1920, height: 1080 } });
for (const id of ['problem', 'close']) {
  await p.goto(pathToFileURL(join(here, 'cards.html')).href + '#' + id);
  await p.reload(); await p.waitForTimeout(id === 'problem' ? 9000 : 400);
  await p.screenshot({ path: join(out, id + '.png') });
}
await b.close();
console.log('cards written to', out);
