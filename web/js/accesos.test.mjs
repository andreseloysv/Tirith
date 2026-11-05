/* Run with: node web/js/accesos.test.mjs  (no framework). «Accesos»: rows grouped by agent, filtered by system and
   date, linked to the agent and its plan point; no counter anywhere. */
import assert from 'node:assert/strict';
import fs from 'node:fs';
import vm from 'node:vm';

const esc = s => String(s == null ? '' : s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
const now = Date.parse('2026-11-05T12:00:00Z');
const T = {
  esc, ui: {}, views: {}, now: () => now, ms: iso => Date.parse(iso), clockTime: iso => String(iso).slice(11, 16),
  proj: p => p, avatar: a => '<i>' + esc(a.name) + '</i>', empty: t => '<p>' + esc(t) + '</p>',
  agentById: (s, id) => (s.agents || []).filter(a => a.id === id)[0] || null
};
const sandbox = { window: { T }, document: { getElementById: () => ({ addEventListener() {} }) } };
vm.createContext(sandbox);
vm.runInContext(fs.readFileSync(new URL('./v-accesos.js', import.meta.url), 'utf8'), sandbox);

const state = {
  agents: [{ id: 'w1', name: 'Tarewenus' }],
  accesses: [
    { ts: '2026-11-05T10:00:00Z', agent: 'anthropic', lane: 'w1', project: 'tirith', plan: '21: Vista', system: 'git', action: 'Bash: git push' },
    { ts: '2026-11-05T11:00:00Z', agent: 'anthropic', lane: 'w1', project: 'tirith', plan: '21: Vista', system: 'network', action: 'WebFetch https://x.dev' },
    { ts: '2026-10-01T11:00:00Z', agent: 'muse', lane: null, project: null, plan: null, system: 'git', action: 'Bash: git status' }
  ]
};
const groups = (sys, days) => T.accessGroups(state, sys, days);
assert.equal(groups('', '').length, 2, 'all dates: two agents');
assert.equal(groups('', '7').length, 1, 'last 7 days drops the old row');
assert.equal(groups('git', '7')[0].rows.length, 1, 'system filter');
assert.equal(groups('', '1')[0].rows.length, 2, 'today');

const el = {};
T.views.accesos(state, el);
assert.match(el.innerHTML, /data-act="goagent" data-key="w1"/, 'linked to the agent');
assert.match(el.innerHTML, /data-act="goplan" data-key="tirith"[^>]*>21: Vista/, 'linked to the plan point');
assert.doesNotMatch(el.innerHTML, /<em>/, 'no counter badge');
T.views.accesos({ accesses: [] }, el);
assert.match(el.innerHTML, /No accesses recorded/);
console.log('accesos ok');
