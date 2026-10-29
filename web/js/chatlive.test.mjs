/* Run with: node web/js/chatlive.test.mjs  (needs playwright-core and the msedge channel; no servers).
   The chat shows the manager working like the CLI: a live block under the owner's message (newest steps, a
   spinner line), which collapses to "12 steps · 48s · $0.21 · show" when the turn ends and opens on a click. */
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
const base = holder.window.TIRITH_STATE;
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-chatlive-'));
const stateFile = path.join(dir, 'state.js');
const stepsFile = path.join(dir, 'manager-live1.steps.js');

const iso = ms => new Date(ms).toISOString().replace(/\.\d+Z$/, 'Z');
const now = Date.now();
const STEPS = [
  { kind: 'text', name: '', summary: 'Checking the plan before I answer.', at: iso(now - 20000) },
  { kind: 'tool', name: 'Read', summary: 'Read docs/TODO.md', at: iso(now - 19000) },
  { kind: 'result', name: 'Read', summary: '# TODO\n\n## 1. First\n… +40 lines', at: iso(now - 19000), error: false },
  { kind: 'tool', name: 'Bash', summary: 'Bash: tirith inbox answer in-live1 "…" --token=***', at: iso(now - 12000) },
  { kind: 'result', name: 'Bash', summary: 'guard: not allowed here', at: iso(now - 11000), error: true },
  { kind: 'tool', name: 'Grep', summary: 'Grep "^## 7" in docs', at: iso(now - 5000) },
];
const MORE = [
  ...STEPS,
  ...Array.from({ length: 6 }, (_, i) => ({ kind: 'tool', name: 'Read', summary: 'Read file-' + i + '.md', at: iso(now - 4000 + i) })),
];

function writeSteps(steps) {
  fs.writeFileSync(stepsFile, 'window.TIRITH_AGENT = ' + JSON.stringify({ id: 'manager-live1.steps', steps }) + ';\n');
}
function writeState(phase) {
  const s = JSON.parse(JSON.stringify(base));
  const ended = phase === 'ended';
  s.generated_at = iso(Date.now());
  s.inbox = [{ id: 'in-live1', at: iso(now - 25000), kind: 'chat', project: 'tirith', step: '', text: 'Why is step 7 not finished?', status: ended ? 'done' : 'open',
    manager_pending: !ended, answers: ended ? [{ at: iso(now - 1000), by: 'manager', auto: false, text: 'Only an owner-only item is left.', lane: null }] : [], lanes: [] }];
  s.says = []; s.waiting_on_you = []; s.answered_waits = [];
  s.manager = { ...(s.manager || {}), presence: { mode: 'on_call', working: !ended, lane: ended ? null : 'manager-live1' } };
  const row = JSON.parse(JSON.stringify(s.agents[0]));
  Object.assign(row, {
    id: 'manager-live1', project: 'tirith', status: ended ? 'done' : 'working', started_at: iso(now - 23000), ended_at: ended ? iso(now - 1000) : null,
    turn_requests: ['in-live1'], request_id: null, elapsed_s: ended ? 22 : 23,
    live: { steps: ended ? MORE.length : STEPS.length, done: ended, error: false, cost_usd: ended ? 0.2134 : null, duration_s: ended ? 48 : null, elapsed_s: ended ? 22 : 23,
      detail: { path: stepsFile, hash: ended ? 'h2' : 'h1' } },
  });
  s.agents = [row];
  fs.writeFileSync(stateFile, 'window.TIRITH_STATE = ' + JSON.stringify(s) + ';');
}

writeSteps(STEPS);
writeState('working');
const browser = await chromium.launch({ channel: 'msedge' });
try {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  const errors = [];
  page.on('pageerror', e => errors.push(String(e)));
  await page.goto(pathToFileURL(path.join(here, '..', 'index.html')).href + '#state=' + pathToFileURL(stateFile).href);
  const live = page.locator('#mc-log .livesteps.run');
  await live.waitFor({ timeout: 5000 });
  await page.waitForFunction(() => document.querySelectorAll('#mc-log .livesteps .ls-list li').length >= 6, null, { timeout: 5000 });

  const text = await live.innerText();
  assert.match(text, /Read docs\/TODO\.md/, 'tool line like the CLI');
  assert.match(text, /Grep "\^## 7" in docs/);
  assert.match(text, /Manager is working\s+·\s+\d+s/, 'spinner line with the time');
  assert.ok(!/[{}]"/.test(text), 'no raw JSON');
  assert.ok(!/abc123/.test(text), 'no secrets');
  assert.equal(await page.locator('#mc-log .livesteps .ls-res.bad').count(), 1, 'an error result is marked');
  const order = await page.evaluate(() => {
    const log = document.getElementById('mc-log'), kids = [...log.children];
    return kids.map(k => k.classList.contains('livesteps') ? 'live' : k.classList.contains('me') ? 'me' : k.classList.contains('mgr') ? 'mgr' : k.className);
  });
  assert.deepEqual(order.slice(0, 2), ['me', 'live'], 'the live block sits under the owner message');
  assert.equal(await page.locator('#mc-log .mc-typing').count(), 0, 'no typing dots next to the live block');
  const t0 = await page.locator('#mc-log .livesteps .ls-line .tick').innerText();
  await page.waitForTimeout(1300);
  const t1 = await page.locator('#mc-log .livesteps .ls-line .tick').innerText();
  assert.notEqual(t0, t1, 'the timer ticks (' + t0 + ' -> ' + t1 + ')');
  await page.screenshot({ path: path.join(here, '..', 'screenshots', 'chat-live-working.png') });

  /* the turn ends: the file and the state change, the next poll collapses it */
  writeSteps(MORE);
  writeState('ended');
  const done = page.locator('#mc-log .livesteps.done');
  await done.waitFor({ timeout: 8000 });
  const line = (await done.locator('.ls-sum').innerText()).replace(/\s+/g, ' ');
  assert.equal(line, '12 steps · 48s · $0.21 · show', 'collapsed to one line');
  assert.equal(await done.locator('.ls-list').count(), 0, 'steps hidden');
  assert.match(await page.locator('#mc-log').innerText(), /Only an owner-only item is left\./, 'the answer shows normally');
  assert.equal(await page.locator('#mc-log .livesteps.run').count(), 0, 'no spinner left');

  await done.locator('.ls-sum').click();
  await page.waitForFunction(() => document.querySelectorAll('#mc-log .livesteps.done .ls-list li').length === 12, null, { timeout: 5000 });
  assert.match(await page.locator('#mc-log .livesteps.done .ls-sum').innerText(), /hide$/);
  await page.screenshot({ path: path.join(here, '..', 'screenshots', 'chat-live.png') });
  await page.locator('#mc-log .livesteps.done .ls-sum').click();
  await page.waitForFunction(() => !document.querySelector('#mc-log .livesteps.done .ls-list'), null, { timeout: 3000 });
  assert.deepEqual(errors, [], 'no page errors');
  console.log('ok  chat shows the live block, collapses it at the end, opens it on a click');
} finally {
  await browser.close();
  fs.rmSync(dir, { recursive: true, force: true });
}
console.log('1 passed');
