// Renders social-preview.html to docs/social-preview.png (1280x640). Usage: PLAYWRIGHT_CORE=<node_modules> node render.mjs
import { createRequire } from 'node:module';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
const here = dirname(fileURLToPath(import.meta.url));
const require = createRequire(process.env.PLAYWRIGHT_CORE ? join(resolve(process.env.PLAYWRIGHT_CORE), 'x.js') : import.meta.url);
const { chromium } = require('playwright-core');
const b = await chromium.launch({ channel: 'msedge' });
const p = await b.newPage({ viewport: { width: 1280, height: 640 } });
await p.goto(pathToFileURL(join(here, 'social-preview.html')).href);
await p.waitForTimeout(400);
await p.screenshot({ path: join(here, '..', 'social-preview.png') });
await b.close();
console.log('written docs/social-preview.png');
