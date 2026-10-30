/* Run with: node web/js/planagents.test.mjs  (needs playwright-core and the msedge channel; no servers).
   A Plan step with 1, 3 and 10 agents at widths 1440 and 900: the header never overlaps, title / bar / count stay
   visible, one control opens a popover that lists every agent. Writes web/screenshots/plan-many-agents.png. */
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
vm.runInNewContext(fs.readFileSync(path.join(here, '..', '..', 'fixtures', 'state.planquiet.js'), 'utf8'), holder);
const base = holder.window.TIRITH_STATE;
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-pagents-'));

function stateWith(n) {
  const s = JSON.parse(JSON.stringify(base));
  s.generated_at = new Date().toISOString();
  const proj = s.projects.filter(p => p.roadmap && p.roadmap.steps.some(st => st.id === '3'))[0];
  const st = proj.roadmap.steps.filter(x => x.id === '3')[0];
  const proto = s.agents.filter(a => a.id === 't-world')[0];
  const open = st.items.filter(i => !i.done);
  s.agents = s.agents.filter(a => a.id !== 't-world');
  st.agents = [];
  for (let k = 0; k < n; k++) {
    const id = 'many-' + k;
    s.agents.push(Object.assign({}, proto, { id, name: 'Agent' + k, project: proj.id, status: 'working', feature: 'Task ' + k,
      item: open[k % open.length].text, started_at: new Date(Date.now() - 60000 * (k + 1)).toISOString() }));
    st.agents.push(id);
  }
  return s;
}
const box = async l => l.first().boundingBox();
const overlap = (a, b) => a.x < b.x + b.width - 0.5 && b.x < a.x + a.width - 0.5 && a.y < b.y + b.height - 0.5 && b.y < a.y + a.height - 0.5;

const browser = await chromium.launch({ channel: 'msedge' });
let n = 0;
try {
  for (const width of [1440, 900]) {
    const page = await browser.newPage({ viewport: { width, height: 1000 } });
    const errors = [];
    page.on('pageerror', e => errors.push(String(e.stack || e)));
    for (const count of [1, 3, 10]) {
      const file = path.join(dir, 'state-' + count + '.js');
      fs.writeFileSync(file, 'window.TIRITH_STATE = ' + JSON.stringify(stateWith(count)) + ';');
      await page.goto(pathToFileURL(path.join(here, '..', 'index.html')).href + '#state=' + pathToFileURL(file).href + '&project=tirith');
      await page.reload();
      const row = page.locator('.pstep:has(.pn:text-is("3.")) > .prow');
      await row.waitFor({ timeout: 5000 }).catch(async e => { console.log(errors, await page.locator('.pstep').count(), (await page.locator('.plan').innerText().catch(() => 'no plan')).slice(0, 700)); throw e; });
      await page.locator('.pstep:has(.pn:text-is("3.")) .iacts').first().evaluate(e => { e.style.opacity = 1; });
      const parts = { title: '.ptit', bar: '.pbar', count: '.pcnt', avas: '.pavas', who: '.pwho', acts: '.iacts' };
      const bx = {};
      for (const k of Object.keys(parts)) { bx[k] = await box(row.locator(parts[k])); assert.ok(bx[k], k + ' is visible'); }
      if (process.env.DBG) console.log('W', width, count, JSON.stringify(Object.fromEntries(Object.keys(bx).map(k => [k, [Math.round(bx[k].x), Math.round(bx[k].y), Math.round(bx[k].width)]]))));
      assert.ok(bx.title.width >= 100, 'title keeps room: ' + bx.title.width);
      assert.ok(bx.bar.width >= 40 && bx.count.width >= 10, 'bar and count keep room');
      const names = Object.keys(bx); if (process.env.DBG) console.log(JSON.stringify(Object.fromEntries(names.map(k => [k, [Math.round(bx[k].x), Math.round(bx[k].width)]]))));
      for (let i = 0; i < names.length; i++) for (let j = i + 1; j < names.length; j++) {
        assert.ok(!overlap(bx[names[i]], bx[names[j]]), JSON.stringify([bx[names[i]], bx[names[j]]]) + ` ${names[i]} overlaps ${names[j]} (${count} agents, ${width}px)`);
      }
      const rb = await row.boundingBox(); if (process.env.DBG) console.log(width, count, JSON.stringify(bx), rb.width);
      for (const k of names) assert.ok(bx[k].x + bx[k].width <= rb.x + rb.width + 1, k + ' stays inside the row');
      assert.equal(await row.locator('.pava').count(), Math.min(count, 3), 'at most 3 avatars');
      assert.equal(await row.locator('.pmore').count(), count > 3 ? 1 : 0, '+N chip only past 3');
      assert.equal(await row.locator('.stopb').count(), 0, 'no Stop pills in the header');
      assert.match(await row.locator('.pwho').textContent(), new RegExp('^' + count + ' working'));
      assert.ok((await page.locator('.pstep:has(.pn:text-is("3.")) .pia').count()) >= 1, 'open items show the agent avatar');
      await row.locator('.pwho').click();
      const pop = row.locator('.pwhopop');
      await pop.waitFor({ timeout: 2000 });
      assert.equal(await pop.locator('.pwrow').count(), count, 'popover lists every agent');
      assert.equal(await pop.locator('.pwrow .stopb').count(), count, 'each agent has its Stop in the popover');
      assert.ok((await pop.textContent()).includes('Agent' + (count - 1)));
      if (count === 10 && width === 1440) await page.screenshot({ path: path.join(here, '..', 'screenshots', 'plan-many-agents.png'), clip: { x: 0, y: Math.max(0, rb.y - 80), width, height: 560 } });
      await row.locator('.pwho').click();
      assert.equal(await row.locator('.pwhopop').count(), 0, 'the popover closes');
      n++;
      console.log(`ok  ${count} agents at ${width}px`);
    }
    assert.deepEqual(errors, [], 'no page errors');
    await page.close();
  }
  console.log(n + ' passed');
} finally {
  await browser.close();
  fs.rmSync(dir, { recursive: true, force: true });
}
