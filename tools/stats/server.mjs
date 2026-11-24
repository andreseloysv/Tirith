// The site's own counter, cookie-less: GET /count?p=/path&r=<referrer>&w=<width> from the page, GET /dash for the owner.
//   STATS_DB=<file> STATS_TOKEN=<secret> PORT=<n> node tools/stats/server.mjs     (behind nginx, which sets X-Forwarded-For)
import { createServer } from 'node:http';
import { timingSafeEqual } from 'node:crypto';
import { open } from './store.mjs';
import { dashboard } from './dash.mjs';

const ALLOWED = (process.env.STATS_SITES || 'andreseloysv.github.io,estudiosv.online').split(',');
const BOT = /bot|spider|crawl|slurp|headless|preview|monitor/i;
const token = process.env.STATS_TOKEN || '';
const store = open(process.env.STATS_DB || 'stats.sqlite');
const recent = new Map();

const host = (u) => { try { return new URL(u).hostname; } catch { return ''; } };
const same = (a, b) => a.length === b.length && timingSafeEqual(Buffer.from(a), Buffer.from(b));
const limited = (ip) => {
  const minute = Math.floor(Date.now() / 60000), r = recent.get(ip);
  recent.set(ip, r && r.minute === minute ? { minute, n: r.n + 1 } : { minute, n: 1 });
  if (recent.size > 5000) recent.clear();
  return recent.get(ip).n > 60;
};

function count(req, url, res) {
  const ip = String(req.headers['x-forwarded-for'] || req.socket.remoteAddress).split(',')[0].trim();
  const ua = String(req.headers['user-agent'] || '');
  const from = host(req.headers.referer || '');
  const path = url.searchParams.get('p') || '';
  const ok = ALLOWED.includes(from) && path.startsWith('/') && path.length <= 200 && !BOT.test(ua) && req.headers.dnt !== '1' && !limited(ip);
  if (ok) {
    const ref = host(url.searchParams.get('r') || '');
    store.hit({ path, ref: ref === from ? '' : ref, width: Number(url.searchParams.get('w')) || 0, ip, ua });
  }
  res.writeHead(204, { 'cache-control': 'no-store' }).end();
}

createServer((req, res) => {
  const url = new URL(req.url, 'http://x');
  if (url.pathname === '/count') return count(req, url, res);
  if (url.pathname === '/dash') {
    const given = (req.headers.authorization || '').replace(/^Basic /, '');
    const pass = Buffer.from(given, 'base64').toString().split(':').slice(1).join(':');
    if (!token || !same(pass, token)) return res.writeHead(401, { 'www-authenticate': 'Basic realm="stats"' }).end();
    return res.writeHead(200, { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store' }).end(dashboard(store));
  }
  res.writeHead(404).end();
}).listen(Number(process.env.PORT) || 8791, '127.0.0.1');
