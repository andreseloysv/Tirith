/* Run with: node web/js/boldlead.test.mjs  (no framework). Who speaks is bold for fast reading: a leading
   "CEO proposal:", "CEO directive <id>:" or "Autopilot:" of a message, and the speaker of the line under it. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

const T = { esc: s => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;') };
const sandbox = { window: { T } };
vm.createContext(sandbox);
vm.runInContext(fs.readFileSync(new URL('./md.js', import.meta.url), 'utf8'), sandbox);

assert.equal(T.boldLead('CEO proposal: Count only Tirith CPU'), '**CEO proposal:** Count only Tirith CPU');
assert.equal(T.boldLead('CEO directive d-1: a deadline'), '**CEO directive d-1:** a deadline');
assert.equal(T.boldLead('Autopilot: step 7 → started lane x'), '**Autopilot:** step 7 → started lane x');
assert.equal(T.boldLead('Waiting: Autopilot: later'), 'Waiting: Autopilot: later', 'only at the start');
assert.equal(T.boldLead(null), '');
assert.match(T.mdInline(T.boldLead('Autopilot: x')), /<strong>Autopilot:<\/strong> x/);
assert.equal(T.ctxHtml('Manager · tirith'), '<b>Manager</b> · tirith');
assert.equal(T.ctxHtml('Autopilot'), '<b>Autopilot</b>');
assert.equal(T.ctxHtml('<x> · a&b'), '<b>&lt;x></b> · a&amp;b');
console.log('boldlead: ok');
