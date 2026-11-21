// Tables: a header row, a separator row of dashes (its colons set each column's alignment) and the rows after it.
import { RE, cells } from './syntax.js';
import { inline } from './inline.js';

export function isTable(lines, i) {
  const line = lines[i];
  return line.indexOf('|') >= 0 && i + 1 < lines.length && lines[i + 1].indexOf('|') >= 0 && lines[i + 1].indexOf('-') >= 0 && RE.tableSep.test(lines[i + 1]);
}

const align = (c) => (/^:-*:$/.test(c) ? 'center' : /-:$/.test(c) ? 'right' : '');

export function table(lines, i) {
  const head = cells(lines[i]), al = cells(lines[i + 1]).map(align);
  const sty = (k) => (al[k] ? ` style="text-align:${al[k]}"` : '');
  const th = head.map((c, k) => `<th${sty(k)}>${inline(c)}</th>`).join('');
  let rows = '';
  for (i += 2; i < lines.length && lines[i].trim() !== '' && lines[i].indexOf('|') >= 0; i++) {
    rows += '<tr>' + cells(lines[i]).slice(0, head.length).map((c, k) => `<td${sty(k)}>${inline(c)}</td>`).join('') + '</tr>';
  }
  return { html: `<div class="mdtbl"><table><thead><tr>${th}</tr></thead><tbody>${rows}</tbody></table></div>`, next: i };
}
