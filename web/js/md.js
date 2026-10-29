/* Tirith web UI - small, safe markdown renderer for agent-written text. No libraries.
   All HTML in the source is escaped first; the only tags that come out are the ones built here, and
   links keep only http, https and file URLs. T.md(text) -> block HTML, T.mdInline(text) -> inline HTML,
   T.mdBox(text, key) -> block HTML clamped to ~8 lines with a fade and "Show all" (see main.js T.mdFit).
   With opts {file(path) -> bool, project}, paths the caller confirms (code spans, plain text, link targets,
   with :line[:col]) become file links (data-act="fopen", files.js opens them; docs/CONTRACT-files.md).
   With opts {lane(id) -> bool}, lane ids the caller knows become lane links (data-act="golane", lanes.js opens
   the lane): always in code spans, as lane/<id> or after "lane", bold, and bare when the id has a -, _ or digit. */
(function () {
  var T = (typeof window !== 'undefined' ? (window.T = window.T || {}) : {});
  var CLAMP_LINES = 8;

  function esc(s) {
    return String(s == null ? '' : s).replace(/[&<>"']/g, function (c) {
      return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c];
    });
  }

  /* ---------- file references (same rules as collector/tirith/filerefs.py) ---------- */
  var SHAPE = /^(?:~\/|\.{1,2}\/|\/)?[\w.@+-]+(?:\/[\w.@+-]+)*\/?$/;
  var NAME_EXT = /^(?:[\w@+-]*[A-Za-z][\w@+-]*(?:\.[\w-]+)*\.[A-Za-z][A-Za-z0-9]{0,9}|\.[A-Za-z][\w.-]*)$/;
  var PLAIN = /(^|[\s(\[*;])((?:~\/|\.{1,2}\/|\/)?[\w.@+-]+(?:\/[\w.@+-]+)*\/?(?::\d+){0,2}[.,:;!?]*)(?=$|[\s)\]*&])/g;
  var refs = null;   /* the opts of the md() call being rendered */
  function looksLikePath(p) {
    if (!p || p.indexOf('://') >= 0 || p.charAt(0) === '-' || !SHAPE.test(p) || !/[A-Za-z]/.test(p)) return false;
    return p.replace(/^\/+|\/+$/g, '').indexOf('/') >= 0 || /\/$/.test(p) || /^(~\/|\.\.?\/|\/)/.test(p) || NAME_EXT.test(p);
  }
  /* "cli.py:42:7." -> {path: "cli.py", line: 42, col: 7, rest: "."}; null when it is not a path */
  function splitRef(text) {
    var t = String(text).replace(/[.,:;!?]+$/, ''), m = /^(.+?)(?::(\d+)(?::(\d+))?)?$/.exec(t);
    if (!m || !looksLikePath(m[1])) return null;
    return { path: m[1], line: m[2] ? +m[2] : null, col: m[3] ? +m[3] : null, rest: String(text).slice(t.length) };
  }
  function normRef(p) { while (p.indexOf('./') === 0) p = p.slice(2); return p.replace(/\/+$/, '') || p; }
  /* the link for a confirmed path, or null */
  function fileLink(ref, inner) {
    if (!refs || !refs.file || !ref || !refs.file(normRef(ref.path))) return null;
    var at = ref.path + (ref.line ? ':' + ref.line + (ref.col ? ':' + ref.col : '') : '');
    return '<a class="fref" href="#" data-act="fopen" data-project="' + esc(refs.project || '') + '" data-path="' + esc(ref.path) + '"' +
      (ref.line ? ' data-line="' + ref.line + '"' : '') + (ref.col ? ' data-col="' + ref.col + '"' : '') + ' title="Open ' + esc(at) + '">' + inner + '</a>';
  }

  /* ---------- lane references ---------- */
  var LANE = /(^|[\s(\[*;])(lane\/)?([A-Za-z0-9][\w.-]*?)(?=[.,:;!?]*(?:$|[\s)\]*&]))/g;
  /* the link for a lane id the caller knows, or null (ids are slugs, so the escaped text is the id itself) */
  function laneLink(id, inner) {
    if (!refs || !refs.lane || !/^[\w.-]+$/.test(id || '') || !refs.lane(id)) return null;
    return '<a class="lref" href="#" data-act="golane" data-key="' + esc(id) + '" title="Open lane ' + esc(id) + '">' + inner + '</a>';
  }
  /* a bare word is a lane only when it cannot be an ordinary word: an id with - _ or a digit, lane/<id>,
     "lane <id>" or **<id>** */
  function laneWord(s, at, m, pre, slash, id) {
    return /[-_\d]/.test(id) || !!slash || /\blanes?\s$/i.test(s.slice(0, at + pre.length)) ||
      (pre === '*' && s.charAt(at + m.length) === '*');
  }

  /* ---------- inline ---------- */
  function inline(src) {
    var hold = [];
    function keep(html) { hold.push(html); return '\u0001' + (hold.length - 1) + '\u0002'; }
    var s = esc(String(src == null ? '' : src).replace(/[\u0001\u0002]/g, ''));
    s = s.replace(/(`+)([^`]|[^`][\s\S]*?[^`])\1(?!`)/g, function (m, t, code) {
      var c = '<code>' + code.trim() + '</code>', ref = refs && !/\s/.test(code.trim()) ? splitRef(code.trim()) : null;
      return keep((ref && !ref.rest && fileLink(ref, c)) || laneLink(code.trim(), c) || c);
    });
    s = s.replace(/\[([^\]]+)\]\(([^)\s]+)\)/g, function (m, label, url) {
      if (/^(https?:|file:)/i.test(url)) return keep('<a href="' + url + '" target="_blank" rel="noopener">' + label + '</a>');
      var ref = refs ? splitRef(url) : null, h = (ref && !ref.rest && fileLink(ref, label)) || laneLink(url.replace(/^lane[:\/]/, ''), label);
      return h ? keep(h) : m;
    });
    s = s.replace(/(^|[\s(])((?:https?|file):\/\/[^\s<]*[^\s<.,;:!?)\]'"&])/gi, function (m, pre, url) {
      return pre + keep('<a href="' + url + '" target="_blank" rel="noopener">' + url + '</a>');
    });
    if (refs && refs.file) s = s.replace(PLAIN, function (m, pre, tok) {
      var ref = splitRef(tok), h = ref && fileLink(ref, tok.slice(0, tok.length - ref.rest.length));
      return h ? pre + keep(h) + ref.rest : m;
    });
    if (refs && refs.lane) s = s.replace(LANE, function (m, pre, slash, id, at, all) {
      var h = laneWord(all, at, m, pre, slash, id) && laneLink(id, (slash || '') + id);
      return h ? pre + keep(h) : m;
    });
    s = s.replace(/\*\*(?=\S)([\s\S]*?\S)\*\*/g, '<strong>$1</strong>')
      .replace(/(^|[^\w])__(?=\S)([\s\S]*?\S)__(?!\w)/g, '$1<strong>$2</strong>')
      .replace(/~~(?=\S)([\s\S]*?\S)~~/g, '<del>$1</del>')
      .replace(/(^|[^*\w])\*(?=[^\s*])([^*]*?[^\s*])\*(?![*\w])/g, '$1<em>$2</em>')
      .replace(/(^|[^_\w])_(?=[^\s_])([^_]*?[^\s_])_(?![_\w])/g, '$1<em>$2</em>');
    /* a placeholder can sit inside another one's label (a code span in a link), so unwrap until none is left */
    for (var i = 0; i < 4 && /\u0001\d+\u0002/.test(s); i++) {
      s = s.replace(/\u0001(\d+)\u0002/g, function (m, n) { return hold[+n] != null ? hold[+n] : ''; });
    }
    return s;
  }

  /* ---------- blocks ---------- */
  var RE = {
    fence: /^\s*(```+|~~~+)\s*([\w+-]*)\s*$/,
    heading: /^\s{0,3}(#{1,6})\s+(.*?)\s*#*\s*$/,
    hr: /^\s{0,3}([-*_])(?:\s*\1){2,}\s*$/,
    quote: /^\s{0,3}>\s?(.*)$/,
    item: /^(\s*)([-*+]|\d+[.)])\s+(.*)$/,
    tableSep: /^\s*\|?\s*:?-+:?\s*(\|\s*:?-+:?\s*)*\|?\s*$/
  };
  function cells(line) {
    var t = line.trim().replace(/^\|/, '').replace(/\|$/, '');
    return t.split(/(?<!\\)\|/).map(function (c) { return c.trim().replace(/\\\|/g, '|'); });
  }
  function startsBlock(line, next) {
    return RE.fence.test(line) || RE.heading.test(line) || RE.hr.test(line) || RE.quote.test(line) || RE.item.test(line) ||
      (line.indexOf('|') >= 0 && next != null && line.trim() !== '' && RE.tableSep.test(next) && next.indexOf('-') >= 0 && next.indexOf('|') >= 0);
  }

  function list(lines, i) {
    /* items carry their indent; deeper indent nests, a shallower one closes levels */
    var stack = [], html = '', first = true;
    function open(ordered, indent) { stack.push({ indent: indent, tag: ordered ? 'ol' : 'ul', open: false }); html += '<' + (ordered ? 'ol' : 'ul') + '>'; }
    function closeItem() { var top = stack[stack.length - 1]; if (top && top.open) { html += '</li>'; top.open = false; } }
    function closeList() { closeItem(); html += '</' + stack.pop().tag + '>'; }
    for (; i < lines.length; i++) {
      var m = RE.item.exec(lines[i]);
      if (!m) {
        /* an indented continuation line belongs to the open item */
        if (stack.length && lines[i].trim() !== '' && /^\s{2,}\S/.test(lines[i]) && !startsBlock(lines[i].trim(), null)) { html += '<br>' + inline(lines[i].trim()); continue; }
        break;
      }
      var indent = m[1].replace(/\t/g, '    ').length, ordered = /\d/.test(m[2]);
      while (stack.length && indent < stack[stack.length - 1].indent - 1) closeList();
      var top = stack[stack.length - 1];
      if (!top || indent > top.indent + 1) open(ordered, indent);
      else closeItem();
      top = stack[stack.length - 1];
      var text = m[3], task = /^\[([ xX])\]\s+(.*)$/.exec(text), box = '';
      if (task) { box = '<span class="mdchk' + (task[1] === ' ' ? '' : ' on') + '" aria-hidden="true"></span>'; text = task[2]; }
      html += '<li' + (task ? ' class="mdtask' + (task[1] === ' ' ? '' : ' done') + '"' : '') + '>' + box + inline(text);
      top.open = true;
    }
    while (stack.length) closeList();
    return { html: html, next: i };
  }

  function blocks(src) {
    var lines = String(src == null ? '' : src).replace(/\r\n?/g, '\n').split('\n'), out = '', i = 0;
    while (i < lines.length) {
      var line = lines[i], m;
      if (line.trim() === '') { i++; continue; }
      if ((m = RE.fence.exec(line))) {
        var buf = [], fence = m[1].charAt(0), n = m[1].length;
        for (i++; i < lines.length && !(new RegExp('^\\s*' + (fence === '`' ? '`' : '~') + '{' + n + ',}\\s*$').test(lines[i])); i++) buf.push(lines[i]);
        i++;
        out += '<pre class="mdpre"><code>' + esc(buf.join('\n')) + '</code></pre>';
        continue;
      }
      if ((m = RE.heading.exec(line))) {
        var lv = Math.min(m[1].length, 4);
        out += '<div class="mdh mdh' + lv + '" role="heading" aria-level="' + (lv + 2) + '">' + inline(m[2]) + '</div>';
        i++; continue;
      }
      if (RE.hr.test(line)) { out += '<hr>'; i++; continue; }
      if (RE.quote.test(line)) {
        var q = [];
        while (i < lines.length && RE.quote.test(lines[i])) { q.push(RE.quote.exec(lines[i])[1]); i++; }
        out += '<blockquote>' + blocks(q.join('\n')) + '</blockquote>';
        continue;
      }
      if (line.indexOf('|') >= 0 && i + 1 < lines.length && lines[i + 1].indexOf('|') >= 0 && lines[i + 1].indexOf('-') >= 0 && RE.tableSep.test(lines[i + 1])) {
        var head = cells(line), al = cells(lines[i + 1]).map(function (c) { return /^:-*:$/.test(c) ? 'center' : /-:$/.test(c) ? 'right' : ''; });
        var th = head.map(function (c, k) { return '<th' + (al[k] ? ' style="text-align:' + al[k] + '"' : '') + '>' + inline(c) + '</th>'; }).join('');
        var rows = '';
        for (i += 2; i < lines.length && lines[i].trim() !== '' && lines[i].indexOf('|') >= 0; i++) {
          rows += '<tr>' + cells(lines[i]).slice(0, head.length).map(function (c, k) { return '<td' + (al[k] ? ' style="text-align:' + al[k] + '"' : '') + '>' + inline(c) + '</td>'; }).join('') + '</tr>';
        }
        out += '<div class="mdtbl"><table><thead><tr>' + th + '</tr></thead><tbody>' + rows + '</tbody></table></div>';
        continue;
      }
      if (RE.item.test(line)) { var l = list(lines, i); out += l.html; i = l.next; continue; }
      var para = [];
      while (i < lines.length && lines[i].trim() !== '' && (para.length === 0 || !startsBlock(lines[i], lines[i + 1]))) { para.push(lines[i].trim()); i++; }
      out += '<p>' + para.map(inline).join('<br>') + '</p>';
    }
    return out;
  }

  function withRefs(opts, fn) {
    var was = refs; refs = opts && (typeof opts.file === 'function' || typeof opts.lane === 'function') ? opts : null;
    try { return fn(); } finally { refs = was; }
  }
  var md = function (text, opts) { var h = withRefs(opts, function () { return blocks(text); }); return h ? '<div class="md">' + h + '</div>' : ''; };
  var mdInline = function (text, opts) {
    return '<span class="md mdi">' + withRefs(opts, function () { return inline(String(text == null ? '' : text).replace(/\s*\n\s*/g, ' ')); }) + '</span>';
  };

  /* Rendered first, clamped with CSS after: nothing is cut inside a markdown construct. The "Show all"
     button stays hidden until T.mdFit finds the content taller than the clamp. */
  var open = {};
  var mdBox = function (text, key, opts) {
    var html = md(text, opts);
    if (!html) return '';
    var k = String(key || ''), on = !!open[k];
    return '<div class="mdc' + (on ? ' open' : '') + '" data-mdk="' + esc(k) + '" style="--mdl:' + CLAMP_LINES + '">' + html +
      '<button class="link mdmore" data-act="mdopen" data-key="' + esc(k) + '" aria-expanded="' + on + '">' + (on ? 'Show less' : 'Show all') + '</button></div>';
  };
  /* mark the boxes whose content is taller than the clamp so their button shows */
  var mdFit = function (root) {
    [].forEach.call((root || document).querySelectorAll('.mdc'), function (b) {
      var body = b.firstElementChild;
      b.classList.toggle('over', !b.classList.contains('open') ? body.scrollHeight > body.clientHeight + 2 : true);
    });
  };
  var mdToggle = function (key) { open[key] = !open[key]; };

  T.md = md; T.mdInline = mdInline; T.mdBox = mdBox; T.mdFit = mdFit; T.mdToggle = mdToggle;
  if (typeof module !== 'undefined' && module.exports) module.exports = { md: md, mdInline: mdInline, mdBox: mdBox, splitRef: splitRef };
})();
