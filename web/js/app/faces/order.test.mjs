// Run with: node --test web/js/app/faces/order.test.mjs
// The sidebar's faces in the order the owner dragged them.
import test from 'node:test';
import assert from 'node:assert/strict';
import { sorted, moved } from './order.js';

const faces = (...ids) => ids.map((id) => ({ id }));
const ids = (list) => list.map((f) => f.id);

test('the saved order wins; faces it does not know come after, as they came', () => {
  assert.deepEqual(ids(sorted(faces('a', 'b', 'c', 'new'), ['c', 'a', 'b'])), ['c', 'a', 'b', 'new']);
  assert.deepEqual(ids(sorted(faces('a', 'b'), [])), ['a', 'b']);
  assert.deepEqual(ids(sorted(faces('a', 'b'), ['gone', 'b'])), ['b', 'a']);
});

test('a face moves to its new place and the others keep theirs', () => {
  assert.deepEqual(moved(['a', 'b', 'c', 'd'], 'd', 0), ['d', 'a', 'b', 'c']);
  assert.deepEqual(moved(['a', 'b', 'c', 'd'], 'a', 2), ['b', 'c', 'a', 'd']);
  assert.deepEqual(moved(['a', 'b'], 'a', 9), ['b', 'a']);
  assert.deepEqual(moved(['a', 'b'], 'b', -3), ['b', 'a']);
});
