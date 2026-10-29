/* Run with: node web/js/chatgrow.test.mjs  (needs playwright-core and the msedge channel; no servers).
   A long multi-line message makes the chat box grow up to half the window, then scroll inside; no text is lost. */
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
const holder = { window: {} };
vm.runInNewContext(fs.readFileSync(path.join(here, '..', '..', 'fixtures', 'state.demo.js'), 'utf8'), holder);
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-chatgrow-'));
const file = path.join(dir, 'state.js');
fs.writeFileSync(file, 'window.TIRITH_STATE = ' + JSON.stringify(holder.window.TIRITH_STATE) + ';');

const sel = 'textarea[data-act="mc-text"]';
const browser = await chromium.launch({ channel: 'msedge' });
try {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  const errors = [];
  page.on('pageerror', e => errors.push(String(e)));
  await page.goto(pathToFileURL(path.join(here, '..', 'index.html')).href + '#state=' + pathToFileURL(file).href);
  await page.waitForTimeout(800);
  const box = page.locator(sel).first();
  await box.waitFor({ timeout: 3000 });
  const h = () => page.evaluate(s => document.querySelector(s).getBoundingClientRect().height, sel);
  const h0 = await h();
  await box.click();
  await page.keyboard.type('Line 1 of my prompt');
  for (let i = 2; i <= 4; i++) { await page.keyboard.press('Shift+Enter'); await page.keyboard.type('Line ' + i + ' of my prompt'); }
  const h4 = await h();
  assert.ok(h4 > h0 + 30, 'grows with 4 lines (' + h0 + ' -> ' + h4 + ')');
  for (let i = 5; i <= 60; i++) { await page.keyboard.press('Shift+Enter'); await page.keyboard.type('Line ' + i + ' of my prompt'); }
  const hm = await h();
  assert.ok(hm <= 501 && hm >= 480, 'capped at half the window (' + hm + ')');
  const r = await page.evaluate(s => { const el = document.querySelector(s); return { v: el.value, sh: el.scrollHeight, ch: el.clientHeight }; }, sel);
  assert.equal(r.v.split('\n').length, 60, 'no lines lost');
  assert.ok(r.v.endsWith('Line 60 of my prompt'), 'text intact');
  assert.ok(r.sh > r.ch, 'scrolls inside once capped');
  const logOk = await page.evaluate(() => { const l = document.getElementById('mc-log'); return l.scrollHeight - l.scrollTop - l.clientHeight < 48; });
  assert.ok(logOk, 'message list stays at the bottom');
  await page.screenshot({ path: path.join(here, '..', 'screenshots', 'chat-grow.png') });
  await page.keyboard.press('Enter');
  await page.waitForTimeout(600);
  const hs = await h();
  assert.ok(hs <= h0 + 2, 'shrinks back after sending (' + hs + ')');
  assert.deepEqual(errors, [], 'no page errors');
  console.log('ok  chat box grows, caps, shrinks (' + h0 + ' -> ' + h4 + ' -> ' + hm + ' -> ' + hs + ')');
} finally {
  await browser.close();
  fs.rmSync(dir, { recursive: true, force: true });
}
console.log('1 passed');
