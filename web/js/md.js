// A small, safe markdown renderer for agent-written text (port of the old page's md.js, same output and classes).
// All HTML in the source is escaped first; the only tags that come out are the ones built here, and links keep
// only http, https and file URLs. md(text) -> block HTML, mdInline(text) -> inline HTML (strings, for
// dangerouslySetInnerHTML). With opts {file(path) -> bool, project}, confirmed paths become file links
// (a.fref); with opts {lane(id) -> bool}, known lane ids become lane links (a.lref, data-key = the lane id).

export function esc(s) {
  return String(s == null ? '' : s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}

/* ---------- file references ---------- */
const SHAPE = /^(?:~\/|\.{1,2}\/|\/)?[\w.@+-]+(?:\/[\w.@+-]+)*\/?$/;
const NAME_EXT = /^(?:[\w@+-]*[A-Za-z][\w@+-]*(?:\.[\w-]+)*\.[A-Za-z][A-Za-z0-9]{0,9}|\.[A-Za-z][\w.-]*)$/;
const PLAIN = /(^|[\s(\[*;])((?:~\/|\.{1,2}\/|\/)?[\w.@+-]+(?:\/[\w.@+-]+)*\/?(?::\d+){0,2}[.,:;!?]*)(?=$|[\s)\]*&])/g;
let refs = null; /* the opts of the md() call being rendered */
function looksLikePath(p) {
  if (!p || p.indexOf('://') >= 0 || p.charAt(0) === '-' || !SHAPE.test(p) || !/[A-Za-z]/.test(p)) return false;
  return p.replace(/^\/+|\/+$/g, '').indexOf('/') >= 0 || /\/$/.test(p) || /^(~\/|\.\.?\/|\/)/.test(p) || NAME_EXT.test(p);
}
/** "cli.py:42:7." -> {path: "cli.py", line: 42, col: 7, rest: "."}; null when it is not a path. */
export function splitRef(text) {
  const t = String(text).replace(/[.,:;!?]+$/, ''), m = /^(.+?)(?::(\d+)(?::(\d+))?)?$/.exec(t);
  if (!m || !looksLikePath(m[1])) return null;
  return { path: m[1], line: m[2] ? +m[2] : null, col: m[3] ? +m[3] : null, rest: String(text).slice(t.length) };
}
function normRef(p) { while (p.indexOf('./') === 0) p = p.slice(2); return p.replace(/\/+$/, '') || p; }
function fileLink(ref, inner) {
  if (!refs || !refs.file || !ref || !refs.file(normRef(ref.path))) return null;
  const at = ref.path + (ref.line ? ':' + ref.line + (ref.col ? ':' + ref.col : '') : '');
  return `<a class="fref" href="#" data-act="fopen" data-project="${esc(refs.project || '')}" data-path="${esc(ref.path)}"` +
    (ref.line ? ` data-line="${ref.line}"` : '') + (ref.col ? ` data-col="${ref.col}"` : '') + ` title="Open ${esc(at)}">${inner}</a>`;
}

/* ---------- lane references ---------- */
const LANE = /(^|[\s(\[*;])(lane\/)?([A-Za-z0-9][\w.-]*?)(?=[.,:;!?]*(?:$|[\s)\]*&]))/g;
function laneLink(id, inner) {
  if (!refs || !refs.lane || !/^[\w.-]+$/.test(id || '') || !refs.lane(id)) return null;
  return `<a class="lref" href="#" data-act="golane" data-key="${esc(id)}" title="Open lane ${esc(id)}">${inner}</a>`;
}
/* a bare word is a lane only when it cannot be an ordinary word: an id with - _ or a digit, lane/<id>,
   "lane <id>" or **<id>** */
function laneWord(s, at, m, pre, slash, id) {
  return /[-_\d]/.test(id) || !!slash || /\blanes?\s$/i.test(s.slice(0, at + pre.length)) || (pre === '*' && s.charAt(at + m.length) === '*');
}

/* ---------- inline ---------- */
function inline(src) {
  const hold = [];
  const keep = (h) => { hold.push(h); return '\u0001' + (hold.length - 1) + '\u0002'; };
  let s = esc(String(src == null ? '' : src).replace(/[\u0001\u0002]/g, ''));
  s = s.replace(/(`+)([^`]|[^`][\s\S]*?[^`])\1(?!`)/g, (m, t, code) => {
    const c = '<code>' + code.trim() + '</code>', ref = refs && !/\s/.test(code.trim()) ? splitRef(code.trim()) : null;
    return keep((ref && !ref.rest && fileLink(ref, c)) || laneLink(code.trim(), c) || c);
  });
  s = s.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, (m, label, url) => {
    if (/^(https?:|file:)/i.test(url)) return keep(`<a href="${url}" target="_blank" rel="noopener">${label}</a>`);
    const ref = refs ? splitRef(url) : null, h = (ref && !ref.rest && fileLink(ref, label)) || laneLink(url.replace(/^lane[:\/]/, ''), label);
    return h ? keep(h) : m;
  });
  s = s.replace(/(^|[\s(])((?:https?|file):\/\/[^\s<]*[^\s<.,;:!?)\]'"&])/gi, (m, pre, url) =>
    pre + keep(`<a href="${url}" target="_blank" rel="noopener">${url}</a>`));
  if (refs && refs.file) s = s.replace(PLAIN, (m, pre, tok) => {
    const ref = splitRef(tok), h = ref && fileLink(ref, tok.slice(0, tok.length - ref.rest.length));
    return h ? pre + keep(h) + ref.rest : m;
  });
  if (refs && refs.lane) s = s.replace(LANE, (m, pre, slash, id, at, all) => {
    const h = laneWord(all, at, m, pre, slash, id) && laneLink(id, (slash || '') + id);
    return h ? pre + keep(h) : m;
  });
  s = s.replace(/\*\*(?=\S)([\s\S]*?\S)\*\*/g, '<strong>$1</strong>')
    .replace(/(^|[^\w])__(?=\S)([\s\S]*?\S)__(?!\w)/g, '$1<strong>$2</strong>')
    .replace(/~~(?=\S)([\s\S]*?\S)~~/g, '<del>$1</del>')
    .replace(/(^|[^*\w])\*(?=[^\s*])([^*]*?[^\s*])\*(?![*\w])/g, '$1<em>$2</em>')
    .replace(/(^|[^_\w])_(?=[^\s_])([^_]*?[^\s_])_(?![_\w])/g, '$1<em>$2</em>');
  /* a placeholder can sit inside another one's label (a code span in a link), so unwrap until none is left */
  for (let i = 0; i < 4 && /\u0001\d+\u0002/.test(s); i++) {
    s = s.replace(/\u0001(\d+)\u0002/g, (m, n) => (hold[+n] != null ? hold[+n] : ''));
  }
  return s;
}

/* ---------- blocks ---------- */
const RE = {
  fence: /^\s*(```+|~~~+)\s*([\w+-]*)\s*$/,
  heading: /^\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$/,
  hr: /^\s{0,3}([-*_])(?:\s*\1){2,}\s*$/,
  quote: /^\s{0,3}>\s?(.*)$/,
  item: /^(\s*)([-*+]|\d+[.)])\s+(.*)$/,
  tableSep: /^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$/,
};
function cells(line) {
  const t = line.trim().replace(/^\|/, '').replace(/\|$/, '');
  return t.split(/(?<!\\)\|/).map((c) => c.trim().replace(/\\\|/g, '|'));
}
function startsBlock(line, next) {
  return RE.fence.test(line) || RE.heading.test(line) || RE.hr.test(line) || RE.quote.test(line) || RE.item.test(line) ||
    (line.indexOf('|') >= 0 && next != null && line.trim() !== '' && RE.tableSep.test(next) && next.indexOf('-') >= 0 && next.indexOf('|') >= 0);
}

function list(lines, i) {
  /* items carry their indent; deeper indent nests, a shallower one closes levels */
  const stack = [];
  let html = '';
  const open = (ordered, indent) => { stack.push({ indent, tag: ordered ? 'ol' : 'ul', open: false }); html += '<' + (ordered ? 'ol' : 'ul') + '>'; };
  const closeItem = () => { const top = stack[stack.length - 1]; if (top && top.open) { html += '</li>'; top.open = false; } };
  const closeList = () => { closeItem(); html += '</' + stack.pop().tag + '>'; };
  for (; i < lines.length; i++) {
    const m = RE.item.exec(lines[i]);
    if (!m) {
      /* an indented continuation line belongs to the open item */
      if (stack.length && lines[i].trim() !== '' && /^\s{2,}\S/.test(lines[i]) && !startsBlock(lines[i].trim(), null)) { html += '<br>' + inline(lines[i].trim()); continue; }
      break;
    }
    const indent = m[1].replace(/\t/g, '    ').length, ordered = /\d/.test(m[2]);
    while (stack.length && indent < stack[stack.length - 1].indent - 1) closeList();
    let top = stack[stack.length - 1];
    if (!top || indent > top.indent + 1) open(ordered, indent);
    else closeItem();
    top = stack[stack.length - 1];
    let text = m[3], box = '';
    const task = /^\[([ xX])\]\s+(.*)$/.exec(text);
    if (task) { box = `<span class="mdchk${task[1] === ' ' ? '' : ' on'}" aria-hidden="true"></span>`; text = task[2]; }
    html += '<li' + (task ? ` class="mdtask${task[1] === ' ' ? '' : ' done'}"` : '') + '>' + box + inline(text);
    top.open = true;
  }
  while (stack.length) closeList();
  return { html, next: i };
}

function blocks(src) {
  const lines = String(src == null ? '' : src).replace(/\r\n?/g, '\n').split('\n');
  let out = '', i = 0;
  while (i < lines.length) {
    const line = lines[i];
    let m;
    if (line.trim() === '') { i++; continue; }
    if ((m = RE.fence.exec(line))) {
      const buf = [], fence = m[1].charAt(0), n = m[1].length;
      const end = new RegExp('^\\s*' + (fence === '`' ? '`' : '~') + '{' + n + ',}\\s*$');
      for (i++; i < lines.length && !end.test(lines[i]); i++) buf.push(lines[i]);
      i++;
      out += '<pre class="mdpre"><code>' + esc(buf.join('\n')) + '</code></pre>';
      continue;
    }
    if ((m = RE.heading.exec(line))) {
      const lv = Math.min(m[1].length, 4);
      out += `<div class="mdh mdh${lv}" role="heading" aria-level="${lv + 2}">${inline(m[2])}</div>`;
      i++; continue;
    }
    if (RE.hr.test(line)) { out += '<hr>'; i++; continue; }
    if (RE.quote.test(line)) {
      const q = [];
      while (i < lines.length && RE.quote.test(lines[i])) { q.push(RE.quote.exec(lines[i])[1]); i++; }
      out += '<blockquote>' + blocks(q.join('\n')) + '</blockquote>';
      continue;
    }
    if (line.indexOf('|') >= 0 && i + 1 < lines.length && lines[i + 1].indexOf('|') >= 0 && lines[i + 1].indexOf('-') >= 0 && RE.tableSep.test(lines[i + 1])) {
      const head = cells(line), al = cells(lines[i + 1]).map((c) => (/^:-*:$/.test(c) ? 'center' : /-:$/.test(c) ? 'right' : ''));
      const sty = (k) => (al[k] ? ` style="text-align:${al[k]}"` : '');
      const th = head.map((c, k) => `<th${sty(k)}>${inline(c)}</th>`).join('');
      let rows = '';
      for (i += 2; i < lines.length && lines[i].trim() !== '' && lines[i].indexOf('|') >= 0; i++) {
        rows += '<tr>' + cells(lines[i]).slice(0, head.length).map((c, k) => `<td${sty(k)}>${inline(c)}</td>`).join('') + '</tr>';
      }
      out += `<div class="mdtbl"><table><thead><tr>${th}</tr></thead><tbody>${rows}</tbody></table></div>`;
      continue;
    }
    if (RE.item.test(line)) { const l = list(lines, i); out += l.html; i = l.next; continue; }
    const para = [];
    while (i < lines.length && lines[i].trim() !== '' && (para.length === 0 || !startsBlock(lines[i], lines[i + 1]))) { para.push(lines[i].trim()); i++; }
    out += '<p>' + para.map(inline).join('<br>') + '</p>';
  }
  return out;
}

function withRefs(opts, fn) {
  const was = refs;
  refs = opts && (typeof opts.file === 'function' || typeof opts.lane === 'function') ? opts : null;
  try { return fn(); } finally { refs = was; }
}

/** Block HTML of markdown text, wrapped in <div class="md">; '' for an empty text. */
export function md(text, opts) {
  const h = withRefs(opts, () => blocks(text));
  return h ? '<div class="md">' + h + '</div>' : '';
}

/** Inline HTML of markdown text (one line), wrapped in <span class="md mdi">. */
export function mdInline(text, opts) {
  return '<span class="md mdi">' + withRefs(opts, () => inline(String(text == null ? '' : text).replace(/\s*\n\s*/g, ' '))) + '</span>';
}

/* who speaks, bold for fast reading: a leading "CEO proposal:", "CEO directive d-1:" or "Autopilot:" of a text */
const LEAD = /^(CEO proposal:|CEO directive [^\s:]+:|CEO review:|Autopilot:)/;
export function boldLead(text) { return String(text == null ? '' : text).replace(LEAD, '**$1**'); }

/** The speaker of a message line, bold: "Manager · tirith" -> <b>Manager</b> · tirith (escaped). */
export function ctxHtml(ctx) {
  const s = String(ctx || ''), at = s.indexOf(' · ');
  return at < 0 ? '<b>' + esc(s) + '</b>' : '<b>' + esc(s.slice(0, at)) + '</b>' + esc(s.slice(at));
}
