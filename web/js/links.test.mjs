/* Run with: node web/js/links.test.mjs  (no framework, no browser)
   links.js: in the app a click on an http(s) link goes to open_url (the webview ignores target="_blank");
   in a plain browser, and for tirith://, file and "#" links, the click is left alone. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const { md } = createRequire(import.meta.url)('./md.js');
const src = fs.readFileSync(path.join(here, 'links.js'), 'utf8');

/* the attributes of every <a> in some HTML, as fake elements with getAttribute and closest */
function anchors(html) {
  return [...html.matchAll(/<a\s([^>]*)>/g)].map(m => {
    const attrs = Object.fromEntries([...m[1].matchAll(/([\w-]+)="([^"]*)"/g)].map(a => [a[1], a[2].replace(/&amp;/g, '&')]));
    const el = { getAttribute: k => (k in attrs ? attrs[k] : null) };
    el.closest = sel => (sel === 'a[href]' && 'href' in attrs ? el : null);
    return el;
  });
}
/* loads links.js in a fresh window; `tauri` false = a plain browser */
function load(tauri, fail) {
  const calls = [], handlers = [], notes = [];
  const window = { T: { note: (t, bad) => notes.push([t, bad]) } };
  /* args are copied out of the vm's realm so deepEqual compares plain objects */
  if (tauri) window.__TAURI__ = { core: { invoke: (cmd, args) => { calls.push({ cmd, args: { ...args } }); return fail ? Promise.reject('boom') : Promise.resolve(); } } };
  const document = { addEventListener: (type, fn, capture) => { assert.equal(type, 'click'); assert.equal(capture, true); handlers.push(fn); } };
  vm.runInNewContext(src, { window, document });
  const click = (target, taken) => {
    const e = { target, defaultPrevented: !!taken, stopped: false, preventDefault() { this.defaultPrevented = true; }, stopPropagation() { this.stopped = true; } };
    handlers.forEach(h => h(e));
    return e;
  };
  return { window, calls, notes, click };
}

let n = 0;
const t = async (name, fn) => { await fn(); n++; };

await t('a markdown link and a bare URL in the chat go to open_url in the app', () => {
  const app = load(true);
  const links = anchors(md('Sign in at https://account.apple.com/ or see [the docs](https://example.com/a?b=1&c=2).'));
  assert.equal(links.length, 2);
  const e1 = app.click(links[0]), e2 = app.click(links[1]);
  assert.ok(e1.defaultPrevented && e2.defaultPrevented);
  assert.ok(!e1.stopped, 'the click still reaches the view');
  assert.deepEqual(app.calls, [
    { cmd: 'open_url', args: { url: 'https://account.apple.com/' } },
    { cmd: 'open_url', args: { url: 'https://example.com/a?b=1&c=2' } },
  ]);
});

await t('the Open and Open on GitHub buttons (http(s) hrefs) go to open_url too', () => {
  const app = load(true);
  const [open, gh] = anchors('<a class="nbtn" href="http://localhost:3000/x" target="_blank" rel="noopener">Open</a>' +
    '<a class="ghlink" href="https://github.com/o/r/pull/1" target="_blank" rel="noopener">Open on GitHub</a>');
  app.click(open); app.click(gh);
  assert.deepEqual(app.calls.map(c => c.args.url), ['http://localhost:3000/x', 'https://github.com/o/r/pull/1']);
});

await t('a plain browser keeps its own behaviour', () => {
  const web = load(false);
  const e = web.click(anchors(md('see https://account.apple.com/'))[0]);
  assert.ok(!e.defaultPrevented);
});

await t('tirith://, file, "#" and file-reference links are left alone in the app', () => {
  const app = load(true);
  const md_ = md('open `docs/a.md` and file:///tmp/x.txt', { project: 'p', file: p => p === 'docs/a.md' });
  const links = [...anchors(md_), ...anchors('<a href="tirith://open?project=p&path=a">x</a><a href="#lane=l1">lane</a><a href="/rel">r</a>')];
  assert.equal(links.length, 5);
  links.forEach(a => assert.ok(!app.click(a).defaultPrevented, a.getAttribute('href')));
  assert.equal(app.click({ closest: () => null }).defaultPrevented, false, 'a click outside any link');
  assert.equal(app.calls.length, 0);
});

await t('a click another handler already took is left alone', () => {
  const app = load(true);
  const a = anchors(md('https://example.com/'))[0];
  app.click(a, true);
  assert.equal(app.calls.length, 0);
  assert.equal(app.window.T.webLink(a), 'https://example.com/');
});

await t('a failure shows in the toast', async () => {
  const app = load(true, true);
  app.click(anchors(md('https://example.com/'))[0]);
  await new Promise(r => setTimeout(r, 0));
  assert.deepEqual(app.notes, [['Could not open https://example.com/: boom', true]]);
});

console.log(`links.test: ${n} passed`);
