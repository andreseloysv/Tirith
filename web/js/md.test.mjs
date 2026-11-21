// Run with: node --test web/js/md.test.mjs
// (the same suite is reachable from the app resources path:
//  node --test app/src-tauri/resources/web/js/md.test.mjs)
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { md, mdInline } from './md.js';

const hrefs = (html) => [...html.matchAll(/ href="([^"]*)"/g)].map((m) => m[1]);
const noLink = (text, opts) => assert.deepEqual(hrefs(md(text, opts)), [], `should not link: ${text} -> ${md(text, opts)}`);

/* ---------- bare domains become https links ---------- */

test('bare domain with a path links to https', () => {
  const h = md('Entra en agentsmessenger.com/chat y mira');
  assert.deepEqual(hrefs(h), ['https://agentsmessenger.com/chat']);
  assert.ok(h.includes('>agentsmessenger.com/chat</a>'), h);
  assert.ok(h.includes('target="_blank" rel="noopener"'), h);
});

test('www domain links to https', () => {
  assert.deepEqual(hrefs(md('visita www.ejemplo.com hoy')), ['https://www.ejemplo.com']);
});

test('bare domain with path and query links whole', () => {
  assert.deepEqual(hrefs(md('ejemplo.com/ruta?x=1')), ['https://ejemplo.com/ruta?x=1']);
});

test('query with & survives the escaping', () => {
  assert.deepEqual(hrefs(md('ejemplo.com/buscar?q=uno&lang=es')),
    ['https://ejemplo.com/buscar?q=uno&amp;lang=es']);
});

test('subdomain, port and fragment link whole', () => {
  assert.deepEqual(hrefs(md('en api.ejemplo.com:8080/v1#top queda')),
    ['https://api.ejemplo.com:8080/v1#top']);
});

test('country and short TLDs link', () => {
  assert.deepEqual(hrefs(md('ejemplo.es y ejemplo.io')), ['https://ejemplo.es', 'https://ejemplo.io']);
});

test('mdInline links bare domains too', () => {
  assert.deepEqual(hrefs(mdInline('en ejemplo.com/chat')), ['https://ejemplo.com/chat']);
});

test('bare domain inside bold stays bold and tappable', () => {
  const h = md('**ejemplo.com**');
  assert.deepEqual(hrefs(h), ['https://ejemplo.com']);
  assert.ok(h.includes('<strong><a href="https://ejemplo.com"'), h);
});

test('a markdown link whose address is a bare domain links to https', () => {
  const h = md('[entrar](ejemplo.com/ruta?x=1)');
  assert.deepEqual(hrefs(h), ['https://ejemplo.com/ruta?x=1']);
  assert.ok(h.includes('>entrar</a>'), h);
});

/* ---------- sentence punctuation and parentheses stay outside ---------- */

test('final sentence punctuation is not part of the link', () => {
  for (const p of ['.', ',', ';', ':', '!', '?', '...']) {
    const h = md(`Mira ejemplo.com/ruta${p} luego`);
    assert.deepEqual(hrefs(h), ['https://ejemplo.com/ruta'], `punct ${p}: ${h}`);
    assert.ok(h.includes(`</a>${p} luego`), `punct ${p} outside: ${h}`);
  }
});

test('a closing parenthesis of the sentence is not part of the link', () => {
  const h = md('(mira ejemplo.com/ruta)');
  assert.deepEqual(hrefs(h), ['https://ejemplo.com/ruta']);
  assert.ok(h.includes('</a>)'), h);
});

test('balanced parentheses inside the address stay in the link', () => {
  assert.deepEqual(hrefs(md('ejemplo.com/wiki/Foo_(bar) es esto')),
    ['https://ejemplo.com/wiki/Foo_(bar)']);
});

test('balanced inner and sentence outer parentheses together', () => {
  const h = md('(ver ejemplo.com/wiki/Foo_(bar))');
  assert.deepEqual(hrefs(h), ['https://ejemplo.com/wiki/Foo_(bar)']);
  assert.ok(h.includes('</a>)'), h);
});

test('spanish opening marks do not stop the link', () => {
  const h = md('¿Viste ejemplo.com? ¡Entra en ejemplo.com!');
  assert.deepEqual(hrefs(h), ['https://ejemplo.com', 'https://ejemplo.com']);
});

test('quoted bare domain keeps the quotes outside', () => {
  const h = md('dijo "ejemplo.com/ruta" y ya');
  assert.deepEqual(hrefs(h), ['https://ejemplo.com/ruta']);
});

/* ---------- scheme URLs keep working, with the same trimming ---------- */

test('https and file URLs with // still link', () => {
  assert.deepEqual(hrefs(md('https://ejemplo.com/a y file:///tmp/x')),
    ['https://ejemplo.com/a', 'file:///tmp/x']);
});

test('balanced parentheses stay in an https link too, sentence ones stay out', () => {
  assert.deepEqual(hrefs(md('mira https://ejemplo.com/wiki/Foo_(bar) ya')),
    ['https://ejemplo.com/wiki/Foo_(bar)']);
  const h = md('(mira https://ejemplo.com/a.)');
  assert.deepEqual(hrefs(h), ['https://ejemplo.com/a']);
  assert.ok(h.includes('</a>.)'), h);
});

test('an https address is linked once, not again as a bare domain', () => {
  assert.deepEqual(hrefs(md('https://ejemplo.com/ruta')), ['https://ejemplo.com/ruta']);
  assert.deepEqual(hrefs(md('[ejemplo.com](https://otro.com)')), ['https://otro.com']);
});

/* ---------- what must NOT become a link ---------- */

test('code does not link: inline code and fenced block', () => {
  const tick = '`';
  noLink(`${tick}ejemplo.com/ruta${tick}`);
  noLink('```\nejemplo.com/ruta\n```');
  assert.ok(md(`${tick}ejemplo.com${tick}`).includes('<code>ejemplo.com</code>'));
});

test('emails do not link, not even their domain half', () => {
  noLink('escribe a nombre.apellido@ejemplo.com hoy');
  noLink('(nombre@ejemplo.com)');
  noLink('mailto:nombre@ejemplo.com');
  assert.ok(md('a nombre@ejemplo.com').includes('nombre@ejemplo.com'));
});

test('a loose local file name does not link (this test decides: it looks like a file)', () => {
  noLink('abre palabra.txt y léelo');
  noLink('mira notas.md, script.js, cli.py y main.rs');
  noLink('el informe.pdf y la foto.png están aquí');
  noLink('descomprime archive.tar.gz, edita next.config.js y lee datos.jsonl');
});

test('false domains do not link', () => {
  noLink('hola.mundo qué tal');
  noLink('vale.Luego lo vemos');
  noLink('la versión 1.2.3 ya está');
  noLink('solo localhost y ejemplo.123 aquí');
});

test('local paths do not link as domains', () => {
  noLink('toca web/js/md.js con cuidado');
  noLink('mira carpeta/ejemplo.com/texto en el disco');
});

test('dangerous schemes in plain text do not link', () => {
  noLink('javascript:alert(1)');
  noLink('javascript://ejemplo.com/alert(1)');
  noLink('data:text/html;base64,PGI+');
  noLink('data:ejemplo.com');
  noLink('file:/etc/passwd');
  noLink('file:ejemplo.com/ruta');
  noLink('https:ejemplo.com');
  noLink('vbscript:ejemplo.com');
});

test('dangerous schemes as markdown link addresses do not link', () => {
  noLink('[pulsar](javascript:alert(1))');
  noLink('[pulsar](data:text/html;base64,PGI+)');
  noLink('[pulsar](file:/etc/passwd)');
  noLink('[pulsar](file:ejemplo.com)');
  noLink('[pulsar](https:ejemplo.com)');
});

test('a confirmed local file wins over a bare domain, the domain still links', () => {
  const opts = { file: (p) => p === 'palabra.txt', project: 'tirith' };
  const h = md('abre palabra.txt y luego ejemplo.com', opts);
  assert.ok(h.includes('class="fref"'), h);
  assert.ok(!hrefs(h).includes('https://palabra.txt'), h);
  assert.ok(hrefs(h).includes('https://ejemplo.com'), h);
});
