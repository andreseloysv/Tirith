/* Tirith web UI - helpers. Classic script, no modules (works from file://). */
(function () {
  var T = window.T = window.T || {};

  T.esc = function (s) {
    return String(s == null ? '' : s).replace(/[&<>"']/g, function (c) {
      return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c];
    });
  };
  T.cap = function (s) { s = String(s || ''); return s.charAt(0).toUpperCase() + s.slice(1); };

  /* Clock: real time, or (for stale demo data) generated_at + time since the page saw it. */
  T.clock = { offset: 0, demo: false };
  T.now = function () { return Date.now() + T.clock.offset; };
  T.ms = function (iso) { var t = Date.parse(iso || ''); return isNaN(t) ? null : t; };

  T.dur = function (sec) {
    if (sec == null || isNaN(sec)) return '';
    sec = Math.max(0, Math.floor(sec));
    var h = Math.floor(sec / 3600), m = Math.floor((sec % 3600) / 60), s = sec % 60;
    var p2 = function (n) { return (n < 10 ? '0' : '') + n; };
    if (h > 0) return h + 'h ' + p2(m) + 'm';
    if (m > 0) return m + 'm ' + p2(s) + 's';
    return s + 's';
  };
  T.ago = function (iso) {
    var t = T.ms(iso); if (t == null) return '';
    var s = Math.floor((T.now() - t) / 1000);
    if (s < 10) return 'now';
    if (s < 60) return s + ' s';
    if (s < 3600) return Math.floor(s / 60) + ' min';
    if (s < 86400) return Math.floor(s / 3600) + ' h';
    return Math.floor(s / 86400) + ' d';
  };
  T.clockTime = function (iso) {
    var t = T.ms(iso); if (t == null) return '';
    var d = new Date(t), n = new Date(T.now()), p = function (x) { return (x < 10 ? '0' : '') + x; };
    var hm = p(d.getHours()) + ':' + p(d.getMinutes());
    if (d.toDateString() === n.toDateString()) return hm;
    return ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'][d.getMonth()] + ' ' + d.getDate() + ' ' + hm;
  };
  T.elapsed = function (startIso, endIso) {
    var a = T.ms(startIso); if (a == null) return null;
    var b = endIso ? T.ms(endIso) : T.now();
    return Math.max(0, Math.round((b - a) / 1000));
  };
  /* A duration that ticks each second while nothing has ended. */
  T.span = function (startIso, endIso, fixedSec) {
    if (endIso || startIso == null) {
      var v = fixedSec != null ? fixedSec : T.elapsed(startIso, endIso);
      return '<span>' + T.esc(T.dur(v)) + '</span>';
    }
    return '<span class="tick" data-start="' + T.esc(startIso) + '">' + T.esc(T.dur(T.elapsed(startIso))) + '</span>';
  };
  T.agoSpan = function (iso) { return '<span class="ago" data-at="' + T.esc(iso || '') + '">' + T.esc(T.ago(iso)) + '</span>'; };

  T.projectName = function (state, pid) {
    var ps = (state && state.projects) || [];
    for (var i = 0; i < ps.length; i++) if (ps[i].id === pid) return ps[i].name;
    return pid || '';
  };
  T.agentById = function (state, id) {
    var as = (state && state.agents) || [];
    for (var i = 0; i < as.length; i++) if (as[i].id === id) return as[i];
    return null;
  };
  /* display name of a flow endpoint ("agent:w1", "steward", "room:x") */
  T.endpointName = function (state, ep, f, side) {
    var given = f && f[side + '_name'];
    if (given) return given;
    if (ep === 'steward') return 'The Steward';
    if (ep === 'owner') return 'You';
    if (ep === 'guard') return 'Guard';
    var i = ep.indexOf(':'), kind = ep.slice(0, i), id = ep.slice(i + 1);
    if (kind === 'agent') { var a = T.agentById(state, id); return a ? a.name : id; }
    return id;
  };
  T.stripAnsi = function (s) { return String(s).replace(/\u001b\[[0-9;?]*[A-Za-z]/g, ''); };

  /* ---------- icons, classes, kinds ---------- */
  T.CLS = {
    stonewright: { label: 'Stonewright', color: '#e07a2f', glyph: 'M4 4h12v6H4z M9 10h2.4v10H9z' },
    loremaster:  { label: 'Loremaster',  color: '#8a55d6', glyph: 'M3.5 5h7.5v14H3.5z M13 5h7.5v14H13z' },
    warden:      { label: 'Warden',      color: '#3a9aa8', glyph: 'M12 2.5l7.5 3v6.2c0 4.3-3.2 7.4-7.5 9.3-4.3-1.9-7.5-5-7.5-9.3V5.5z' },
    swiftrider:  { label: 'Swiftrider',  color: '#d9a514', glyph: 'M13.5 2L5 13.2h5.8L9.5 22l9-12H12.6z' },
    herald:      { label: 'Herald',      color: '#c62f3a', glyph: 'M5 3h2.2v18H5z M7.2 4h12.3l-3.4 4.2 3.4 4.2H7.2z' },
    glasswright: { label: 'Glasswright', color: '#3da5cf', glyph: 'M12 2.5l7.5 7.5-7.5 11.5L4.5 10z' },
    archivist:   { label: 'Archivist',   color: '#5f9a3a', glyph: 'M8 3.5a4.5 4.5 0 100 9 4.5 4.5 0 000-9z M11 11l9.5 9.5-2.2 2-2-2-2 2-2.2-2.2-1.6-1.6z' },
    steward:     { label: 'Steward',     color: '#2f5fc4', glyph: 'M12 2l2.6 6.4 6.9.6-5.2 4.6 1.6 6.8L12 16.7 6.1 20.4l1.6-6.8L2.5 9l6.9-.6z' }
  };
  T.clsOf = function (c) { return T.CLS[c] || { label: T.cap(c || 'hero'), color: '#7b8494', glyph: 'M12 3a5 5 0 100 10 5 5 0 000-10z M4 21c0-5 3.5-7 8-7s8 2 8 7z' }; };
  /* One color per agent, not per class (the class shows through the glyph). Ordered so that
     neighbours are far apart in hue: the agents working at the same time take the first colors in
     start order, every other agent takes a color from a hash of its id. */
  T.PALETTE = ['#2f6fed', '#e0661f', '#2e9f4f', '#c2358f', '#b08400', '#7c4dd6', '#138f8f', '#d63a3a', '#8a5a3c', '#5b6b86'];
  var colorKey = null, colorMap = {};
  function idHash(id) { var h = 5381, i; for (i = 0; i < id.length; i++) h = ((h * 33) ^ id.charCodeAt(i)) >>> 0; return h; }
  T.agentColors = function (list) {
    var all = list || [], key = all.map(function (a) { return a.id + (a.status === 'working' ? '*' + a.started_at : ''); }).join('|');
    if (key === colorKey) return colorMap;
    var map = {}, n = T.PALETTE.length;
    all.filter(function (a) { return a.status === 'working'; }).sort(function (a, b) {
      return (T.ms(a.started_at) || 0) - (T.ms(b.started_at) || 0) || (a.id < b.id ? -1 : 1);
    }).forEach(function (a, i) { map[a.id] = T.PALETTE[i % n]; });
    all.forEach(function (a) { if (!map[a.id]) map[a.id] = T.PALETTE[idHash(String(a.id)) % n]; });
    colorKey = key; colorMap = map;
    return map;
  };
  T.grad = function (c) { return 'linear-gradient(135deg, color-mix(in srgb, ' + c + ' 62%, #fff), ' + c + ')'; };
  T.KIND = { brief: '#a78bfa', message: '#9ca3af', report: '#34a853', request: '#e8a317', alert: '#e5484d', answer: '#9ca3af', decision: '#a78bfa', merge: '#34a853' };
  T.fillIcon = function (path, size, color) {
    return '<svg width="' + size + '" height="' + size + '" viewBox="0 0 24 24" aria-hidden="true"><path d="' + path + '" fill="' + (color || '#fff') + '" fill-rule="evenodd"/></svg>';
  };
  T.lineIcon = function (path, size, color, w) {
    return '<svg width="' + size + '" height="' + size + '" viewBox="0 0 24 24" fill="none" stroke="' + (color || '#fff') + '" stroke-width="' + (w || 1.8) + '" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="' + path + '"/></svg>';
  };
  T.STAGE_ICON = {
    build:   ['M12 3l8 4.5v9L12 21l-8-4.5v-9z M12 12l8-4.5 M12 12v9 M12 12L4 7.5', '#5b8dff', '#6a5cff'],
    tests:   ['M9 3h6 M10 3v6l-5 9a2 2 0 002 3h10a2 2 0 002-3l-5-9V3', '#3ccf94', '#12a4a0'],
    lint:    ['M4 6h16 M4 12h10 M4 18h13', '#a77bff', '#e46bd0'],
    '16kb':  ['M7 7h10v10H7z M10 3v4 M14 3v4 M10 17v4 M14 17v4 M3 10h4 M3 14h4 M17 10h4 M17 14h4', '#ffa24d', '#ff5a6a'],
    sign:    ['M8 15a4 4 0 100-8 4 4 0 000 8z M11 12l9 9 M17 18l2-2', '#ffc247', '#ff8f2e'],
    device:  ['M8 3h8a1 1 0 011 1v16a1 1 0 01-1 1H8a1 1 0 01-1-1V4a1 1 0 011-1z M11 18h2', '#38bdf8', '#3b6bff'],
    store:   ['M12 16V4 M7 9l5-5 5 5 M4 20h16', '#ff78a8', '#ff8a5c'],
    review:  ['M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z M12 9a3 3 0 100 6 3 3 0 000-6z', '#2dd4bf', '#4ade80'],
    export:  ['M12 4v12 M7 11l5 5 5-5 M4 20h16', '#7c83ff', '#4f5bd5'],
    archive: ['M4 5h16v4H4z M5 9v10h14V9 M10 13h4', '#94a3be', '#5b6b88'],
    other:   ['M12 8a4 4 0 100 8 4 4 0 000-8z M12 2v3 M12 19v3 M2 12h3 M19 12h3', '#aab1be', '#7b8494']
  };
  T.STAGE_ALIAS = { test: 'tests', upload: 'store' };
  T.stageIcon = function (name, size) {
    var k = String(name || '').toLowerCase(); k = T.STAGE_ALIAS[k] || k;
    var d = T.STAGE_ICON[k] || T.STAGE_ICON.other;
    return { bg: 'linear-gradient(135deg, ' + d[1] + ', ' + d[2] + ')', svg: T.lineIcon(d[0], size || 20) };
  };
  T.ICON = {
    overview: 'M4 4h7v7H4z M13 4h7v4h-7z M13 10h7v10h-7z M4 13h7v7H4z',
    pipelines: 'M5 5a2 2 0 100 4 2 2 0 000-4z M19 15a2 2 0 100 4 2 2 0 000-4z M7 7h4a3 3 0 013 3v4a3 3 0 003 3',
    agents: 'M9 11a3.5 3.5 0 100-7 3.5 3.5 0 000 7z M2.5 20c0-3.6 2.8-5.5 6.5-5.5s6.5 1.9 6.5 5.5 M16 4.5a3.5 3.5 0 010 6.5 M18 14.8c2 .6 3.5 2.2 3.5 5.2',
    flow: 'M6 12a2 2 0 100-4 2 2 0 000 4z M18 7a2 2 0 100-4 2 2 0 000 4z M18 21a2 2 0 100-4 2 2 0 000 4z M8 10l8-4 M8 11l8 7',
    clock: 'M12 21a9 9 0 100-18 9 9 0 000 18z M12 7v5l3 2',
    play: 'M8 5l11 7-11 7z',
    close: 'M6 6l12 12M18 6L6 18',
    owner: 'M12 3a4.5 4.5 0 100 9 4.5 4.5 0 000-9z M3.5 21c0-5.2 3.7-7.5 8.5-7.5s8.5 2.3 8.5 7.5z',
    guard: 'M12 2.5l7.5 3v6.2c0 4.3-3.2 7.4-7.5 9.3-4.3-1.9-7.5-5-7.5-9.3V5.5z',
    room: 'M4 4h16v12H10l-5 4v-4H4z',
    pipe: 'M3 6l6 6-6 6z M10 6l6 6-6 6z M17 6l4 6-4 6z',
    check: 'M5 12.5l4.5 4.5L19 7.5'
  };
})();
