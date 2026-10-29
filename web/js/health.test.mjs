/* Run with: node web/js/health.test.mjs  (needs playwright-core and the msedge channel; no servers).
   The header dot follows state.health: hidden without it, green / amber / red with it, a click lists the checks. */
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

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-health-'));
const write = health => {
  const s = JSON.parse(JSON.stringify(base));
  s.generated_at = new Date().toISOString().replace(/\.\d+Z$/, 'Z');
  if (health) s.health = health;
  fs.writeFileSync(path.join(dir, 'state.js'), 'window.TIRITH_STATE = ' + JSON.stringify(s) + ';');
};
const checks = (level, text) => [
  { id: 'collector', label: 'Tirith itself', level: 'ok', text: 'Tirith is collecting normally.' },
  { id: 'manager', label: 'The manager', level, text }];
const dot = page => page.evaluate(() => {
  const b = document.getElementById('health-btn'), p = document.getElementById('health-panel');
  return { hidden: b.hidden || getComputedStyle(b).display === 'none', cls: b.className, panel: p.hidden ? null : p.textContent };
});

const browser = await chromium.launch({ channel: 'msedge' });
try {
  const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
  const errors = [];
  page.on('pageerror', e => errors.push(String(e)));
  write(null);
  await page.goto(page_url + '#state=' + pathToFileURL(path.join(dir, 'state.js')).href);
  await page.waitForTimeout(800);
  assert.equal((await dot(page)).hidden, true, 'no health in the state: no dot');
  console.log('ok  no health, no dot');

  write({ level: 'green', at: 'x', checks: checks('ok', 'The manager is answering normally.') });
  await page.waitForTimeout(3500);
  let d = await dot(page);
  assert.equal(d.hidden, false);
  assert.match(d.cls, /green/);
  assert.equal(d.panel, null, 'closed until clicked');
  console.log('ok  green dot');

  await page.click('#health-btn');
  d = await dot(page);
  assert.match(d.panel, /All is well/);
  assert.match(d.panel, /Tirith itself/);
  assert.match(d.panel, /The manager is answering normally\./);
  console.log('ok  click lists the checks in plain words');

  write({ level: 'amber', at: 'x', checks: checks('healing', 'The manager was stuck; restarting it.') });
  await page.waitForTimeout(3500);
  d = await dot(page);
  assert.match(d.cls, /amber/);
  assert.match(d.panel, /Tirith is fixing something/);
  assert.match(d.panel, /restarting it/);
  console.log('ok  amber while healing, panel stays open');

  write({ level: 'red', at: 'x', checks: checks('failed', 'The manager is not answering and restarting it did not help.') });
  await page.waitForTimeout(3500);
  d = await dot(page);
  assert.match(d.cls, /red/);
  assert.match(d.panel, /Something needs a look/);
  await page.keyboard.press('Escape');
  assert.equal((await dot(page)).panel, null);
  console.log('ok  red only for a failed fix; Escape closes');

  assert.deepEqual(errors, []);
} finally {
  await browser.close();
}
