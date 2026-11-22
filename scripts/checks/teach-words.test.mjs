// Run with: node --test scripts/checks/teach-words.test.mjs
// The extension's words (tools/teach/words.js and outward.js, no page needed) against the table the daemon passes too
// (rust/tirith/checks/teach_table.json), and the mail script's rules (web/js/views/bots/mailtext.js).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFileSync } from 'node:fs';

const require = createRequire(import.meta.url);
const table = JSON.parse(readFileSync(new URL('../../rust/tirith/checks/teach_table.json', import.meta.url)));
const words = require('../../tools/teach/words.js');
globalThis.teachWords = words.teachWords;
const outward = require('../../tools/teach/outward.js');

test('names are whole words', () => {
  for (const [text, want] of table.names) assert.equal(words.teachNamed(text), want, `«${text}»`);
});

test('fields are secret by type, autocomplete, name, label, or having been a password', () => {
  for (const [field, want] of table.fields) assert.equal(words.teachSecretField(field), want, JSON.stringify(field));
});

test('addresses lose their secrets only', () => {
  for (const [url, want] of table.urls) assert.equal(words.teachMaskUrl(url), want, url);
});

test('a click on a contenteditable never records what is typed in it', () => {
  for (const [click, want] of table.clicks) assert.equal(words.teachClickText(click), want, JSON.stringify(click));
});

test('controls that send, buy, pay, delete or publish are told by their words', () => {
  for (const [text, want] of table.outward) assert.equal(outward.teachSends(text), want, `«${text}»`);
  assert.equal(outward.teachOutward(['', null, 'Seguir', 'Enviar pedido']), 'Enviar pedido');
  assert.equal(outward.teachOutward(['Filtrar', undefined]), '');
});

/* ---------- the mail script ---------- */

import vm from 'node:vm';

const mailText = readFileSync(new URL('../../web/js/views/bots/mailtext.js', import.meta.url), 'utf8').replace('export const', 'var');
const script = (url, name) => { const box = {}; vm.runInNewContext(mailText + '\nthis.out = mailScript;', box); return box.out(url, name); };

/** The script run against a fake Gmail and a fake Tirith: `answers` is what Tirith says to each call (a status, or 'throw'). */
function run(messages, answers) {
  const sent = [], warned = [];
  const msg = (m) => ({ isUnread: () => true, markRead() { m.read = true; }, getSubject: () => m.subject, getFrom: () => 'Ana <ana@example.com>',
    getDate: () => new Date('2026-11-22T10:00:00Z'), getPlainBody: () => m.body });
  const thread = { getPermalink: () => 'https://mail.google.com/x', getMessages: () => messages.map(msg) };
  const box = {
    Utilities: { newBlob: (s) => ({ getBytes: () => Buffer.from(s, 'utf8') }) },
    UrlFetchApp: { fetch(url, o) { sent.push(o.payload); const a = answers.shift(); if (a === 'throw') throw new Error('boom'); return { getResponseCode: () => a }; } },
    GmailApp: { getUserLabelByName: () => ({ getThreads: () => [thread] }) },
    ScriptApp: {}, console: { warn: (w) => warned.push(w) },
  };
  vm.runInNewContext(script('https://hook.example/h', 'Ana') + '\nsendToTirith();', box);
  return { sent, warned };
}

test('the mail script marks a message read only after a 2xx and goes on after a failure', () => {
  const ms = [1, 2, 3, 4].map((n) => ({ subject: `mail ${n}`, body: 'hola', read: false }));
  const { sent, warned } = run(ms, [200, 500, 'throw', 204]);
  assert.deepEqual(ms.map((m) => m.read), [true, false, false, true]);
  assert.equal(sent.length, 4, 'a failure does not stop the next message');
  assert.equal(warned.length, 2);
  assert.equal(JSON.parse(sent[0]).subject, 'mail 1');
});

test('the mail script stays under 64 KB whatever the characters', () => {
  const bodies = ['a'.repeat(100000), '€'.repeat(20000), '😀'.repeat(20000), '\u0000'.repeat(20000), '"\\\n'.repeat(20000), '\ud83d'.repeat(5000)];
  for (const body of bodies) {
    const ms = [{ subject: 'S'.repeat(5000), body, read: false }];
    const { sent } = run(ms, [200]);
    assert.ok(Buffer.byteLength(sent[0], 'utf8') <= 64 * 1024, `${body.slice(0, 3)} -> ${Buffer.byteLength(sent[0], 'utf8')} bytes`);
    assert.ok(JSON.parse(sent[0]).text.length > 0 || body.length === 0, 'a body is cut, not dropped');
    assert.equal(ms[0].read, true);
  }
});

test('the script carries the address and the label as strings, whatever the name', () => {
  const text = script('https://hook.example/h?x="y"', 'Bo\nb"; evil(); //');
  assert.ok(!/^evil\(\)/m.test(text), text.split('\n').slice(0, 3).join('|'));
  assert.doesNotThrow(() => new vm.Script(text));
});
