// Run with: node --test web/js/views/bots/face/face-map.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { SHAPES } from './shapes2d.js';
import { faceShape, faceState, stateForIntent, reactionFor } from './face-map.js';
import { SHAPE_IDS, STATE_IDS } from '../../../../vendor/botface/catalog.js';
import { intentOf } from '../../../intent.js';
import { EXPRESSIONS, FACE_STATE_DATA } from '../../../../vendor/botface/face-data.js';

/* The owner's reference sheet of 2026-11-23 (35 expressions), each as the state that shows it. */
const REFERENCE = ['happy', 'sad', 'angry', 'confused', 'pleased', 'pain', 'scared', 'serious', 'silly', 'flirty',
  'nervous', 'tired', 'surprised', 'irritated', 'wtf', 'rage', 'disgusted', 'confident', 'drunk', 'concerned',
  'curious', 'pouty', 'sick', 'laughing', 'shame', 'incredulous', 'bored', 'snooty', 'cold', 'idle',
  'crying', 'devious', 'pensive', 'excited', 'triumph'];

test('the face shapes themselves pass through exact; the shapes of before land on real ones', () => {
  for (const id of SHAPE_IDS) assert.equal(faceShape(id), id);
  for (const name of Object.keys(SHAPES)) {
    assert.ok(SHAPE_IDS.includes(faceShape(name)), `${name} -> ${faceShape(name)}`);
  }
  assert.equal(faceShape('nope'), 'blob');
  assert.equal(faceShape(), 'blob');
});

test('the four states of before land on real states; new ones pass through', () => {
  assert.equal(faceState('work'), 'working');
  assert.equal(faceState('watch'), 'listening');
  assert.equal(faceState('sleep'), 'sleeping');
  assert.equal(faceState('idle'), 'idle');
  for (const s of STATE_IDS) assert.equal(faceState(s), s);
  assert.equal(faceState('nope'), 'idle');
  assert.equal(faceState(), 'idle');
});

test('what a turn is doing, in plain words, has a face state of its own', () => {
  const at = (step) => stateForIntent(intentOf(step));
  assert.equal(at(null), 'thinking');
  assert.equal(at({ role: 'tool', text: 'Grep: needle' }), 'searching');
  assert.equal(at({ role: 'tool', text: 'WebSearch: weather' }), 'searching');
  assert.equal(at({ role: 'tool', text: 'Edit: file.js' }), 'writing');
  assert.equal(at({ role: 'tool', text: 'Bash: cargo test' }), 'working');
  for (const step of [{ role: 'manager', text: 'Voy a revisar eso' }, { role: 'tool', text: '???' }]) {
    assert.ok(STATE_IDS.includes(at(step)), at(step));
  }
});

test('every expression of the reference sheet is a state whose own ring leads it', () => {
  assert.equal(REFERENCE.length, 35);
  for (const id of REFERENCE) {
    assert.ok(STATE_IDS.includes(id), `${id} is a state`);
    const pool = FACE_STATE_DATA.EXPRESSION_POOLS[id];
    assert.ok(pool && pool.length, `${id} has a pool`);
    assert.ok(pool[0] >= 25, `${id} leads with its reference ring, got ${pool[0]}`);
  }
});

test('every eye ring can melt into every other: two eyes, 48 finite points in the head', () => {
  assert.equal(EXPRESSIONS.length, 60);
  EXPRESSIONS.forEach((pair, i) => {
    assert.equal(pair.length, 2, `expression ${i}`);
    for (const ring of pair) {
      assert.equal(ring.length, 48, `expression ${i} ring`);
      for (const [x, y] of ring) {
        assert.ok(Number.isFinite(x) && Number.isFinite(y), `expression ${i} point`);
        assert.ok(x > -10 && x < 240 && y > -10 && y < 240, `expression ${i} point in head: ${x},${y}`);
      }
    }
  });
  for (const [state, pool] of Object.entries(FACE_STATE_DATA.EXPRESSION_POOLS)) {
    for (const i of pool) assert.ok(EXPRESSIONS[i], `${state} pool ${i}`);
  }
});

test('a finished turn is celebrated or mourned, by how it ended', () => {
  assert.equal(reactionFor('ok'), 'celebrate');
  assert.equal(reactionFor('failed'), 'sad');
  assert.equal(reactionFor('waiting'), null);
  assert.equal(reactionFor(''), null);
  assert.equal(reactionFor(), null);
});
