/* Run with: node web/js/company.test.mjs  (needs playwright-core and bundled headless Chromium; no servers).
   The Company view draws state.company (company.jsonl) as a timeline: each promotion or retirement with when, why. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import vm from 'node:vm';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { chromium } from './pw.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const page_url = pathToFileURL(path.join(here, '..', 'index.html')).href;
const holder = { window: {} };
vm.runInNewContext(fs.readFileSync(path.join(here, '..', '..', 'fixtures', 'state.demo.js'), 'utf8'), holder);
const base = holder.window.TIRITH_STATE;
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-company-'));
const iso = ms => new Date(ms).toISOString().replace(/\.\d+Z$/, 'Z');
const write = company => {
  const s = JSON.parse(JSON.stringify(base));
  s.generated_at = iso(Date.now());
  s.company = company;
  fs.writeFileSync(path.join(dir, 'state.js'), 'window.TIRITH_STATE = ' + JSON.stringify(s) + ';');
};
const text = page => page.evaluate(() => document.getElementById('view').textContent);

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
  const errors = [];
  page.on('pageerror', e => errors.push(String(e)));
  write([]);
  await page.goto(page_url + '#view=company&state=' + pathToFileURL(path.join(dir, 'state.js')).href);
  await page.waitForTimeout(800);
  assert.match(await text(page), /Nothing yet/);
  console.log('ok  empty log says so');

  write([
    { ts: iso(Date.now() - 2 * 3600e3), event: 'promote', role: 'manager', rule: 'parallel_items', measured: 3, threshold: 2, reason: '3 items run in parallel' },
    { ts: iso(Date.now() - 600e3), event: 'retire', role: 'ceo', rule: 'idle_seconds', measured: 3700, threshold: 3600, reason: 'idle for an hour' }]);
  await page.waitForTimeout(3500);
  const t = await text(page);
  assert.match(t, /Promoted/); assert.match(t, /Retired/);
  assert.match(t, /3 items run in parallel/); assert.match(t, /3700 against 3600/);
  assert.match(t, /10 min ago/); assert.match(t, /2 h ago/);
  const order = await page.$$eval('.co-row', rs => rs.map(r => r.className));
  assert.match(order[0], /co-retire/, 'newest first');
  console.log('ok  timeline shows event, role, rule, numbers, when and why');
  assert.deepEqual(errors, []);
} finally {
  await browser.close();
}
