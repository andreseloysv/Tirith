// Run with: node --test web/js/views/overview/crew.test.mjs
// Who works now on the Home: helpers grouped under their project's boss, titles made from items, bots at work.
import test from 'node:test';
import assert from 'node:assert/strict';
import { titleOf, crew } from './crew.js';

test('a title is the first sentence of an item, without its check and its parentheses', () => {
  assert.equal(titleOf('Add the double jump to the hero and a sound when it lands (check: `test -f x`)'), 'Add the double jump to the hero and a sound when it lands');
  assert.equal(titleOf('Mezclar la rama home-concept (la portada nueva: lo que espera)'), 'Mezclar la rama home-concept');
  assert.equal(titleOf('A per-bot secret store only the daemon can use. Add `neobot secret set`'), 'A per-bot secret store only the daemon can use');
  assert.equal(titleOf('Make level 3 harder: more enemies near the end'), 'Make level 3 harder');
  assert.ok(titleOf('x'.repeat(200)).length <= 91);
  assert.equal(titleOf(''), '');
});

const state = () => ({
  projects: [{ id: 'relagame', name: 'RelaGame' }, { id: 'tirith', name: 'Tirith' }],
  lanes: {
    a: { id: 'a', project: 'relagame', status: 'running', item: 'Jump', created_at: '2026-11-23T10:00:00Z' },
    b: { id: 'b', project: 'relagame', status: 'waiting', item: 'Level 3', created_at: '2026-11-23T09:00:00Z' },
    c: { id: 'c', project: 'tirith', status: 'merged', item: 'Done long ago', created_at: '2026-11-20T09:00:00Z' },
    d: { id: 'd', project: 'tirith', status: 'queued', item: 'Next one', created_at: '2026-11-23T11:00:00Z' },
  },
  busy: { 'bot-1': true, judge: true },
  talks: { 'bot-1': { rows: [{ role: 'routine', text: 'due' }, { role: 'tool', text: 'WebSearch: jobs' }] } },
});
const shown = (k) => k === 'bot-1';

test('helpers group under their project, the busiest boss first, nothing done', () => {
  const g = crew(state(), { project: '', q: '', cards: [], shown });
  assert.deepEqual(g.map((x) => [x.key, x.helpers.map((h) => h.id)]), [['relagame', ['b', 'a']], ['tirith', ['d']], ['bot-1', []]]);
  assert.equal(g[2].busy, true);
  assert.equal(g[2].doing, 'Looking on the web');
});

test('a helper whose question waits on a card waits for the owner; hidden bots never show', () => {
  const g = crew(state(), { project: '', q: '', cards: [{ id: 'o1', kind: 'decision', lane: 'b' }], shown });
  assert.equal(g[0].helpers.find((h) => h.id === 'b').state, 'you');
  assert.ok(!g.some((x) => x.key === 'judge'));
});

test('inside a project only its own work shows, and the search narrows it', () => {
  assert.deepEqual(crew(state(), { project: 'tirith', q: '', cards: [], shown }).map((x) => x.key), ['tirith']);
  assert.deepEqual(crew(state(), { project: '', q: 'jump', cards: [], shown }).map((x) => x.helpers.map((h) => h.id)).flat(), ['a']);
});
