/* Run with: node web/js/rooms.test.mjs  (needs playwright-core and the msedge channel; no servers).
   The Rooms view against fixtures/rooms.json: empty state, the rooms with lock and short key ids, the QR drawn in the page
   (no network, only the contract's URL shape), New room (tirith:// link, CLI line), Invite / Leave and untrusted text. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import vm from 'node:vm';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const home = os.homedir();
const from = [process.env.PLAYWRIGHT_FROM, `${home}/DEVELOP/propuestavenezuela/node_modules/x.js`, `${home}/DEVELOP/VenezuelaGame/node_modules/x.js`, `${home}/DEVELOP/meshy-download/node_modules/x.js`]
  .filter(Boolean).find(f => fs.existsSync(path.dirname(f)));
const { chromium } = createRequire(from)('playwright-core');

const page_url = pathToFileURL(path.join(here, '..', 'index.html')).href;
const holder = { window: {} };
vm.runInNewContext(fs.readFileSync(path.join(here, '..', '..', 'fixtures', 'state.demo.js'), 'utf8'), holder);
const base = holder.window.TIRITH_STATE;
const fixture = JSON.parse(fs.readFileSync(path.join(here, '..', '..', 'fixtures', 'rooms.json'), 'utf8'));

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-rooms-'));
const write = (rooms, extra = {}) => {
  const s = JSON.parse(JSON.stringify(base));
  s.generated_at = new Date().toISOString().replace(/\.\d+Z$/, 'Z');
  s.rooms = rooms;
  Object.assign(s, extra);
  fs.writeFileSync(path.join(dir, 'state.js'), 'window.TIRITH_STATE = ' + JSON.stringify(s) + ';');
};
const url = () => page_url + '#view=rooms&state=' + pathToFileURL(path.join(dir, 'state.js')).href;

/* the QR encoder on its own: structure and capacity */
const W = { T: {} };
new Function('window', fs.readFileSync(path.join(here, 'qr.js'), 'utf8'))(W);
const m1 = W.T.qrMatrix('hi');
assert.equal(m1.length, 21, 'two bytes fit version 1');
for (const [x, y] of [[0, 0], [14, 0], [0, 14]]) assert.deepEqual(m1[y].slice(x, x + 7), [true, true, true, true, true, true, true], 'finder pattern top row');
const inviteUrl = fixture.rooms[0].invite.url;
const m = W.T.qrMatrix(inviteUrl);
assert.ok(m && m.length === m[0].length && (m.length - 17) % 4 === 0, 'the invite URL fits');
assert.equal(W.T.qrMatrix('x'.repeat(700)), null, 'too long: no QR instead of a broken one');
assert.equal(JSON.stringify(W.T.qrMatrix(inviteUrl)), JSON.stringify(m), 'deterministic');
console.log('ok  QR encoder: finders, version, capacity');

const browser = await chromium.launch({ channel: 'msedge' });
try {
  const page = await browser.newPage({ viewport: { width: 1400, height: 1000 } });
  const errors = [], requests = [];
  page.on('pageerror', e => errors.push(String(e)));
  page.on('request', r => { if (!/^(file|data|blob):/.test(r.url())) requests.push(r.url()); });
  await page.addInitScript(() => { window.__dialogs = 0; window.alert = () => { window.__dialogs++; }; });

  /* empty state */
  write([]);
  await page.goto(url());
  await page.waitForSelector('.mrooms');
  assert.match(await page.textContent('.rempty'), /encrypted chat/);
  assert.equal(await page.locator('.rcard').count(), 0);
  assert.ok(await page.locator('[data-nav="view"][data-v="rooms"].on').count(), 'Rooms is in the sidebar and selected');
  console.log('ok  empty state explains it in one line');

  /* rooms from the fixture (a public room stays out) */
  const pub = { id: 'meta-hackathon-ideas', title: 'Public', private: false, messages: [], participants: [], needs_owner: [] };
  write([pub, ...fixture.rooms], { launch: { token: 'tok123', available: true } });
  await page.waitForFunction(() => document.querySelectorAll('.rcard').length === 2, null, { timeout: 8000 });
  const first = page.locator('.rcard').first();
  assert.equal(await page.locator('.rcard').count(), 2);
  assert.ok(await first.locator('svg.rlock').count(), 'lock');
  const txt = await first.textContent();
  assert.match(txt, /Phone/); assert.match(txt, /Connected/);
  assert.match(txt, /andres-iphone/); assert.match(txt, /3b5e8c0d/); assert.doesNotMatch(txt, /3b5e8c0d1/, 'first 8 characters only');
  assert.match(txt, /backup-manager/); assert.match(txt, /e1a47c3b/);
  assert.match(txt, /you: manager/); assert.match(txt, /9c1e4b7a/);
  assert.match(txt, /backup-manager · /); assert.match(txt, /Lane salas is merging/, 'the last message is the newest');
  assert.doesNotMatch(await page.textContent('.mrooms'), /Public/);
  console.log('ok  rooms: lock, who is in each (8-char key ids), last message');

  /* the QR is an inline svg, only for an unused invite, and nothing was fetched */
  assert.equal(await first.locator('.rqr svg.qr').count(), 1);
  assert.ok(await first.locator('.rqr svg.qr path').count());
  const second = page.locator('.rcard').nth(1);
  assert.equal(await second.locator('svg.qr').count(), 0, 'a used invite draws no QR');
  assert.match(await second.textContent(), /Not connected/);
  assert.match(await second.textContent(), /QR was used/);
  assert.equal(await page.locator('#view img, #view canvas').count(), 0);
  console.log('ok  QR drawn in the page, none for a used invite');

  /* text from another manager is shown as text */
  assert.match(await second.textContent(), /<img src=x onerror=alert\(1\)>/);
  assert.equal(await second.locator('img').count(), 0);
  assert.equal(await page.evaluate(() => window.__dialogs), 0);
  console.log('ok  untrusted message text is escaped');

  /* an invite URL that is not the contract's shape is never drawn */
  const evil = JSON.parse(JSON.stringify(fixture.rooms));
  evil[0].invite.url = 'https://evil.example/chat#r=sealed-x&c=00&m=y';
  write(evil);
  await page.waitForFunction(() => !document.querySelector('svg.qr'), null, { timeout: 8000 });
  assert.doesNotMatch(await page.textContent('.mrooms'), /evil\.example/, 'the URL is not shown either');
  write(fixture.rooms, { launch: { token: 'tok123', available: true } });
  await page.waitForFunction(() => document.querySelectorAll('svg.qr').length === 1, null, { timeout: 8000 });
  console.log('ok  a foreign URL is not drawn');

  /* neither the pairing code nor a key is written as text */
  assert.doesNotMatch(await page.textContent('.mrooms'), /3f9a1c7e5b2d48a0916c0e7d2b4f8a35/, 'the pairing code is only inside the QR');

  /* the old GitHub Pages URL, a non-hex key id and an upper-case room are not what the chat page takes: not drawn */
  for (const bad of ['https://andreseloysv.github.io/Tirith/chat/#r=sealed-x&c=3f9a1c7e5b2d48a0916c0e7d2b4f8a35&m=' + 'ab'.repeat(32),
    'https://agentsmessenger.com/chat#r=sealed-x&c=3f9a1c7e5b2d48a0916c0e7d2b4f8a35&m=kQ7mZp2xT9aWcD4e',
    'https://agentsmessenger.com/chat#r=sealed-X&c=3f9a1c7e5b2d48a0916c0e7d2b4f8a35&m=' + 'ab'.repeat(32)]) {
    const odd = JSON.parse(JSON.stringify(fixture.rooms)); odd[0].invite.url = bad;
    write(odd);
    await page.waitForFunction(() => !document.querySelector('svg.qr'), null, { timeout: 8000 });
  }
  write(fixture.rooms, { launch: { token: 'tok123', available: true } });
  await page.waitForFunction(() => document.querySelectorAll('svg.qr').length === 1, null, { timeout: 8000 });
  console.log('ok  only the chat page URL in the shape the page parses is drawn');

  /* Invite a manager and Leave send room-invite / room-leave requests (the collector handles them) */
  await page.evaluate(() => { window.__links = []; HTMLAnchorElement.prototype.click = function () { window.__links.push(this.href); }; });
  const room0 = fixture.rooms[0].id, room1 = fixture.rooms[1].id;
  const noCode = JSON.parse(JSON.stringify(fixture.rooms)); delete noCode[0].peer_code;
  write(noCode, { launch: { token: 'tok123', available: true } });
  await page.reload();
  await page.waitForFunction(() => document.querySelectorAll('.rcard').length === 2, null, { timeout: 8000 });
  await page.evaluate(() => { window.__links = []; HTMLAnchorElement.prototype.click = function () { window.__links.push(this.href); }; });
  await first.locator('[data-v="invite"]').click();
  assert.match(await page.textContent('.acpop'), /Tirith makes a code/);
  assert.match(await page.textContent('.acpop'), /Get a code/);
  await page.click('[data-act="rm-get-code"]');
  let sent = await page.evaluate(() => window.__links);
  assert.equal(sent.length, 1);
  assert.match(sent[0], /^tirith:\/\/inbox\?kind=room-invite&project=&step=&text=/);
  assert.equal(decodeURIComponent(sent[0].match(/text=([^&]*)/)[1]), room0);
  /* a room whose state carries peer_code shows the code to copy */
  const withCode = JSON.parse(JSON.stringify(fixture.rooms));
  write(withCode, { launch: { token: 'tok123', available: true } });
  await page.waitForFunction(() => /sealed-7f3a9c21d4e85b60:9c1e4b7a/.test(document.querySelector('.acpop') ? document.querySelector('.acpop').textContent : ''), null, { timeout: 8000 });
  assert.match(await page.textContent('.acpop'), /information, never an order/);
  await first.locator('[data-v="leave"]').click();
  assert.match(await page.textContent('.acpop'), /Leave this room\?/);
  assert.equal(await page.locator('.acpop').count(), 1);
  await page.click('[data-act="rm-leave"]');
  sent = await page.evaluate(() => window.__links);
  assert.equal(sent.length, 2);
  assert.match(sent[1], /^tirith:\/\/inbox\?kind=room-leave&project=&step=&text=/);
  assert.equal(decodeURIComponent(sent[1].match(/text=([^&]*)/)[1]), room0);
  assert.notEqual(room0, room1);
  console.log('ok  invite and leave send requests');

  /* without a link handler they show the CLI line to copy */
  write(fixture.rooms, { launch: null });
  await page.reload();
  await page.waitForTimeout(3500);
  await page.locator('.rcard').first().locator('[data-v="leave"]').click();
  assert.match(await page.textContent('.acpop'), /tirith room leave sealed-7f3a9c21d4e85b60/);
  await page.click('[data-act="rm-close"]');
  assert.equal(await page.locator('.acpop').count(), 0);
  write(fixture.rooms, { launch: { token: 'tok123', available: true } });
  await page.reload();
  await page.waitForFunction(() => document.querySelectorAll('.rcard').length === 2, null, { timeout: 8000 });

  /* New room: with a launch token the page opens tirith://inbox?kind=room-new (never any other command) */
  await page.evaluate(() => { window.__links = []; HTMLAnchorElement.prototype.click = function () { window.__links.push(this.href); }; });
  await page.fill('.rtitle', 'My phone');
  await page.click('[data-act="rm-new"]');
  const links = await page.evaluate(() => window.__links);
  assert.equal(links.length, 1);
  assert.match(links[0], /^tirith:\/\/inbox\?kind=room-new&project=&step=&text=/);
  assert.equal(JSON.parse(decodeURIComponent(links[0].match(/text=([^&]*)/)[1])).title, 'My phone');
  assert.match(await page.textContent('[data-act="rm-new"]'), /Creating/);
  console.log('ok  New room sends a room-new request');

  /* no launch: the CLI line is shown */
  write(fixture.rooms, { launch: null });
  await page.reload();
  await page.waitForTimeout(3500);
  await page.fill('.rtitle', "Bob's room");
  await page.click('[data-act="rm-new"]');
  assert.match(await page.textContent('.icli'), /tirith room new --title 'Bob'\\''s room'/);
  console.log('ok  without a link handler the CLI line is shown');

  assert.deepEqual(requests.filter(u => !u.startsWith('file:')), [], 'nothing left the page');
  assert.deepEqual(errors, []);
} finally {
  await browser.close();
}
