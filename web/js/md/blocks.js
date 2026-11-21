// Block markdown: the lines are read in order and each block is one named step that returns its HTML and the
// line after it (null when the line does not start that block); a paragraph takes whatever is left.
import { esc } from './esc.js';
import { RE, startsBlock } from './syntax.js';
import { inline } from './inline.js';
import { list } from './list.js';
import { isTable, table } from './table.js';

function fence(lines, i) {
  const m = RE.fence.exec(lines[i]);
  if (!m) return null;
  const buf = [], ch = m[1].charAt(0), n = m[1].length;
  const end = new RegExp('^\\s*' + (ch === '`' ? '`' : '~') + '{' + n + ',}\\s*$');
  for (i++; i < lines.length && !end.test(lines[i]); i++) buf.push(lines[i]);
  return { html: '<pre class="mdpre"><code>' + esc(buf.join('\n')) + '</code></pre>', next: i + 1 };
}

function heading(lines, i) {
  const m = RE.heading.exec(lines[i]);
  if (!m) return null;
  const lv = Math.min(m[1].length, 4);
  return { html: `<div class="mdh mdh${lv}" role="heading" aria-level="${lv + 2}">${inline(m[2])}</div>`, next: i + 1 };
}

function rule(lines, i) {
  return RE.hr.test(lines[i]) ? { html: '<hr>', next: i + 1 } : null;
}

function quote(lines, i) {
  if (!RE.quote.test(lines[i])) return null;
  const q = [];
  while (i < lines.length && RE.quote.test(lines[i])) { q.push(RE.quote.exec(lines[i])[1]); i++; }
  return { html: '<blockquote>' + blocks(q.join('\n')) + '</blockquote>', next: i };
}

function tableBlock(lines, i) {
  return isTable(lines, i) ? table(lines, i) : null;
}

function listBlock(lines, i) {
  return RE.item.test(lines[i]) ? list(lines, i) : null;
}

function paragraph(lines, i) {
  const para = [];
  while (i < lines.length && lines[i].trim() !== '' && (para.length === 0 || !startsBlock(lines[i], lines[i + 1]))) { para.push(lines[i].trim()); i++; }
  return { html: '<p>' + para.map(inline).join('<br>') + '</p>', next: i };
}

const STEPS = [fence, heading, rule, quote, tableBlock, listBlock, paragraph];

export function blocks(src) {
  const lines = String(src == null ? '' : src).replace(/\r\n?/g, '\n').split('\n');
  let out = '', i = 0;
  while (i < lines.length) {
    if (lines[i].trim() === '') { i++; continue; }
    for (const step of STEPS) {
      const b = step(lines, i);
      if (b) { out += b.html; i = b.next; break; }
    }
  }
  return out;
}
