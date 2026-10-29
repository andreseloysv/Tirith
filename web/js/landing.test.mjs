/* Run with: node web/js/landing.test.mjs  (needs playwright-core and the msedge channel; no servers).
   site/index.html loads from file:// with no console errors, every image exists, and the page does not
   scroll sideways at 360px and 1440px. Writes web/screenshots/landing-1440.png and landing-360.png. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';

const require = createRequire(process.env.PLAYWRIGHT_FROM || '/Users/andreseloyasanchezva/DEVELOP/propuestavenezuela/node_modules/x.js');
const { chromium } = require('playwright-core');

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(here, '..', '..');
const site = path.join(root, 'site');
const url = pathToFileURL(path.join(site, 'index.html')).href;

const html = fs.readFileSync(path.join(site, 'index.html'), 'utf8');
for (const m of html.matchAll(/(?:src|href)="(img\/[^"]+|style\.css)"/g)) {
  assert.ok(fs.existsSync(path.join(site, m[1])), `missing file ${m[1]}`);
}
assert.ok(!/<script/i.test(html), 'the page must not load scripts');
assert.ok(!/(?:src|href)="https?:\/\/[^"]*\.(?:js|png|jpe?g|webp|gif)/i.test(html), 'no third-party assets');

const b = await chromium.launch({ channel: 'msedge' });
try {
  for (const width of [360, 1440]) {
    const p = await b.newPage({ viewport: { width, height: 900 } });
    const errors = [];
    p.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
    p.on('pageerror', e => errors.push(String(e)));
    p.on('requestfailed', r => errors.push('request failed: ' + r.url()));
    await p.goto(url);
    // lazy images load only near the viewport: scroll through the page once
    await p.evaluate(async () => {
      for (let y = 0; y < document.body.scrollHeight; y += 600) { window.scrollTo({ top: y, behavior: 'instant' }); await new Promise(r => setTimeout(r, 120)); }
      window.scrollTo({ top: 0, behavior: 'instant' });
    });
    await p.waitForTimeout(800);
    const broken = await p.evaluate(() => [...document.images].filter(i => !i.complete || i.naturalWidth === 0).map(i => i.getAttribute('src')));
    assert.deepEqual(broken, [], `broken images at ${width}px`);
    const over = await p.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    assert.ok(over <= 0, `horizontal overflow of ${over}px at ${width}px`);
    assert.deepEqual(errors, [], `console errors at ${width}px`);
    await p.screenshot({ path: path.join(root, 'web', 'screenshots', `landing-${width}.png`), fullPage: true });
    await p.close();
  }
} finally {
  await b.close();
}
console.log('landing ok');
