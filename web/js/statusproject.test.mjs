/* Run with: node web/js/statusproject.test.mjs  (needs playwright-core and bundled headless Chromium; no servers).
   With a project selected, the Overview's Status card shows state.status.by_project[project], not the global numbers. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import vm from 'node:vm';
import { fileURLToPath, pathToFileURL } from 'node:url';

import { chromium } from './pw.mjs';
import { shotPath } from './shots.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const page_url = pathToFileURL(path.join(here, '..', 'index.html')).href;
const holder = { window: {} };
vm.runInNewContext(fs.readFileSync(path.join(here, '..', '..', 'fixtures', 'state.demo.js'), 'utf8'), holder);
const base = holder.window.TIRITH_STATE;
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-statusproj-'));
const write = status => {
  const s = JSON.parse(JSON.stringify(base));
  s.generated_at = new Date().toISOString().replace(/\.\d+Z$/, 'Z');
  s.status = status;
  s.sentinel = { enabled: true, model: 'm', every_min: 5, last: { at: s.generated_at } };
  fs.writeFileSync(path.join(dir, 'state.js'), 'window.TIRITH_STATE = ' + JSON.stringify(s) + ';');
};
const pid = base.projects[1].id;
const glob = { level: 'warn', why: 'Nothing working while 10 plan items are open', merged_today: 134, working: 0, limit: 5, ci: 'green',
  steps: [], plan: { done: 1, total: 2 }, recent: [{ text: 'GLOBAL landed', at: new Date().toISOString(), project: 'other' }],
  owner: [{ id: 'g', text: 'GLOBAL wait' }], owner_count: 6,
  by_project: { [pid]: { level: 'warn', why: 'Nothing working while 7 plan items are open', merged_today: 2, working: 0, limit: 5,
    ci: 'red', steps: [], plan: { done: 1, total: 8 }, recent: [{ text: 'PROJECT landed', at: new Date().toISOString(), project: pid }],
    owner: [{ id: 'p', text: 'PROJECT wait' }], owner_count: 1 } } };
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
  const errors = [];
  page.on('pageerror', e => errors.push(String(e)));
  write(glob);
  await page.goto(page_url + '#state=' + pathToFileURL(path.join(dir, 'state.js')).href + '&project=' + encodeURIComponent(pid));
  await page.waitForTimeout(900);
  const text = await page.evaluate(() => (document.querySelector('.stcard') || {}).textContent || '');
  assert.match(text, /7 plan items are open/);
  assert.match(text, /2 merged today/);
  assert.match(text, /CI red/);
  assert.match(text, /PROJECT landed/);
  assert.match(text, /PROJECT wait/);
  assert.doesNotMatch(text, /GLOBAL|134 merged|10 plan items/);
  assert.match(text, /all projects/);  /* the sentinel line says it is not about this project alone */
  console.log('ok  the Status card follows the selected project');
  assert.deepEqual(errors, []);
} finally {
  await browser.close();
}
