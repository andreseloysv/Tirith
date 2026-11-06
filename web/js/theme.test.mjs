/* Run with: node web/js/theme.test.mjs [screenshot-dir]  (needs playwright-core and bundled headless Chromium; no servers).
   Settings > Appearance: System (default) follows the OS color scheme; Light and Dark force data-theme on <html>,
   the choice is kept in localStorage and applied again on reload, and picking System clears it. */
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { chromium } from './pw.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(here, '..', '..');
const url = pathToFileURL(path.join(root, 'web', 'index.html')).href + '#state=' + pathToFileURL(path.join(root, 'fixtures', 'state.demo.js')).href + '&project=tirith&view=settings';
const shots = process.argv[2];

const browser = await chromium.launch({ headless: true });
try {
  const ctx = await browser.newContext({ viewport: { width: 1200, height: 800 }, colorScheme: 'light' });
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', e => errors.push(String(e)));
  const attr = () => page.evaluate(() => document.documentElement.dataset.theme || null);
  const stored = () => page.evaluate(() => localStorage.getItem('tirith.theme'));
  const bg = () => page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  const pick = async name => { await page.click(`[data-theme-pick="${name}"]`); await page.waitForTimeout(50); };

  await page.goto(url); await page.reload();
  await page.waitForSelector('#theme-seg');
  assert.equal(await attr(), null); assert.equal(await stored(), null);
  assert.equal(await page.getAttribute('[data-theme-pick="system"]', 'aria-pressed'), 'true');
  const light = await bg();
  console.log('ok  default is System, no data-theme');

  await page.emulateMedia({ colorScheme: 'dark' });
  assert.notEqual(await bg(), light, 'System follows a dark OS');
  await page.emulateMedia({ colorScheme: 'light' });
  assert.equal(await bg(), light, 'System follows a light OS');
  console.log('ok  System follows prefers-color-scheme');

  await pick('dark');
  assert.equal(await attr(), 'dark'); assert.equal(await stored(), 'dark');
  assert.notEqual(await bg(), light, 'Dark overrides a light OS');
  if (shots) await page.screenshot({ path: path.join(shots, 'theme-dark.png') });
  await page.reload(); await page.waitForSelector('#theme-seg');
  assert.equal(await attr(), 'dark', 'applied again after reload');
  assert.equal(await page.getAttribute('[data-theme-pick="dark"]', 'aria-pressed'), 'true');
  console.log('ok  Dark is applied, stored and survives a reload');

  await page.emulateMedia({ colorScheme: 'dark' });
  await pick('light');
  assert.equal(await attr(), 'light'); assert.equal(await stored(), 'light');
  assert.equal(await bg(), light, 'Light overrides a dark OS');
  console.log('ok  Light overrides a dark OS');

  await pick('system');
  assert.equal(await attr(), null); assert.equal(await stored(), null);
  assert.notEqual(await bg(), light, 'System back on the dark OS');
  console.log('ok  System clears the choice');
  assert.deepEqual(errors, []);
  await ctx.close();
} finally { await browser.close(); }
