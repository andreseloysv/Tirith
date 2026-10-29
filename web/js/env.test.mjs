/* Run with: node web/js/env.test.mjs [screenshot.png]  (needs playwright-core and the msedge channel; no servers).
   Settings > Environment: the selected project's variables are a table with the values in full; add, edit, delete,
   keys are validated, Save sends the whole set through `project_env_set` (app), `tirith://env-set` (browser with the
   handler) or shows the CLI line, and the table settles once the state shows the saved set. */
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

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-env-'));
let n = 0;
function write(file, env, launch) {
  const s = JSON.parse(JSON.stringify(base));
  n++;
  s.generated_at = new Date().toISOString();
  s.projects.forEach(p => { p.env = p.id === 'tirith' ? env : {}; });
  if (launch) s.launch = launch;
  fs.writeFileSync(file, 'window.TIRITH_STATE = ' + JSON.stringify(s) + ';');
}
const file = path.join(dir, 'state.js');
let env = { API_URL: 'https://example.invalid/v1', DEBUG: '1' };
write(file, env);
const timer = setInterval(() => write(file, env), 1000);

const fake = `
  window.__calls = [];
  window.__TAURI__ = { core: { invoke: (cmd, args) => { window.__calls.push({ cmd, args }); return Promise.resolve('ok'); } } };`;

const browser = await chromium.launch({ channel: 'msedge' });
let passed = 0;
const ok = name => { console.log('ok  ' + name); passed++; };
try {
  const ctx = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  await ctx.addInitScript(fake);
  const page = await ctx.newPage();
  const errors = [];
  page.on('pageerror', e => errors.push(String(e)));
  await page.goto(page_url + '#state=' + pathToFileURL(file).href + '&project=tirith&view=settings');
  const card = page.locator('.envcard');
  await card.waitFor({ timeout: 4000 });

  /* 1. the values are visible, in full */
  const vals = await card.locator('input.envv').evaluateAll(els => els.map(e => e.value));
  const keys = await card.locator('input.envk').evaluateAll(els => els.map(e => e.value));
  assert.deepEqual(keys, ['API_URL', 'DEBUG']);
  assert.deepEqual(vals, ['https://example.invalid/v1', '1']);
  assert.equal(await card.locator('input[type=password]').count(), 0);
  assert.equal(await card.locator('[data-act="env-save"]').isDisabled(), true, 'Save waits for a change');
  ok('the table shows every name and value in full');
  if (process.argv[2]) await page.screenshot({ path: process.argv[2] });

  /* 2. edit, add, delete; invalid names are refused before anything is sent */
  await card.locator('input.envv').nth(1).fill('0');
  await card.locator('[data-act="env-add"]').click();
  await card.locator('input.envk').nth(2).fill('1BAD');
  await card.locator('input.envv').nth(2).fill('x');
  await card.locator('[data-act="env-save"]').click();
  assert.match(await card.locator('.ierr').innerText(), /"1BAD" is not a valid name/);
  assert.equal((await page.evaluate(() => window.__calls)).length, 0, 'nothing sent');
  await card.locator('input.envk').nth(2).fill('API_URL');
  await card.locator('[data-act="env-save"]').click();
  assert.match(await card.locator('.ierr').innerText(), /API_URL appears twice/);
  await card.locator('input.envk').nth(2).fill('NEW_ONE');
  await card.locator('input.envv').nth(2).fill('a b=c');
  await card.locator('[data-act="env-rm"]').first().click();
  assert.deepEqual(await card.locator('input.envk').evaluateAll(els => els.map(e => e.value)), ['DEBUG', 'NEW_ONE']);
  ok('edit, add and delete rows; bad and duplicate names are refused');

  /* 3. a state refresh does not eat the unsaved table */
  await page.waitForTimeout(2500);
  assert.deepEqual(await card.locator('input.envv').evaluateAll(els => els.map(e => e.value)), ['0', 'a b=c']);
  ok('unsaved rows survive state refreshes');

  /* 4. Save sends the whole set through the app command */
  await card.locator('[data-act="env-save"]').click();
  const calls = await page.evaluate(() => window.__calls);
  assert.deepEqual(calls, [{ cmd: 'project_env_set', args: { project: 'tirith', env: { DEBUG: '0', NEW_ONE: 'a b=c' } } }]);
  await card.locator('.ichip').waitFor({ timeout: 2000 });
  ok('Save calls project_env_set with the whole set');

  /* 5. once the state shows it, the table follows the state */
  env = { DEBUG: '0', NEW_ONE: 'a b=c' };
  write(file, env);
  await page.waitForFunction(() => !document.querySelector('.envcard .ichip'), null, { timeout: 8000 });
  assert.equal(await card.locator('[data-act="env-save"]').isDisabled(), true);
  assert.deepEqual(await card.locator('input.envk').evaluateAll(els => els.map(e => e.value)), ['DEBUG', 'NEW_ONE']);
  ok('the saved set comes back from the state');

  /* 6. another project has its own (empty) table */
  await page.goto(page_url + '#state=' + pathToFileURL(file).href + '&project=chromasol&view=settings');
  await page.reload();
  await page.locator('.envcard').waitFor({ timeout: 4000 });
  assert.equal(await page.locator('.envcard input.envk').count(), 0);
  assert.match(await page.locator('.envcard').innerText(), /No variables/);
  ok('each project shows its own variables');
  await ctx.close();

  /* 7. a browser with the handler opens tirith://env-set; the token travels, the CLI line is the last resort */
  const link = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  await link.addInitScript('window.__opened = []; document.addEventListener("click", e => { const a = e.target.closest && e.target.closest("a"); if (a && /^tirith:/.test(a.href)) { window.__opened.push(a.href); e.preventDefault(); } }, true);');
  const lfile = path.join(dir, 'state-link.js');
  write(lfile, { A: '1' }, { token: 'tok', available: true });
  const lp = await link.newPage();
  await lp.goto(page_url + '#state=' + pathToFileURL(lfile).href + '&project=tirith&view=settings');
  await lp.locator('.envcard').waitFor({ timeout: 4000 });
  await lp.locator('input.envv').first().fill('2');
  await lp.locator('[data-act="env-save"]').click();
  const opened = await lp.evaluate(() => window.__opened);
  assert.equal(opened.length, 1);
  const u = new URL(opened[0].replace('tirith://', 'http://x/'));
  assert.equal(u.hostname === 'x' && u.pathname, '/env-set');
  assert.equal(u.searchParams.get('project'), 'tirith');
  assert.deepEqual(JSON.parse(u.searchParams.get('env')), { A: '2' });
  assert.equal(u.searchParams.get('token'), 'tok');
  ok('the browser path opens tirith://env-set with the set and the token');
  await link.close();

  const cli = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const cfile = path.join(dir, 'state-cli.js');
  write(cfile, { A: "it's" }, { token: 'tok', available: false });
  const cp = await cli.newPage();
  await cp.goto(page_url + '#state=' + pathToFileURL(cfile).href + '&project=tirith&view=settings');
  await cp.locator('.envcard').waitFor({ timeout: 4000 });
  await cp.locator('input.envv').first().fill("it's 2");
  await cp.locator('[data-act="env-save"]').click();
  const line = await cp.locator('.envcard .icli code').innerText();
  assert.match(line, /^printf %s '\{"A":"it'\\''s 2"\}' \| tirith project env set --project 'tirith' --replace-stdin$/);
  ok('without the app or the handler the CLI line is shown, quoted');
  await cli.close();

  assert.deepEqual(errors, [], 'no page errors');
} finally {
  clearInterval(timer);
  await browser.close();
  fs.rmSync(dir, { recursive: true, force: true });
}
console.log(passed + ' passed');
