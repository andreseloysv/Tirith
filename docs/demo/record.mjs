// Records the Tirith demo (Edge, 1920x1080, playwright video) on the demo story in story.mjs.
//
//   node narrate.mjs [outDir]   # first: narration audio + <out>/timings.json
//   node record.mjs  [outDir]   # then:  <out>/raw/*.webm and <out>/scenes.json
//   ./assemble.sh    [outDir]   # finally: docs/demo/out/tirith-demo.mp4 and tirith-demo.gif
//
// Needs Microsoft Edge and playwright-core (PLAYWRIGHT_CORE can point at a node_modules folder that has it). No
// collector, no server, no port, no network: the page is a local file that follows <out>/state/state.js, which this
// script rewrites as the story goes on (a request in the chat, the manager's steps, two agents, a rule, a merge...).
// Nothing is written to ~/.tirith.
import { createRequire } from 'node:module';
import { readFileSync, writeFileSync, mkdirSync, readdirSync, rmSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { Story, MANAGER, WREN, IVREN, REQUEST } from './story.mjs';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '../..');
const out = resolve(process.argv[2] || join(here, 'out/work'));
const require = createRequire(join(resolve(process.env.PLAYWRIGHT_CORE || '/Users/andreseloyasanchezva/DEVELOP/propuestavenezuela/node_modules'), 'x.js'));
const { chromium } = require('playwright-core');

const stateDir = join(out, 'state');
const BASE = pathToFileURL(join(root, 'web/index.html')).href + '#state=' + pathToFileURL(join(stateDir, 'state.js')).href + '&project=chromasol';
const CARDS = pathToFileURL(join(here, 'cards.html')).href;
const T = JSON.parse(readFileSync(join(out, 'timings.json'), 'utf8'));
mkdirSync(join(out, 'raw'), { recursive: true });
for (const f of readdirSync(join(out, 'raw'))) rmSync(join(out, 'raw', f));

// scene length = narration + a breath, but never shorter than the on-screen action needs
const LEN = {
  problem: Math.max(T.problem + 0.9, 12), ask: Math.max(T.ask + 1, 24), plan: Math.max(T.plan + 1.5, 12),
  rule: Math.max(T.rule + 1.5, 36), merge: Math.max(T.merge + 1.5, 18), health: Math.max(T.health + 1.5, 12),
  decision: Math.max(T.decision + 1.5, 13), brain: Math.max(T.brain + 1.5, 15), close: T.close + 2.4,
};
const CAPTION = {
  ask: 'One request, written in the chat', plan: 'The plan grows; every agent starts from a plan item',
  ruleIntro: 'Wren is working on dark mode', ruleBlock: 'The hook blocks the raw test run, before it runs', ruleInstead: 'It tells Wren the exact command to use instead', rulePipe: 'The run Wren made shows up in Pipelines', merge: 'Claim check, then auto-merge. The plan ticks itself',
  health: 'The health dot: Tirith heals itself', decision: 'Only a real decision reaches the owner',
};

const story = new Story(stateDir);
story.write();
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const beat = (o) => { story.set(o).write(); };
const heartbeat = setInterval(() => story.write(), 2000);   // generated_at stays fresh, like a live collector

const browser = await chromium.launch({ channel: 'msedge', headless: true });
const ctx = await browser.newContext({
  viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1,
  recordVideo: { dir: join(out, 'raw'), size: { width: 1920, height: 1080 } },
});
await ctx.addInitScript(() => {
  // a tirith:// link is the page's way to send a reply; headless has no handler, so swallow it
  const click = HTMLAnchorElement.prototype.click;
  HTMLAnchorElement.prototype.click = function () { if (/^tirith:/.test(this.href)) return; return click.call(this); };
  const mk = () => {
    if (!document.documentElement || document.getElementById('__cur')) return;
    // a visible cursor and click ripple (headless video has none), and the caption bar
    const c = document.createElement('div'); c.id = '__cur';
    c.innerHTML = '<svg width="34" height="34" viewBox="0 0 24 24"><path d="M5 3l14 8-6 2-3 6z" fill="#111" stroke="#fff" stroke-width="1.6" stroke-linejoin="round"/></svg>';
    c.style.cssText = 'position:fixed;left:-50px;top:-50px;z-index:2147483647;pointer-events:none;transition:none;filter:drop-shadow(0 2px 3px rgba(0,0,0,.3))';
    document.documentElement.appendChild(c);
    const st = document.createElement('style');
    st.textContent = '@keyframes __rip{from{transform:scale(.3);opacity:.6}to{transform:scale(1.6);opacity:0}}' +
      '#__cap{position:fixed;left:50%;bottom:30px;transform:translateX(-50%);z-index:2147483645;pointer-events:none;background:rgba(17,24,39,.88);color:#fff;font:600 28px/1.3 -apple-system,"SF Pro Text",Inter,system-ui,sans-serif;padding:12px 30px;border-radius:99px;opacity:0;transition:opacity .5s;white-space:nowrap;box-shadow:0 8px 30px rgba(0,0,0,.25)}#__cap.on{opacity:1}';
    document.documentElement.appendChild(st);
    const cap = document.createElement('div'); cap.id = '__cap'; document.documentElement.appendChild(cap);
    window.__caption = (t) => { if (t) { cap.textContent = t; cap.classList.add('on'); } else cap.classList.remove('on'); };
    addEventListener('mousemove', (e) => { c.style.left = e.clientX - 5 + 'px'; c.style.top = e.clientY - 3 + 'px'; }, true);
    addEventListener('mousedown', (e) => {
      const r = document.createElement('div');
      r.style.cssText = 'position:fixed;z-index:2147483646;pointer-events:none;width:44px;height:44px;border-radius:50%;background:rgba(47,111,237,.35);left:' + (e.clientX - 22) + 'px;top:' + (e.clientY - 22) + 'px;animation:__rip .5s ease-out forwards';
      document.documentElement.appendChild(r); setTimeout(() => r.remove(), 600);
    }, true);
  };
  addEventListener('DOMContentLoaded', mk); mk();
});
const pageAt = Date.now();
const page = await ctx.newPage();
const errors = []; page.on('pageerror', (e) => errors.push(String(e)));

let mx = 960, my = 540;
const ease = (t) => t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
async function glide(x, y, ms = 900) {
  const n = Math.max(6, Math.round(ms / 16)), x0 = mx, y0 = my;
  for (let i = 1; i <= n; i++) { const k = ease(i / n); await page.mouse.move(x0 + (x - x0) * k, y0 + (y - y0) * k); await sleep(ms / n); }
  mx = x; my = y;
}
async function center(loc, fx = 0.5, fy = 0.5) {
  let b = null;
  for (let i = 0; i < 16 && !b; i++) { b = await loc.first().boundingBox().catch(() => null); if (!b) await sleep(250); }   // the page re-renders every 2 s
  if (!b) throw new Error('no box for ' + loc);
  return [b.x + b.width * fx, b.y + b.height * fy];
}
async function hover(loc, ms = 900, fx = 0.5, fy = 0.5) { const [x, y] = await center(loc, fx, fy); await glide(x, y, ms); }
async function click(loc, ms = 900, fx = 0.5, fy = 0.5) {
  await hover(loc, ms, fx, fy); await sleep(220);
  await page.mouse.down(); await sleep(70); await page.mouse.up(); await sleep(350);
}
const closeBtn = () => page.locator('[data-act="close"]');
async function closeDrawer(ms = 800) { if (await closeBtn().count()) await click(closeBtn(), ms); }
async function waitFor(loc, timeout = 25000) { await loc.first().waitFor({ state: 'visible', timeout }); }
const nav = (v) => page.locator('[data-nav="view"][data-v="' + v + '"]');
// click a view in the sidebar; the page re-renders every 2 s and can swallow a click, so check and click again
async function goView(v, ms = 900) {
  for (let i = 0; i < 3; i++) {
    await click(nav(v), i ? 300 : ms);
    if (await nav(v).first().evaluate((e) => e.classList.contains('on')).catch(() => false)) return;
  }
}

const marks = {};
let sceneStart = 0;
const wall0 = Date.now();
async function scene(name, caption) {
  marks[name] = Date.now(); sceneStart = Date.now();
  console.log(((Date.now() - wall0) / 1000).toFixed(1) + 's scene ' + name);
  await page.evaluate((c) => window.__caption && window.__caption(c), caption || '').catch(() => {});
}
async function caption(t) { await page.evaluate((c) => window.__caption && window.__caption(c), t).catch(() => {}); }
async function until(sec) { const ms = sceneStart + sec * 1000 - Date.now(); if (ms > 0) await sleep(ms); }
// reveal the steps of a list one by one: [count, at second] pairs, each applied when its time comes
async function timeline(plan) { for (const [at, o] of plan) { await until(at); beat(typeof o === 'function' ? o() : o); } }

try {
  // ---- 1. problem (card) ---------------------------------------------------------------------------------------------
  await page.goto(CARDS + '#problem'); await page.reload();
  await scene('problem'); await until(LEN.problem);

  // ---- 2. one request in the chat; the manager works live ------------------------------------------------------------
  await page.goto(BASE + '&view=overview'); marks.cut_ask = Date.now(); await page.reload(); await sleep(1200);
  await scene('ask', CAPTION.ask);
  const box = page.locator('textarea[data-act="mc-text"]').first();
  await waitFor(box, 10000);
  await click(box, 1100);
  await page.keyboard.type(REQUEST, { delay: 42 });
  await sleep(900);
  await box.fill(''); await box.evaluate((e) => e.blur());
  // the send: the request is in the inbox, the manager starts its turn
  beat({ asked: true, mgr: 1 });
  await glide(1560, 330, 900);
  const sent = Date.now();
  for (let n = 2; n <= MANAGER.length; n++) {      // one more step every 0.9 s; the plan items and the lanes appear with their results
    await sleep(sent + (n - 1) * 900 - Date.now());
    beat({ mgr: n, plan: n >= 8 ? 1 : 0, lanes: n >= 12, wren: n >= 12 ? 1 : 0, ivren: n >= 12 ? 1 : 0 });
  }
  await until(LEN.ask);

  // ---- 3. the plan: progress and the agents per item ---------------------------------------------------------------------
  await scene('plan', CAPTION.plan);
  await glide(520, 380, 1000);
  beat({ wren: 2, ivren: 2 });
  await until(3);
  await hover(page.locator('.pstep').filter({ hasText: 'Dark mode' }).first(), 1200, 0.35, 0.2);
  await until(6);
  await hover(page.locator('.pstep').filter({ hasText: 'Faster scan' }).first(), 1200, 0.35, 0.2);
  await until(9);
  await glide(930, 330, 900);
  await until(LEN.plan);

  // ---- 4. a rule in code blocks an agent; it uses the alternative -------------------------------------------------------------
  // each caption sits on the screen that shows it: the agent's live steps (blocked command, then the alternative), then Pipelines
  await scene('rule', CAPTION.ruleIntro);
  await goView('agents', 900);
  const wren = page.locator('[data-act="agent"]').filter({ hasText: 'Wren' });
  beat({ wren: 3 }); await until(2.5);
  await waitFor(wren, 15000);
  await click(wren, 1000);                           // the drawer follows Wren's live steps
  beat({ wren: 4, violation: true });               // the raw test run, blocked with the command to use
  await caption(CAPTION.ruleBlock); await until(12);
  beat({ wren: 5 }); await until(15);
  beat({ wren: 6 }); await caption(CAPTION.ruleInstead); await until(19);
  await page.mouse.wheel(0, 300); await until(22);   // "Rules broken" and the command to run instead
  beat({ wren: 7 }); await until(25);                // it ran the tests through Tirith
  await closeDrawer(900); await caption(CAPTION.rulePipe);
  await goView('pipelines', 1000); await until(28);
  const run = page.locator('[data-act="prun"]');
  await waitFor(run, 15000);
  await click(run.first(), 900); await until(32);
  await click(page.locator('[data-act="pstage"]').nth(1), 900);
  await until(LEN.rule);

  // ---- 5. claim check, auto-merge --------------------------------------------------------------------------------------
  await goView('overview', 900); await scene('merge', CAPTION.merge);
  beat({ wren: 8 }); await until(2);
  beat({ wren: WREN.length, ivren: 3, merged: true });             // reported, claims checked, merged: step 4 ticks
  await hover(page.locator('.pstep').filter({ hasText: 'Dark mode' }).first(), 1200, 0.35, 0.2);
  await until(7);
  await goView('agents', 1000); await until(8.5);
  await click(wren, 1000); await until(16);        // Review · Merged · Claims verified
  await closeDrawer(800);
  await until(LEN.merge);

  // ---- 6. the health dot heals itself ----------------------------------------------------------------------------------
  await goView('overview', 900); await scene('health', CAPTION.health);
  beat({ health: 'amber' }); await sleep(2600);
  await click(page.locator('#health-btn'), 1100); await until(6.5);
  beat({ health: 'green' }); await until(10);
  await page.keyboard.press('Escape');
  await until(LEN.health);

  // ---- 7. only a real decision reaches the owner ---------------------------------------------------------------------------
  await scene('decision', CAPTION.decision);
  beat({ decision: 'open' }); await sleep(2600);
  const ship = page.locator('.mc-opt, [data-act="nopt"]').filter({ hasText: 'Ship in v1.2' });
  await waitFor(ship, 10000);
  await hover(ship, 1100); await until(5);
  await click(ship, 300);
  beat({ decision: 'answered', closed: true });
  await glide(1500, 700, 900);
  await until(LEN.decision);

  // ---- 8. the brain: what Muse Spark writes into the state --------------------------------------------------------------------
  const st = story.state(), ivren = st.agents.find((a) => a.id === 'ivren'), wr = st.agents.find((a) => a.id === 'wren');
  await page.goto(CARDS + '#brain'); await scene('brain');
  await page.evaluate((o) => window.setBrain(o), {
    k1: 'agents[' + ivren.name + '].summary · one sentence from the log, so nobody has to ask',
    agent: '“' + ivren.summary + '”',
    k3: 'agents[' + wr.name + '].epithet · a lasting name for each agent',
    hero: '“' + wr.name + ' ' + wr.epithet + '”',
    k2: 'owner check · why this one reached the owner and nothing else did',
    room: '“Choosing which release ships a finished feature is the owner’s call.”',
  });
  await page.evaluate(() => { for (const id of ['b-agent', 'b-hero', 'b-room']) { const e = document.getElementById(id).closest('.row'); e.style.transition = 'opacity .8s'; e.style.opacity = 0; } });
  const reveal = (id) => page.evaluate((i) => { document.getElementById(i).closest('.row').style.opacity = 1; }, id);
  await until(3.4); await reveal('b-agent');
  await until(6.2); await reveal('b-hero');
  await until(9); await reveal('b-room');
  await until(LEN.brain);

  // ---- 9. closing card -----------------------------------------------------------------------------------------------------
  await page.goto(CARDS + '#close'); await page.reload(); await scene('close'); await until(LEN.close); marks.end = Date.now();
} catch (e) {
  console.log('FAILED:', e.message); await page.screenshot({ path: join(out, 'fail.png') }).catch(() => {});
  process.exitCode = 1;
} finally {
  clearInterval(heartbeat);
  const tEnd = Date.now();
  await page.close(); await ctx.close(); await browser.close();
  const vids = readdirSync(join(out, 'raw')).filter((f) => f.endsWith('.webm'));
  const vid = join(out, 'raw', vids[0]);
  const dur = +execFileSync('/opt/homebrew/bin/ffprobe', ['-v', 'error', '-show_entries', 'format=duration', '-of', 'csv=p=0', vid]).toString();
  // Scene marks are wall seconds since the page was created. The webm does not follow the wall clock exactly (it
  // drops time while a page loads), so assemble.mjs finds the two hard cuts in the video (card -> app, app -> card)
  // and maps these marks onto the video's own clock.
  const scenes = {}, narration = {};
  for (const [k, v] of Object.entries(marks)) scenes[k] = +((v - pageAt) / 1000).toFixed(2);
  for (const k of Object.keys(LEN)) if (scenes[k] !== undefined) narration[k] = +(scenes[k] + (k === 'close' ? 0.6 : 0.4)).toFixed(2);
  writeFileSync(join(out, 'scenes.json'), JSON.stringify({ video: vid, duration: dur, scenes, narration }, null, 2));
  console.log('video', vid, dur.toFixed(1) + 's', scenes, 'page errors:', errors);
}
