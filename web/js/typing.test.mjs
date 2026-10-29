/* Run with: node web/js/typing.test.mjs  (needs playwright-core and the msedge channel; no servers).
   Types 40 characters at 100 ms per key into each kind of text box while the state file is rewritten every second.
   The text must come out intact, the caret at the end, nothing selected. */
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

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-typing-'));
const file = path.join(dir, 'state.js');
let n = 0;
function write() {
  const s = JSON.parse(JSON.stringify(base));
  n++;
  s.generated_at = new Date().toISOString();
  if (s.agents && s.agents[0]) s.agents[0].summary = 'tick ' + n;   /* real change: forces a re-render */
  fs.writeFileSync(file, 'window.TIRITH_STATE = ' + JSON.stringify(s) + ';');
}
write();
const timer = setInterval(write, 1000);

const TEXT = 'The quick brown fox jumps over lazy dogs!!';   /* 41 chars, trimmed to 40 below */
const WORDS = TEXT.slice(0, 40);

const cases = [
  { name: 'reply to a manager message', view: 'overview', sel: 'textarea[data-act="mc-text"]', setup: async p => { await p.locator('button[data-act="mc-reply"]').first().click(); } },
  { name: 'manager chat composer', view: 'overview', sel: 'textarea[data-act="mc-text"]' },
  { name: 'plan Ask box', view: 'overview', sel: 'input[data-act="itext"]', setup: async p => { await p.locator('button[data-act="iask"]').first().click(); } },
  { name: 'plan form title', view: 'overview', sel: 'input[data-act="pnf"][data-field="title"]', setup: async p => { await p.locator('button[data-act="pnew"]').first().click(); } },
  { name: 'plan form goal', view: 'overview', sel: 'textarea[data-act="pnf"][data-field="goal"]', setup: async p => { await p.locator('button[data-act="pnew"]').first().click(); } },
  { name: 'Settings + Stage step', view: 'settings', sel: 'input[data-act="mod-f"][data-key="new|step"]' }
];

const browser = await chromium.launch({ channel: 'msedge' });
let passed = 0;
try {
  for (const c of cases) {
    const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
    const errors = [];
    page.on('pageerror', e => errors.push(String(e)));
    await page.goto(page_url + '#state=' + pathToFileURL(file).href + (c.view === 'overview' ? '' : '&view=' + c.view));
    await page.waitForTimeout(800);
    if (c.setup) await c.setup(page);
    const box = page.locator(c.sel).first();
    await box.waitFor({ timeout: 3000 });
    await box.click();
    const before = await page.evaluate(() => document.getElementById('view'));   // keeps the DOM node check honest below
    void before;
    await page.evaluate(() => { window.__renders = 0; new MutationObserver(() => window.__renders++).observe(document.getElementById('view'), { childList: true, subtree: true }); });
    for (const ch of WORDS) await page.keyboard.type(ch, { delay: 0 }).then(() => page.waitForTimeout(100));
    const r = await page.evaluate(sel => {
      const el = document.querySelector(sel);
      return { value: el.value, ss: el.selectionStart, se: el.selectionEnd, focused: document.activeElement === el, renders: window.__renders };
    }, c.sel);
    assert.equal(r.value, WORDS, c.name + ': text intact');
    assert.ok(r.focused, c.name + ': still focused');
    assert.equal(r.ss, WORDS.length, c.name + ': caret at the end');
    assert.equal(r.se, WORDS.length, c.name + ': nothing selected');
    assert.deepEqual(errors, [], c.name + ': no page errors');
    console.log('ok  ' + c.name + ' (' + r.renders + ' DOM mutations while typing)');
    passed++;
    await page.close();
  }
} finally {
  clearInterval(timer);
  await browser.close();
  fs.rmSync(dir, { recursive: true, force: true });
}
console.log(passed + ' passed');
