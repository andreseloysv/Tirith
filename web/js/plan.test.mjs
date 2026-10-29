/* Run with: node web/js/plan.test.mjs  (no framework). The Plan shows only what is not done, and its one state. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

const noop = () => {};
const view = { addEventListener: noop };
const esc = s => String(s == null ? '' : s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
const T = {
  esc, ui: {}, md: esc, mdInline: esc, proj: esc, ms: s => (s ? Date.parse(s) : null), now: () => Date.parse('2026-10-28T19:40:00Z'),
  render: noop, filter: {}, clockTime: s => String(s), dur: () => '1m', elapsed: () => 60, dot: () => '', avatar: () => '<i class="av"></i>',
  agentById: (st, id) => (st.agents || []).filter(a => a.id === id)[0], actions: (l) => '<ul class="acts">' + l.map(a => '<li class="act">' + a.label + '</li>').join('') + '</ul>'
};
const sandbox = { window: { T }, document: { getElementById: () => view, querySelectorAll: () => [] }, setInterval: noop };
vm.createContext(sandbox);
vm.runInContext(fs.readFileSync(new URL('./v-plan.js', import.meta.url), 'utf8'), sandbox);

const sandboxState = fs.readFileSync(new URL('../../fixtures/state.planquiet.js', import.meta.url), 'utf8');
const holder = { window: {} };
vm.runInNewContext(sandboxState, holder);
const state = holder.window.TIRITH_STATE;
const html = T.plan(state);
const steps = html.split('<li class="pstep ').slice(1);
const stepHtml = id => steps.filter(h => h.indexOf('<span class="pn">' + id + '.</span>') >= 0)[0];

/* done steps: one compact row, no items, no state, only the History link */
for (const id of ['1', '2']) {
  const h = stepHtml(id);
  assert.ok(h.startsWith('done'), 'step ' + id + ' is done');
  assert.ok(!h.includes('pitems') && !h.includes('pstate') && !h.includes('Manager answered'), 'done step ' + id + ' shows nothing under its row');
  assert.ok(h.includes('History ('), 'done step ' + id + ' keeps History');
}
/* history is closed by default: answers, merges and handled-by agents are not in the page */
assert.ok(!html.includes('Manager answered') && !html.includes('Handled by') && !html.includes('Merged '), 'finished history stays hidden');
assert.ok(!html.includes('Passed'), 'passed actions stay hidden');

/* each state line, once per step, and open items only */
const line = (id, re) => { const h = stepHtml(id); assert.match(h, re, 'step ' + id); assert.equal((h.match(/class="pstate/g) || []).length <= 2, true); return h; };
let h = line('3', /Working: .*Marrow.*Writing the guard/);
assert.ok(!h.includes('Instalar hooks'), 'done items are not listed');
assert.ok(h.includes('Proteger los hooks') && h.includes('Probar en un proyecto'), 'open items are listed');
line('4', /Open: the manager decides/);
line('5', /Testing <b>Quilitara<\/b>/);
h = line('6', /The manager is resolving a merge/);
assert.ok(!h.includes('test_rules_load'), 'a failed review shows no detail in the Plan');
h = line('7', /Waiting for you: <button/);
assert.ok(h.includes('data-act="goowner"') && h.includes('owner-tirith-7-1a2b3c4d'), 'owner item has its button');
h = line('8', /From Nov 2/);
assert.ok(h.includes('from Nov 2') && !h.replace(/<[^>]*>/g, '').includes('desde 2026'), 'dated item shows "from Nov 2" and not the raw tag');
h = line('9', /Waiting for the manager/);
h = stepHtml('10');
assert.ok(h.includes('Migrate the config file'), 'a failed action stays visible');

/* the paused autopilot changes the queued line */
state.autopilot.enabled = false;
assert.match(stepHtml.call(null, '4') && T.plan(state).split('<li class="pstep ').filter(x => x.includes('<span class="pn">4.')).join(''), /Waiting for the autopilot/);
state.autopilot.enabled = true;

/* a ready step the autopilot cannot start says why, calmly, and is not "queued" */
state.autopilot.skipped = [{ project: 'tirith', step: '4', why: 'an agent is already on it' }];
const skippedHtml = T.plan(state).split('<li class="pstep ').filter(x => x.includes('<span class="pn">4.')).join('');
assert.match(skippedHtml, /Not started: <span class="mut">an agent is already on it/);
assert.ok(!skippedHtml.includes('Open: the manager decides'), 'a step with an agent on it is not open');
state.autopilot.skipped = [];

/* the autopilot line says what is true in plain words, never "nothing to start" while items are open */
const bar = (patch) => { Object.assign(state.autopilot, { steps_working: 0, steps_in_review: 0, proposed: 0, queued: 0, open_items: 0 }, patch); return T.plan(state); };
assert.match(bar({ steps_working: 2, steps_in_review: 1, open_items: 3 }), /2 steps in progress.*1 waiting for a review/);
assert.match(bar({ proposed: 1, open_items: 1 }), /1 proposed to the manager/);
assert.match(bar({ open_items: 2 }), /2 open items for the owner or a later date/);
assert.ok(!bar({ open_items: 2 }).includes('nothing to start'), 'never "nothing to start"');
state.autopilot.enabled = false;
assert.match(bar({ steps_working: 2 }), /<b>Paused<\/b>/);
assert.ok(!T.plan(state).includes('steps in progress'), 'paused says only that');
state.autopilot.enabled = true;
bar({});

/* History opens the full thread */
T.ui.planHist['tirith|1'] = true;
const open = T.plan(state);
assert.ok(open.includes('Manager answered') && open.includes('Handled by'), 'History shows the thread');
console.log('plan.test.mjs: ok');
