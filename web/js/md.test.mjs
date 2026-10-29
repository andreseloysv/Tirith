/* Run with: node web/js/md.test.mjs  (no framework) */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const { md, mdInline, mdBox } = createRequire(import.meta.url)('./md.js');

let n = 0;
const t = (name, fn) => { fn(); n++; };
const has = (html, ...bits) => bits.forEach(b => assert.ok(html.includes(b), `missing ${b} in ${html}`));
const lacks = (html, ...bits) => bits.forEach(b => assert.ok(!html.includes(b), `unexpected ${b} in ${html}`));

t('escapes html first', () => {
  const h = md('<script>alert(1)</script> & <img src=x onerror=alert(1)>');
  lacks(h, '<script', '<img');
  has(h, '&lt;script&gt;', '&amp;');
});
t('inline: bold, italic, code, strike', () => {
  has(md('**Collector** - **New `roadmap.py`:** done'), '<strong>Collector</strong>', '<strong>New <code>roadmap.py</code>:</strong>');
  has(md('a *b* and _c_ and ~~d~~'), '<em>b</em>', '<em>c</em>', '<del>d</del>');
  lacks(md('snake_case_name and 2*3*4'), '<em>');
  has(md('use `<b>x</b>` here'), '<code>&lt;b&gt;x&lt;/b&gt;</code>');
  lacks(md('`**not bold**`'), '<strong>');
});
t('headings', () => {
  has(md('# One\n## Two\n#### Four\n###### Six'), 'mdh1', 'mdh2', 'mdh4', '>Six<');
  lacks(md('#nospace'), 'mdh');
});
t('fenced code', () => {
  const h = md('```js\nlet a = "<b>";\n**x**\n```\nafter');
  has(h, '<pre class="mdpre"><code>let a = &quot;&lt;b&gt;&quot;;\n**x**</code></pre>', '<p>after</p>');
  has(md('```\nunclosed *x*'), '<pre');
});
t('lists nest by indent', () => {
  const h = md('- a\n  - b\n  - c\n- d\n\n1. one\n2. two');
  assert.equal((h.match(/<ul>/g) || []).length, 2);
  has(h, '<ul><li>a<ul><li>b</li><li>c</li></ul></li><li>d</li></ul>', '<ol><li>one</li><li>two</li></ol>');
});
t('task items', () => {
  const h = md('- [ ] todo\n- [x] done');
  has(h, 'mdtask', 'mdchk on', '>todo', '>done');
});
t('links: only http, https, file', () => {
  has(md('[ok](https://a.b/c?x=1&y=2)'), '<a href="https://a.b/c?x=1&amp;y=2" target="_blank" rel="noopener">ok</a>');
  has(md('[f](file:///tmp/x)'), 'href="file:///tmp/x"');
  lacks(md('[bad](javascript:alert(1))'), '<a');
  lacks(md('[bad](data:text/html;base64,AAA)'), '<a');
  lacks(md('[q](https://a.b/"onmouseover="x)'), 'onmouseover="x');
  has(md('see https://example.com/a. Then'), 'href="https://example.com/a"');
});
t('tables', () => {
  const h = md('| A | B |\n|---|:-:|\n| 1 | `x` |\n| 2 | 3 |');
  has(h, '<table>', '<th>A</th>', 'text-align:center', '<td>1</td>', '<code>x</code>');
  assert.equal((h.match(/<tr>/g) || []).length, 3);
  lacks(md('a | b is not a table'), '<table>');
});
t('blockquote, hr, paragraphs, line breaks', () => {
  has(md('> quoted **x**\n> more'), '<blockquote>', '<strong>x</strong>');
  has(md('a\n\n---\n\nb'), '<hr>');
  has(md('line one\nline two\n\nnext'), '<p>line one<br>line two</p>', '<p>next</p>');
});
t('empty and odd input', () => {
  assert.equal(md(''), ''); assert.equal(md(null), ''); assert.equal(md('   \n\n'), '');
  has(md(42), '42');
  lacks(md('x\u0001 0\u0002'), '\u0001');
});
t('inline mode keeps one line', () => {
  const h = mdInline('**a**\nb');
  has(h, 'mdi', '<strong>a</strong> b'); lacks(h, '<p>', '<br>');
});
t('box: renders then clamps with css, button carries the key', () => {
  const h = mdBox('# T\n' + '- x\n'.repeat(30), 'k"1');
  has(h, 'class="mdc', 'data-mdk="k&quot;1"', 'data-act="mdopen"', 'Show all');
  assert.equal((h.match(/<li>/g) || []).length, 30);
  assert.equal(mdBox('', 'k'), '');
});
console.log(n + ' md groups passed');
