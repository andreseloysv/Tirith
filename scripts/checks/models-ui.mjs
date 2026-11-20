// Real browser, encrypted phone service and daemon, with temporary state and no model calls.
// AM_ROOT points at an AgentsMessenger checkout; PLAYWRIGHT_MODULE at an installed Playwright module.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtemp, mkdir, writeFile, open } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { createServer } from 'node:net';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const am = resolve(process.env.AM_ROOT || join(root, '../agentsmessenger'));
const { createPlatform } = await import(pathToFileURL(join(am, 'src/server/server.js')));
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE || 'playwright');
const home = await mkdtemp(join(tmpdir(), 'tirith-model-ui-'));
const platform = createPlatform();
const phoneBase = await platform.listen();
const port = await new Promise(resolve => { const s = createServer(); s.listen(0, '127.0.0.1', () => { const p = s.address().port; s.close(() => resolve(p)); }); });
const base = `http://127.0.0.1:${port}`;
const api = async (path, data, method) => {
  const r = await fetch(base + path, { method: method || (data ? 'POST' : 'GET'), headers: { 'content-type': 'application/json' }, body: data ? JSON.stringify(data) : undefined });
  assert.ok(r.ok, `${path}: ${r.status}`); return r.json();
};
const wait = async fn => {
  const until = Date.now() + 20000;
  while (Date.now() < until) { try { if (await fn()) return; } catch {} await new Promise(r => setTimeout(r, 100)); }
  throw new Error('Timed out waiting for ' + fn);
};
await mkdir(join(home, 'catalog'));
await writeFile(join(home, 'catalog/models_cache.json'), JSON.stringify({ models: [
  { slug: 'gpt-6-astra', display_name: 'Astra', visibility: 'list', supported_reasoning_levels: ['low', 'high', 'ultra'].map(effort => ({ effort })) },
  { slug: 'gpt-6-luna', display_name: 'Luna', visibility: 'list', supported_reasoning_levels: ['low', 'medium', 'max'].map(effort => ({ effort })) },
] }));
await writeFile(join(home, 'config.toml'), `
[agents.codex]
command = "/usr/bin/false"
[agents.anthropic]
command = "/usr/bin/false"
[agents.muse]
command = "/usr/bin/false"
[models]
choices = ["muse-sample"]
[models.manager]
agent = "codex"
[models.workers]
agent = "codex"
[models.ceo]
agent = "codex"
[network]
sandbox = false
proxy_port = 0
[autopilot]
enabled = false
[messenger]
url = ${JSON.stringify(phoneBase)}
`);
const log = await open(join(home, 'daemon.log'), 'w');
const daemon = spawn(join(root, 'target/debug/tirith'), ['serve'], { cwd: root, env: { ...process.env, TIRITH_HOME: home, TIRITH_PORT: String(port), CODEX_HOME: join(home, 'catalog') }, stdio: ['ignore', log.fd, log.fd] });
let browser;
try {
  await wait(() => api('/api/models'));
  const room = await api('/api/rooms', { title: 'Model controls check', project: '' });
  browser = await chromium.launch({ headless: true });
  const phone = await browser.newPage({ viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true });
  const errors = [];
  phone.on('pageerror', e => errors.push(e.message));
  phone.on('console', m => { if (m.type() === 'error' && /Content Security Policy/.test(m.text())) errors.push(m.text()); });
  await phone.goto(room.invite);
  await phone.getByText(/Connected to tirith-manager/).waitFor({ timeout: 20000 });
  const panel = phone.locator('#models-panel');
  await panel.locator('summary').click();
  await panel.getByRole('button', { name: 'OpenAI · Codex', exact: true }).waitFor();
  const choose = async (p, name) => p.getByRole('button', { name, exact: false }).click();
  const save = async () => {
    await panel.getByRole('button', { name: 'Save', exact: true }).click();
    await panel.getByText(/Saved on the computer/).waitFor();
  };
  await choose(panel, 'Astra');
  const slider = panel.getByRole('slider', { name: 'Reasoning effort' });
  await slider.focus(); await slider.press('End');
  assert.equal(await slider.getAttribute('aria-valuetext'), 'Ultra');
  assert.equal((await api('/api/models')).roles.manager.model, null, 'editing waits for Save');
  await save();
  assert.equal((await api('/api/models')).roles.manager.effort, 'ultra');
  assert.equal(await panel.getByRole('button', { name: 'muse-sample' }).count(), 0);
  await phone.screenshot({ path: join(home, 'phone.png'), fullPage: true });
  assert.ok(await phone.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'no horizontal overflow');
  await panel.getByRole('button', { name: 'Workers', exact: true }).click();
  await choose(panel, 'Luna');
  assert.equal(await panel.getByRole('button', { name: 'Ultra', exact: true }).count(), 0);
  await slider.focus(); await slider.press('End'); await save();
  assert.equal((await api('/api/models')).roles.workers.effort, 'max');
  await panel.getByRole('button', { name: 'CEO', exact: true }).click();
  await choose(panel, 'Astra'); await panel.getByRole('button', { name: 'High', exact: true }).click(); await save();
  assert.deepEqual(Object.values((await api('/api/models')).roles).map(r => r.agent), Array(3).fill('codex'));
  await panel.getByRole('button', { name: 'Use configured defaults' }).click();
  await panel.getByText(/Saved on the computer/).waitFor();
  assert.equal((await api('/api/models')).roles.ceo.source, 'config');
  console.log('PASS: encrypted visual phone controls, all roles, valid efforts, explicit save and reset');

  const desktop = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  desktop.on('pageerror', e => errors.push(e.message));
  await desktop.goto(base);
  const appPanel = desktop.locator('.mc-models');
  await appPanel.locator('summary').click();
  await choose(appPanel, 'Luna');
  const appSlider = appPanel.getByRole('slider');
  await appSlider.focus(); await appSlider.press('End');
  // Arrow input must preserve the native slider element and keyboard focus across app updates.
  await appSlider.press('ArrowLeft'); await appSlider.press('ArrowRight');
  assert.equal(await appSlider.getAttribute('aria-valuetext'), 'Max');
  await appPanel.getByRole('button', { name: 'Save', exact: true }).click();
  await wait(async () => (await api('/api/models')).roles.manager.model === 'gpt-6-luna');
  assert.equal((await api('/api/models')).roles.manager.effort, 'max');
  await desktop.screenshot({ path: join(home, 'desktop.png'), fullPage: true });
  assert.equal(await appPanel.locator('.scard, .sfallback').count(), 0, 'chat must not embed the Settings form');
  assert.ok(await appPanel.evaluate(el => el.previousElementSibling.classList.contains('mc-log') && el.nextElementSibling.classList.contains('mc-comp')), 'controls sit next to the chat composer');
  await panel.getByRole('button', { name: 'Manager', exact: true }).click();
  await panel.getByRole('button', { name: 'Refresh', exact: true }).click();
  await wait(async () => await slider.getAttribute('aria-valuetext') === 'Max');
  await phone.reload();
  await panel.locator('summary').click();
  await wait(async () => await slider.getAttribute('aria-valuetext') === 'Max');
  await desktop.getByText('Settings', { exact: true }).click();
  const settings = desktop.locator('.msettings');
  await settings.locator('.mfields').first().waitFor();
  assert.equal(await settings.locator('model-controls, input[type=range]').count(), 0, 'Settings keeps its original selects');
  const manager = settings.locator('.scard').filter({ has: desktop.locator('.phead > b', { hasText: /^Manager$/ }) });
  await manager.getByRole('combobox', { name: 'Model', exact: true }).selectOption('gpt-6-astra');
  await manager.getByRole('combobox', { name: 'Effort', exact: true }).selectOption('ultra');
  assert.deepEqual(await manager.getByRole('combobox', { name: 'Model', exact: true }).locator('option').evaluateAll(os => os.map(o => o.value)), ['', 'gpt-6-astra', 'gpt-6-luna']);
  await manager.getByRole('button', { name: 'Save', exact: true }).click();
  await wait(async () => (await api('/api/models')).roles.manager.effort === 'ultra');
  await desktop.screenshot({ path: join(home, 'settings.png'), fullPage: true });
  console.log('PASS: original Settings dropdowns, Codex filtering and saving; visual controls only in chat');
  assert.deepEqual(errors, []);
  console.log('PASS: desktop slider, phone refresh and reload, no script or CSP errors');
  console.log('Screenshots and daemon log:', home);
} catch (e) {
  console.error('Check artifacts:', home);
  if (browser) for (const [i, page] of browser.contexts().flatMap(c => c.pages()).entries()) await page.screenshot({ path: join(home, `failure-${i}.png`), fullPage: true }).catch(() => {});
  throw e;
} finally {
  await browser?.close(); daemon.kill(); await platform.close(); await log.close();
}
