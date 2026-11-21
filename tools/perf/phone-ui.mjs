// Phone UI check: serves the real web/ page (its own throwaway server, never the
// daemon) with example API answers, opens it in headless Edge/Chrome at a phone
// size (390x844), and measures the rendered boxes: every face inside its row or
// head, no row overlapping the next, no face covering the text beside it, no
// horizontal scroll, the navigation not squeezing the content into a strip of
// the screen, and an avatar's glow fading inside its box, not clipped square at
// its edge. This is the check the 2026-11-21 phone captures asked for:
// faces that painted outside their row, over the name and text next to them.
//
//   node tools/perf/phone-ui.mjs [--shots <dir>]
// --shots writes phone-list.png (rooms/chats), phone-chat.png (a project chat)
// and phone-bot.png (a bot chat) for an eyeball comparison.
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { existsSync, readFile, writeFile, mkdir, rm, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, extname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const read = promisify(readFile), write = promisify(writeFile), mkdirp = promisify(mkdir);
const WEB = join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'web');
const args = process.argv.slice(2);
const opt = (name, dflt) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : dflt; };
const SHOTS = opt('--shots', null);
const W = 390, H = 844;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const at = (n) => new Date(Date.now() - n * 60000).toISOString();

/* ---------- example answers, in the daemon's shapes ---------- */
const PROJECTS = [
  { id: 'tirith', name: 'Tirith', color: '#3b82f6', autopilot: true },
  { id: 'zoo', name: 'Your Zoo — un proyecto con un nombre bastante largo', color: '#4fb86e', autopilot: false },
  { id: 'pigs', name: 'thethreelittlepigs', color: '#ec4899', autopilot: false },
];
const BOTS = [
  { id: 'ceo', key: 'ceo', name: 'CEO', shape: 'burst', color: '#8b5cf6', show: true, system: true, session: 'turn', access: 'read', routines: [], routes: [{ on: 'metric.red' }] },
  { id: 'mgr', key: 'manager-general', name: 'Manager', shape: 'cloud', color: '#3fb6a8', show: true, system: true, session: 'chat', routines: [], routes: [] },
  { id: 'don', key: 'bot-don', name: 'Don Eloy, el vigilante de ofertas', shape: 'pill', color: '#f07a3a', show: true, session: 'chat', routines: [{ id: 'r1', title: 'Revisar ofertas cada mañana laborable', days: [1, 2, 3, 4, 5], at: '08:00', on: true }], routes: [] },
  { id: 'luna', key: 'bot-luna', name: 'Luna', shape: 'flower', color: '#ec4899', show: true, session: 'chat', routines: [], routes: [] },
  { id: 'rex', key: 'bot-rex', name: 'Rex', shape: 'tri', color: '#e5484d', show: true, session: 'chat', routines: [], routes: [] },
  { id: 'sol', key: 'bot-sol', name: 'Sol', shape: 'round', color: '#f2a93b', show: true, session: 'chat', routines: [], routes: [] },
];
const face = (shape, color) => ({ shape, color });
const MODELS = {
  choices: { agents: ['muse'], models: ['muse-wonder'], efforts: [], codex: [] },
  events: ['lane.failed', 'metric.red'],
  roles: {
    manager: { name: 'Project Manager', face: face('hex', '#8e7cf0'), agent: 'muse', model: 'muse-wonder' },
    'manager-general': { name: 'Manager', face: face('cloud', '#3fb6a8'), agent: 'muse', model: 'muse-wonder' },
    ceo: { name: 'CEO', face: face('burst', '#8b5cf6'), agent: 'muse', model: 'muse-wonder' },
    'bot-don': { name: 'Don Eloy', face: face('pill', '#f07a3a'), agent: 'muse', model: 'muse-wonder' },
    worker: { name: 'Worker', agent: 'muse', model: 'muse-wonder' },
    judge: { name: 'Judge', agent: 'muse', model: 'muse-wonder', show: false },
  },
};
const LONG = 'Un mensaje largo de verdad, con una ruta larguísima /Users/dueño/proyectos/tirith/web/js/views/bots/face.js y una palabraqueNoSeRompeNiAunqueLaPantallaSeaUnTeléfonoPequeñoDe390Píxeles para ver si algo se sale por la derecha.';
const chatRows = (project) => [
  { id: 1, at: at(50), role: 'owner', project, text: '¿Cómo va lo de la interfaz del móvil? En las capturas las caras se montan sobre el texto.' },
  { id: 2, at: at(49), role: 'manager', project, text: LONG },
  { id: 3, at: at(40), role: 'owner', project, text: 'Vale, arréglalo de raíz y déjame captura.' },
  { id: 4, at: at(39), role: 'manager', project, text: 'Hecho:\n\n```\nnode tools/perf/phone-ui.mjs --shots /tmp/shots # un comando también bastante largo para el ancho del teléfono\n```\n\n| Pantalla | Resultado |\n| --- | --- |\n| Lista de chats con nombres largos | Sin solapes |\n| Chat abierto | Sin scroll horizontal |' },
  { id: 5, at: at(20), role: 'tirith', project, text: `merged on ${project === 'bot-don' ? 'tirith' : project} at f679356c: Faces in lists and heads stay in their avatar box (check: \`cargo test -q -p tirith\`)` },
  { id: 6, at: at(10), role: 'manager', project, text: 'Sin novedad en las demás medidas; sigo con el paso siguiente del plan y te aviso solo si hace falta algo.' },
];
const ROOMS = { rooms: [
  { id: 'room-tirith', kind: 'manager', private: true, project: 'tirith', title: '', invite: 'demo', devices: [{ key_id: 'phonekey1', name: 'iPhone del dueño', last_seen: at(5) }], managers: [], messages: [{ n: 1, from: 'Manager', at: at(30), text: LONG }] },
  { id: 'room-zoo', kind: 'manager', private: true, project: 'zoo', title: '', devices: [], managers: [], messages: [{ n: 1, from: 'Manager', at: at(90), text: 'Los leones ya tienen su audio en tres idiomas y el recinto nuevo está en el plan.' }] },
  { id: 'room-pigs', kind: 'manager', private: true, project: 'pigs', title: '', devices: [], managers: [], messages: [] },
  { id: 'room-mgr', kind: 'manager', private: true, project: 'manager-general', title: '', devices: [], managers: [], messages: [{ n: 1, from: 'Manager', at: at(15), text: 'Todo en verde salvo una métrica; ya hay una lane en ello.' }] },
  { id: 'pub-1', kind: 'public', private: false, project: '', title: 'Sala pública con un título también largo de agentes', summary: 'Agentes comparando notas sobre interfaces de teléfono.', participants: [{ name: 'Otro agente con nombre largo', messages: 3, last_seen: at(12) }], messages: [{ n: 1, from: 'Otro agente', at: at(12), text: LONG }], needs_owner: [] },
] };
const PLAN = { steps: [{ title: 'Paso 22 — la interfaz del móvil, con un título de paso también largo', items: [
  { text: 'La UI del móvil sigue rota: reproducir a 390×844 y arreglar los solapes', done: false },
  { text: 'Las caras dentro de su fila o cabecera, medido en cajas reales', done: true },
] }], autopilot: true };

function apiAnswer(path, url) {
  if (path === '/api/state') return { projects: PROJECTS, lanes: [], events: [], owner: [], busy: [] };
  if (path === '/api/bots') return BOTS;
  if (path === '/api/models') return MODELS;
  if (path === '/api/metrics') return { metrics: [{ id: 'merged', label: 'En main 24 h', value: 3, red: false }], reviews: [], reviewing: false };
  if (path === '/api/chat') return chatRows(url.searchParams.get('project') || '');
  if (path === '/api/chat/keep') return { keep: '', keep_s: 0 };
  if (path === '/api/rooms') return ROOMS;
  if (path.startsWith('/api/plan/')) return PLAN;
  if (path === '/api/guard' || path === '/api/pipelines' || path === '/api/turns') return [];
  if (path.includes('/library')) return { memory: '', skills: [] };
  return {};
}

const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.png': 'image/png', '.svg': 'image/svg+xml' };
const server = createServer(async (req, res) => {
  const url = new URL(req.url, 'http://x');
  const path = decodeURIComponent(url.pathname);
  if (path === '/api/stream') { res.writeHead(200, { 'content-type': 'text/event-stream', 'cache-control': 'no-cache' }); res.write(': open\n\n'); return; } // stays open
  if (path.endsWith('/qr.svg')) { res.writeHead(200, { 'content-type': 'image/svg+xml' }); res.end('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10" fill="#fff"/><rect x="2" y="2" width="6" height="6"/></svg>'); return; }
  if (path.startsWith('/api/')) { res.writeHead(200, { 'content-type': 'application/json' }); res.end(JSON.stringify(apiAnswer(path, url))); return; }
  try {
    const file = join(WEB, path === '/' ? 'index.html' : path);
    if (!file.startsWith(WEB)) throw new Error('outside web/');
    const body = await read(file);
    res.writeHead(200, { 'content-type': MIME[extname(file)] || 'application/octet-stream' });
    res.end(body);
  } catch { if (!res.headersSent) res.writeHead(404); res.end('not found'); }
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const base = `http://127.0.0.1:${server.address().port}`;

/* ---------- headless browser over CDP (no dependencies, as in faces.mjs) ---------- */
const CANDIDATES = [process.env.PHONE_BROWSER, process.env.FACES_BROWSER,
  '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge',
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  'microsoft-edge', 'google-chrome', 'chromium', 'chromium-browser'].filter(Boolean);
const browserBin = CANDIDATES.find((c) => !c.includes('/') || existsSync(c));
if (!browserBin) { console.error('no Edge/Chrome found (set PHONE_BROWSER)'); process.exit(2); }
const dbgPort = 19000 + Math.floor(Math.random() * 5000);
const profile = mkdtempSync(join(tmpdir(), 'phone-ui-'));
const browser = spawn(browserBin, ['--headless=new', `--remote-debugging-port=${dbgPort}`,
  `--user-data-dir=${profile}`, '--no-first-run', `--window-size=${W},${H}`,
  ...(process.env.TIRITH_SANDBOXED ? ['--no-sandbox'] : []), 'about:blank'],
{ stdio: ['ignore', 'ignore', 'pipe'] });
let browserLog = '';
browser.stderr.on('data', (d) => { browserLog += d; });

async function pageSocket() {
  for (let i = 0; i < 100; i++) {
    try {
      const list = await (await fetch(`http://127.0.0.1:${dbgPort}/json/list`)).json();
      const page = list.find((t) => t.type === 'page' && t.webSocketDebuggerUrl);
      if (page) return page.webSocketDebuggerUrl;
    } catch { /* not up yet */ }
    if (browser.exitCode != null) throw new Error(`browser exited (${browser.exitCode}): ${browserLog.slice(-400)}`);
    await sleep(150);
  }
  throw new Error('browser debugging port never answered');
}

class CDP {
  constructor(ws) { this.ws = ws; this.id = 0; this.waiting = new Map(); this.handlers = new Map();
    ws.addEventListener('message', (ev) => {
      const m = JSON.parse(ev.data);
      if (m.id && this.waiting.has(m.id)) { const w = this.waiting.get(m.id); this.waiting.delete(m.id); m.error ? w.no(new Error(m.error.message)) : w.ok(m.result); }
      else if (m.method) (this.handlers.get(m.method) || []).forEach((f) => f(m.params));
    });
  }
  send(method, params = {}) { const id = ++this.id; return new Promise((ok, no) => { this.waiting.set(id, { ok, no }); this.ws.send(JSON.stringify({ id, method, params })); }); }
  on(method, f) { if (!this.handlers.has(method)) this.handlers.set(method, []); this.handlers.get(method).push(f); }
  once(method) { return new Promise((r) => { const f = (p) => { this.handlers.set(method, this.handlers.get(method).filter((h) => h !== f)); r(p); }; this.on(method, f); }); }
}

const ws = new WebSocket(await pageSocket());
await new Promise((ok, no) => { ws.addEventListener('open', ok); ws.addEventListener('error', no); });
const cdp = new CDP(ws);
const pageErrors = [];
cdp.on('Runtime.exceptionThrown', (p) => pageErrors.push(p.exceptionDetails?.exception?.description || p.exceptionDetails?.text || 'exception'));
await cdp.send('Page.enable');
await cdp.send('Runtime.enable');
await cdp.send('Emulation.setDeviceMetricsOverride', { width: W, height: H, deviceScaleFactor: 2, mobile: true });
await cdp.send('Emulation.setTouchEmulationEnabled', { enabled: true });

async function load(hash, ready) {
  const loaded = cdp.once('Page.loadEventFired');
  await cdp.send('Page.navigate', { url: `${base}/?theme=dark#${hash}` });
  // A later screen is the same document with another hash: no load event fires, the route just redraws.
  await Promise.race([loaded, sleep(3000)]);
  // Wait until this screen has drawn what it is measured on (or give up and let the measures say what is missing).
  for (let i = 0; i < 40; i++) {
    const r = await evaluate(`location.hash === ${JSON.stringify('#' + hash)} && document.querySelectorAll(${JSON.stringify(ready)}).length > 0`);
    if (r) break;
    await sleep(150);
  }
  await sleep(700); // faces settle into their animation; boxes are the animated ones
}

async function evaluate(expr) {
  const r = await cdp.send('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: true });
  if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description || r.exceptionDetails.text);
  return r.result.value;
}

async function shot(name) {
  if (!SHOTS) return;
  await mkdirp(SHOTS, { recursive: true });
  const s = await cdp.send('Page.captureScreenshot', { format: 'png' });
  await write(join(SHOTS, name), Buffer.from(s.data, 'base64'));
  console.log(`shot: ${join(SHOTS, name)}`);
}

/* ---------- the measures, run in the page on the real rendered boxes ---------- */
const MEASURE = `(() => {
  const EPS = 1.5;
  const out = { viewport: [innerWidth, innerHeight], problems: [], counts: {} };
  const prob = (m) => out.problems.push(m);
  const rect = (el) => { const r = el.getBoundingClientRect(); return { l: r.left, t: r.top, r: r.right, b: r.bottom, w: r.width, h: r.height }; };
  const show = (el) => { const r = rect(el); return r.w > 0 && r.h > 0 && getComputedStyle(el).visibility !== 'hidden'; };
  const name = (el) => el.className && el.className.baseVal !== undefined ? 'svg' : String(el.className || el.tagName).slice(0, 60);
  const inside = (a, b, eps = EPS) => a.l >= b.l - eps && a.t >= b.t - eps && a.r <= b.r + eps && a.b <= b.b + eps;
  const overlaps = (a, b, eps = EPS) => a.l < b.r - eps && b.l < a.r - eps && a.t < b.b - eps && b.t < a.b - eps;
  const clipX = (el) => { // is this element's paint clipped to some box between it and the row?
    for (let e = el; e && e !== document.body; e = e.parentElement) {
      const o = getComputedStyle(e).overflow;
      if (o.includes('hidden') || o.includes('clip')) return true;
    }
    return false;
  };
  // 1. No horizontal scroll on the page, and nothing in the first screen sticks out of it.
  const dw = Math.max(document.documentElement.scrollWidth, document.body ? document.body.scrollWidth : 0);
  if (dw > innerWidth + EPS) prob('horizontal scroll: page is ' + dw + 'px wide on a ' + innerWidth + 'px phone');
  // 2. Every face: inside its row/head box, sized like an avatar there, clipped there, and not over the text by it.
  const ROW = '.nbot,.rface,.mc-head,.mc-line,.bhero,.bprev,.bshapes button,.blineup,.bpop';
  const faces = [...document.querySelectorAll('.bot.face')].filter(show);
  out.counts.faces = faces.length;
  if (!faces.length) prob('no faces rendered at all');
  for (const f of faces) {
    const fr = rect(f), row = f.closest(ROW);
    const tag = 'face ' + Math.round(fr.w) + 'x' + Math.round(fr.h) + ' in ' + (row ? name(row) : '(no row)');
    if (!row) { prob(tag + ': has no row/head container'); continue; }
    const rr = rect(row);
    if (!inside(fr, rr, 6)) prob(tag + ': sticks out of its row ' + JSON.stringify(fr) + ' vs ' + JSON.stringify(rr));
    if (fr.l < -EPS || fr.r > innerWidth + EPS) prob(tag + ': sticks out of the phone width');
    const avatar = f.closest('.nbot,.rface,.mc-face,.bshapes');
    if (avatar) {
      if (fr.w > 68 || fr.h > 68) prob(tag + ': an avatar face bigger than 68px');
      // The animated layer's rect rides a few px past the box mid-hop; what keeps its paint in is the clip.
      if (!clipX(f)) prob(tag + ': its glow/paint is not clipped to the avatar box (overflow visible up to the row)');
    }
    // the animated layer may ride a few px, but never into the next row or over the text: when clipped, its paint cannot.
    const float = f.querySelector('.ffloat');
    if (float && avatar && !clipX(f)) {
      const xr = rect(float);
      if (!inside(xr, rr, 2)) prob(tag + ': its animated layer leaves the row and nothing clips it');
    }
  }
  // 3. Rows of a list do not overlap each other, and faces do not cover the words beside them.
  const lists = [
    ['.nbots', '.nbot'], ['.mrooms', '.rcard, .rphone'], ['.mc-log', '.mc-m'],
    ['.brout', '.brr'], ['.blineup', '.bpop'],
  ];
  for (const [listSel, rowSel] of lists) {
    for (const list of document.querySelectorAll(listSel)) {
      const rows = [...list.querySelectorAll(':scope ' + rowSel.split(',').join(', :scope '))].filter(show)
        .map((el) => ({ el, r: rect(el) })).sort((a, b) => a.r.t - b.r.t || a.r.l - b.r.l);
      out.counts[listSel + ' ' + rowSel] = rows.length;
      for (let i = 1; i < rows.length; i++) {
        const a = rows[i - 1], b = rows[i];
        // rows side by side (same band) must not overlap either; stacked rows must not mount each other
        if (overlaps(a.r, b.r, 2)) prob('rows overlap in ' + listSel + ': ' + name(a.el) + ' over ' + name(b.el));
      }
    }
  }
  for (const head of document.querySelectorAll('.mc-head, .phead')) {
    if (!show(head)) continue;
    const f = head.querySelector('.bot.face');
    const words = head.querySelector('.mc-who, b, .mc-name');
    if (f && words && show(f) && show(words) && overlaps(rect(f), rect(words), 1))
      prob('a head face covers its title text in .' + name(head));
    if (!inside(rect(head), { l: -EPS, t: -1e6, r: innerWidth + EPS, b: 1e6 })) prob('a head sticks out of the phone width: .' + name(head));
  }
  // 4. The composer and the top bar fit the phone, so typing and sending are reachable.
  for (const sel of ['#top', '.mc-box', '.mc-comp', '.mc-head']) {
    for (const el of document.querySelectorAll(sel)) {
      if (!show(el)) continue;
      const r = rect(el);
      if (r.l < -EPS || r.r > innerWidth + EPS) prob(sel + ' sticks out sideways: ' + Math.round(r.l) + '..' + Math.round(r.r) + ' on ' + innerWidth);
    }
  }
  // 4b. The top bar's own pieces do not cover each other (the crumb's words ran under the search box).
  const bits = [...document.querySelectorAll('#top .crumb > span, #top .cmd, #top .health-btn')].filter(show);
  for (let i = 1; i < bits.length; i++)
    if (overlaps(rect(bits[i - 1]), rect(bits[i]), 1)) prob('the top bar overlaps itself: ' + name(bits[i - 1]) + ' over ' + name(bits[i]));
  // 5. The shell at phone size: navigation may not squeeze the content into a strip of the screen
  //    (the stacked-sidebar layout left the chat in the bottom half, its log a few lines high).
  const nav = document.querySelector('#nav'), content = document.querySelector('#content');
  if (content && show(content)) {
    const cr = rect(content);
    out.counts.content = Math.round(cr.w) + 'x' + Math.round(cr.h);
    if (cr.w < 240) prob('the content is squeezed to ' + Math.round(cr.w) + 'px wide on a ' + innerWidth + 'px phone');
    if (cr.h < innerHeight * 0.62) prob('the content is squeezed to ' + Math.round(cr.h) + 'px high on a ' + innerHeight + 'px phone (navigation eats the screen)');
    if (nav && show(nav) && overlaps(rect(nav), cr, 2)) prob('the navigation overlaps the content');
  }
  // 6. An avatar's animated layers sit inside the face box with room to spare, so the blurred glow
  //    fades before the clip edge instead of ending in a hard square at the box's border.
  for (const f of faces) {
    if (!f.closest('.nbot,.rface,.mc-face,.bshapes')) continue;
    const fl = f.querySelector('.ffloat');
    if (!fl) continue;
    // offsets ignore the float's animation transform: this is the room the layout leaves it
    const m = Math.min(fl.offsetLeft, fl.offsetTop,
      f.clientWidth - (fl.offsetLeft + fl.offsetWidth), f.clientHeight - (fl.offsetTop + fl.offsetHeight));
    if (m < 2) prob('an avatar glow is clipped hard at its box edge (float inset ' + m + 'px in a ' + f.clientWidth + 'px face)');
  }
  return out;
})()`;

const failures = [];
async function check(label, hash, shotName, ready) {
  await load(hash, ready);
  await shot(shotName);
  const m = await evaluate(MEASURE);
  console.log(`${label}: viewport ${m.viewport.join('x')}, faces ${m.counts.faces}, rows ${JSON.stringify(m.counts)}`);
  for (const p of m.problems) { console.log(`  FAIL ${p}`); failures.push(`${label}: ${p}`); }
  if (!m.problems.length) console.log('  ok');
  return m;
}

try {
  await check('chat list (rooms)', '/rooms/tirith', 'phone-list.png', '.rcard, .rphone');
  await check('open chat (project)', '/overview/tirith', 'phone-chat.png', '.mc-head');
  await check('open chat (bot)', '/bots/don', 'phone-bot.png', '.bhero');
  for (const e of pageErrors.slice(0, 5)) { console.log(`  FAIL page error: ${String(e).slice(0, 200)}`); failures.push(`page error: ${e}`); }
  console.log(failures.length ? `FAIL: ${failures.length} phone UI problem(s)` : 'ok: phone UI boxes fit at 390x844');
  process.exitCode = failures.length ? 1 : 0;
} finally {
  try { ws.close(); } catch { /* already closed */ }
  browser.kill('SIGKILL');
  server.close();
  await promisify(rm)(profile, { recursive: true, force: true }).catch(() => {});
}
