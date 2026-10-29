/* Run with: node web/js/padding.test.mjs [screenshot-dir]  (needs playwright-core and the msedge channel; no servers).
   Every view keeps its content inside the header's left margin: no heading, intro line or card is closer to the left
   edge than the page title, and no view overflows horizontally, at 1440, 1100 and 760 px. */
import assert from 'node:assert/strict';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';

const require = createRequire(process.env.PLAYWRIGHT_FROM || '/Users/andreseloyasanchezva/DEVELOP/propuestavenezuela/node_modules/x.js');
const { chromium } = require('playwright-core');

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(here, '..', '..');
const page_url = pathToFileURL(path.join(root, 'web', 'index.html')).href;
const state_url = pathToFileURL(path.join(root, 'fixtures', 'state.demo.js')).href;
const shots = process.argv[2];

const views = ['overview', 'pipelines', 'agents', 'settings'];
const widths = [1440, 1100, 760];
const SELECTOR = 'h1, h2, h3, p, .card, .col, .ptable, table, .quiet, .mlead, #view > *, #view > * > *';

const browser = await chromium.launch({ channel: 'msedge' });
let passed = 0;
try {
  for (const w of widths) {
    const page = await browser.newPage({ viewport: { width: w, height: 900 } });
    const errors = [];
    page.on('pageerror', e => errors.push(String(e)));
    for (const v of views) {
      await page.goto(page_url + '#state=' + state_url + '&project=tirith&view=' + v);
      await page.reload();
      await page.waitForSelector('#view > *', { timeout: 4000 });
      await page.waitForTimeout(600);
      const r = await page.evaluate(sel => {
        const edge = document.querySelector('.crumb').getBoundingClientRect().left;
        const view = document.getElementById('view');
        const bad = [];
        view.querySelectorAll(sel).forEach(e => {
          const b = e.getBoundingClientRect();
          if (!b.width || !b.height || e.closest('[hidden]') || getComputedStyle(e).position === 'absolute') return;
          const own = [...e.childNodes].some(n => n.nodeType === 3 && n.textContent.trim());
          const box = e.matches('.card, .col, .ptable, table');
          if ((own || box) && b.left < edge - 0.5) bad.push(e.tagName + '.' + e.className + ' @' + Math.round(b.left));
        });
        return { edge, bad, over: Math.max(document.documentElement.scrollWidth - innerWidth, view.scrollWidth - view.clientWidth) };
      }, SELECTOR);
      assert.deepEqual(r.bad, [], `${v} @${w}: content left of the header edge ${r.edge}`);
      assert.ok(r.over <= 0, `${v} @${w}: overflows horizontally by ${r.over}px`);
      if (shots && w === 1440) await page.screenshot({ path: path.join(shots, 'padding-' + v + '.png') });
      console.log(`ok  ${v} @${w} (edge ${Math.round(r.edge)})`);
      passed++;
    }
    assert.deepEqual(errors, []);
    await page.close();
  }
  console.log(passed + ' passed');
} finally { await browser.close(); }
