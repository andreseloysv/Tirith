/* Tirith web UI - shared helpers. Classic script, no modules (works from file://). */
(function () {
  var T = window.T = window.T || {};

  T.CLS = {
    stonewright: { label: 'Stonewright', color: '#e07a2f', glyph: 'M4 4h12v6H4z M9 10h2.4v10H9z' },
    loremaster:  { label: 'Loremaster',  color: '#8a55d6', glyph: 'M3.5 5h7.5v14H3.5z M13 5h7.5v14H13z' },
    warden:      { label: 'Warden',      color: '#3a9aa8', glyph: 'M12 2.5l7.5 3v6.2c0 4.3-3.2 7.4-7.5 9.3-4.3-1.9-7.5-5-7.5-9.3V5.5z' },
    swiftrider:  { label: 'Swiftrider',  color: '#d9b814', glyph: 'M13.5 2L5 13.2h5.8L9.5 22l9-12H12.6z' },
    herald:      { label: 'Herald',      color: '#c62f3a', glyph: 'M5 3h2.2v18H5z M7.2 4h12.3l-3.4 4.2 3.4 4.2H7.2z' },
    glasswright: { label: 'Glasswright', color: '#5fb8d6', glyph: 'M12 2.5l7.5 7.5-7.5 11.5L4.5 10z' },
    archivist:   { label: 'Archivist',   color: '#6f9a3a', glyph: 'M8 3.5a4.5 4.5 0 100 9 4.5 4.5 0 000-9z M11 11l9.5 9.5-2.2 2-2-2-2 2-2.2-2.2-1.6-1.6z' },
    steward:     { label: 'Steward',     color: '#2f5fc4', glyph: 'M12 2l2.6 6.4 6.9.6-5.2 4.6 1.6 6.8L12 16.7 6.1 20.4l1.6-6.8L2.5 9l6.9-.6z' }
  };
  T.RANKS = { recruit: 'Recruit', sentinel: 'Sentinel', sergeant: 'Sergeant', captain: 'Captain', marshal: 'Marshal' };
  T.STATUS_COLOR = { working: '#4c8dff', running: '#4c8dff', done: '#3fb950', passed: '#3fb950', failed: '#f2564f',
                     stalled: '#8b8f98', pending: '#6b6f78', skipped: '#4b4f57', canceled: '#8b8f98' };
  T.KIND = {
    brief:    { color: '#f2cc60', label: 'brief' },
    message:  { color: '#58a6ff', label: 'message' },
    report:   { color: '#3fb950', label: 'report' },
    request:  { color: '#ff9f1a', label: 'request' },
    alert:    { color: '#ff9f1a', label: 'alert' },
    answer:   { color: '#ffffff', label: 'answer' },
    decision: { color: '#b392f0', label: 'decision' },
    merge:    { color: '#39d0c4', label: 'merge' }
  };

  T.esc = function (s) {
    return String(s == null ? '' : s).replace(/[&<>"']/g, function (c) {
      return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c];
    });
  };
  T.cap = function (s) { s = String(s || ''); return s.charAt(0).toUpperCase() + s.slice(1); };
  T.clsOf = function (c) { return T.CLS[c] || { label: T.cap(c || 'hero'), color: '#9aa0aa', glyph: 'M12 3a5 5 0 100 10 5 5 0 000-10z M4 21c0-5 3.5-7 8-7s8 2 8 7z' }; };
  T.flowColor = function (f) {
    if (f.kind === 'report' && /fail|error|broke/i.test(f.text || '')) return '#f2564f';
    return (T.KIND[f.kind] || { color: '#c9c9c9' }).color;
  };

  /* Clock: real time, or (for stale demo data) generated_at + time since the page saw it. */
  T.clock = { offset: 0, demo: false };
  T.now = function () { return Date.now() + T.clock.offset; };
  T.ms = function (iso) { var t = Date.parse(iso || ''); return isNaN(t) ? null : t; };

  T.dur = function (sec) {
    if (sec == null || isNaN(sec)) return '--';
    sec = Math.max(0, Math.floor(sec));
    var h = Math.floor(sec / 3600), m = Math.floor((sec % 3600) / 60), s = sec % 60;
    var p2 = function (n) { return (n < 10 ? '0' : '') + n; };
    if (h > 0) return h + 'h ' + p2(m) + 'm ' + p2(s) + 's';
    if (m > 0) return m + 'm ' + p2(s) + 's';
    return s + 's';
  };
  T.durShort = function (sec) { /* no seconds above one hour */
    if (sec == null) return '--';
    sec = Math.max(0, Math.floor(sec));
    if (sec >= 3600) return Math.floor(sec / 3600) + 'h ' + ((Math.floor(sec / 60) % 60 < 10 ? '0' : '') + Math.floor(sec / 60) % 60) + 'm';
    return T.dur(sec);
  };
  T.ago = function (iso) {
    var t = T.ms(iso); if (t == null) return '';
    var s = Math.floor((T.now() - t) / 1000);
    if (s < 10) return 'just now';
    if (s < 60) return s + ' s ago';
    if (s < 3600) return Math.floor(s / 60) + ' min ago';
    if (s < 86400) { var h = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60); return h + ' h' + (h < 6 && m ? ' ' + m + ' min' : '') + ' ago'; }
    return Math.floor(s / 86400) + ' d ago';
  };
  T.clockTime = function (iso) {
    var t = T.ms(iso); if (t == null) return '';
    var d = new Date(t), p = function (n) { return (n < 10 ? '0' : '') + n; };
    return p(d.getHours()) + ':' + p(d.getMinutes());
  };
  /* seconds of a running/finished thing */
  T.elapsed = function (startIso, endIso) {
    var a = T.ms(startIso); if (a == null) return null;
    var b = endIso ? T.ms(endIso) : T.now();
    return Math.max(0, Math.round((b - a) / 1000));
  };
  /* live span: ticks every second between refreshes */
  T.tickSpan = function (startIso, endIso, cls, fixedSec) {
    if (endIso || startIso == null) {
      var v = fixedSec != null ? fixedSec : T.elapsed(startIso, endIso);
      return '<span class="' + (cls || '') + '">' + T.dur(v) + '</span>';
    }
    return '<span class="tick ' + (cls || '') + '" data-start="' + T.esc(startIso) + '">' + T.dur(T.elapsed(startIso)) + '</span>';
  };
  T.agoSpan = function (iso) { return '<span class="agoel" data-at="' + T.esc(iso || '') + '">' + T.esc(T.ago(iso)) + '</span>'; };

  /* inline svg icons */
  T.glyphSvg = function (cls, size, color) {
    var c = T.clsOf(cls);
    return '<svg class="gly" width="' + size + '" height="' + size + '" viewBox="0 0 24 24"><path d="' + c.glyph + '" fill="' + (color || '#fff') + '" fill-rule="evenodd"/></svg>';
  };
  T.badge = function (cls, size) { /* hex class badge */
    var c = T.clsOf(cls), s = size || 40;
    return '<svg class="hexbadge" width="' + s + '" height="' + s + '" viewBox="0 0 40 40"><polygon points="20,2 36,11 36,29 20,38 4,29 4,11" fill="' + c.color + '" stroke="rgba(255,255,255,.55)" stroke-width="1.5"/>' +
      '<g transform="translate(10 10) scale(.83)"><path d="' + c.glyph + '" fill="#fff" fill-rule="evenodd" opacity=".95"/></g></svg>';
  };
  T.icons = {
    owner: '<path d="M12 3a4.5 4.5 0 100 9 4.5 4.5 0 000-9z M3.5 21c0-5.2 3.7-7.5 8.5-7.5s8.5 2.3 8.5 7.5z"/>',
    guard: '<path d="M12 2.5l7.5 3v6.2c0 4.3-3.2 7.4-7.5 9.3-4.3-1.9-7.5-5-7.5-9.3V5.5z"/>',
    room:  '<path d="M4 4h16v12H10l-5 4v-4H4z"/>',
    pipeline: '<path d="M3 6l6 6-6 6z M10 6l6 6-6 6z M17 6l4 6-4 6z"/>',
    steward: T.CLS.steward.glyph,
    check: '<path d="M5 12.5l4.5 4.5L19 7.5" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>',
    cross: '<path d="M6 6l12 12M18 6L6 18" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"/>',
    pause: '<path d="M8 5v14M16 5v14" fill="none" stroke="currentColor" stroke-width="3.2" stroke-linecap="round"/>'
  };
  T.ico = function (name, size, color) {
    return '<svg class="ico" width="' + size + '" height="' + size + '" viewBox="0 0 24 24" style="color:' + (color || 'currentColor') + '" fill="' + (color || 'currentColor') + '">' + T.icons[name] + '</svg>';
  };
  T.statusIcon = function (st, size) {
    size = size || 22;
    var col = T.STATUS_COLOR[st] || '#8b8f98', inner;
    if (st === 'running' || st === 'working') {
      return '<svg class="sicon spin" width="' + size + '" height="' + size + '" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9" fill="none" stroke="' + col + '" stroke-opacity=".25" stroke-width="3"/><path d="M12 3a9 9 0 019 9" fill="none" stroke="' + col + '" stroke-width="3" stroke-linecap="round"/></svg>';
    }
    if (st === 'passed' || st === 'done') inner = T.icons.check;
    else if (st === 'failed') inner = T.icons.cross;
    else if (st === 'stalled') inner = T.icons.pause;
    else inner = '<circle cx="12" cy="12" r="2.5" fill="currentColor"/>';
    return '<svg class="sicon" width="' + size + '" height="' + size + '" viewBox="0 0 24 24" style="color:' + col + '"><circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2" ' + (st === 'pending' || st === 'skipped' ? 'stroke-dasharray="3 3"' : '') + '/><g transform="translate(5.5 5.5) scale(.54)">' + inner + '</g></svg>';
  };
  T.projectChip = function (state, pid) {
    var p = T.projectById(state, pid);
    return '<span class="chip proj" style="--pc:' + p.color + '"><i></i>' + T.esc(p.name) + '</span>';
  };
  T.projectById = function (state, pid) {
    var ps = (state && state.projects) || [];
    for (var i = 0; i < ps.length; i++) if (ps[i].id === pid) return ps[i];
    return { id: pid, name: pid || 'other', color: '#8b8f98' };
  };
  T.agentById = function (state, id) {
    var as = (state && state.agents) || [];
    for (var i = 0; i < as.length; i++) if (as[i].id === id) return as[i];
    return null;
  };
  T.agentFull = function (a) { return a.name + (a.epithet ? ' ' + a.epithet : ''); };
  T.agentElapsed = function (a) {
    if (a.status === 'working' || a.status === 'stalled') return T.elapsed(a.started_at, a.ended_at);
    if (a.elapsed_s != null && !a.ended_at) return a.elapsed_s;
    return a.ended_at ? T.elapsed(a.started_at, a.ended_at) : a.elapsed_s;
  };
  /* name of an endpoint like "agent:w1" / "steward" */
  T.endpointName = function (state, ep, f, side) {
    var given = f && f[side + '_name'];
    if (given) return given;
    if (ep === 'steward') return 'The Steward';
    if (ep === 'owner') return 'You';
    if (ep === 'guard') return 'Guard';
    var i = ep.indexOf(':'), kind = ep.slice(0, i), id = ep.slice(i + 1);
    if (kind === 'agent') { var a = T.agentById(state, id); return a ? T.agentFull(a) : id; }
    return id;
  };
  T.endpointIcon = function (state, ep, size) {
    size = size || 18;
    if (ep === 'steward') return '<span class="eic" style="background:' + T.CLS.steward.color + '">' + T.glyphSvg('steward', size - 6) + '</span>';
    if (ep === 'owner') return '<span class="eic" style="background:#c9a24a">' + T.ico('owner', size - 6, '#1a1408') + '</span>';
    if (ep === 'guard') return '<span class="eic" style="background:#8b8f98">' + T.ico('guard', size - 6, '#111') + '</span>';
    var i = ep.indexOf(':'), kind = ep.slice(0, i), id = ep.slice(i + 1);
    if (kind === 'agent') { var a = T.agentById(state, id); var c = T.clsOf(a && a['class']); return '<span class="eic" style="background:' + c.color + '">' + T.glyphSvg(a && a['class'], size - 6) + '</span>'; }
    if (kind === 'room') return '<span class="eic" style="background:#3b6ea5">' + T.ico('room', size - 6, '#fff') + '</span>';
    return '<span class="eic" style="background:#5a5f6a">' + T.ico('pipeline', size - 6, '#fff') + '</span>';
  };
  T.latestPipeline = function (state, pid) {
    var best = null;
    (state.pipelines || []).forEach(function (p) {
      if (p.project !== pid) return;
      if (!best || (T.ms(p.started_at) || 0) > (T.ms(best.started_at) || 0)) best = p;
    });
    return best;
  };
})();
