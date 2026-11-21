// MCP launcher for Tirith's browser (Playwright MCP; its node_modules stay in
// ~/.tirith/browser-mcp). No desktop-app credentials are needed.
// Modes, chosen by the caller (the daemon, per role):
//   (default)    Tirith's persistent profile ~/.tirith/browser/profile. Chromium
//                allows one browser on a profile: when its SingletonLock names a
//                live process, this run falls back to --isolated instead of failing.
//   --isolated   A throwaway in-memory profile (lanes: several at once, nothing left).
//   --extension  The owner's own Edge, through the "Playwright Extension"
//                (id mmlmfjhmonkocbjadbfplnigmagldckm); the token comes from
//                PLAYWRIGHT_MCP_EXTENSION_TOKEN or ~/.tirith/env. If Edge is closed
//                or the extension does not answer, the browser tools say so and the
//                agent simply goes on without a browser.
// stdout is the MCP protocol: everything this file says goes to stderr, one line.
import { existsSync, mkdirSync, readFileSync, readdirSync, readlinkSync, statSync } from 'node:fs';
import { homedir } from 'node:os';
import { createServer, connect } from 'node:net';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const home = process.env.TIRITH_HOME || join(homedir(), '.tirith');
const runtime = join(home, 'browser-mcp/node_modules/@playwright/mcp/cli.js');
const profile = join(home, 'browser/profile');
const extra = process.argv.slice(2).filter(a => a !== '--isolated' && a !== '--extension');
const extension = process.argv.includes('--extension');
const busy = () => {
  try {
    const pid = Number(readlinkSync(join(profile, 'SingletonLock')).match(/-(\d+)$/)?.[1]);
    if (pid > 0) process.kill(pid, 0);
    return pid > 0;
  } catch { return false; }
};
const isolated = !extension && (process.argv.includes('--isolated') || busy());
if (extension && !process.env.PLAYWRIGHT_MCP_EXTENSION_TOKEN) {
  // ~/.tirith/env: a KEY=value file, or a folder of such files (toml style counts).
  try {
    const envDir = join(home, 'env');
    const files = statSync(envDir).isDirectory() ? readdirSync(envDir).map(f => join(envDir, f)) : [envDir];
    for (const f of files) {
      const m = readFileSync(f, 'utf8').match(/PLAYWRIGHT_MCP_EXTENSION_TOKEN\s*=\s*"?([^\s"#]+)"?/);
      if (m) { process.env.PLAYWRIGHT_MCP_EXTENSION_TOKEN = m[1]; break; }
    }
  } catch { /* no token file: the extension asks on screen instead */ }
}
if (!extension && !isolated) mkdirSync(profile, { recursive: true, mode: 0o700 });
const browser = process.env.TIRITH_BROWSER || (existsSync('/Applications/Microsoft Edge.app') ? 'msedge' : 'chrome');
// macOS cannot nest browser sandboxes inside Tirith's sandbox-exec network guard.
const sandbox = process.platform === 'darwin' && process.env.TIRITH_SANDBOXED === '1' ? ['--no-sandbox'] : [];
// Chromium does not honor HTTP_PROXY itself. Keep browser traffic subject to Tirith's existing network guard.
const proxy = process.env.HTTPS_PROXY || process.env.HTTP_PROXY;
if (proxy) {
  process.env.PLAYWRIGHT_MCP_PROXY_SERVER = proxy;
  const upstream = new URL(proxy);
  // Edge can decline Basic proxy challenges. Authenticate to Tirith through a loopback relay;
  // every request still reaches its proxy and block list, under the caller's existing identity.
  if (upstream.protocol === 'http:' && ['localhost', '127.0.0.1', '[::1]'].includes(upstream.hostname) && upstream.username) {
    const authorization = Buffer.from(`${decodeURIComponent(upstream.username)}:${decodeURIComponent(upstream.password)}`).toString('base64');
    const relay = createServer(socket => {
      let header = Buffer.alloc(0);
      socket.on('error', () => {});
      socket.setTimeout(30000, () => socket.destroy());
      const readHeader = chunk => {
        header = Buffer.concat([header, chunk]);
        if (header.length > 65536) return socket.destroy();
        const end = header.indexOf('\r\n\r\n');
        if (end < 0) return;
        socket.pause();
        socket.removeListener('data', readHeader);
        socket.setTimeout(0);
        const target = connect({ host: upstream.hostname.replace(/^\[|\]$/g, ''), port: Number(upstream.port || 80) });
        target.on('error', () => socket.destroy());
        socket.on('close', () => target.destroy());
        target.on('close', () => socket.end());
        target.on('connect', () => {
          const lines = header.subarray(0, end).toString('latin1').split('\r\n').filter(line => !/^(proxy-authorization|proxy-connection|connection):/i.test(line));
          if (!lines[0].startsWith('CONNECT ')) lines.push('Connection: close');
          target.write(`${lines.join('\r\n')}\r\nProxy-Authorization: Basic ${authorization}\r\n\r\n`, 'latin1');
          target.write(header.subarray(end + 4));
          socket.pipe(target).pipe(socket);
          socket.resume();
        });
      };
      socket.on('data', readHeader);
    });
    await new Promise((resolve, reject) => { relay.once('error', reject); relay.listen(0, '127.0.0.1', resolve); });
    relay.unref();
    process.env.PLAYWRIGHT_MCP_PROXY_SERVER = `http://127.0.0.1:${relay.address().port}`;
  }
}
const mode = extension ? ['--extension'] : ['--browser', browser, ...(isolated ? ['--isolated'] : ['--user-data-dir', profile]), ...sandbox];
process.argv = [process.argv[0], runtime, ...mode,
  '--output-dir', join(home, 'browser/output'), '--no-webmcp', '--codegen', 'none',
  '--redact-url-params', 'access_token,refresh_token,token,code,id_token,password', ...extra];
try {
  await import(pathToFileURL(runtime).href);
} catch (e) {
  console.error(`tirith_browser: browser unavailable (${String(e?.message || e).split('\n')[0]})`);
  process.exit(1);
}
