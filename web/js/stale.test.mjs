/* Run with: node web/js/stale.test.mjs  (needs playwright-core and the msedge channel; no servers).
   A calm banner shows only when the state is older than 30 s, and goes away when it is fresh again. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import vm from 'node:vm';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';

const require = createRequire(process.env.PLAYWRIGHT_FROM || '/Users/andreseloyasanchezva/DEVELOP/propuestavenezuela/node_modules/x.js');
const { chromium } = require('playwright-core');

const here = path.dirname(fileURLToPath(import.meta.url));
const page_url = pathToFileURL(path.join(here, '..', 'index.html')).href;
const holder = { window: {} };
vm.runInNewContext(fs.readFileSync(path.join(here, '..', '..', 'fixtures', 'state.demo.js'), 'utf8'), holder);
const base = holder.window.TIRITH_STATE;

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-stale-'));
const iso = ms => new Date(Date.now() - ms).toISOString().replace(/\.\d+Z$/, 'Z');
const write = ago => {
  const s = JSON.parse(JSON.stringify(base));
  s.generated_at = iso(ago);
  fs.writeFileSync(path.join(dir, 'state.js'), 'window.TIRITH_STATE = ' + JSON.stringify(s) + ';');
};
const banner = page => page.evaluate(() => {
  const b = document.getElementById('stale');
  return b && !b.hidden && getComputedStyle(b).display !== 'none' ? b.textContent : null;
});

const browser = await chromium.launch({ channel: 'msedge' });
try {
  const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
  const errors = [];
  page.on('pageerror', e => errors.push(String(e)));
  write(5 * 1000);
  await page.goto(page_url + '#state=' + pathToFileURL(path.join(dir, 'state.js')).href);
  await page.waitForTimeout(800);
  assert.equal(await banner(page), null, 'fresh state: no banner');
  console.log('ok  fresh state shows no banner');

  write(45 * 1000);
  await page.waitForTimeout(3500);
  assert.equal(await banner(page), 'Tirith is not updating — restarting…');
  console.log('ok  state older than 30 s shows the calm banner');

  write(1 * 1000);
  await page.waitForTimeout(3500);
  assert.equal(await banner(page), null, 'fresh again: banner gone');
  console.log('ok  banner is gone when the state is fresh again');
  assert.deepEqual(errors, []);
} finally {
  await browser.close();
  fs.rmSync(dir, { recursive: true, force: true });
}
console.log('3 passed');
