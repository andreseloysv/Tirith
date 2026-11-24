import { test } from 'node:test';
import assert from 'node:assert';
import { spawn } from 'node:child_process';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

test('counts a page view without a cookie, refuses what it should, guards the dashboard', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'stats-'));
  const env = { ...process.env, STATS_DB: join(dir, 's.sqlite'), STATS_TOKEN: 'sekret', PORT: '18791' };
  const srv = spawn(process.execPath, [new URL('./server.mjs', import.meta.url).pathname], { env, stdio: 'ignore' });
  await new Promise((r) => setTimeout(r, 800));
  const hit = (p, h = {}) => fetch(`http://127.0.0.1:18791/count?p=${p}&r=https%3A%2F%2Fnews.example%2Fx&w=390`, { headers: { referer: 'https://andreseloysv.github.io/Tirith/', 'user-agent': 'Mozilla/5.0 Safari', ...h } });
  try {
    for (const r of [await hit('/marketplace/'), await hit('/marketplace/')]) {
      assert.equal(r.status, 204);
      assert.equal(r.headers.get('set-cookie'), null);
    }
    await hit('/x', { 'user-agent': 'Googlebot' });
    await hit('/x', { dnt: '1' });
    await hit('/x', { referer: 'https://evil.example/' });
    assert.equal((await fetch('http://127.0.0.1:18791/dash')).status, 401);
    const auth = { authorization: 'Basic ' + Buffer.from('owner:sekret').toString('base64') };
    const page = await (await fetch('http://127.0.0.1:18791/dash', { headers: auth })).text();
    assert.match(page, /<td>\/marketplace\/<\/td><td>1<\/td><td>2<\/td>/, 'one visitor, two views');
    assert.match(page, /news\.example/);
    assert.ok(!page.includes('/x</td>'), 'bots, do-not-track and foreign pages are not counted');
  } finally { srv.kill(); }
});
