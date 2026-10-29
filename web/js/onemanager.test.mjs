/* Run with: node web/js/onemanager.test.mjs  (needs playwright-core and the msedge channel; no servers).
   The one persistent manager in the chat, CLI style: the turn under the owner's message is a live block (task list,
   coloured diff, text as it is typed, the lane it started nested), a second message typed meanwhile is "Queued",
   Esc / Stop interrupt, Up recalls the last message. Writes web/screenshots/one-manager.png. */
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
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-onemanager-'));
const stateFile = path.join(dir, 'state.js');
const turnFile = path.join(dir, 'manager-turn-aa11.steps.js');
const laneFile = path.join(dir, 'lane-1.steps.js');

const iso = ms => new Date(ms).toISOString().replace(/\.\d+Z$/, 'Z');
const now = Date.now();
const long = Array.from({ length: 30 }, (_, i) => ['+', 'line ' + i + ' of the new function body']);
const STEPS = [
  { kind: 'text', name: '', summary: 'Reading the plan before I touch anything.', at: iso(now - 20000) },
  { kind: 'tool', name: 'Read', summary: 'Read docs/TODO.md', at: iso(now - 19000) },
  { kind: 'result', name: 'Read', summary: '# TODO\n## 1. First\n… +40 lines', at: iso(now - 19000), error: false },
  { kind: 'todo', name: 'TodoWrite', summary: 'Task list', at: iso(now - 18000), todos: [
    { content: 'Read the plan', status: 'completed' }, { content: 'Start the lane for step 7', status: 'in_progress' }, { content: 'Report back', status: 'pending' }] },
  { kind: 'tool', name: 'Edit', summary: 'Edit notes.md', at: iso(now - 12000), diff: { path: 'notes.md', rows: [['-', 'step 7: open'], ['+', 'step 7: lane started'], ['+', 'owner asked at 12:00']], more: 0 } },
  { kind: 'tool', name: 'Write', summary: 'Write brief.md', at: iso(now - 9000), diff: { path: 'brief.md', rows: long, more: 0 } },
  { kind: 'tool', name: 'Bash', summary: 'Bash: tirith lane run --project tirith --class warden --feature "Step 7"', at: iso(now - 5000) },
  { kind: 'text', name: '', summary: 'Started the lane, now waiting for', at: iso(now - 1000), partial: true },
];
const LANE = [
  { kind: 'tool', name: 'Read', summary: 'Read web/js/v-plan.js', at: iso(now - 3000) },
  { kind: 'tool', name: 'Grep', summary: 'Grep "step 7" in docs', at: iso(now - 2000) },
];

function writeFile(file, id, steps) {
  fs.writeFileSync(file, 'window.TIRITH_AGENT = ' + JSON.stringify({ id, steps }) + ';\n');
}
function writeState(busy) {
  const s = JSON.parse(JSON.stringify(base));
  s.generated_at = iso(Date.now());
  const row = (id, at, text) => ({ id, at: iso(at), kind: 'chat', project: 'tirith', step: '', text, status: 'open', manager_pending: true, answers: [], lanes: [] });
  s.inbox = [row('in-one1', now - 25000, 'Start step 7 and tell me when the lane is done.'),
    ...(busy ? [row('in-one2', now - 2000, 'Also check the build afterwards.')] : [])];
  s.launch = { available: false };   /* no tirith:// link: a send shows the CLI line instead of opening the link handler */
  s.says = []; s.waiting_on_you = []; s.answered_waits = [];
  const lane = JSON.parse(JSON.stringify(s.agents[0]));
  Object.assign(lane, { id: 'lane-1', name: 'Wren', feature: 'Step 7', status: 'working', started_at: iso(now - 5000), ended_at: null, parent: 'manager-s1', turn_requests: [],
    live: { steps: LANE.length, done: false, error: false, cost_usd: null, duration_s: null, elapsed_s: 5, detail: { path: laneFile, hash: 'l1' } } });
  s.agents = [lane];
  s.manager = { ...(s.manager || {}),
    presence: { mode: 'on_call', working: true, lane: 'manager-s1', session: { lane: 'manager-s1', busy: true, turns: 3, context_tokens: 41000, asks: 0 } },
    turns: [{ id: 'manager-turn-aa11', name: 'Manager', status: 'working', started_at: iso(now - 23000), ended_at: null, elapsed_s: 23, parent: 'manager',
      turn_requests: ['in-one1'], lanes: ['lane-1'], live: { steps: STEPS.length, done: false, error: false, cost_usd: null, duration_s: null, elapsed_s: 23, detail: { path: turnFile, hash: 't1' } } }] };
  fs.writeFileSync(stateFile, 'window.TIRITH_STATE = ' + JSON.stringify(s) + ';');
}

writeFile(turnFile, 'manager-turn-aa11.steps', STEPS);
writeFile(laneFile, 'lane-1.steps', LANE);
writeState(true);
const browser = await chromium.launch({ channel: 'msedge' });
try {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1100 } });
  const errors = [];
  page.on('pageerror', e => errors.push(String(e)));
  await page.goto(pathToFileURL(path.join(here, '..', 'index.html')).href + '#state=' + pathToFileURL(stateFile).href);
  const live = page.locator('#mc-log .livesteps.run').first();
  await live.waitFor({ timeout: 5000 });
  await page.waitForFunction(() => document.querySelector('#mc-log .livesteps.run .ls-diff'), null, { timeout: 5000 });

  /* the CLI look: tool lines, the task list as one checklist, diffs coloured, the text arriving */
  assert.equal(await live.locator('.ls-checks .ls-ck').count(), 3, 'one task list with three tasks');
  assert.equal(await live.locator('.ls-ck.completed').count(), 1);
  assert.equal(await live.locator('.ls-ck.in_progress').count(), 1);
  assert.equal(await live.locator('.ls-todo').count(), 1, 'the task list shows once');
  const small = live.locator('.ls-diff').nth(0);
  assert.equal(await small.locator('.ls-dl.del').count(), 1, 'removed line');
  assert.equal(await small.locator('.ls-dl.add').count(), 2, 'added lines');
  const red = await small.locator('.ls-dl.del').evaluate(e => getComputedStyle(e).backgroundColor);
  const green = await small.locator('.ls-dl.add').first().evaluate(e => getComputedStyle(e).backgroundColor);
  assert.notEqual(red, green, 'red and green differ');
  assert.match(await small.locator('.ls-dh').innerText(), /notes\.md\s+\+2\s+−1/, 'file path header with counts');
  const big = live.locator('.ls-diff').nth(1);
  assert.equal(await big.locator('.ls-dl').count(), 20, 'a long diff is collapsed after 20 lines');
  await big.locator('.ls-more').click();
  await page.waitForFunction(() => document.querySelectorAll('#mc-log .livesteps.run .ls-diff')[1].querySelectorAll('.ls-dl').length === 30, null, { timeout: 3000 });
  assert.equal(await page.locator('#mc-log .livesteps.run .ls-diff').nth(1).locator('.ls-more').innerText(), 'collapse');
  assert.equal(await live.locator('.ls-say.typing .ls-cur').count(), 1, 'the text as it is typed has a cursor');
  assert.match(await live.innerText(), /Started the lane, now waiting for/);
  const font = await live.evaluate(e => getComputedStyle(e).fontFamily);
  assert.match(font, /mono|Menlo|Consolas/i, 'monospace like the CLI');

  /* the lane the manager started is a nested block with its own live steps */
  const nest = live.locator('.ls-nest .ls-lane');
  assert.equal(await nest.count(), 1);
  assert.match(await nest.innerText(), /Step 7/);
  assert.match(await nest.innerText(), /Grep "step 7" in docs/);
  assert.match(await nest.innerText(), /Wren is working/);

  /* the second message, typed while it works, is queued into the same session */
  const log = await page.locator('#mc-log').innerText();
  assert.match(log, /Also check the build afterwards\./);
  assert.equal(await page.locator('#mc-log .mc-m.me').nth(1).locator('.mc-wait', { hasText: 'Queued' }).count(), 1, 'queued label');
  assert.equal(await page.locator('#mc-log .mc-m.me').nth(0).locator('.mc-wait', { hasText: 'Queued' }).count(), 0, 'the first one is being answered, not queued');

  /* Stop, and Esc, interrupt (in a page opened from a file the CLI line is shown) */
  assert.equal(await page.locator('.mchat .mc-stop').count(), 1, 'Stop in the header');
  assert.equal(await live.locator('.ls-stop').count(), 1, 'Stop in the live block');
  await live.locator('.ls-stop').click();
  assert.match(await page.locator('.mchat .icli').innerText(), /tirith manager interrupt/);
  await page.evaluate(() => { document.querySelector('.mchat .icli').remove(); });

  /* Up on an empty box brings back the last message sent from here; Esc interrupts */
  const box = page.locator('textarea[data-act="mc-text"]').first();
  await box.fill('check the plan');
  await box.press('Enter');
  await page.waitForFunction(() => !document.querySelector('textarea[data-act="mc-text"]').value, null, { timeout: 3000 });
  /* a key can land while the page re-renders: press again a few times */
  const pressUntil = async (key, check) => {
    for (let i = 0; i < 6; i++) {
      await box.press(key);
      try { await page.waitForFunction(check, null, { timeout: 700 }); return; } catch (e) { /* press again */ }
    }
    assert.fail('key ' + key + ' had no effect');
  };
  await pressUntil('ArrowUp', () => document.querySelector('textarea[data-act="mc-text"]').value === 'check the plan');
  await box.fill('');
  await page.evaluate(() => { const c = document.querySelector('.mchat .icli'); if (c) c.remove(); });
  await pressUntil('Escape', () => /tirith manager interrupt/.test(document.querySelector('.mchat').innerText));
  const open = page.locator('#mc-log .livesteps.run .ls-diff').nth(1);
  if (await open.locator('.ls-dl').count() === 30) await open.locator('.ls-more').click();   /* collapsed again for the picture */
  await page.waitForFunction(() => document.querySelectorAll('#mc-log .livesteps.run .ls-diff')[1].querySelectorAll('.ls-dl').length === 20, null, { timeout: 3000 });
  await page.evaluate(() => { document.getElementById('mc-log').scrollTop = 0; });
  await page.screenshot({ path: path.join(here, '..', 'screenshots', 'one-manager.png') });

  assert.deepEqual(errors, [], 'no page errors');
  console.log('ok  the one manager in the chat: live turn, task list, diffs, nested lane, queued, Stop, Esc, Up');
} finally {
  await browser.close();
  fs.rmSync(dir, { recursive: true, force: true });
}
console.log('1 passed');
