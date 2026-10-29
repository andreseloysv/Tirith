/* Run with: node web/js/chatdrop.test.mjs  (needs playwright-core and the msedge channel; no servers).
   An image dropped on the chat, or pasted into the composer, becomes a thumbnail in the composer strip;
   a non-image file does not. */
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
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-chatdrop-'));
const file = path.join(dir, 'state.js');
fs.writeFileSync(file, 'window.TIRITH_STATE = ' + JSON.stringify(Object.assign({}, holder.window.TIRITH_STATE, { generated_at: new Date().toISOString() })) + ';');

const fake = 'window.__TAURI__ = { core: { invoke: () => Promise.resolve(null) } };';
/* a 1x1 png */
const PNG = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/q842iQAAAABJRU5ErkJggg==';
/* build a DataTransfer holding one file in the page, then fire the event on `sel` */
const fire = (page, type, sel, kind) => page.evaluate(([type, sel, kind, png]) => {
  const dt = new DataTransfer();
  if (kind === 'png') {
    const b = Uint8Array.from(atob(png), c => c.charCodeAt(0));
    dt.items.add(new File([b], 'shot.png', { type: 'image/png' }));
  } else dt.items.add(new File(['hello'], 'notes.txt', { type: 'text/plain' }));
  const el = document.querySelector(sel);
  const ev = type === 'paste' ? new ClipboardEvent('paste', { clipboardData: dt, bubbles: true, cancelable: true })
    : new DragEvent(type, { dataTransfer: dt, bubbles: true, cancelable: true });
  el.dispatchEvent(ev);
}, [type, sel, kind, PNG]);

const browser = await chromium.launch({ channel: 'msedge' });
let passed = 0;
const ok = name => { console.log('ok  ' + name); passed++; };
try {
  const ctx = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  await ctx.addInitScript(fake);
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', e => errors.push(String(e)));
  await page.goto(page_url + '#state=' + pathToFileURL(file).href);
  await page.locator('textarea[data-act="mc-text"]').first().waitFor({ timeout: 3000 });

  await fire(page, 'drop', '.mchat', 'txt');
  await page.waitForTimeout(300);
  assert.equal(await page.locator('.mc-strip img').count(), 0, 'a text file adds nothing');
  ok('a dropped non-image file adds no thumbnail');

  await fire(page, 'drop', '.mchat', 'png');
  await page.locator('.mc-strip img').first().waitFor({ timeout: 3000 });
  assert.equal(await page.locator('.mc-strip img').count(), 1);
  ok('a dropped image adds a thumbnail');

  await fire(page, 'paste', 'textarea[data-act="mc-text"]', 'png');
  await page.waitForFunction(() => document.querySelectorAll('.mc-strip img').length === 2, null, { timeout: 3000 });
  ok('a pasted image adds a thumbnail');

  assert.deepEqual(errors, [], 'no page errors');
  await ctx.close();
} finally {
  await browser.close();
  fs.rmSync(dir, { recursive: true, force: true });
}
console.log(passed + ' passed');
