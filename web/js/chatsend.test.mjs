/* Run with: node web/js/chatsend.test.mjs  (needs playwright-core and the msedge channel; no servers).
   A chat message is never lost: a send whose Tauri command rejects shows
   "Not sent · Retry · Copy text" and gives the text back to the composer; a command that resolves is "Delivered",
   never "Not sent", however slowly the state shows it; the tirith:// link path says "Not confirmed" after 30 s; a send that reaches the state turns normal;
   a draft survives a reload. Pass a png path as the first argument to save a screenshot of the "Not sent" message. */
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

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-chatsend-'));
const file = path.join(dir, 'state.js');
let n = 0;
function write(inbox = []) {
  const s = JSON.parse(JSON.stringify(base));
  n++;
  s.generated_at = new Date().toISOString();
  s.inbox = inbox;
  if (s.agents && s.agents[0]) s.agents[0].summary = 'tick ' + n;
  fs.writeFileSync(file, 'window.TIRITH_STATE = ' + JSON.stringify(s) + ';');
}
write();
const timer = setInterval(() => write(currentInbox), 1000);
let currentInbox = [];

/* the fake app: inbox_add fails, or "succeeds" without writing, or writes the row into the state */
const fake = mode => `
  window.__calls = [];
  window.__TAURI__ = { core: { invoke: (cmd, args) => {
    window.__calls.push({ cmd, args });
    if (cmd !== 'inbox_add') return Promise.resolve(null);
    const mode = window.__mode || ${JSON.stringify(mode)};
    if (mode === 'error') return Promise.reject('disk full');
    return Promise.resolve(args.id || 'in-00000000');
  } } };`;

const browser = await chromium.launch({ channel: 'msedge' });
let passed = 0;
const ok = name => { console.log('ok  ' + name); passed++; };
try {
  const ctx = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  await ctx.addInitScript(fake('error'));
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', e => errors.push(String(e)));
  await page.goto(page_url + '#state=' + pathToFileURL(file).href);
  await page.waitForTimeout(800);
  const box = page.locator('textarea[data-act="mc-text"]').first();
  await box.waitFor({ timeout: 3000 });

  /* 1. the command errors: the text is back, the thread says Not sent */
  await box.fill('Please fix the login page');
  await page.locator('[data-act="mc-send"]').first().click();
  const bad = page.locator('.mc-m.failed').first();
  await bad.waitFor({ timeout: 3000 });
  assert.match(await bad.innerText(), /Please fix the login page/);
  assert.match(await bad.innerText(), /Not sent\s*·\s*Retry\s*·\s*Copy text/);
  assert.equal(await box.inputValue(), 'Please fix the login page', 'text back in the composer');
  assert.equal(await page.locator('.mc-m.sending').count(), 0);
  ok('a failed command shows "Not sent · Retry · Copy text" and restores the text');
  if (process.argv[2]) await page.screenshot({ path: process.argv[2] });

  /* 2. retry with the same client id reaches the command again; the page says Sending… meanwhile */
  await page.evaluate(() => { window.__mode = 'silent'; });
  await page.locator('[data-act="mc-retry"]').first().click();
  await page.locator('.mc-m.sent').first().waitFor({ timeout: 3000 });
  assert.equal(await box.inputValue(), '', 'retry takes the restored text out of the composer');
  const calls = await page.evaluate(() => window.__calls.filter(c => c.cmd === 'inbox_add').map(c => c.args));
  assert.equal(calls.length, 2);
  assert.ok(calls[0].cid && calls[0].cid === calls[1].cid, 'same client id on the retry');
  assert.equal(calls[0].text, 'Please fix the login page');
  ok('retry resends with the same client id and ends Delivered');

  /* 3. accepted but the state is slow: still Delivered well past the old 10 s, never Not sent */
  await page.waitForTimeout(11500);
  assert.equal(await page.locator('.mc-m.failed').count(), 0, 'no Not sent after the old timeout');
  assert.match(await page.locator('.mc-m.sent').first().innerText(), /Delivered/);
  assert.equal(await box.inputValue(), '', 'text not restored to the composer');
  ok('invoke resolved + state slow -> Delivered, no Not sent, no restored draft');

  /* 4. it shows up in the state (late): the message turns normal and the restored text leaves the composer */
  const cid = calls[0].cid;
  currentInbox = [{ id: 'in-0a0b0c0d', cid, at: new Date().toISOString(), kind: 'chat', project: '', step: '', text: 'Please fix the login page', from: 'owner', status: 'open', answers: [], manager_pending: true, action_runs: [] }];
  write(currentInbox);
  await page.waitForFunction(() => !document.querySelector('.mc-m.failed') && !document.querySelector('.mc-m.sending') && !document.querySelector('.mc-m.sent'), null, { timeout: 8000 });
  assert.match(await page.locator('#mc-log').first().innerText(), /Please fix the login page/);
  assert.equal(await box.inputValue(), '', 'composer cleared once confirmed');
  ok('a message found in the state by client id becomes a normal message');

  /* 5. a fresh send that the state confirms never goes through Not sent */
  await page.evaluate(() => { window.__mode = 'silent'; });
  await box.fill('second message');
  await page.locator('[data-act="mc-send"]').first().click();
  await page.locator('.mc-m.sent').first().waitFor({ timeout: 3000 });
  const cid2 = await page.evaluate(() => window.__calls.filter(c => c.cmd === 'inbox_add').pop().args.cid);
  currentInbox = [...currentInbox, { id: 'in-0a0b0c0e', cid: cid2, at: new Date().toISOString(), kind: 'chat', project: '', step: '', text: 'second message', from: 'owner', status: 'open', answers: [], manager_pending: true, action_runs: [] }];
  write(currentInbox);
  await page.waitForFunction(() => !document.querySelector('.mc-m.sending'), null, { timeout: 8000 });
  assert.equal(await page.locator('.mc-m.failed').count(), 0);
  ok('a confirmed send turns normal without failing');

  /* 6. link path (no app): nothing in the state for 30 s -> Not confirmed with Retry, never "failed" wording */
  const ctx2 = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  await ctx2.addInitScript('window.__opened = []; document.addEventListener("click", e => { const a = e.target.closest && e.target.closest("a"); if (a && /^tirith:/.test(a.href)) { window.__opened.push(a.href); e.preventDefault(); } }, true);');
  const page2 = await ctx2.newPage();
  const lfile = path.join(dir, 'state-link.js');
  const ls = JSON.parse(JSON.stringify(base)); ls.launch = { token: 'tok', available: true };
  fs.writeFileSync(lfile, 'window.TIRITH_STATE = ' + JSON.stringify(ls) + ';');
  await page2.goto(page_url + '#state=' + pathToFileURL(lfile).href);
  await page2.waitForTimeout(800);
  const box2 = page2.locator('textarea[data-act="mc-text"]').first();
  await box2.fill('via the link');
  await page2.locator('[data-act="mc-send"]').first().click();
  await page2.locator('.mc-m.sending').first().waitFor({ timeout: 3000 });
  assert.equal((await page2.evaluate(() => window.__opened)).length, 1, 'link opened');
  await page2.waitForTimeout(12000);
  assert.equal(await page2.locator('.mc-m.failed').count(), 0, 'still waiting after the old 10 s');
  await page2.locator('.mc-m.failed').first().waitFor({ timeout: 25000 });
  const txt = await page2.locator('.mc-m.failed').first().innerText();
  assert.match(txt, /Not confirmed\s*·\s*Retry/);
  assert.doesNotMatch(txt, /Not sent/);
  assert.equal(await box2.inputValue(), 'via the link');
  ok('link path: not confirmed after 30 s, with Retry');
  await ctx2.close();

  /* 7. the draft survives a reload */
  await box.fill('half written thought');
  await page.reload();
  await page.waitForTimeout(800);
  assert.equal(await page.locator('textarea[data-act="mc-text"]').first().inputValue(), 'half written thought');
  ok('a draft survives a reload');

  assert.deepEqual(errors, [], 'no page errors');
  await ctx.close();
} finally {
  clearInterval(timer);
  await browser.close();
  fs.rmSync(dir, { recursive: true, force: true });
}
console.log(passed + ' passed');
