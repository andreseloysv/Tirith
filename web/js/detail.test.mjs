/* Run with: node web/js/detail.test.mjs  (needs playwright-core and the msedge channel; no servers).
   The state carries a trimmed agent (no log entries, short report) and points at agents/<id>.js.
   Opening its drawer must load that file with a <script> tag (file://) and show the full log and report. */
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

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-detail-'));
fs.mkdirSync(path.join(dir, 'agents'));
const detailFile = path.join(dir, 'agents', 'old-1.js');
const MARK = 'Only the detail file knows this sentence.';
const REPORT_TAIL = 'Last section of the long report.';
fs.writeFileSync(detailFile, 'window.TIRITH_AGENT = ' + JSON.stringify({
  id: 'old-1',
  log_entries: [{ kind: 'say', ts: new Date().toISOString(), text: MARK }],
  log_tail: [], events: [], files: [], tests: [],
  report: 'Short first paragraph.\n\n' + REPORT_TAIL
}) + ';\n');

const s = JSON.parse(JSON.stringify(base));
s.generated_at = new Date().toISOString();
const proto = s.agents.filter(a => a.status === 'done')[0] || s.agents[0];
const trimmed = Object.assign({}, proto, {
  id: 'old-1', project: proto.project, status: 'done', feature: 'Trimmed old agent', summary: 'Short first paragraph.',
  ended_at: new Date(Date.now() - 3600e3).toISOString(),
  log_entries: [], log_tail: [], events: [], files: [], tests: [], report: 'Short first paragraph.', report_clipped: true,
  detail_more: true, detail: { path: detailFile, hash: 'abc123' }
});
s.agents = [trimmed];
const stateFile = path.join(dir, 'state.js');
fs.writeFileSync(stateFile, 'window.TIRITH_STATE = ' + JSON.stringify(s) + ';');

const browser = await chromium.launch({ channel: 'msedge' });
try {
  const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
  const errors = [];
  page.on('pageerror', e => errors.push(String(e)));
  await page.goto(page_url + '#state=' + pathToFileURL(stateFile).href + '&view=agents');
  await page.waitForTimeout(800);
  const card = page.locator('[data-act="agent"][data-key="old-1"]').first();
  await card.waitFor({ timeout: 3000 });
  await card.click();
  const drawer = page.locator('aside.drawer');
  await drawer.waitFor({ timeout: 3000 });
  await page.waitForFunction(t => document.querySelector('aside.drawer').textContent.includes(t), MARK, { timeout: 5000 });
  const text = await drawer.textContent();
  assert.ok(text.includes(MARK), 'the full log entry from agents/<id>.js is shown');
  assert.ok(text.includes(REPORT_TAIL), 'the full report is shown');
  assert.deepEqual(errors, [], 'no page errors');
  console.log('ok  drawer loads agents/<id>.js on open');

  // a missing detail file must not break the drawer
  fs.rmSync(detailFile);
  trimmed.detail = { path: detailFile, hash: 'def456' };
  trimmed.id = 'old-2'; trimmed.feature = 'Second';
  s.agents.push(trimmed);
  fs.writeFileSync(stateFile, 'window.TIRITH_STATE = ' + JSON.stringify(s) + ';');
  await page.waitForTimeout(2500);
  await page.locator('[data-act="agent"][data-key="old-2"]').first().click();
  await page.waitForTimeout(500);
  assert.ok((await drawer.textContent()).includes('Second'), 'drawer opens without its detail file');
  assert.deepEqual(errors.filter(e => !/Failed to load/.test(e)), [], 'no page errors without the file');
  console.log('ok  missing detail file is harmless');
  console.log('2 passed');
} finally {
  await browser.close();
  fs.rmSync(dir, { recursive: true, force: true });
}
