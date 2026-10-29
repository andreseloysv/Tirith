/* Agent log: structured entries, newest first, one look per kind. Shared by the Agents drawer
   (T.logView). The collector classifies and parses; this only renders. */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  ui.logFilter = ui.logFilter || 'all';
  ui.logSys = ui.logSys || {};
  ui.logOpen = ui.logOpen || {};
  var SHOW_LINES = 3, VALUE_MAX = 120;
  var FILTERS = [['all', 'All'], ['command', 'Commands'], ['edit', 'Edits'], ['error', 'Errors'], ['test', 'Tests']];
  var KINDS = { command: ['command', 'test', 'output'], edit: ['edit'], error: ['error'], test: ['test'] };

  function ic(d, size) {
    return '<svg class="lic" width="' + size + '" height="' + size + '" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' + d + '</svg>';
  }
  var ICON = {
    file: ic('<path d="M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8z"/><path d="M14 3v5h5"/>', 14),
    flask: ic('<path d="M9 3h6M10 3v6L4.5 19a1.6 1.6 0 0 0 1.4 2.4h12.2a1.6 1.6 0 0 0 1.4-2.4L14 9V3"/><path d="M7.5 15h9"/>', 14),
    branch: ic('<circle cx="6" cy="5" r="2"/><circle cx="6" cy="19" r="2"/><circle cx="18" cy="9" r="2"/><path d="M6 7v10M18 11c0 4-6 3-12 6"/>', 14),
    flag: ic('<path d="M5 21V4M5 4h11l-2 4 2 4H5"/>', 14)
  };

  /* ---------- values and keys ---------- */
  function humanKey(k) {
    var s = String(k).replace(/([a-z0-9])([A-Z])/g, '$1 $2').replace(/[_\-.]+/g, ' ').trim().toLowerCase();
    return s.charAt(0).toUpperCase() + s.slice(1);
  }
  function bytes(n) { return n >= 1048576 ? (n / 1048576).toFixed(1) + ' MB' : n >= 1024 ? Math.round(n / 1024) + ' KB' : n + ' B'; }
  function num(key, n) {
    var k = String(key).toLowerCase();
    if (/(^|_)(ms|millis|milliseconds)$|_ms$/.test(k) || /duration_ms/.test(k)) return T.dur(n / 1000);
    if (/(^|_)(s|sec|secs|seconds)$|duration(_s)?$|elapsed(_s)?$/.test(k)) return T.dur(n);
    if (/bytes?$|size$/.test(k) && n >= 1024) return bytes(n);
    if (/tokens?$/.test(k)) return (n >= 1000 ? (n / 1000).toFixed(1) + 'k' : n) + ' tokens';
    if (/(cost|usd|price)/.test(k)) return '$' + Number(n).toFixed(2);
    return Number(n).toLocaleString('en-US', { maximumFractionDigits: 3 });
  }
  function value(key, v, id, depth) {
    if (v === null || v === undefined) return '<span class="mut">—</span>';
    if (v === true) return '<span class="lok" title="true">✓</span>';
    if (v === false) return '<span class="lno" title="false">✗</span>';
    if (typeof v === 'number') return esc(num(key, v));
    if (typeof v === 'string') {
      if (/^\d{4}-\d\d-\d\d[T ]\d\d:\d\d/.test(v) && T.ms(v) != null) return T.agoSpan(v) + ' <span class="mut sm">ago</span>';
      return '<span title="' + esc(v) + '">' + esc(v.length > VALUE_MAX ? v.slice(0, VALUE_MAX - 1) + '…' : v) + '</span>';
    }
    var arr = Array.isArray(v), n = arr ? v.length : Object.keys(v).length, k = id + '/' + key, open = !!ui.logOpen[k];
    var label = arr ? n + (n === 1 ? ' item' : ' items') : n + (n === 1 ? ' field' : ' fields');
    if (!n) return '<span class="mut">—</span>';
    return '<button class="link lnest" data-act="logopen" data-key="' + esc(k) + '" aria-expanded="' + open + '">' + label + ' ' + (open ? '▾' : '▸') + '</button>' +
      (open && depth < 3 ? card(arr ? v.reduce(function (o, x, i) { o[i + 1] = x; return o; }, {}) : v, k, depth + 1) : '');
  }
  function card(fields, id, depth) {
    var rows = Object.keys(fields).map(function (k) {
      return '<div class="lkv"><span class="lk">' + esc(humanKey(k)) + '</span><span class="lv">' + value(k, fields[k], id, depth) + '</span></div>';
    }).join('');
    return '<div class="lcard' + (depth ? ' nested' : '') + '">' + rows + '</div>';
  }

  /* ---------- one entry ---------- */
  function time(e) {
    if (!e.ts || T.ms(e.ts) == null) return '<span class="lt"></span>';
    return '<span class="lt ago" data-at="' + esc(e.ts) + '" title="' + esc(new Date(T.ms(e.ts)).toLocaleString()) + '">' + esc(T.ago(e.ts)) + '</span>';
  }
  function more(e, key) {
    var open = !!ui.logOpen[key], lines = String(e.text || '').split('\n').length;
    if (!e.detail && lines <= SHOW_LINES) return '';
    return '<button class="link lmore" data-act="logopen" data-key="' + esc(key) + '" aria-expanded="' + open + '">' + (open ? 'show less' : 'show more') + '</button>';
  }
  function counts(detail) {
    var m = /^\+(\d+)\s+[−-](\d+)$/.exec(detail || '');
    return m ? '<span class="lcnt"><span class="plus">+' + m[1] + '</span> <span class="minus">−' + m[2] + '</span></span>'
      : detail ? '<span class="lcnt mut">' + esc(detail) + '</span>' : '';
  }
  function body(e, key, a) {
    var open = !!ui.logOpen[key], full = e.detail && e.kind !== 'test' && e.kind !== 'edit' ? e.text + '\n' + e.detail : e.text;
    switch (e.kind) {
      case 'command':
        return '<span class="lprompt">$</span><code class="lcmd" title="' + esc(e.text) + '">' + esc(e.text) + '</code>';
      case 'output':
        return '<pre class="lout' + (open || !more(e, key) ? ' open' : '') + '">' + esc(open ? full : e.text) + '</pre>' + more(e, key);
      case 'edit':
        var i = e.text.lastIndexOf('/');
        return ICON.file + '<span class="lpath" title="' + esc(e.path || e.text) + '">' + (i >= 0 ? '<span class="mut">' + esc(e.text.slice(0, i + 1)) + '</span>' : '') + '<b>' + esc(e.text.slice(i + 1)) + '</b></span>' + counts(e.detail);
      case 'read':
        return '<span class="lread" title="' + esc(e.path || e.text) + '">' + esc(e.text) + '</span>';
      case 'error':
        return '<div class="lerr"><span class="lerrt">' + esc(open ? full : e.text) + '</span>' + more(e, key) + '</div>';
      case 'test':
        var f = e.fields || {}, res = e.detail === 'running' || !e.detail ? 'running' : e.detail,
          sum = f.passed != null ? f.passed + ' passed, ' + (f.failed || 0) + ' failed' : '';
        return ICON.flask + '<code class="lcmd" title="' + esc(e.text) + '">' + esc(e.text) + '</code>' +
          (sum ? '<span class="mut lsum">' + esc(sum) + '</span>' : '') + '<span class="rb ' + esc(res) + '">' + esc(res) + '</span>';
      case 'spawn':
        return ICON.branch + '<span class="lspawn"><span class="mut">Started sub-agent</span> <b>' + esc(e.text) + '</b></span>' + (e.detail ? '<span class="chip">' + esc(e.detail) + '</span>' : '');
      case 'done':
        return '<div class="ldone"><div class="ldh">' + ICON.flag + 'Final answer' + (e.fields && e.fields.usage ? '<span class="mut lusage">' + esc(e.fields.usage) + '</span>' : '') + '</div>' +
          '<div class="ldt">' + T.mdBox(e.detail ? e.text + '\n' + e.detail : e.text, 'log:' + key) + '</div></div>';
      case 'system':
        return '<span class="lsys" title="' + esc(e.detail || '') + '">' + esc(e.text) + '</span>';
      default:   /* say */
        if (e.fields) {
          var raw = !!ui.logOpen[key + '#raw'];
          return '<div class="lsay">' + (e.text ? T.mdBox(e.text, 'log:' + key) : '') + card(e.fields, key, 0) +
            (e.detail ? '<button class="link lraw" data-act="logopen" data-key="' + esc(key + '#raw') + '" aria-expanded="' + raw + '">raw</button>' : '') +
            (raw ? '<pre class="lout open">' + esc(e.detail) + '</pre>' : '') + '</div>';
        }
        return '<div class="lsay">' + T.mdBox(e.detail ? e.text + '\n' + e.detail : e.text, 'log:' + key) + '</div>';
    }
  }
  function keyOf(a, e, n) {
    var s = e.kind + '|' + (e.ts || '') + '|' + String(e.text).slice(0, 60), h = 0, i;
    for (i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) | 0;
    return a.id + ':' + (h >>> 0).toString(36) + (n ? '.' + n : '');
  }

  /* ---------- the list ---------- */
  T.logView = function (a) {
    var all = a.log_entries;
    if (!all) return T.logBox(a.log_tail);          /* state from an older collector */
    var f = ui.logFilter, sysOn = !!ui.logSys[a.id], seen = {};
    var hidden = all.filter(function (e) { return e.kind === 'system'; }).length;
    var shown = all.filter(function (e) {
      if ((e.kind === 'done' || e.kind === 'say') && a.report && T.sameText(e.text, a.report)) return false;   /* the Status section shows it */
      if (e.kind === 'system') return sysOn && f === 'all';
      return f === 'all' || (KINDS[f] || []).indexOf(e.kind) >= 0;
    });
    var chips = '<div class="lchips" role="group" aria-label="Filter log">' + FILTERS.map(function (c) {
      var n = c[0] === 'all' ? all.length - hidden : all.filter(function (e) { return (KINDS[c[0]] || []).indexOf(e.kind) >= 0; }).length;
      return '<button class="lchip' + (f === c[0] ? ' on' : '') + '" data-act="logf" data-key="' + c[0] + '" aria-pressed="' + (f === c[0]) + '">' + c[1] + (c[0] !== 'all' && n ? ' <b>' + n + '</b>' : '') + '</button>';
    }).join('') + '</div>';
    var list = shown.map(function (e) {
      var key = keyOf(a, e, 0), dup = seen[key] = (seen[key] || 0) + 1;
      if (dup > 1) key = keyOf(a, e, dup);
      return '<li class="le ' + esc(e.kind) + (e.kind === 'test' ? ' ' + esc(e.detail || '') : '') + '">' + time(e) + '<div class="lb">' + body(e, key, a) + '</div></li>';
    }).join('');
    var sys = hidden && f === 'all' ? '<button class="link lsyst" data-act="logsys" data-key="' + esc(a.id) + '">' +
      (sysOn ? 'Hide ' : '') + hidden + ' system event' + (hidden === 1 ? '' : 's') + (sysOn ? '' : ' hidden') + '</button>' : '';
    var empty = !shown.length ? T.empty(all.length ? 'Nothing matches this filter' : 'No log output yet') : '';
    return '<div class="logv">' + chips + '<ul class="llist" data-scroll="log-' + esc(a.id) + '" data-anchor="1" tabindex="0">' + list + '</ul>' + empty + sys + '</div>';
  };
})();
