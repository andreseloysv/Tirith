// A small, safe markdown renderer for agent-written text (port of the old page's md.js, same output and classes).
// All HTML in the source is escaped first; the only tags that come out are the ones built here, and links keep
// only http://, https:// and file:// URLs; a bare domain (ejemplo.com/ruta) links to https://… md(text) ->
// block HTML, mdInline(text) -> inline HTML (strings, for
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

/* ---------- bare domains ---------- */
/* a loose word whose last label is a known file extension is a local file (palabra.txt, notas.md, cli.py),
   not a domain; a www. prefix overrides this. A bare host with no tail also needs a two-letter or known TLD,
   so hola.mundo (or a full stop missing its space: vale.Luego) is not linked. */
const FILE_EXT = new Set('txt text md markdown js mjs cjs ts tsx jsx json jsonl ndjson rs py pyw rb go java class c h cpp hpp css scss sass less html htm shtml xml xsl yml yaml toml ini cfg conf log csv tsv png jpg jpeg jpe gif svg webp avif ico bmp tif tiff mp3 mp4 m4a wav ogg ogv mov webm pdf zip gz tgz tar bz2 xz 7z rar sh bash zsh fish exe dll dmg pkg iso img db sqlite sqlite3 sql php swift kt kts scala hs lhs lua jl ex exs erl clj cljs vue svelte lock diff patch bin dat bak tmp temp cache doc docx xls xlsx ppt pptx odt key pem crt cer der wasm'.split(' '));
const KNOWN_TLD = new Set('com net org edu gov mil int info biz name pro aero asia coop jobs mobi museum post tel travel app dev xyz online site website top club shop store tech blog news media art audio video cloud digital social chat live studio design games game play zone world today life space agency company business solutions services support help guide guru expert academy school center centre group team network systems software technology ventures capital finance money health care legal global email link page web click direct express gallery graphics photo photography photos pictures press pub review reviews science show software solar solutions stream studio style supply support surgery systems tax taxi team tennis theater theatre tickets tips tires today tools tours town toys trade trading training travel tube university vacations ventures video villas vision vodka vote voting voyage watch webcam wedding wiki wine works yoga zone'.split(' '));
const BARE = /(^|[\s(\[{;*¿¡«“])((?:[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?\.)+[A-Za-z]{2,24}(?::\d+)?(?:[/?#](?:[^\s<\u0001\u0002&*"'`]|&amp;)*)?)/g;
const CLOSE_OF = { ')': '(', ']': '[', '}': '{' };
/** End of a URL as written: sentence punctuation and an unbalanced closing bracket stay outside the link,
    a balanced one (ejemplo.com/wiki/Foo_(bar)) stays inside. */
function trimUrlEnd(url) {
  let u = String(url);
  while (u) {
    const last = u.charAt(u.length - 1), open = CLOSE_OF[last];
    if ('.,;:!?'.indexOf(last) >= 0) { u = /&amp;$/i.test(u) ? u.slice(0, -5) : u.slice(0, -1); continue; }
    if ('»”'.indexOf(last) >= 0) { u = u.slice(0, -1); continue; }
    if (!open) break;
    let opens = 0, closes = 0;
    for (const c of u) { if (c === open) opens++; else if (c === last) closes++; }
    if (closes <= opens) break;
    u = u.slice(0, -1);
  }
  return u;
}
function isBareDomain(url) {
  const host = /^[^/?#:]+/.exec(String(url || ''))?.[0] || '';
  if (!host || host.indexOf('@') >= 0) return false;
  const labels = host.split('.');
  if (labels.length < 2 || labels.some((l) => !/^[A-Za-z0-9](?:[A-Za-z0-9-]*[A-Za-z0-9])?$/.test(l))) return false;
  const tld = labels[labels.length - 1].toLowerCase();
  if (!/^[a-z]{2,24}$/.test(tld)) return false;
  if (labels[0].toLowerCase() === 'www' && labels.length >= 3) return true;
  if (FILE_EXT.has(tld)) return false;
  if (url.length > host.length) return true; /* a port, path, query or fragment follows the host */
  return tld.length === 2 || KNOWN_TLD.has(tld);
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
    if (/^(?:https?:\/\/|file:\/\/)/i.test(url)) return keep(`<a href="${url}" target="_blank" rel="noopener">${label}</a>`);
    const ref = refs ? splitRef(url) : null, h = (ref && !ref.rest && fileLink(ref, label)) || laneLink(url.replace(/^lane[:\/]/, ''), label);
    if (h) return keep(h);
    return isBareDomain(url) ? keep(`<a href="https://${url}" target="_blank" rel="noopener">${label}</a>`) : m;
  });
  s = s.replace(/(^|[\s(\[{;*¿¡«“])((?:https?|file):\/\/(?:[^\s<\u0001\u0002&*"'`]|&amp;)+)/gi, (m, pre, raw) => {
    const url = trimUrlEnd(raw);
    return !/[A-Za-z0-9]/.test(url.replace(/^(?:https?|file):\/\//i, '')) ? m :
      pre + keep(`<a href="${url}" target="_blank" rel="noopener">${url}</a>`) + raw.slice(url.length);
  });
  if (refs && refs.file) s = s.replace(PLAIN, (m, pre, tok) => {
    const ref = splitRef(tok), h = ref && fileLink(ref, tok.slice(0, tok.length - ref.rest.length));
    return h ? pre + keep(h) + ref.rest : m;
  });
  if (refs && refs.lane) s = s.replace(LANE, (m, pre, slash, id, at, all) => {
    const h = laneWord(all, at, m, pre, slash, id) && laneLink(id, (slash || '') + id);
    return h ? pre + keep(h) : m;
  });
  /* after code, markdown links, scheme URLs, file refs and lanes, so a bare domain steals none of them;
     the prefix it needs (start, space or an opening bracket) is never @, : or /, so emails and a scheme
     written without // (javascript:, data:, file:) never reach it */
  s = s.replace(BARE, (m, pre, raw) => {
    const url = trimUrlEnd(raw);
    return isBareDomain(url) ? pre + keep(`<a href="https://${url}" target="_blank" rel="noopener">${url}</a>`) + raw.slice(url.length) : m;
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
const LEAD = /^(CEO proposal:|CEO directive [^\s:]+:|CEO review:|Review:|Autopilot:)/;
export function boldLead(text) { return String(text == null ? '' : text).replace(LEAD, '**$1**'); }

/** The speaker of a message line, bold: "Manager · tirith" -> <b>Manager</b> · tirith (escaped). */
export function ctxHtml(ctx) {
  const s = String(ctx || ''), at = s.indexOf(' · ');
  return at < 0 ? '<b>' + esc(s) + '</b>' : '<b>' + esc(s.slice(0, at)) + '</b>' + esc(s.slice(at));
}
