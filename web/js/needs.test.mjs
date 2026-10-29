/* Run with: node web/js/needs.test.mjs  (no framework). "Your turn" shows each action exactly once. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

const noop = () => {};
const view = { addEventListener: noop };
const esc = s => String(s == null ? '' : s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
const T = {
  esc, ui: {}, md: esc, proj: esc, ms: () => 0, now: () => 0, agoSpan: () => '', render: noop,
  empty: t => '<p>' + t + '</p>', actions: acts => acts.map(a => '<li class="act" data-aid="' + a.id + '"></li>').join('')
};
const sandbox = { window: { T }, document: { getElementById: () => view }, localStorage: { getItem: () => null, setItem: noop } };
vm.runInNewContext(fs.readFileSync(new URL('./v-needs.js', import.meta.url), 'utf8'), sandbox);

const actions = [1, 2, 3, 4].map(i => ({ id: 'a' + i, label: 'Run ' + i, command: 'tirith run ' + i }));
const html = T.needs({ waiting_on_you: [{ id: 'w1', kind: 'action', text: 'Review failed', since: '2026-01-01T00:00:00Z', project: 'p', actions }] });
for (const a of actions) {
  assert.equal(html.split('data-aid="' + a.id + '"').length - 1, 1, a.id + ' must render exactly once');
}
console.log('needs: ok');
