/* Run with: node web/js/needs.test.mjs  (no framework). Owner items are manager messages in the chat:
   each action and each item shows exactly once, answered items read "You: <choice>", plain messages have no buttons. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

const noop = () => {};
const view = { addEventListener: noop };
const esc = s => String(s == null ? '' : s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
const T = {
  esc, ui: {}, md: esc, fileOpts: () => null, proj: esc, ms: s => Date.parse(s) || 0, now: () => Date.parse('2026-10-29T12:00:00Z'), render: noop,
  empty: t => '<p>' + t + '</p>', copyText: noop, clockTime: () => '12:00', filter: {}, keepCapture: noop, keepRestore: noop,
  dur: () => '1s', elapsed: () => 1
};
const el = { addEventListener: noop, insertAdjacentHTML: noop, classList: { toggle: noop } };
const sandbox = { window: { T, addEventListener: noop }, document: { getElementById: id => (id === 'view' ? view : el), querySelector: () => null }, localStorage: { getItem: () => null, setItem: noop } };
vm.createContext(sandbox);
for (const f of ['./actions.js', './v-needs.js', './v-manager.js']) vm.runInContext(fs.readFileSync(new URL(f, import.meta.url), 'utf8'), sandbox);

const runRows = html => (html.match(/<li class="act/g) || []).length;
const row = (id, actions, extra) => Object.assign({
  id, kind: 'decision', text: 'Replace the old guard.', since: '2026-10-29T07:44:39Z', project: 'chromasol',
  options: [{ label: 'Not now', value: 'later' }], command: null, actions
}, extra);
const blocks = s => T.turnItems(s).map(it => T.turnBlock(it, s)).join('');

/* the real row guard-chromasol-3: one action, created with `--run ... --option "Not now|later"` */
const real = { id: 'ac-c5967d8e', label: "Update ChromaSol's old guard", command: 'uv run tirith guard reinstall chromasol --chain', cwd: '/work/collector', created_by: 'steward', at: '2026-10-29T07:44:39Z' };
let html = blocks({ waiting_on_you: [row('guard-chromasol-3', [real])] });
assert.equal(runRows(html), 1, 'one action row');
assert.ok(html.includes('data-act="nopt"') && html.includes('Not now'), 'the option is a button');
assert.ok(html.includes('data-act="mc-reply"'), 'a Reply link');

/* the same action twice (same id, or same command and label) renders once */
html = blocks({ waiting_on_you: [row('guard-chromasol-3', [real, Object.assign({}, real), Object.assign({}, real, { id: 'ac-other' })])] });
assert.equal(runRows(html), 1, 'repeated action renders once');

/* an item command equal to its action adds no second copy row */
html = blocks({ waiting_on_you: [row('w2', [real], { command: real.command })] });
assert.equal(html.includes('data-act="copy"'), false);

/* two items with the same text and project collapse into one */
const two = { waiting_on_you: [row('guard-chromasol-3', [real]), row('guard-chromasol-4', [Object.assign({}, real)])] };
assert.equal(T.turnItems(two).length, 1, 'one item');
assert.equal(T.turnOpen(two), 1);

/* the chat: an open item is a manager message with a "waits for you" mark, buttons and a count in the header */
const s = {
  projects: [{ id: 'chromasol', name: 'ChromaSol' }], inbox: [],
  says: [{ id: 'in-1', at: '2026-10-29T08:00:00Z', project: 'chromasol', text: 'Merged the release.' }],
  waiting_on_you: [row('w-9', [real])],
  answered_waits: [{ id: 'w-1', project: 'chromasol', text: 'Install the guard?', since: '2026-10-29T06:00:00Z', status: 'answered', answer: { at: '2026-10-29T06:30:00Z', value: 'done', label: 'Done', text: '' } }]
};
const chat = T.managerChat(s);
assert.ok(chat.includes('Merged the release.'));
assert.equal((chat.match(/class="mc-m mgr turn wait"/g) || []).length, 1, 'one message waits for the owner');
assert.ok(chat.includes('Waits for you'));
assert.ok(chat.includes('data-act="mc-jump"'), 'Jump to next');
assert.ok(chat.includes('<span class="icount"'), 'count badge');
assert.ok(chat.includes('Install the guard?') && chat.includes('<div class="mc-b">Done</div>'), 'answered item shows the choice');
/* a plain message has no buttons */
const plain = chat.split('Merged the release.')[1].split('class="mc-m')[0];
assert.equal(plain.includes('nbtn'), false);
/* order: oldest first */
assert.ok(chat.indexOf('Install the guard?') < chat.indexOf('Replace the old guard.') && chat.indexOf('Replace the old guard.') < chat.indexOf('Merged the release.'));
/* nothing to answer: no badge */
assert.equal(T.managerChat({ projects: [], inbox: [], waiting_on_you: [], answered_waits: [] }).includes('mc-jump'), false);
/* a concrete how-to: already done, steps with a page link and an "Open in Terminal" button, the automatic check */
const how = { steps: [{ text: 'Open the Apple page', url: 'https://appleid.apple.com/' }, { text: 'Type the password in the prompt', terminal: 'xcrun notarytool store-credentials tirith-notary' }], done_check: 'xcrun notarytool history --keychain-profile tirith-notary', already_done: ['Certificate exists'] };
html = blocks({ waiting_on_you: [row('h1', [], { howto: how })] });
assert.ok(html.includes('✓ Certificate exists') && html.includes('href="https://appleid.apple.com/"'), 'facts and link');
assert.ok(html.includes('Copy command') && !html.includes('data-act="ht-term"'), 'no Tauri: copy instead');
assert.ok(html.includes('closes the item when it is done'));
sandbox.window.__TAURI__ = { core: { invoke: noop } };
html = blocks({ waiting_on_you: [row('h1', [], { howto: how })] });
assert.ok(html.includes('data-act="ht-term"') && html.includes('data-key="h1"') && html.includes('data-i="1"') && html.includes('Open in Terminal'), 'terminal button');
assert.equal((html.match(/data-act="ht-term"/g) || []).length, 1, 'only the step with a command');
delete sandbox.window.__TAURI__;
console.log('needs: ok');
