// Run with: node --test web/js/views/overview/tree.test.mjs
// The Home's tree of helpers: the faces the helpers wear (kin.js) and the lines between them (lines.js).
import test from 'node:test';
import assert from 'node:assert/strict';
import { helperFaces } from './kin.js';
import { rows, comb } from './lines.js';

const SHAPES = ['round', 'cloud', 'tile', 'star', 'clover', 'blob', 'flower', 'drop', 'pill', 'tri', 'dome', 'burst', 'hex', 'hero'];
const COLORS = ['#f2f2f2', '#9a6b45', '#e5484d', '#f07a3a', '#f2a93b', '#4fb86e', '#3fb6a8', '#3b82f6', '#8b5cf6', '#ec4899', '#8c8c8c'];
const boss = { shape: 'hex', color: '#8b5cf6' };
const ids = ['add-the-double-jump-1a2b', 'make-level-3-harder-3c4d', 'new-title-screen-5e6f', 'fix-the-jump-sound-7a8b'];

test('a helper wears the same face every time, from its own id', () => {
  assert.deepEqual(helperFaces(ids, boss, SHAPES, COLORS), helperFaces(ids, boss, SHAPES, COLORS));
  assert.deepEqual(helperFaces(ids.slice(0, 1), boss, SHAPES, COLORS)[0], helperFaces(ids, boss, SHAPES, COLORS)[0]);
});

test('never the boss own shape or colour, never the star, and no two helpers alike while the lists allow', () => {
  const faces = helperFaces(ids, boss, SHAPES, COLORS);
  for (const f of faces) {
    assert.notEqual(f.shape, boss.shape);
    assert.notEqual(f.color, boss.color);
    assert.notEqual(f.shape, 'hero');
    assert.ok(SHAPES.includes(f.shape) && COLORS.includes(f.color));
  }
  assert.equal(new Set(faces.map((f) => f.shape)).size, faces.length);
  assert.equal(new Set(faces.map((f) => f.color)).size, faces.length);
});

test('many helpers still get a face each, every pair different from the boss', () => {
  const many = Array.from({ length: 30 }, (_, i) => `lane-${i}`);
  const faces = helperFaces(many, boss, SHAPES, COLORS);
  assert.equal(faces.length, 30);
  assert.ok(faces.every((f) => `${f.shape}|${f.color}` !== `${boss.shape}|${boss.color}`));
  assert.equal(new Set(faces.map((f) => `${f.shape}|${f.color}`)).size, 30);
});

const box = (x, top) => ({ x, top, bottom: top + 40, y: top + 20, left: x - 20 });

test('a row of helpers hangs from one bar with rounded ends, the middle ones from drops', () => {
  const d = rows({ x: 200, bottom: 100 }, [box(100, 160), box(200, 160), box(300, 160)]);
  assert.match(d, /^M200 104V144/);                /* the trunk, down to the bar */
  assert.equal((d.match(/Q/g) || []).length, 2);    /* the bar's two rounded ends */
  assert.match(d, /M200 144V160/);                  /* the middle helper's drop */
});

test('one helper under the boss: one straight line', () => {
  assert.equal(rows({ x: 200, bottom: 100 }, [box(200, 160)]), 'M200 104V144M200 144V160');
});

test('two rows: the trunk runs down to the second bar', () => {
  const d = rows({ x: 200, bottom: 100 }, [box(120, 160), box(280, 160), box(200, 300)]);
  assert.match(d, /^M200 104V284/);
});

test('narrow: a comb, a branch to each helper and the last one rounded', () => {
  const d = comb({ x: 30, bottom: 60 }, [box(80, 80), box(80, 130), box(80, 180)]);
  assert.match(d, /^M30 62V190Q30 200 40 200H54/);
  assert.match(d, /M30 100H54/);
  assert.match(d, /M30 150H54/);
});
