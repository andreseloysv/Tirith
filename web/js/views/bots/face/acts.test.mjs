// Run with: node --test web/js/views/bots/face/acts.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { TAU } from './motion.js';
import { ACTS10, NAMES } from './acts.js';
import { REST, poseOf } from './pose.js';
import { Acts } from './schedule.js';

const near = (a, b, what) => assert.ok(Math.abs(a - b) < 1e-6, `${what}: ${a}, not ${b}`);

/** Steps a schedule 30 times a second; the acts it started, with when. */
function played(acts, { from = 0, secs, mood = 'watch', held = false }) {
  const out = [];
  let was = null;
  for (let t = from; t < from + secs; t += 1 / 30) {
    acts.step(t, mood, held);
    if (acts.playing && acts.playing !== was) out.push({ name: acts.playing.name, at: acts.playing.at });
    was = acts.playing;
  }
  return out;
}

test('there are exactly ten acts', () => {
  assert.equal(NAMES.length, 10);
});

test('every act starts and ends at rest, and moves in between, mirrored or not', () => {
  for (const name of NAMES) {
    for (const flip of [false, true]) {
      for (const x of [0, 1]) {
        const p = poseOf(ACTS10[name], x, { flip });
        for (const key of ['x', 'y', 'pitch', 'ex', 'ey', 'open', 'puff', 'big', 'sx', 'sy']) near(p[key], REST[key], `${name} ${key} at ${x}`);
        for (const key of ['yaw', 'roll']) near(p[key] - TAU * Math.round(p[key] / TAU), 0, `${name} ${key} at ${x}`);
        p.lids.forEach((lid) => near(lid, 1, `${name} lid at ${x}`));
      }
      const most = { moved: 0 };
      for (let x = 0; x <= 1; x += 0.01) {
        const p = poseOf(ACTS10[name], x, { flip });
        const all = [p.x, p.y, p.yaw, p.pitch, p.roll, p.ex, p.ey, p.open, p.sy, p.sx, p.puff, p.big, ...p.lids];
        assert.ok(all.every(Number.isFinite), `${name} at ${x}`);
        most.moved = Math.max(most.moved, Math.abs(p.yaw), Math.abs(p.roll), 1 - Math.min(...p.lids), Math.abs(p.ey) / 4, Math.abs(p.y) / 10, p.open);
      }
      assert.ok(most.moved > 0.25, `${name} is seen`);
    }
  }
});

test('the spin and the roll are whole turns', () => {
  const most = (name, key) => Math.max(...Array.from({ length: 101 }, (_, i) => poseOf(ACTS10[name], i / 100)[key]));
  assert.ok(most('spin', 'yaw') >= TAU);
  assert.ok(most('roll', 'roll') >= TAU);
});

test('a wink shuts one eye, and mirrored the other', () => {
  const [left, right] = poseOf(ACTS10.wink, 0.4).lids;
  assert.ok(left === 1 && right < 0.2);
  assert.deepEqual(poseOf(ACTS10.wink, 0.4, { flip: true }).lids, [right, left]);
});

test('a face at work in a chat plays all ten before one comes again, and never one twice in a row', () => {
  const list = played(new Acts({ lively: true, key: 'test:all' }), { secs: 600, mood: 'work' }).map((a) => a.name);
  assert.ok(list.length > 100, `lively: ${list.length} in ten minutes`);
  for (let i = 0; i + 10 <= list.length; i += 10) assert.equal(new Set(list.slice(i, i + 10)).size, 10, `round ${i / 10}`);
  list.forEach((name, i) => i && assert.notEqual(name, list[i - 1]));
});

test('the next turn goes on where the last turn was', () => {
  const key = 'test:turns', seen = new Set();
  for (let turn = 0; turn < 10; turn += 1) seen.add(played(new Acts({ lively: true, key }), { from: turn * 20, secs: 1, mood: 'work' })[0].name);
  assert.equal(seen.size, 10);
});

test('idle it is rare, and asleep or held by the pointer it starts none', () => {
  const list = played(new Acts(), { from: 1000, secs: 600, mood: 'idle' });
  assert.ok(list.length > 10 && list.length < 70, `idle: ${list.length} in ten minutes`);
  list.forEach((a, i) => i && assert.ok(a.at - list[i - 1].at >= 9, 'nine seconds apart at least'));
  assert.equal(played(new Acts(), { secs: 120, mood: 'sleep' }).length, 0);
  assert.equal(played(new Acts(), { secs: 120, mood: 'watch', held: true }).length, 0);
});

test('faces that go round together never start together', () => {
  const faces = Array.from({ length: 12 }, () => new Acts()), starts = [];
  const was = faces.map(() => null);
  for (let t = 5000; t < 5180; t += 1 / 30) {
    faces.forEach((f, i) => {
      f.step(t, 'watch');
      if (f.playing && f.playing !== was[i]) starts.push(t);
      was[i] = f.playing;
    });
  }
  assert.ok(starts.length > 20);
  starts.forEach((t, i) => i && assert.ok(t - starts[i - 1] >= 0.44, `two started ${t - starts[i - 1]} s apart`));
});
