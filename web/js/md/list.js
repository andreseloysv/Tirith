// Lists: items carry their indent; deeper indent nests, a shallower one closes levels. "[ ]" and "[x]" make tasks.
import { RE, startsBlock } from './syntax.js';
import { inline } from './inline.js';

/* the open lists, innermost last, and the HTML written so far */
function builder() {
  const b = { stack: [], html: '' };
  b.top = () => b.stack[b.stack.length - 1];
  b.open = (ordered, indent) => { b.stack.push({ indent, tag: ordered ? 'ol' : 'ul', open: false }); b.html += '<' + (ordered ? 'ol' : 'ul') + '>'; };
  b.closeItem = () => { const top = b.top(); if (top && top.open) { b.html += '</li>'; top.open = false; } };
  b.closeList = () => { b.closeItem(); b.html += '</' + b.stack.pop().tag + '>'; };
  return b;
}

/* an indented continuation line belongs to the open item */
function continues(b, line) {
  return b.stack.length && line.trim() !== '' && /^\s{2,}\S/.test(line) && !startsBlock(line.trim(), null);
}

/* close the levels deeper than this item, then open a list for it or close the item before it */
function place(b, indent, ordered) {
  while (b.stack.length && indent < b.top().indent - 1) b.closeList();
  const top = b.top();
  if (!top || indent > top.indent + 1) b.open(ordered, indent);
  else b.closeItem();
}

function item(text) {
  const task = /^\[([ xX])\]\s+(.*)$/.exec(text);
  if (!task) return '<li>' + inline(text);
  const on = task[1] === ' ' ? '' : ' on', done = task[1] === ' ' ? '' : ' done';
  return `<li class="mdtask${done}">` + `<span class="mdchk${on}" aria-hidden="true"></span>` + inline(task[2]);
}

export function list(lines, i) {
  const b = builder();
  for (; i < lines.length; i++) {
    const m = RE.item.exec(lines[i]);
    if (!m) {
      if (continues(b, lines[i])) { b.html += '<br>' + inline(lines[i].trim()); continue; }
      break;
    }
    place(b, m[1].replace(/\t/g, '    ').length, /\d/.test(m[2]));
    b.html += item(m[3]);
    b.top().open = true;
  }
  while (b.stack.length) b.closeList();
  return { html: b.html, next: i };
}
