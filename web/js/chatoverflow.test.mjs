/* Run with: node web/js/chatoverflow.test.mjs  (needs playwright-core and the msedge channel; no servers).
   Long paths, code lines, a checklist, long button labels and live steps in the chat at 360, 380 and 900px:
   nothing extends past its bubble, the chat and the page have no horizontal scroll, `pre` blocks scroll inside the bubble. */
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
vm.runInNewContext(fs.readFileSync(path.join(here, '..', '..', 'fixtures', 'state.demo.js'), 'utf8'), holder);
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'tirith-chatoverflow-'));
const file = path.join(dir, 'state.js');
fs.writeFileSync(file, 'window.TIRITH_STATE = ' + JSON.stringify(holder.window.TIRITH_STATE) + ';');

const LONG = '/Users/andreseloyasanchezva/DEVELOP/Tirith-lanes/chatoverflow/web/screenshots/a-very-long-directory-name/another-quite-long-file-name-for-the-chat.png';
const TEXT = [
  'Before you merge, check these:', '',
  '- [ ] `cd /Users/andreseloyasanchezva/DEVELOP/Tirith-lanes/chatoverflow && git status --short --branch --untracked-files=all`',
  '- [x] `pgrep -x cargo >/dev/null && echo \'cargo is still running, wait for it to finish before you start the next build\' || echo idle`',
  '- The file is at ' + LONG + ' and nothing else changed, ' + 'x'.repeat(90),
  '', '```', 'tirith run --project tirith --stage test -- cargo test --manifest-path app/src-tauri/Cargo.toml --features custom-protocol -- --nocapture', '```',
  '', 'A plain paragraph with a URL https://example.com/' + 'segment/'.repeat(14) + 'end and ' + 'longwordwithoutspaces'.repeat(5) + '.',
].join('\n');
const ACTS = [
  { id: 'a1', label: 'Install the agent shims into every project directory that has none yet', command: 'tirith install --shims --all-projects --path ' + LONG },
  { id: 'a2', label: 'Run', command: 'cd ' + LONG + ' && cargo test' },
];

const browser = await chromium.launch({ channel: 'msedge' });
let n = 0;
try {
  for (const W of [360, 380, 900]) {
    const page = await browser.newPage({ viewport: { width: W < 600 ? W + 460 : 1400, height: 900 } });
    const errors = [];
    page.on('pageerror', e => errors.push(String(e)));
    await page.goto(pathToFileURL(path.join(here, '..', 'index.html')).href + '#state=' + pathToFileURL(file).href);
    await page.waitForTimeout(800);
    await page.evaluate(({ W, TEXT, ACTS, LONG }) => {
      const T = window.T;
      T.filter.view = 'pipelines'; T.ui.chatOpen = true; T.render();
      const d = document.querySelector('.mdrawer'); d.style.width = W + 'px'; d.style.maxWidth = 'none';
      const log = document.getElementById('mc-log');
      const mgr = '<div class="mc-m mgr"><div class="mc-b md">' + T.md(TEXT, {}) + '</div><div class="mc-acts">' + T.actions(ACTS, null, null) + '</div><div class="mc-meta">now</div></div>';
      const turn = '<div class="mc-m mgr turn wait"><div class="mc-b md">' + T.md(TEXT, {}) + '</div><div class="nacts">' +
        '<span class="nchip nreason">' + 'reason'.repeat(12) + '</span>' +
        '<button class="nbtn primary">Yes, merge the lane into main and push it to origin right now, then clean up the worktree</button>' +
        '<button class="nbtn">' + LONG + '</button></div><div class="mc-meta">Waits for you</div></div>';
      const me = '<div class="mc-m me"><div class="mc-b">' + LONG + ' ' + 'y'.repeat(100) + '</div></div>';
      const live = '<div class="livesteps"><ul class="ls-list">' +
        '<li class="ls-tool"><span class="ls-dot">●</span><span class="ls-t">Bash ' + LONG + ' ' + LONG + '</span></li>' +
        '<li class="ls-res"><span class="ls-dot">⎿</span><span class="ls-t">' + 'z'.repeat(120) + '</span></li>' +
        '<li class="ls-todo"><ul class="ls-checks"><li class="ls-ck pending"><span class="ls-dot">☐</span><span class="ls-t">' + LONG + '</span></li></ul></li>' +
        '<li><div class="ls-diff"><div class="ls-dh">' + LONG + '</div><div class="ls-dl add"><span class="ls-sg">+</span>' + 'q'.repeat(150) + '</div></div></li>' +
        '</ul><div class="ls-line">Manager is working · 23s <button class="ls-stop">Stop</button></div></div>';
      T.render = function () {};   /* a later render must not wipe the injected messages */
      log.innerHTML = me + mgr + turn + live; log.scrollTop = 0;
    }, { W, TEXT, ACTS, LONG });
    await page.waitForTimeout(200);
    assert.equal(await page.locator('#mc-log .mc-m').count(), 3, 'injected messages are in place');
    const bad = await page.evaluate(() => {
      const out = [], log = document.getElementById('mc-log'), d = document.querySelector('.mdrawer');
      const lr = log.getBoundingClientRect();
      if (log.scrollWidth > log.clientWidth + 1) out.push('log scrolls horizontally ' + log.scrollWidth + '>' + log.clientWidth);
      if (d.scrollWidth > d.clientWidth + 1) out.push('drawer scrolls horizontally');
      if (document.documentElement.scrollWidth > innerWidth) out.push('page scrolls horizontally');
      const holders = '.mc-b, .livesteps, .mc-acts, .nacts';
      log.querySelectorAll(holders).forEach(b => {
        const br = b.getBoundingClientRect();
        if (br.right > lr.right + 1 || br.left < lr.left - 1) out.push('holder outside log: ' + b.className);
        b.querySelectorAll('*').forEach(e => {
          if (e.closest('.mdpre, .mdtbl') && e !== e.closest('.mdpre, .mdtbl')) return;   /* scrolls inside its own block */
          const r = e.getBoundingClientRect();
          if (!r.width) return;
          if (r.right > br.right + 1) out.push((e.className || e.tagName) + ' past ' + b.className + ' by ' + Math.round(r.right - br.right));
        });
      });
      log.querySelectorAll('.mdpre').forEach(p => {
        const b = p.closest('.mc-b').getBoundingClientRect(), r = p.getBoundingClientRect();
        if (r.right > b.right + 1) out.push('pre past bubble');
        if (getComputedStyle(p).overflowX !== 'auto' && getComputedStyle(p).overflowX !== 'scroll') out.push('pre does not scroll');
      });
      return out;
    });
    await page.screenshot({ path: path.join(here, '..', 'screenshots', W === 380 ? 'chat-overflow.png' : path.join(dir, 'o' + W + '.png')) });
    assert.deepEqual([...new Set(bad)], [], 'overflow at ' + W + 'px');
    assert.deepEqual(errors, [], 'no page errors');
    n++; console.log('ok  no overflow at ' + W + 'px');
    await page.close();
  }
} finally {
  await browser.close();
  fs.rmSync(dir, { recursive: true, force: true });
}
console.log(n + ' passed');
