// Records the Tirith demo against the LIVE page (Edge, 1920x1080) while real pipelines run.
//
//   node narrate.mjs            # first: narration audio + <out>/timings.json
//   node record.mjs [outDir]    # then: <out>/raw/*.webm and <out>/scenes.json
//   ./assemble.sh [outDir]      # finally: the mp4
//
// Requirements: a running collector (./run), Microsoft Edge, playwright-core (PLAYWRIGHT_CORE can point at a
// node_modules folder that has it; otherwise it is resolved normally), uv, the Android SDK for the ChromaSol stages.
// The recorder starts a session "Hackathon demo" and real pipelines through activity.sh, and ends the session.
import { createRequire } from 'node:module';
import { readFileSync, writeFileSync, mkdirSync, readdirSync, appendFileSync, existsSync, rmSync } from 'node:fs';
import { spawn, execFileSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { homedir } from 'node:os';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '../..');
const out = resolve(process.argv[2] || join(root, '../Tirith-video/work'));
const require = createRequire(process.env.PLAYWRIGHT_CORE ? join(resolve(process.env.PLAYWRIGHT_CORE), 'x.js') : import.meta.url);
const { chromium } = require('playwright-core');

const HOME = process.env.TIRITH_HOME || join(homedir(), '.tirith');
const STATE_JS = join(HOME, 'state.js');
const BASE = pathToFileURL(join(root, 'web/index.html')).href + '#state=' + pathToFileURL(STATE_JS).href;
const CARDS = pathToFileURL(join(here, 'cards.html')).href;
const SESSION = 'hackathon-demo';
const SESSION_NAME = 'Hackathon demo';
const OLD_SESSION_FILE = join(HOME, 'session');
const oldSession = existsSync(OLD_SESSION_FILE) ? readFileSync(OLD_SESSION_FILE, 'utf8') : null;

const T = JSON.parse(readFileSync(join(out, 'timings.json'), 'utf8'));
const agentLog = join(out, 'agent.log');
mkdirSync(join(out, 'raw'), { recursive: true });
for (const f of readdirSync(join(out, 'raw'))) rmSync(join(out, 'raw', f));

// scene length = narration + a breath, but never shorter than the on-screen action needs
const LEN = {
  problem: Math.max(T.problem + 0.7, 14.4), run: Math.max(T.run + 1.1, 8.7), overview: Math.max(T.overview + 1.6, 12.5),
  pipelines: 40, agents: Math.max(T.agents + 4, 14.5), flow: Math.max(T.flow + 6, 19),
  brain: Math.max(T.brain + 1.0, 19.5), filters: Math.max(T.filters + 4.2, 12), built: Math.max(T.built + 0.6, 16.6),
};
// where each narration clip starts, in seconds after its scene starts
const NARR = { problem: ['problem', 0.4], run: ['run', 0.9], overview: ['overview', 0.4], pipelines: ['pipelines', 0.4],
  pipelines2: ['pipelines', 14.2], pipelines3: ['pipelines', 35.5], agents: ['agents', 0.4], flow: ['flow', 0.4],
  brain: ['brain', 0.4], filters: ['filters', 0.4], built: ['built', 0.4] };

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const act = (step) => spawn(join(here, 'activity.sh'), [step], { env: { ...process.env, DEMO_WORK: out }, stdio: 'ignore', detached: true }).unref();
const actWait = (step) => execFileSync(join(here, 'activity.sh'), [step], { env: { ...process.env, DEMO_WORK: out }, stdio: 'ignore' });
const say = (line) => appendFileSync(agentLog, line + '\n');

const browser = await chromium.launch({ channel: 'msedge', headless: true });
const ctx = await browser.newContext({
  viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1,
  recordVideo: { dir: join(out, 'raw'), size: { width: 1920, height: 1080 } },
});
await ctx.addInitScript(() => {
  // a visible cursor and click ripple (headless video has none)
  const mk = () => {
    if (document.getElementById('__cur')) return;
    const c = document.createElement('div'); c.id = '__cur';
    c.innerHTML = '<svg width="34" height="34" viewBox="0 0 24 24"><path d="M5 3l14 8-6 2-3 6z" fill="#111" stroke="#fff" stroke-width="1.6" stroke-linejoin="round"/></svg>';
    c.style.cssText = 'position:fixed;left:-50px;top:-50px;z-index:2147483647;pointer-events:none;transition:none;filter:drop-shadow(0 2px 3px rgba(0,0,0,.3))';
    document.documentElement.appendChild(c);
    const st = document.createElement('style');
    st.textContent = '@keyframes __rip{from{transform:scale(.3);opacity:.6}to{transform:scale(1.6);opacity:0}}';
    document.documentElement.appendChild(st);
    addEventListener('mousemove', (e) => { c.style.left = e.clientX - 5 + 'px'; c.style.top = e.clientY - 3 + 'px'; }, true);
    addEventListener('mousedown', (e) => {
      const r = document.createElement('div');
      r.style.cssText = 'position:fixed;z-index:2147483646;pointer-events:none;width:44px;height:44px;border-radius:50%;background:rgba(47,111,237,.35);left:' + (e.clientX - 22) + 'px;top:' + (e.clientY - 22) + 'px;animation:__rip .5s ease-out forwards';
      document.documentElement.appendChild(r); setTimeout(() => r.remove(), 600);
    }, true);
  };
  addEventListener('DOMContentLoaded', mk); mk();
});
const page = await ctx.newPage();
const wall0 = Date.now();

let mx = 960, my = 540;
const ease = (t) => t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
async function glide(x, y, ms = 900) {
  const n = Math.max(6, Math.round(ms / 16)), x0 = mx, y0 = my;
  for (let i = 1; i <= n; i++) {
    const k = ease(i / n);
    await page.mouse.move(x0 + (x - x0) * k, y0 + (y - y0) * k);
    await sleep(ms / n);
  }
  mx = x; my = y;
}
async function center(loc, fx = 0.5, fy = 0.5) {
  let b = null;
  for (let i = 0; i < 12 && !b; i++) { b = await loc.first().boundingBox().catch(() => null); if (!b) await sleep(250); }   // the page re-renders every 2 s
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
async function openStage(loc, ms = 800) {   // a click opens the drawer; if the stage was already selected, click once more
  await click(loc, ms);
  if (!(await closeBtn().count())) await click(loc, 300);
}
async function waitFor(loc, timeout = 25000) { await loc.first().waitFor({ state: 'visible', timeout }); }

const marks = {};
let sceneStart = 0;
function scene(name) {
  marks[name] = Date.now(); sceneStart = Date.now();
  console.log(((Date.now() - wall0) / 1000).toFixed(1) + 's scene ' + name);
  say('[recorder] Scene: ' + name);
}
async function until(sec) { const ms = sceneStart + sec * 1000 - Date.now(); if (ms > 0) await sleep(ms); }
const nav = (v) => page.locator('[data-nav="view"][data-v="' + v + '"]');
const proj = (id) => page.locator('[data-nav="project"][data-v="' + id + '"]');
async function setSession(id, ms = 800) {   // id = session to select, or null to clear
  const on = async (x) => (await sess(x).count()) && (await sess(x).first().evaluate((e) => e.classList.contains('on')));
  if (id) { if (!(await on(id))) await click(sess(id), ms); }
  else for (const x of [SESSION, '2026-10-28-tirith']) if (await on(x)) await click(sess(x), ms);
}
const sess = (id) => page.locator('[data-nav="session"][data-v="' + id + '"]');

try {
  // ---- 1. problem: the whole fleet, unfiltered Flow ------------------------------------------------------------
  await page.goto(BASE + '&view=flow'); await page.reload(); await sleep(1500);
  scene('problem');
  await page.mouse.move(mx, my);
  const nodes = page.locator('.fnode');
  await waitFor(nodes);
  const nn = await nodes.count();
  const pick = [Math.min(2, nn - 1), Math.min(5, nn - 1), Math.min(7, nn - 1), Math.min(9, nn - 1), 1];
  for (const [i, k] of pick.entries()) { await hover(nodes.nth(k), 1500); await until(1.5 + (i + 1) * 2.2); }
  await until(LEN.problem);

  // ---- 2. one command ------------------------------------------------------------------------------------------
  act('session');
  await page.goto(CARDS + '#terminal'); scene('run'); await until(LEN.run);

  // ---- 3. overview, live -----------------------------------------------------------------------------------------
  await page.goto(BASE + '&view=overview&session=' + SESSION); await page.reload(); scene('overview');
  await sleep(500);
  setTimeout(() => act('events'), 1500);
  setTimeout(() => act('pipelines'), 11500);
  await glide(700, 300, 1400);
  await until(5);
  await waitFor(page.locator('.needs li'), 15000).catch(() => {});
  await hover(page.locator('.needs li'), 1200, 0.3, 0.5).catch(() => {});
  await until(8.5);
  await glide(1600, 260, 1400);
  await until(11);
  await glide(780, 480, 1400);
  await until(LEN.overview);

  // ---- 4. pipelines, step by step ---------------------------------------------------------------------------------
  await click(nav('pipelines'), 1000); scene('pipelines');
  const runCard = (name) => page.locator('[data-act="run"]').filter({ hasText: name });
  await waitFor(runCard('tirith-checks'), 20000);
  await click(runCard('tirith-checks'), 900); await until(4);
  const stages = page.locator('[data-act="stage"]');
  await waitFor(stages, 10000);
  await hover(stages.first(), 1200); await until(6);
  await openStage(stages.first(), 600); await until(10.5);        // log drawer of the first stage
  await closeDrawer(800); await until(12.5);
  await waitFor(runCard('android-release'), 30000);
  await click(runCard('android-release'), 1000); await until(16);
  await waitFor(stages, 10000);
  await hover(stages.nth(1), 1200); await until(18.5);
  await openStage(stages.first(), 800); await until(25);      // the real build, running
  await closeDrawer(800); await until(26.5);
  await hover(stages.nth(1), 1000); await until(28);
  await openStage(stages.nth(1), 800); await until(33);       // the tests
  await closeDrawer(800); await until(34.5);
  await hover(stages.nth(2), 1200);
  await until(LEN.pipelines);

  // ---- 5. agents -------------------------------------------------------------------------------------------------
  await click(nav('agents'), 900); scene('agents');
  const acards = page.locator('[data-act="agent"]');
  await waitFor(acards, 15000);
  await sleep(600);
  await click(acards.first(), 900); await until(6.5);
  await glide(1100, 700, 1200); await until(8);
  await closeDrawer(700); await until(9);
  // the earlier heroes: other session, narrowed by a search
  await click(page.locator('#f-q'), 800); await page.keyboard.type('glass', { delay: 150 }); await sleep(500);
  await setSession('2026-10-28-tirith', 900); await sleep(900);
  await until(LEN.agents - 3.5);
  const hero = page.locator('[data-act="agent"]').filter({ hasText: 'Neluvika' });
  if (await hero.count()) await hover(hero, 1200); else await glide(700, 400, 1200);
  await until(LEN.agents);
  await setSession(null, 700);
  await page.locator('#f-q').fill(''); await page.locator('#f-q').press('Escape');

  // ---- 6. flow ---------------------------------------------------------------------------------------------------
  await click(nav('flow'), 900); scene('flow');
  await setSession(SESSION, 800); await sleep(900);
  await until(3.5);
  const pills = page.locator('[data-act="edge"]');
  await waitFor(pills, 10000);
  await hover(pills.first(), 1000); await until(5.5);
  await openStage(pills.first(), 400); await until(9.5);     // all messages of one edge
  await closeDrawer(700); await until(10.5);
  await setSession(null, 800); await sleep(1000);            // clear: the whole graph, rooms included
  await until(12.3);
  // a room edge: the pills on the right-hand side of the graph
  const info = await pills.evaluateAll((els) => els.map((e) => [e.getBoundingClientRect().left, e.getAttribute('title') || '', +(e.querySelector('b') || {}).textContent || 0]));
  let ri = -1, best = -1;
  info.forEach(([l, t, n], i) => { if (l > 1150 && !/spam/i.test(t) && n > best) { best = n; ri = i; } });
  await openStage(pills.nth(ri >= 0 ? ri : 0), 1200);
  await until(LEN.flow);
  await closeDrawer(600);

  // ---- 7. the brain: what Muse Spark wrote into the state --------------------------------------------------------
  const st = JSON.parse(readFileSync(STATE_JS, 'utf8').replace(/^[^{]*/, '').replace(/;\s*$/, ''));
  const agents = st.agents || [];
  const mine = agents.find((a) => a.id.startsWith('demo-rec'));
  const logged = new Set(readFileSync(agentLog, 'utf8').split('\n'));
  const live = mine && mine.summary ? mine : null;
  const bh = agents.find((a) => a.id === 's-sessions') || agents.find((a) => a.status === 'done' && a.summary);
  const room = (st.rooms || []).find((r) => r.id === 'meta-hackathon-ideas') || (st.rooms || [])[0];
  console.log('brain live:', live && live.summary, '| bh:', bh && bh.summary, '| room:', room && room.summary, '| log-echo:', live && logged.has(live.summary));
  await page.goto(CARDS + '#brain'); scene('brain');
  await page.evaluate((o) => window.setBrain(o), {
    k1: 'agents[' + (live ? live.name : '') + '].summary · the agent recording this demo, read from its log',
    agent: '“' + (live ? live.summary : '') + '”',
    k3: 'agents[' + (bh ? bh.name : '') + '].summary · a hero that built Tirith',
    hero: '“' + (bh ? bh.summary : '') + '”',
    k2: 'rooms[' + (room ? room.id : '') + '].summary · ' + (room ? room.message_count : '') + ' messages in one line',
    room: '“' + (room ? room.summary : '') + '”',
  });
  await page.evaluate(() => { for (const id of ['b-agent', 'b-hero', 'b-room']) { const e = document.getElementById(id).closest('.row'); e.style.transition = 'opacity .8s'; e.style.opacity = 0; } });
  const reveal = (id) => page.evaluate((i) => { document.getElementById(i).closest('.row').style.opacity = 1; }, id);
  await until(8.6); await reveal('b-agent');
  await until(11.4); await reveal('b-hero');
  await until(16); await reveal('b-room');
  await until(LEN.brain);

  // ---- 8. search and filters --------------------------------------------------------------------------------------
  await page.goto(BASE + '&view=pipelines'); await page.reload(); scene('filters'); await sleep(1500);
  await click(proj('chromasol'), 1100); await until(4);
  await click(proj('chromasol'), 500); await sleep(500);
  await setSession(SESSION, 800); await until(7);
  await click(page.locator('#f-q'), 900); await page.keyboard.type('android', { delay: 170 }); await until(10.5);
  await page.keyboard.press('Escape'); await setSession(null, 600);
  await until(LEN.filters);

  // ---- 9. built during the hackathon ------------------------------------------------------------------------------
  act('finish');
  await page.goto(CARDS + '#built'); scene('built'); await until(LEN.built); marks.end = Date.now();
} catch (e) {
  console.log('FAILED:', e.message); await page.screenshot({ path: join(out, 'fail.png') }).catch(() => {});
} finally {
  const tEnd = Date.now();
  await page.close(); await ctx.close(); await browser.close();
  if (!marks.built) { try { actWait('finish'); } catch (e) { console.log('finish failed', e.message); } }
  const vids = readdirSync(join(out, 'raw')).filter((f) => f.endsWith('.webm'));
  const vid = join(out, 'raw', vids[0]);
  const dur = +execFileSync('/opt/homebrew/bin/ffprobe', ['-v', 'error', '-show_entries', 'format=duration', '-of', 'csv=p=0', vid]).toString();
  const start0 = tEnd - dur * 1000;   // wall time of the first video frame (the video ends when the page closes)
  const scenes = {}, narration = {};
  for (const [k, v] of Object.entries(marks)) scenes[k] = +((v - start0) / 1000).toFixed(2);
  for (const [clip, [sc, off]] of Object.entries(NARR)) if (scenes[sc] !== undefined) narration[clip] = +(scenes[sc] + off).toFixed(2);
  writeFileSync(join(out, 'scenes.json'), JSON.stringify({ video: vid, duration: dur, scenes, narration }, null, 2));
  console.log('video', vid, dur.toFixed(1) + 's', scenes);
  // let the finish step end the session, then put back the session file the owner had
  await sleep(9000);
  if (oldSession !== null && (!existsSync(OLD_SESSION_FILE) || readFileSync(OLD_SESSION_FILE, 'utf8') !== oldSession)) writeFileSync(OLD_SESSION_FILE, oldSession);
}
