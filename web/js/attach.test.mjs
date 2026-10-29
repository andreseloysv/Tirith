/* Run with: node web/js/attach.test.mjs  (needs playwright-core and the msedge channel; no servers).
   A long quick-reply option wraps onto as many lines as it needs, never clipped, next to the other buttons, wide
   and narrow. Files dropped on the chat are attached: in the app through its file-drop events (the drop target
   shows while dragging over the chat, only a drop on the chat reads the files), as removable chips (a picture as a
   thumbnail, any other file with its name and size), saved with their type before the chat row; in a browser the
   drop target shows and the page says files need the desktop app. Pass a png path to save a screenshot. */
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
const state = JSON.parse(JSON.stringify(holder.window.TIRITH_STATE));

const LONG = "I'll allow git for agents in the Tirith config, then relaunch the merge";
state.generated_at = new Date().toISOString();
state.waiting_on_you[0].options = [{ label: LONG, value: 'allow' }, { label: "I'll merge it myself", value: 'self' }, { label: 'Leave it for now', value: 'later' }];
state.inbox = [{ id: 'in-0a0b0c01', at: new Date(Date.now() - 60000).toISOString(), kind: 'chat', project: '', step: '', text: 'the log', from: 'owner', status: 'open',
  answers: [], manager_pending: false, action_runs: [], attachments: [{ path: '/h/attachments/in-0a0b0c01/1-build.log', type: 'text/plain', size: 2048, width: 0, height: 0, name: 'build.log' }] }];

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-attach-'));
const file = path.join(dir, 'state.js');
fs.writeFileSync(file, 'window.TIRITH_STATE = ' + JSON.stringify(state) + ';');

const PNG = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==';
/* the fake app: the state comes by command, events are kept to be fired by the test, files "dropped" are read */
const fakeApp = `
  window.__calls = []; window.__ev = {};
  window.__TAURI__ = {
    core: { invoke: (cmd, args) => {
      window.__calls.push({ cmd, args });
      if (cmd === 'current_state') return Promise.resolve(${JSON.stringify(state)});
      if (cmd === 'attachment_dropped') return Promise.resolve([
        { name: 'notes.txt', type: 'text/plain', size: 5, b64: 'aGVsbG8=' },
        { name: 'shot.png', type: 'image/png', size: 68, b64: '${PNG}' },
        { name: 'huge.bin', error: 'huge.bin is over 10 MB' }]);
      if (cmd === 'attachment_save') {
        const n = window.__calls.filter(c => c.cmd === 'attachment_save').length;
        return Promise.resolve({ path: '/h/attachments/' + args.messageId + '/' + n + '-' + args.name, type: args.mime, size: 5, name: args.name });
      }
      if (cmd === 'inbox_add') return Promise.resolve(args.id);
      return Promise.resolve(null);
    } },
    event: { listen: (name, fn) => { window.__ev[name] = fn; return Promise.resolve(() => {}); } }
  };`;

const browser = await chromium.launch({ channel: 'msedge' });
let passed = 0;
const ok = name => { console.log('ok  ' + name); passed++; };
const overlap = (a, b) => a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;
try {
  /* 1. the long option wraps, keeps its padding and its whole text, wide and narrow */
  for (const width of [1600, 1000, 560]) {
    const ctx = await browser.newContext({ viewport: { width, height: 1000 }, deviceScaleFactor: 2 });
    await ctx.addInitScript(fakeApp);
    const page = await ctx.newPage();
    await page.goto(page_url);
    const item = page.locator('.mc-m.turn[data-wait="' + state.waiting_on_you[0].id + '"]').first();
    const opt = item.locator('.nbtn.primary').first();
    await opt.waitFor({ timeout: 4000 });
    const m = await opt.evaluate(b => {
      const r = b.getBoundingClientRect(), row = b.parentElement.getBoundingClientRect(), cs = getComputedStyle(b);
      return { text: b.textContent, sh: b.scrollHeight, ch: b.clientHeight, sw: b.scrollWidth, cw: b.clientWidth, h: r.height, right: r.right, rowRight: row.right, pt: parseFloat(cs.paddingTop) };
    });
    assert.equal(m.text, LONG, 'the whole label');
    assert.ok(m.sh <= m.ch + 1 && m.sw <= m.cw + 1, `not clipped at ${width}px: ${JSON.stringify(m)}`);
    assert.ok(m.right <= m.rowRight + 0.5, `inside its row at ${width}px`);
    assert.ok(m.pt >= 4, 'keeps its vertical padding');
    await opt.scrollIntoViewIfNeeded();
    const boxes = await item.locator('.nacts > .nbtn, .nacts > .nreply').evaluateAll(els => els.map(e => { const r = e.getBoundingClientRect(); return { x: r.x, y: r.y, width: r.width, height: r.height }; }));
    for (let i = 0; i < boxes.length; i++) for (let j = i + 1; j < boxes.length; j++) assert.ok(!overlap(boxes[i], boxes[j]), `buttons ${i} and ${j} overlap at ${width}px`);
    if (process.argv[2] && width === 560) await page.screenshot({ path: process.argv[2].replace(/\.png$/, '-narrow.png') });
    await ctx.close();
  }
  ok('a long option wraps whole inside its row, no button overlaps another (1600, 1000, 560 px)');

  const ctx = await browser.newContext({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 2 });
  await ctx.addInitScript(fakeApp);
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', e => errors.push(String(e)));
  await page.goto(page_url);
  await page.locator('textarea[data-act="mc-text"]').first().waitFor({ timeout: 4000 });

  /* 2. a file already sent shows as a chip with its name and size */
  const sent = page.locator('.mc-m.me .mc-file').first();
  assert.match(await sent.innerText(), /build\.log\s*2 KB/);
  assert.equal(await sent.getAttribute('title'), '/h/attachments/in-0a0b0c01/1-build.log');
  ok('a sent file shows as a chip with its name and size');

  /* 3. the app's drag events: the drop target shows only over the chat (positions in physical pixels) */
  const at = async sel => page.locator(sel).first().evaluate(e => { const r = e.getBoundingClientRect(); return { x: (r.x + r.width / 2) * devicePixelRatio, y: (r.y + r.height / 2) * devicePixelRatio }; });
  const fire = (name, payload) => page.evaluate(([n, p]) => window.__ev[n]({ payload: p }), [name, payload]);
  const chat = await at('.mc-box'), away = await at('#nav');
  await fire('tauri://drag-enter', { paths: ['/u/notes.txt'], position: chat });
  assert.equal(await page.locator('.mchat.drop').count(), 1, 'drop target over the chat');
  assert.match(await page.locator('.mchat.drop').first().evaluate(e => getComputedStyle(e, '::after').content), /Drop files to attach/);
  await fire('tauri://drag-over', { position: away });
  assert.equal(await page.locator('.mchat.drop').count(), 0, 'no drop target away from the chat');
  await fire('tauri://drag-drop', { paths: ['/u/notes.txt'], position: away });
  assert.equal(await page.evaluate(() => window.__calls.filter(c => c.cmd === 'attachment_dropped').length), 0, 'a drop beside the chat reads nothing');
  await fire('tauri://drag-over', { position: chat });
  assert.equal(await page.locator('.mchat.drop').count(), 1);
  await fire('tauri://drag-leave', null);
  assert.equal(await page.locator('.mchat.drop').count(), 0, 'leaving the window clears it');
  ok('the drop target follows the pointer over the chat only');

  /* 4. a drop on the chat: chips for each file, the one over 10 MB is refused with the reason */
  await fire('tauri://drag-drop', { paths: ['/u/notes.txt', '/u/shot.png', '/u/huge.bin'], position: chat });
  await page.locator('.mc-strip .mc-file').first().waitFor({ timeout: 3000 });
  assert.deepEqual(await page.evaluate(() => window.__calls.filter(c => c.cmd === 'attachment_dropped').map(c => c.args.paths)), [['/u/notes.txt', '/u/shot.png', '/u/huge.bin']]);
  assert.match(await page.locator('.mc-strip .mc-file').first().innerText(), /notes\.txt\s*5 B/);
  assert.equal(await page.locator('.mc-strip .mc-th img').count(), 1, 'the picture as a thumbnail');
  assert.match(await page.locator('.mc-comp .ierr').first().innerText(), /huge\.bin is over 10 MB/);
  assert.equal(await page.locator('.mchat.drop').count(), 0);
  if (process.argv[2]) await page.screenshot({ path: process.argv[2] });
  ok('a drop on the chat adds removable chips (file and picture) and says why one was left out');

  /* 5. chips are removable; the message is sent with each file saved under its type first */
  await page.locator('.mc-strip .mc-th [data-act="mc-unpic"]').click();
  assert.equal(await page.locator('.mc-strip .mc-th').count(), 0);
  assert.equal(await page.locator('.mc-strip .mc-file').count(), 1);
  await page.locator('textarea[data-act="mc-text"]').first().fill('see the notes');
  await page.locator('[data-act="mc-send"]').first().click();
  await page.locator('.mc-m.sent').first().waitFor({ timeout: 3000 });
  const calls = await page.evaluate(() => window.__calls.filter(c => c.cmd === 'attachment_save' || c.cmd === 'inbox_add').map(c => c.args));
  assert.equal(calls.length, 2);
  assert.deepEqual([calls[0].name, calls[0].mime, calls[0].bytesBase64], ['notes.txt', 'text/plain', 'aGVsbG8=']);
  assert.equal(calls[1].text, 'see the notes');
  assert.deepEqual(calls[1].attachments.map(a => [a.name, a.type]), [['notes.txt', 'text/plain']]);
  assert.equal(calls[0].messageId, calls[1].id);
  assert.equal(await page.locator('.mc-strip').count(), 0, 'the composer is empty again');
  assert.match(await page.locator('.mc-m.sent .mc-file').first().innerText(), /notes\.txt/);
  ok('a removed chip is not sent; the file is saved with its type, then the chat row lists it');
  /* 6. the app pushes a drop straight into the page (files already read in Rust); it is logged; the thumbnail is small and tidy */
  await page.waitForTimeout(900);   /* a second drop within a moment is taken as the same one */
  const chat2 = await at('.mc-box');
  const push = d => page.evaluate(d => window.__tirithDrag(d), d);
  await push({ phase: 'enter', x: chat2.x, y: chat2.y });
  assert.equal(await page.locator('.mchat.drop').count(), 1, 'pushed enter marks the chat');
  await push({ phase: 'drop', x: chat2.x, y: chat2.y, files: [
    { name: 'a-rather-long-screenshot-name.png', type: 'image/png', size: 68, b64: PNG },
    { name: 'build-notes.txt', type: 'text/plain', size: 2048, b64: 'aGVsbG8=' }] });
  await page.locator('.mc-strip .mc-th img').first().waitFor({ timeout: 3000 });
  assert.equal(await page.locator('.mc-strip .mc-file').count(), 1);
  assert.equal(await page.locator('.mchat.drop').count(), 0);
  assert.match(await page.locator('.mc-strip .mc-th').first().innerText(), /a-rather-long.*\n?.*68 B/s);
  const geo = await page.evaluate(() => {
    const th = document.querySelector('.mc-strip .mc-th img').getBoundingClientRect(), x = document.querySelector('.mc-strip .mc-th button').getBoundingClientRect();
    return { inside: x.left >= th.left && x.right <= th.right && x.top >= th.top && x.bottom <= th.bottom, w: x.width };
  });
  assert.equal(geo.inside, true, 'the x sits inside the thumbnail');
  assert.ok(geo.w <= 20, 'the x is small');
  assert.ok(await page.evaluate(() => window.__calls.some(c => c.cmd === 'app_log' && /drop 2 file/.test(c.args.line))), 'the drop is logged');
  await page.screenshot({ path: path.join(here, '..', 'screenshots', 'chat-attach-thumb.png'), clip: await page.locator('.mc-comp').first().boundingBox() });
  ok('a drop pushed by the app adds the thumbnail and chip, the small x is inside the corner, and it is logged');
  assert.deepEqual(errors, [], 'no page errors');
  await ctx.close();

  /* 6. a browser (no app): the drop target shows, the drop says files need the app, a stray drop does not leave */
  const bctx = await browser.newContext({ viewport: { width: 1600, height: 1000 } });
  const bp = await bctx.newPage();
  await bp.goto(page_url + '#state=' + pathToFileURL(file).href);
  await bp.locator('.mc-box').first().waitFor({ timeout: 4000 });
  const dt = await bp.evaluateHandle(() => { const d = new DataTransfer(); d.items.add(new File(['hi'], 'a.txt', { type: 'text/plain' })); return d; });
  await bp.locator('.mc-box textarea').first().dispatchEvent('dragover', { dataTransfer: dt });
  assert.equal(await bp.locator('.mchat.drop').count(), 1, 'drop target in a browser');
  await bp.locator('.mc-box textarea').first().dispatchEvent('drop', { dataTransfer: dt });
  assert.equal(await bp.locator('.mchat.drop').count(), 0);
  assert.match(await bp.locator('.mc-comp .ierr').first().innerText(), /desktop app/);
  const before = bp.url();
  const prevented = await bp.evaluate(() => { const d = new DataTransfer(); d.items.add(new File(['x'], 'b.txt')); const e = new DragEvent('drop', { dataTransfer: d, bubbles: true, cancelable: true }); document.body.dispatchEvent(e); return e.defaultPrevented; });
  assert.ok(prevented && bp.url() === before, 'a drop beside the chat does not open the file');
  ok('browser: drop target, "attach files in the desktop app", a stray drop stays on the page');
  await bctx.close();
} finally {
  await browser.close();
  fs.rmSync(dir, { recursive: true, force: true });
}
console.log(passed + ' passed');
