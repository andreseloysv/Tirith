/* Run with: node web/js/landing.test.mjs  (needs the repo's own playwright-core or PLAYWRIGHT_FROM, else skips with a warning, and bundled headless Chromium; no servers).
   site/index.html loads from file:// with no console errors, every image exists, and the page does not
   scroll sideways at 360px and 1440px. Screenshots go to a temp dir (web/screenshots/ only with UPDATE_SCREENSHOTS=1). */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(here, '..', '..');

import { chromium } from './pw.mjs';  // the repo's own playwright-core (or PLAYWRIGHT_FROM); skips with a warning if missing

const shots = process.env.UPDATE_SCREENSHOTS === '1' ? path.join(root, 'web', 'screenshots') : fs.mkdtempSync(path.join(os.tmpdir(), 'landing-'));
fs.mkdirSync(shots, { recursive: true });

const site = path.join(root, 'site');
const url = pathToFileURL(path.join(site, 'index.html')).href;

const html = fs.readFileSync(path.join(site, 'index.html'), 'utf8');
for (const m of html.matchAll(/(?:src|href)="(img\/[^"]+|style\.css)"/g)) {
  assert.ok(fs.existsSync(path.join(site, m[1])), `missing file ${m[1]}`);
}
// the app screenshots show the current app: no removed view in the alt texts, and each stays small
for (const m of html.matchAll(/<img\b[^>]*>/g)) {
  const alt = (m[0].match(/alt="([^"]*)"/) || [])[1] || '';
  assert.ok(!/\bFlow\b/.test(alt), `alt text mentions the removed Flow view: ${alt.slice(0, 40)}`);
}
for (const m of html.matchAll(/src="(img\/[^"]+\.webp)"/g)) {
  assert.ok(fs.statSync(path.join(site, m[1])).size <= 300 * 1024, `${m[1]} is over 300 KB`);
}
const scripts = [...html.matchAll(/<script\b[^>]*>/gi)].map(m => m[0]);
assert.ok(scripts.every(t => /src="scroll\.js"/.test(t)), 'the only script is the local scroll.js');
assert.ok(fs.existsSync(path.join(site, 'scroll.js')), 'missing scroll.js');
assert.ok(!/(?:src|href)="https?:\/\/[^"]*\.(?:js|png|jpe?g|webp|gif)/i.test(html), 'no third-party assets');

const b = await chromium.launch({ headless: true });
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
    // the hero caption never sits over the screenshot, at the top and after scrolling a little
    for (const y of [0, 200]) {
      await p.evaluate((y) => window.scrollTo({ top: y, behavior: 'instant' }), y);
      await p.waitForTimeout(150);
      const clip = await p.evaluate(() => { const c = document.querySelector('.hero .shot .crop').getBoundingClientRect(), t = document.querySelector('.hero figcaption').getBoundingClientRect(); return c.bottom - t.top; });
      assert.ok(clip <= 1, `hero caption overlaps the screenshot by ${clip}px at ${width}px, scroll ${y}`);
    }
    // nothing inside the page scrolls sideways: the diagram and every code block fit whole (matters at 360px)
    const wide = await p.evaluate(() => [...document.querySelectorAll('.diagram, .diagram svg, pre, pre code')].filter(e => e.scrollWidth > e.clientWidth + 1 && getComputedStyle(e).display !== 'none').map(e => e.tagName + '.' + e.className + ' ' + (e.scrollWidth - e.clientWidth)));
    assert.deepEqual(wide, [], `diagram or code block overflows sideways at ${width}px`);
    const diag = await p.evaluate(() => [...document.querySelectorAll('.diagram svg')].filter(e => getComputedStyle(e).display !== 'none').map(e => { const r = e.getBoundingClientRect(), c = e.closest('.diagram').getBoundingClientRect(); return { n: 1, inside: r.left >= c.left - 1 && r.right <= c.right + 1, w: r.width }; }));
    assert.equal(diag.length, 1, `exactly one diagram variant shows at ${width}px`);
    assert.ok(diag[0].inside && diag[0].w >= 250, `the diagram fits its box at ${width}px`);
    assert.ok(await p.evaluate(() => document.querySelectorAll('pre').length) >= 2, 'code blocks exist');
    await p.screenshot({ path: path.join(shots, `landing-${width}.png`), fullPage: true });
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
    const kinds = await p.evaluate(() => ({ card: document.querySelectorAll('.card.rv').length, shot: document.querySelectorAll('.shot.rv').length, sec: document.querySelectorAll('main > section > h2.rv').length }));
    assert.ok(kinds.card >= 3 && kinds.shot >= 2 && kinds.sec >= 3, `sections, screenshots and cards all reveal: ${JSON.stringify(kinds)}`);
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
    const anim = await q.evaluate(() => [...document.querySelectorAll('.rv, .hero .shot img')].filter(e => { const c = getComputedStyle(e); return c.animationName !== 'none' || c.transform !== 'none'; }).length);
    assert.equal(anim, 0, `reduced motion: no transform or animation at ${width}px`);
    await q.evaluate(() => window.scrollTo({ top: 400, behavior: 'instant' })); await q.waitForTimeout(200);
    assert.equal(await q.evaluate(() => getComputedStyle(document.querySelector('.hero .shot img')).transform), 'none', 'reduced motion: no hero parallax');
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
