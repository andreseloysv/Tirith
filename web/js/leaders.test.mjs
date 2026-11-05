/* Run with: node web/js/leaders.test.mjs  (no framework). "Running Tirith": the manager card says what the turn
   running now handles and shows its steps, or that it waits; its last decisions, newest first; the CEO card says it
   is not running until state.ceo is there, then its report, directives and what waits for the owner. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

const esc = s => String(s == null ? '' : s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
let running = null;
const T = {
  esc, proj: p => p, clockTime: iso => String(iso).slice(11, 16), mdInline: esc, avatar: a => '<i>' + esc(a.name) + '</i>',
  managerTurn: () => running, liveBlock: row => '<div class="steps">' + esc(row.id) + '</div>'
};
const sandbox = { window: { T } };
vm.createContext(sandbox);
vm.runInContext(fs.readFileSync(new URL('./leaders.js', import.meta.url), 'utf8'), sandbox);

const state = {
  agents: [{ id: 'manager-1', name: 'Tarewenus', status: 'working', started_at: '2026-11-04T12:42:00Z', ran: { model: 'm-5', effort: 'medium' } }],
  inbox: [
    { id: 'in-1', kind: 'autopilot-proposal', project: 'tirith', step: '7', text: 'Autopilot: tirith step 7: 6 items',
      answers: [{ by: 'manager', at: '2026-11-04T12:48:03Z', text: 'Autopilot: step 7 → started lane x' }] },
    { id: 'in-2', kind: 'chat', project: 'tirith', text: 'why only one agent?',
      answers: [{ by: 'tirith', at: '2026-11-04T12:50:00Z', text: 'not the manager' },
                { by: 'manager', at: '2026-11-04T13:10:00Z', text: 'The load limit is 1.' }] }
  ],
  manager: { presence: { working: true, session: { context_tokens: 58271 } },
             turns: [{ id: 'manager-turn-a', started_at: new Date().toISOString(), live: { cost_usd: 0.71 } }] }
};

let html = T.leaders(state);
assert.match(html, /Running Tirith/);
assert.match(html, /Tarewenus/);
assert.match(html, /<b>Manager<\/b><span class="pill">Tarewenus<\/span>/, 'role is the bold title, the name the grey line under it');
assert.match(html, /m-5 · medium/);
assert.match(html, /Waiting for the next message · last answer 13:10/, 'idle: it waits, with its last answer');
assert.ok(html.indexOf('The load limit is 1.') < html.indexOf('step 7 → started lane x'), 'decisions newest first');
assert.doesNotMatch(html, /not the manager/, 'only the manager\'s own answers are its decisions');
assert.match(html, /Today: 1 turn · \$0\.71 · context 58k tokens/);
assert.match(html, /Not running yet/, 'no state.ceo: the CEO card says so');

running = { id: 'manager-turn-b', turn_requests: ['in-1'], live: {} };
html = T.leaders(state);
assert.match(html, /Working on autopilot step 7 · tirith: Autopilot: tirith step 7: 6 items/);
assert.match(html, /<div class="steps">manager-turn-b<\/div>/, 'the live steps of the turn');

state.ceo = { enabled: true, last_at: '2026-11-04T09:00:00Z', last_result: 'directives', next_at: '2026-11-04T14:00:00Z',
  report: ['3 items reached main today (target 10).'], directives: [{ text: 'Freeze step 17', status: 'open' }],
  proposals: [{ text: 'Count only Tirith CPU', decided: false }, { text: 'old', decided: true }], findings_open: 4 };
html = T.leaders(state);
assert.match(html, /Last review 09:00 · directives · next 14:00/);
assert.match(html, /3 items reached main today/);
assert.match(html, /open<\/span> Freeze step 17/);
assert.match(html, /Waiting for you<\/span><ul class="lead-list"><li>Count only Tirith CPU<\/li><\/ul>/, 'only undecided proposals');
assert.match(html, /4 open findings/);
console.log('leaders: ok');
