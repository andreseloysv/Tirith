/* Run with: node web/js/statusdone.test.mjs  (needs playwright-core and bundled headless Chromium; no servers).
   state.status.level "done": the Overview shows a closed "All done" card (drawn check, full bar, confetti once),
   and a redraw on the next poll does not start the confetti or the check again. */
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
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-done-'));
const write = status => {
  const s = JSON.parse(JSON.stringify(base));
  s.generated_at = new Date().toISOString().replace(/\.\d+Z$/, 'Z');
  s.status = status;
  fs.writeFileSync(path.join(dir, 'state.js'), 'window.TIRITH_STATE = ' + JSON.stringify(s) + ';');
};
const done = { level: 'done', why: 'All done: nothing is left for the agents', merged_today: 131, working: 0, limit: 5,
  ci: 'green', steps: [], recent: [], plan: { done: 678, total: 682 },
  owner: [{ id: 'w-1', text: 'Step 14, inspiration images: needs a Gemini API key' }], owner_count: 1 };
const card = page => page.evaluate(() => {
  const c = document.querySelector('.stcard');
  return c && { cls: c.className, text: c.textContent, confetti: c.querySelectorAll('.stconf i').length,
    drawing: c.querySelectorAll('.drawing').length };
});

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1400, height: 900 } });
  const errors = [];
  page.on('pageerror', e => errors.push(String(e)));
  write(done);
  await page.goto(page_url + '#state=' + pathToFileURL(path.join(dir, 'state.js')).href);
  await page.waitForTimeout(900);
  let c = await card(page);
  assert.match(c.cls, /\bdone\b/);
  assert.match(c.text, /All done/);
  assert.match(c.text, /678 of 682 plan items done/);
  assert.match(c.text, /Left for you/);
  assert.doesNotMatch(c.text, /Nothing working/);
  assert.ok(c.confetti > 0 && c.drawing === 2, 'confetti and the check drawing at first');
  await page.screenshot({ path: shotPath('status-done.png'), clip: { x: 0, y: 0, width: 1400, height: 330 } });
  console.log('ok  done card with confetti');

  await page.waitForTimeout(4500);  /* past the confetti */
  write({ ...done, merged_today: 132 });  /* a later poll redraws the page */
  await page.waitForTimeout(3500);
  c = await card(page);
  assert.match(c.text, /132 merged today/);
  assert.equal(c.confetti, 0, 'confetti falls once');
  assert.equal(c.drawing, 0, 'the check is drawn once');
  console.log('ok  a redraw keeps it calm');

  write({ ...done, level: 'warn', why: 'Nothing working while 3 plan items are open' });
  await page.waitForTimeout(3500);
  c = await card(page);
  assert.doesNotMatch(c.cls, /\bdone\b/);
  assert.match(c.text, /Nothing working/);
  console.log('ok  back to the four lines when work shows up');
  assert.deepEqual(errors, []);
} finally {
  await browser.close();
}
