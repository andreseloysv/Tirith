// Perf meter for the bots' faces (web/js/views/bots/face.js): serves web/ on its own
// throwaway server (never the daemon on 7717), opens a page with 17 animated faces in
// headless Edge/Chrome and traces 5 s with motion and 5 s with reduced motion, summing
// the same timeline events the 2026-11-21 measurement used (GPUTask, RasterTask, Paint,
// Layout, Layerize) as ms per second. With motion the sum must stay at or below the
// limit (default 180 ms/s). The faces changed renderer on 2026-11-23 (the owner asked
// for the vendored animation exact, free-running): the old 40 ms/s belonged to the old
// layered faces; the exact ones measured 67-162 ms/s here, the spread being the headless
// software GPU. 180 keeps this meter to catching a new order-of-magnitude regression.
//
//   node tools/perf/faces.mjs [--shots <dir>] [--limit <ms>]
// --shots also writes faces-dark.png / faces-light.png for an eyeball comparison.
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { existsSync, readFile, writeFile, mkdir, rm } from 'node:fs';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, extname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const read = promisify(readFile), write = promisify(writeFile), mkdirp = promisify(mkdir);
const WEB = join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'web');
const EVENTS = ['GPUTask', 'RasterTask', 'Paint', 'Layout', 'Layerize'];
const TRACE_S = 5;
const args = process.argv.slice(2);
const opt = (name, dflt) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : dflt; };
const LIMIT = Number(opt('--limit', process.env.FACES_LIMIT_MS || 180));
const SHOTS = opt('--shots', null);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/* ---------- the 17-faces page (generated, not part of the app) ---------- */
const PAGE = `<!doctype html><meta charset="utf-8"><title>faces perf</title>
<link rel="stylesheet" href="/css/style.css"><link rel="stylesheet" href="/css/bots.css">
<style>body{background:var(--bg);margin:0}#grid{display:flex;flex-wrap:wrap;gap:18px;padding:24px;max-width:1000px}</style>
<div id="grid"></div>
<script type="module">
import { html, render } from '/vendor/preact-htm.js';
import { Face, SHAPES, COLORS } from '/js/views/bots/face.js';
const q = new URLSearchParams(location.search);
if (q.get('theme')) document.documentElement.dataset.theme = q.get('theme');
const shapes = Object.keys(SHAPES), states = ['idle', 'watch', 'work', 'sleep'];
const faces = Array.from({ length: 17 }, (_, i) => ({
  shape: shapes[i % shapes.length], color: COLORS[(i * 3 + 1) % COLORS.length],
  state: states[i % states.length], size: 64,
}));
render(html\`\${faces.map((f) => html\`<\${Face} ...\${f} />\`)}\`, document.getElementById('grid'));
</script>`;

const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.png': 'image/png', '.svg': 'image/svg+xml', '.wav': 'audio/wav' };
const server = createServer(async (req, res) => {
  const path = decodeURIComponent(new URL(req.url, 'http://x').pathname);
  if (path === '/__faces.html') { res.writeHead(200, { 'content-type': 'text/html' }); res.end(PAGE); return; }
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

/* ---------- headless browser over CDP (no dependencies) ---------- */
const CANDIDATES = [process.env.FACES_BROWSER,
  '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge',
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  'microsoft-edge', 'google-chrome', 'chromium', 'chromium-browser'].filter(Boolean);
const browserBin = CANDIDATES.find((c) => !c.includes('/') || existsSync(c));
if (!browserBin) { console.error('no Edge/Chrome found (set FACES_BROWSER)'); process.exit(2); }
const dbgPort = 19000 + Math.floor(Math.random() * 5000);
const profile = mkdtempSync(join(tmpdir(), 'faces-perf-'));
const browser = spawn(browserBin, ['--headless=new', `--remote-debugging-port=${dbgPort}`,
  `--user-data-dir=${profile}`, '--no-first-run', '--window-size=1100,800',
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
await cdp.send('Page.enable');

async function loadAndSettle(url) {
  const loaded = cdp.once('Page.loadEventFired');
  await cdp.send('Page.navigate', { url });
  await loaded;
  await sleep(1500); // first frames, one-time rasters, layout settle
}

async function trace(label) {
  const events = [];
  cdp.on('Tracing.dataCollected', (p) => events.push(...p.value));
  await cdp.send('Tracing.start', {
    transferMode: 'ReportEvents',
    traceConfig: { recordMode: 'recordUntilFull', includedCategories: [
      'devtools.timeline', 'disabled-by-default-devtools.timeline',
      'disabled-by-default-devtools.timeline.frame', 'disabled-by-default-devtools.timeline.layers',
      'gpu', 'toplevel', 'blink', 'cc', 'viz'] },
  });
  const t0 = Date.now();
  await sleep(TRACE_S * 1000);
  const done = cdp.once('Tracing.tracingComplete');
  await cdp.send('Tracing.end');
  await done;
  const secs = (Date.now() - t0) / 1000;
  const sums = Object.fromEntries(EVENTS.map((e) => [e, 0]));
  for (const e of events) if (e.ph === 'X' && e.dur && e.name in sums) sums[e.name] += e.dur / 1000; // µs -> ms
  const per = EVENTS.map((e) => `${e} ${(sums[e] / secs).toFixed(1)}`).join(', ');
  const total = EVENTS.reduce((s, e) => s + sums[e], 0) / secs;
  console.log(`${label}: ${per} | total ${total.toFixed(1)} ms/s`);
  return total;
}

try {
  await loadAndSettle(`${base}/__faces.html?theme=dark`);
  if (SHOTS) {
    await mkdirp(SHOTS, { recursive: true });
    for (const theme of ['dark', 'light']) {
      await loadAndSettle(`${base}/__faces.html?theme=${theme}`);
      const shot = await cdp.send('Page.captureScreenshot', { format: 'png' });
      await write(join(SHOTS, `faces-${theme}.png`), Buffer.from(shot.data, 'base64'));
      console.log(`shot: ${join(SHOTS, `faces-${theme}.png`)}`);
    }
    await loadAndSettle(`${base}/__faces.html?theme=dark`);
  }
  const motion = await trace('motion      ');
  await cdp.send('Emulation.setEmulatedMedia', { media: '', features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] });
  await loadAndSettle(`${base}/__faces.html?theme=dark`);
  await trace('reduced     ');
  console.log(motion <= LIMIT ? `ok: motion total ${motion.toFixed(1)} <= ${LIMIT} ms/s` : `FAIL: motion total ${motion.toFixed(1)} > ${LIMIT} ms/s`);
  process.exitCode = motion <= LIMIT ? 0 : 1;
} finally {
  try { ws.close(); } catch { /* already closed */ }
  browser.kill('SIGKILL');
  server.close();
  await promisify(rm)(profile, { recursive: true, force: true }).catch(() => {});
}
