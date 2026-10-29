/* Run with: node web/js/needs.test.mjs  (no framework). "Your turn" shows each action and each item exactly once. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

const noop = () => {};
const view = { addEventListener: noop };
const esc = s => String(s == null ? '' : s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
const T = {
  esc, ui: {}, md: esc, proj: esc, ms: () => 0, now: () => 0, agoSpan: () => '', render: noop,
  empty: t => '<p>' + t + '</p>', copyText: noop
};
const sandbox = { window: { T }, document: { getElementById: () => view }, localStorage: { getItem: () => null, setItem: noop } };
vm.createContext(sandbox);
for (const f of ['./actions.js', './v-needs.js']) vm.runInContext(fs.readFileSync(new URL(f, import.meta.url), 'utf8'), sandbox);

const runRows = html => (html.match(/<li class="act/g) || []).length;
const row = (id, actions, extra) => Object.assign({
  id, kind: 'decision', text: 'Replace the old guard.', since: '2026-10-29T07:44:39Z', project: 'chromasol',
  options: [{ label: 'Not now', value: 'later' }], command: null, actions
}, extra);

/* the real row guard-chromasol-3: one action, created with `--run ... --option "Not now|later"` */
const real = { id: 'ac-c5967d8e', label: "Update ChromaSol's old guard", command: 'uv run tirith guard reinstall chromasol --chain', cwd: '/work/collector', created_by: 'steward', at: '2026-10-29T07:44:39Z' };
let html = T.needs({ waiting_on_you: [row('guard-chromasol-3', [real])] });
assert.equal(runRows(html), 1, 'one action row');

/* the same action twice (same id, or same command and label) renders once */
html = T.needs({ waiting_on_you: [row('guard-chromasol-3', [real, Object.assign({}, real), Object.assign({}, real, { id: 'ac-other' })])] });
assert.equal(runRows(html), 1, 'repeated action renders once');

/* an item command equal to its action adds no second copy row */
html = T.needs({ waiting_on_you: [row('w2', [real], { command: real.command })] });
assert.equal(html.includes('data-act="copy"'), false);

/* two items with the same text and project collapse into one */
html = T.needs({ waiting_on_you: [row('guard-chromasol-3', [real]), row('guard-chromasol-4', [Object.assign({}, real)])] });
assert.equal(html.split('class="ncard').length - 1, 1, 'one item');
assert.equal(runRows(html), 1);
console.log('needs: ok');
