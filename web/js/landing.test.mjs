/* Run with: node web/js/landing.test.mjs  (needs the repo's own playwright-core, else skips with a warning, and the msedge channel; no servers).
   site/index.html loads from file:// with no console errors, every image exists, and the page does not
   scroll sideways at 360px and 1440px. Writes web/screenshots/landing-1440.png and landing-360.png. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(here, '..', '..');

// playwright-core must come from this repo (root or web/ node_modules), never from another project's node_modules.
let chromium = null;
for (const dir of [root, path.join(root, 'web')]) {
  try { ({ chromium } = createRequire(path.join(dir, 'x.js'))('playwright-core')); break; } catch {}
}
if (!chromium) {
  console.warn('SKIP landing.test.mjs: playwright-core is not installed in this repo (npm i --no-save playwright-core in the repo root to run it)');
  process.exit(0);
}

const site = path.join(root, 'site');
const url = pathToFileURL(path.join(site, 'index.html')).href;

const html = fs.readFileSync(path.join(site, 'index.html'), 'utf8');
for (const m of html.matchAll(/(?:src|href)="(img\/[^"]+|style\.css)"/g)) {
  assert.ok(fs.existsSync(path.join(site, m[1])), `missing file ${m[1]}`);
}
const scripts = [...html.matchAll(/<script\b[^>]*>/gi)].map(m => m[0]);
assert.ok(scripts.every(t => /src="scroll\.js"/.test(t)), 'the only script is the local scroll.js');
assert.ok(fs.existsSync(path.join(site, 'scroll.js')), 'missing scroll.js');
assert.ok(!/(?:src|href)="https?:\/\/[^"]*\.(?:js|png|jpe?g|webp|gif)/i.test(html), 'no third-party assets');

const b = await chromium.launch({ channel: 'msedge' });
try {
  for (const width of [360, 1440]) {
    const p = await b.newPage({ viewport: { width, height: 900 } });
    const errors = [];
    p.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
    p.on('pageerror', e => errors.push(String(e)));
    p.on('requestfailed', r => errors.push('request failed: ' + r.url()));
    await p.goto(url);
    // lazy images load only near the viewport: scroll through the page once
    await p.evaluate(async () => {
      for (let y = 0; y < document.body.scrollHeight; y += 600) { window.scrollTo({ top: y, behavior: 'instant' }); await new Promise(r => setTimeout(r, 120)); }
      window.scrollTo({ top: 0, behavior: 'instant' });
    });
    await p.waitForTimeout(800);
    const broken = await p.evaluate(() => [...document.images].filter(i => !i.complete || i.naturalWidth === 0).map(i => i.getAttribute('src')));
    assert.deepEqual(broken, [], `broken images at ${width}px`);
    const over = await p.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    assert.ok(over <= 0, `horizontal overflow of ${over}px at ${width}px`);
    assert.deepEqual(errors, [], `console errors at ${width}px`);
    await p.screenshot({ path: path.join(root, 'web', 'screenshots', `landing-${width}.png`), fullPage: true });
    await p.close();
  }

  // scroll reveals
  for (const width of [360, 1440]) {
    const p = await b.newPage({ viewport: { width, height: 900 } });
    const errors = [];
    p.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
    p.on('pageerror', e => errors.push(String(e)));
    await p.goto(url);
    await p.waitForTimeout(100);
    const hidden0 = await p.evaluate(() => [...document.querySelectorAll('.rv')].filter(e => getComputedStyle(e).opacity === '0').length);
    assert.ok(hidden0 > 5, `some elements start hidden below the fold at ${width}px (got ${hidden0})`);
    const total = await p.evaluate(() => document.querySelectorAll('.rv').length);
    assert.ok(total >= 20, `revealed elements marked (got ${total})`);
    assert.equal(await p.evaluate(() => document.documentElement.classList.contains('js')), true);
    const h = await p.evaluate(() => document.documentElement.scrollHeight);
    const frames = process.env.LANDING_FRAMES;
    let n = 0;
    for (let y = 0; y < h; y += 450) {
      await p.mouse.wheel(0, 450);
      await p.waitForTimeout(150);
      if (frames && width === 1440 && n++ % 3 === 0) await p.screenshot({ path: path.join(frames, `scroll-${String(n).padStart(2, '0')}.png`) });
    }
    await p.waitForTimeout(900);
    const bad = await p.evaluate(() => [...document.querySelectorAll('.rv')].filter(e => getComputedStyle(e).opacity !== '1').map(e => e.tagName + '.' + e.className));
    assert.deepEqual(bad, [], `all revealed elements visible after scrolling at ${width}px`);
    assert.ok(await p.evaluate(() => document.querySelector('header.top').classList.contains('scrolled')), 'nav blur class after scrolling');
    const over = await p.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    assert.ok(over <= 0, `horizontal overflow of ${over}px after reveals at ${width}px`);
    assert.deepEqual(errors, [], `console errors while scrolling at ${width}px`);
    await p.close();

    // reduced motion: everything visible at once, nothing transitions
    const ctx = await b.newContext({ viewport: { width, height: 900 }, reducedMotion: 'reduce' });
    const q = await ctx.newPage();
    await q.goto(url);
    const st = await q.evaluate(() => [...document.querySelectorAll('.rv')].filter(e => { const c = getComputedStyle(e); return c.opacity !== '1' || c.transform !== 'none' || parseFloat(c.transitionDuration) > 0; }).length);
    assert.equal(st, 0, `reduced motion shows everything immediately at ${width}px`);
    await ctx.close();
  }
  // no JS: everything visible
  const ctx = await b.newContext({ viewport: { width: 1440, height: 900 }, javaScriptEnabled: false });
  const q = await ctx.newPage();
  await q.goto(url);
  const hid = await q.evaluate(() => [...document.querySelectorAll('h2, .card, .shot')].filter(e => getComputedStyle(e).opacity !== '1').length);
  assert.equal(hid, 0, 'without JS everything is visible');
  await ctx.close();
} finally {
  await b.close();
}
console.log('landing ok');
