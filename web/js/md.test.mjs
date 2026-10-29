/* Run with: node web/js/md.test.mjs  (no framework) */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
const { md, mdInline, mdBox, splitRef } = createRequire(import.meta.url)('./md.js');

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
/* file references: only paths the caller confirms become links (docs/CONTRACT-files.md) */
const known = ['web/js/v-plan.js', 'app/src-tauri/icons', 'docs/TODO.md', 'collector/tirith/cli.py', 'README.md', 'my_file_name.py', '/abs/x.md'];
const refs = { project: 'tirith', file: p => known.includes(p) };
const links = h => (h.match(/data-act="fopen"/g) || []).length;
t('file refs: code spans, plain text and link targets', () => {
  const h = md('Changed `web/js/v-plan.js`, **docs/TODO.md** and [the cli](collector/tirith/cli.py:42).', refs);
  has(h, '<a class="fref" href="#" data-act="fopen" data-project="tirith" data-path="web/js/v-plan.js" title="Open web/js/v-plan.js"><code>web/js/v-plan.js</code></a>',
    'data-path="docs/TODO.md"', '<strong><a class="fref"', 'data-path="collector/tirith/cli.py" data-line="42"', '>the cli</a>');
  assert.equal(links(h), 3);
  has(md('see app/src-tauri/icons/ and ./README.md', refs), 'data-path="app/src-tauri/icons/"', 'data-path="./README.md"');
  has(md('"docs/TODO.md" and (README.md)', refs), 'data-path="docs/TODO.md"', 'data-path="README.md"');
  has(md('- `/abs/x.md`\n\n| a |\n|---|\n| web/js/v-plan.js |', refs), 'data-path="/abs/x.md"', '<td><a class="fref"');
});
t('file refs: path:line and path:line:col, trailing punctuation stays text', () => {
  const h = md('See collector/tirith/cli.py:42. Then `collector/tirith/cli.py:42:7`, and README.md:3:', refs);
  has(h, 'data-line="42" title="Open collector/tirith/cli.py:42">collector/tirith/cli.py:42</a>.',
    'data-line="42" data-col="7"', 'data-path="README.md" data-line="3" title="Open README.md:3">README.md:3</a>:');
});
t('file refs: no links for URLs, commands, words or unconfirmed paths', () => {
  const h = md('https://example.com/web/js/v-plan.js and [x](https://a.b/docs/TODO.md) and `uv run pytest docs/TODO.md` ' +
    'and `tirith` and docs and --project and 12:30 and web/js/missing.js and `web/js/missing.js` and [y](javascript:alert(1))', refs);
  assert.equal(links(h), 0, h);
  has(h, 'href="https://example.com/web/js/v-plan.js"', '<code>uv run pytest docs/TODO.md</code>');
  lacks(h, 'href="javascript');
  assert.equal(links(md('`web/js/v-plan.js` docs/TODO.md')), 0);   /* no opts: nothing is linked */
  assert.equal(links(md('```\nweb/js/v-plan.js\n```', refs)), 0);   /* fenced code stays code */
});
t('file refs: emphasis does not break a path, attributes stay escaped', () => {
  has(md('edit my_file_name.py now', refs), 'data-path="my_file_name.py"', '>my_file_name.py</a>');
  const evil = { project: 'a"b<c', file: () => true };
  const h = md('`docs/x.md`', evil);
  has(h, 'data-project="a&quot;b&lt;c"'); lacks(h, 'a"b<c');
  has(mdInline('`README.md`', refs), 'data-path="README.md"');
});
t('splitRef: the shared path rules', () => {
  assert.deepEqual(splitRef('collector/tirith/cli.py:42:7.'), { path: 'collector/tirith/cli.py', line: 42, col: 7, rest: '.' });
  assert.deepEqual(splitRef('.github/workflows/ci.yml'), { path: '.github/workflows/ci.yml', line: null, col: null, rest: '' });
  ['https://a.b/c.js', 'file:///tmp/x.md', 'tirith', 'docs', '--project', '12:30', 'v1.2', '1/2', 'a:b', '$HOME/x', '']
    .forEach(w => assert.equal(splitRef(w), null, w));
});
console.log(n + ' md groups passed');
