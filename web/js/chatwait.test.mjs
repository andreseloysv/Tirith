/* Run with: node web/js/chatwait.test.mjs  (needs playwright-core and the msedge channel; no servers).
   The chat says "Manager is typing…" only while something is really writing; otherwise a plain waiting line.
   Pass a .png path as the argument to also save a screenshot. */
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

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-chatwait-'));
const iso = ms => new Date(Date.now() - ms).toISOString().replace(/\.\d+Z$/, 'Z');
const MIN = 60 * 1000;

function req(over) {
  return Object.assign({ id: 'in-1', at: iso(12 * MIN), kind: 'chat', project: null, step: '', text: 'what is writing?',
    status: 'open', manager_pending: true, answers: [], action_runs: [], lanes: [] }, over);
}
function stateFor(inbox, live, agents) {
  const s = JSON.parse(JSON.stringify(base));
  s.generated_at = iso(0);
  s.inbox = inbox;
  s.manager = Object.assign({}, s.manager || {}, { live: live || null });
  if (agents) s.agents = (s.agents || []).concat(agents);
  return s;
}

const cases = [
  { name: 'no live session: plain waiting line, no dots', state: () => stateFor([req()]),
    check: r => { assert.equal(r.typing, 0); assert.match(r.wait, /^Waiting for the manager · 12 min$/); assert.equal(r.late, false); } },
  { name: 'a request reopened 3h ago is still not typing', state: () => stateFor([req({ at: iso(197 * MIN) })]),
    check: r => { assert.equal(r.typing, 0); assert.match(r.wait, /· 3 h 17 min$/); } },
  { name: 'after 30 minutes: not answered yet, amber', state: () => stateFor([req({ at: iso(40 * MIN) })]),
    check: r => { assert.equal(r.typing, 0); assert.match(r.wait, /^The manager has not answered yet · 40 min$/); assert.equal(r.late, true); } },
  { name: 'live session heartbeat after the message: typing', state: () => stateFor([req()], { heartbeat: iso(20 * 1000) }),
    check: r => { assert.equal(r.typing, 1); assert.equal(r.wait, null); } },
  { name: 'heartbeat before the message: waiting', state: () => stateFor([req({ at: iso(2 * MIN) })], { heartbeat: iso(5 * MIN) }),
    check: r => { assert.equal(r.typing, 0); assert.match(r.wait, /^Waiting for the manager · 2 min$/); } },
  { name: 'stale heartbeat (older than 5 min): waiting', state: () => stateFor([req({ at: iso(20 * MIN) })], { heartbeat: iso(10 * MIN) }),
    check: r => { assert.equal(r.typing, 0); assert.match(r.wait, /Waiting for the manager/); } },
  { name: 'a headless lane working on it: typing', state: () => stateFor([req()], null, [{ id: 'hm-1', name: 'x', status: 'working', request_id: 'in-1', project: 'tirith' }]),
    check: r => { assert.equal(r.typing, 1); } },
  { name: 'reopened by a failed action: shows the result line', state: () => stateFor([req({
      status: 'answered', at: iso(60 * MIN),
      answers: [{ at: iso(50 * MIN), by: 'manager', text: 'Run this', auto: false, actions: [{ id: 'a-shims', label: 'Install the agent shims', command: 'x' }] }],
      action_runs: [{ action_id: 'a-shims', status: 'failed', exit_code: 1, at: iso(8 * MIN) }] })]),
    check: r => { assert.equal(r.typing, 0); assert.equal(r.reason, '✗ Install the agent shims failed'); assert.match(r.wait, /· 8 min$/); } },
  { name: 'answered request: nothing pending, nothing shown', state: () => stateFor([req({ status: 'answered', manager_pending: false,
      answers: [{ at: iso(1 * MIN), by: 'manager', text: 'done', auto: false }] })]),
    check: r => { assert.equal(r.typing, 0); assert.equal(r.wait, null); } }
];

const shot = process.argv[2];   /* optional: save a screenshot of the reopened-by-action case */
const browser = await chromium.launch({ channel: 'msedge' });
let passed = 0;
try {
  for (const c of cases) {
    const file = path.join(dir, 'state.js');
    fs.writeFileSync(file, 'window.TIRITH_STATE = ' + JSON.stringify(c.state()) + ';');
    const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
    const errors = [];
    page.on('pageerror', e => errors.push(String(e)));
    await page.goto(page_url + '#state=' + pathToFileURL(file).href);
    await page.waitForTimeout(800);
    const r = await page.evaluate(() => {
      const w = document.querySelector('.mc-wait'), re = document.querySelector('.mc-reason');
      return { typing: document.querySelectorAll('.mc-typing').length, wait: w ? w.textContent : null,
        late: !!(w && w.classList.contains('late')), reason: re ? re.textContent : null };
    });
    c.check(r);
    assert.deepEqual(errors, [], c.name + ': no page errors');
    if (shot && c === cases[cases.length - 2]) await page.screenshot({ path: shot });
    console.log('ok  ' + c.name);
    passed++;
    await page.close();
  }
  /* timestamps without a zone are UTC, not local time */
  const page = await browser.newPage();
  await page.goto(page_url);
  const ms = await page.evaluate(() => [window.T.ms('2026-10-29T09:28:57'), window.T.ms('2026-10-29T09:28:57Z'), window.T.ms('2026-10-29 09:28:57'), window.T.ms('2026-10-29T11:28:57+02:00')]);
  assert.equal(ms[0], Date.UTC(2026, 9, 29, 9, 28, 57));
  assert.deepEqual([ms[1], ms[2], ms[3]], [ms[0], ms[0], ms[0]]);
  console.log('ok  timestamps without a zone are read as UTC');
  passed++;
} finally {
  await browser.close();
  fs.rmSync(dir, { recursive: true, force: true });
}
console.log(passed + ' passed');
