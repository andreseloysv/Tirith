// Run with: node --test web/js/views/chat/used.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { usedTokens, shortCount } from './used.js';

test('a turn\'s tokens are what the model read, cache included, and what it wrote', () => {
  const turn = { tokens_in: 142, tokens_out: 67674, tokens_cache_read: 7562527, tokens_cache_write: 132634 };
  assert.deepEqual(usedTokens(turn), { read: 7695303, written: 67674, all: 7762977 });
});

test('a turn whose agent reported nothing counts as none', () => {
  assert.deepEqual(usedTokens({ tokens_in: null, tokens_out: undefined }), { read: 0, written: 0, all: 0 });
  assert.equal(usedTokens({ tokens_in: 41672, tokens_out: 134, tokens_cache_read: 0 }).all, 41806);
});

test('a count is said in few characters', () => {
  const said = [[0, '0'], [950, '950'], [1000, '1k'], [67674, '68k'], [999499, '999k'], [999500, '1M'], [7762977, '7.8M'], [12000000, '12M']];
  for (const [n, want] of said) assert.equal(shortCount(n), want, String(n));
});
