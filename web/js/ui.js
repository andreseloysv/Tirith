/* App shell: sidebar, breadcrumb, shared components, Overview. Views live in v-*.js */
(function () {
  var T = window.T, esc = T.esc;
  var ui = T.ui = { selRun: null, stageSel: {}, drawer: null, openAgent: null, edge: null, showEarlier: false, fit: true, zoom: 1, fopen: {}, fids: [] };
  var state = null, raw = null, common = null;
  T.views = {};

  var STAGE = {
    build: 'Compile the app.', tests: 'Run the automated tests.', test: 'Run the automated tests.',
    lint: 'Check the code and the project rules.', '16kb': 'Google Play memory-page check (16 KB pages).',
    sign: 'Sign the package with the release key.', device: 'Try the app on a real phone.',
    store: 'Upload the build to the store.', upload: 'Upload the build to the store.',
    review: "The store's review of the release.", export: 'Export the project.', archive: 'Build the signed archive.'
  };
  var VIEWS = [['overview', 'Overview'], ['pipelines', 'Pipelines'], ['agents', 'Agents'], ['flow', 'Flow']];
  var CHEV = '<svg class="chev" width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><path d="M4 2.5l3.5 3.5L4 9.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>';

  /* ---------- shared helpers (used by views) ---------- */
  T.stageDesc = function (st) { return st.description || STAGE[String(st.name).toLowerCase()] || ''; };
  T.stageSecs = function (st) {
    if (st.status === 'running') return T.elapsed(st.started_at);
    if (st.duration_s != null) return st.duration_s;
    return st.ended_at ? T.elapsed(st.started_at, st.ended_at) : null;
  };
  T.stageDur = function (st) {
    if (st.status === 'pending' || st.status === 'skipped') return '';
    return st.status === 'running' ? T.span(st.started_at, null) : esc(T.dur(T.stageSecs(st)));
  };
  T.defaultStage = function (p) {
    var st = p.stages || [], i;
    for (i = 0; i < st.length; i++) if (st[i].status === 'failed') return i;
    for (i = 0; i < st.length; i++) if (st[i].status === 'running') return i;
    for (i = st.length - 1; i >= 0; i--) if (st[i].status !== 'pending' && st[i].status !== 'skipped') return i;
    return 0;
  };
  T.curStage = function (p) {
    var st = p.stages || [], c = null, i;
    for (i = 0; i < st.length; i++) if (st[i].status === 'failed') return st[i];
    for (i = 0; i < st.length; i++) if (st[i].status === 'running') return st[i];
    return c;
  };
  T.errLine = function (p) {
    var f = (p.stages || []).filter(function (x) { return x.status === 'failed'; })[0];
    if (!f) return '';
    var l = (f.log_tail || []).map(T.stripAnsi).filter(function (x) { return String(x).trim(); });
    for (var i = l.length - 1; i >= 0; i--) if (/error|fail|exception|denied|no .*found/i.test(l[i])) return l[i].trim();
    return l.length ? l[l.length - 1].trim() : f.name + ' failed';
  };
  T.runNum = function (p) { return (String(p.run).match(/#\d+$/) || [''])[0]; };
  T.runName = function (p) { return String(p.pipeline).replace(/#\d+$/, ''); };
  T.pct = function (p) {
    var st = p.stages || []; if (!st.length) return 0;
    return Math.round(100 * st.filter(function (x) { return x.status === 'passed'; }).length / st.length);
  };
  T.proj = function (pid) {
    if (T.filter.project || (common && common === pid)) return '';
    return T.projectName(state, pid);
  };
  T.rawState = function () { return raw; };
  T.dot = function (cls, title) { return '<i class="dot ' + cls + '" title="' + esc(title || cls) + '"></i>'; };
  T.logBox = function (lines) {
    var t = (lines || []).map(T.stripAnsi).join('\n').replace(/\s+$/, '');
    return t ? '<pre class="log" tabindex="0">' + esc(t) + '</pre>' : '<p class="quiet">No log output yet</p>';
  };
  T.empty = function (t) { return '<p class="quiet">' + esc(t) + '</p>'; };
  T.agentColor = function (a) { return T.agentColors(raw && raw.agents)[a.id] || T.PALETTE[0]; };
  T.avatar = function (a, size) {
    var c = T.clsOf(a['class']);
    return '<span class="avatar" style="width:' + size + 'px;height:' + size + 'px;background:' + T.grad(T.agentColor(a)) + '">' + T.fillIcon(c.glyph, Math.round(size * .56)) + '</span>';
  };
  T.agentClock = function (a) {
    if (a.status === 'working') return T.span(a.started_at, null);
    return esc(T.dur(a.ended_at ? T.elapsed(a.started_at, a.ended_at) : a.elapsed_s));
  };
  T.agentFiles = function (a) {
    if (!a.files_changed && !a.insertions && !a.deletions) return '';
    return a.files_changed + (a.files_changed === 1 ? ' file' : ' files') + ' changed' +
      (a.insertions || a.deletions ? ' <span class="plus">+' + (a.insertions || 0) + '</span> <span class="minus">−' + (a.deletions || 0) + '</span>' : '');
  };
  /* the goal as the card shows it: one flat markdown line cut at CARD_GOAL chars; `clamped` tells the
     drawer to show the full text (a goal equal to the feature is never shown twice) */
  var CARD_GOAL = 200;
  T.cardGoal = function (a) {
    var g = String(a.goal || '').trim();
    if (!g || g === String(a.feature || '').trim()) return { text: '', clamped: false };
    var flat = g.replace(/\s*\n\s*/g, ' '), cut = flat.length > CARD_GOAL;
    if (cut) { flat = flat.slice(0, CARD_GOAL); flat = (flat.indexOf(' ') > 0 ? flat.slice(0, flat.lastIndexOf(' ')) : flat).replace(/[\s,;:]+$/, '') + '…'; }
    return { text: flat, clamped: cut || g.indexOf('\n') >= 0 };
  };
  function testBadge(a) {
    var l = a.tests || [], t = l[l.length - 1];
    if (!t || !t.result) return '';
    var txt = t.result === 'running' ? 'running' : t.result === 'failed' ? (t.failed != null ? t.failed + ' failed' : 'failed') : (t.passed != null ? t.passed + ' passed' : 'passed');
    if (t.result === 'passed' && t.failed) txt += ', ' + t.failed + ' failed';
    return '<span class="rb ' + esc(t.result) + '" title="Last test run: ' + esc(t.command || '') + '">' + esc(txt) + '</span>';
  }
  function workingFoot(a) {
    var files = a.files_changed || a.insertions || a.deletions ? '<span class="afiles">' + T.agentFiles(a).replace(/ changed/, '') + '</span>' : '',
        step = a.step ? '<span class="chip astep" title="Plan step">Step ' + esc(a.step) + '</span>' : '',
        proj = T.proj(a.project);
    return '<span class="afoot">' + files + testBadge(a) + step + '<span class="aproj">' + esc(proj) + '</span>' +
      '<span class="aclock">' + T.lineIcon(T.ICON.clock, 13, 'currentColor', 1.7) + '<span>' + T.agentClock(a) + '</span></span></span>';
  }
  /* remote work (source "github"): a small mark and an "Open on GitHub" link */
  T.ghMark = function (x) {
    return x && x.source === 'github' ? '<svg class="ghmark" viewBox="0 0 16 16" width="13" height="13" role="img" aria-label="GitHub" fill="currentColor"><title>GitHub</title><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38v-1.33c-2.23.48-2.7-1.07-2.7-1.07-.36-.92-.89-1.17-.89-1.17-.73-.5.05-.49.05-.49.8.06 1.23.83 1.23.83.72 1.22 1.88.87 2.33.66.07-.52.28-.87.5-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.83-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82a7.6 7.6 0 0 1 4 0c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.28.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48v2.2c0 .21.15.46.55.38A8 8 0 0 0 16 8c0-4.42-3.58-8-8-8z"/></svg>' : '';
  };
  T.ghLink = function (x) {
    return x && x.source === 'github' && /^https:\/\//.test(x.html_url || '') ? '<a class="ghlink" href="' + esc(x.html_url) + '" target="_blank" rel="noopener">' + T.ghMark(x) + 'Open on GitHub</a>' : '';
  };
  T.agentCard = function (a, selected) {
    var head = '<span class="ahead">' + T.avatar(a, 36) + '<span class="aname"><b title="' + esc((a.name || '') + (a.epithet ? ' ' + a.epithet : '')) + '">' + esc(a.name) + '</b>' +
      '<span class="pill">' + (a.source === 'github' ? T.ghMark(a) + ' ' + esc(a.epithet) : esc([T.cap(a.rank), T.cap(a['class'])].filter(Boolean).join(' · '))) + '</span></span></span>';
    if (a.status === 'working') {
      var g = T.cardGoal(a).text;
      return '<button class="card acard work' + (selected ? ' sel' : '') + '" data-act="agent" data-key="' + esc(a.id) + '">' + head +
        '<span class="afeat">' + esc(a.feature) + '</span>' +
        (g ? '<span class="agoalc">' + T.mdInline(g) + '</span>' : '') +
        (a.summary ? '<span class="anow' + (a.active ? ' live' : '') + '"><i class="pdot2"></i>' + T.mdInline(a.summary) + '</span>' : '') +
        workingFoot(a) + '</button>';
    }
    return '<button class="card acard' + (selected ? ' sel' : '') + '" data-act="agent" data-key="' + esc(a.id) + '">' + head +
      '<span class="afeat">' + esc(a.feature) + '</span>' +
      '<span class="afoot"><span class="aproj">' + esc(T.proj(a.project)) + '</span><span class="aclock">' + T.lineIcon(T.ICON.clock, 13, 'currentColor', 1.7) + '<span>' + T.agentClock(a) + '</span></span></span></button>';
  };
  T.drawer = function (title, body, head) {
    return '<aside class="drawer" role="dialog" aria-label="' + esc(title) + '"><header>' + (head || '<span>' + esc(title) + '</span>') + '<button class="x" data-act="close" aria-label="Close">' + T.lineIcon(T.ICON.close, 14, 'currentColor', 2) + '</button></header><div class="dbody" data-scroll="drawer">' + body + '</div></aside>';
  };
  /* zoomable dotted canvas; inner is unscaled html with fixed content size */
  T.canvas = function (inner, extraBar) {
    return '<div class="canvas" data-scroll="canvas"><div class="zoomer"><div class="zstage">' + inner + '</div></div></div>' +
      '<div class="zbar"><button data-act="zoom-out" aria-label="Zoom out">−</button><span class="zv">100%</span><button data-act="zoom-in" aria-label="Zoom in">+</button><button data-act="fit" class="fit">Fit</button>' + (extraBar || '') + '</div>';
  };
  function applyZoom() {
    var cv = document.querySelector('.canvas'); if (!cv) return;
    var zs = cv.querySelector('.zstage'), zr = cv.querySelector('.zoomer');
    var w = zs.offsetWidth, h = zs.offsetHeight, sc;
    if (ui.fit) sc = Math.max(.35, Math.min(1, (cv.clientWidth - 48) / w, (cv.clientHeight - 130) / h)); else sc = ui.zoom;
    ui.zoom = sc;
    zs.style.transform = 'scale(' + sc + ')';
    zr.style.width = Math.round(w * sc) + 'px'; zr.style.height = Math.round(h * sc) + 'px';
    var zv = document.querySelector('.zv'); if (zv) zv.textContent = Math.round(sc * 100) + '%';
  }
  T.applyZoom = applyZoom;

  /* ---------- sidebar and breadcrumb ---------- */
  function renderNav() {
    var f = T.filter, r = raw || {}, h = '';
    h += '<div class="brand"><svg width="22" height="22" viewBox="0 0 24 24" aria-hidden="true"><defs><linearGradient id="lg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#6fdccb"/><stop offset=".45" stop-color="#79a8ff"/><stop offset=".75" stop-color="#c79bf2"/><stop offset="1" stop-color="#ffb299"/></linearGradient></defs><path d="M12 2l8 8-8 12L4 10z" fill="url(#lg)" fill-opacity=".85"/><path d="M12 2l8 8H4z" fill="#fff" fill-opacity=".45"/><path d="M12 2l8 8-8 12L4 10z" fill="none" stroke="#fff" stroke-opacity=".9"/></svg><span>Tirith</span></div><nav><div class="grp">Views</div>' + VIEWS.map(function (v) {
      return '<button class="nav' + (f.view === v[0] ? ' on' : '') + '" data-nav="view" data-v="' + v[0] + '">' + T.lineIcon(T.ICON[v[0]], 16, 'currentColor', 1.7) + '<span>' + v[1] + '</span></button>';
    }).join('');
    var prj = r.projects || [];
    if (prj.length) {
      h += '<div class="grp">Projects</div>' + prj.map(function (p) {
        var n = (r.pipelines || []).filter(function (x) { return x.project === p.id && x.status === 'running'; }).length +
                (r.agents || []).filter(function (x) { return x.project === p.id && x.status === 'working'; }).length;
        return '<button class="nav' + (f.project === p.id ? ' on' : '') + '" data-nav="project" data-v="' + esc(p.id) + '"><i class="pdot" style="background:' + esc(p.color || '#9ca3af') + '"></i><span>' + esc(p.name) + '</span>' + (n ? '<em>' + n + '</em>' : '') + '</button>';
      }).join('');
    }
    var ses = (r.sessions || []).slice(0, 6);
    if (ses.length) {
      h += '<div class="grp">Sessions</div>' + ses.map(function (x) {
        return '<button class="nav' + (f.session === x.id ? ' on' : '') + '" data-nav="session" data-v="' + esc(x.id) + '" title="' + esc(x.title || x.id) + '"><span>' + esc(x.title || x.id) + '</span></button>';
      }).join('');
    }
    h += '</nav>';
    var nv = document.getElementById('navlist'); nv.innerHTML = h;
  }
  function renderCrumb() {
    var f = T.filter, vl = VIEWS.filter(function (v) { return v[0] === f.view; })[0][1], c = [vl];
    var pid = f.project || common;
    if (pid) c.push(T.projectName(raw, pid));
    if (f.session) {
      var s = ((raw && raw.sessions) || []).filter(function (x) { return x.id === f.session; })[0];
      c.push(s ? (s.title || s.id) : f.session);
    }
    document.getElementById('crumb').innerHTML = c.map(function (x, i) {
      return '<span class="' + (i === c.length - 1 ? 'last' : '') + '">' + esc(x) + '</span>';
    }).join('<span class="sep">/</span>');
  }
  function computeCommon(s) {
    var items = [], v = T.filter.view;
    if (v === 'overview') items = (s.pipelines || []).filter(function (p) { return p.status === 'running'; }).concat((s.agents || []).filter(function (a) { return a.status === 'working' || a.status === 'stalled'; }));
    else if (v === 'pipelines') items = s.pipelines || [];
    else if (v === 'agents') items = s.agents || [];
    var ids = {}; items.forEach(function (x) { ids[x.project] = 1; });
    var k = Object.keys(ids);
    return k.length === 1 && k[0] && k[0] !== 'null' ? k[0] : null;
  }

  /* ---------- merged messages (flows + room messages) ---------- */
  function norm(x) { return String(x || '').toLowerCase().replace(/\s+/g, ' ').trim(); }
  T.messages = function (s) {
    var out = [], seen = {}, roomTitle = {};
    (s.rooms || []).forEach(function (r) {
      roomTitle[r.id] = r.title || r.id;
      var parts = {}; (r.participants || []).forEach(function (p) { parts[p.name] = p; });
      (r.messages || []).forEach(function (m) {
        var p = parts[m.from], fromId, fromName = m.from;
        if (p && p.kind === 'owner') { fromId = 'owner'; fromName = 'You'; }
        else if (p && p.kind === 'tirith-agent' && p.hero_id) { fromId = 'agent:' + p.hero_id; var a = T.agentById(s, p.hero_id); fromName = (a && a.name) || p.hero_name || m.from; }
        else fromId = 'ext:' + m.from;
        out.push({ at: m.at, fromId: fromId, toId: 'room:' + r.id, fromName: fromName, toName: r.title || r.id, text: m.text, kind: m.kind || 'message', src: 'room' });
        seen[r.id + '|' + String(m.text).slice(0, 60)] = 1;
      });
    });
    (s.flows || []).forEach(function (f) {
      var ep = [f.from, f.to].filter(function (e) { return /^room:/.test(e); })[0];
      if (ep && seen[ep.slice(5) + '|' + String(f.text).slice(0, 60)]) return;
      var nm = function (e, side) { return /^room:/.test(e) ? (roomTitle[e.slice(5)] || T.endpointName(s, e, f, side)) : T.endpointName(s, e, f, side); };
      out.push({ at: f.at, fromId: f.from, toId: f.to, fromName: nm(f.from, 'from'), toName: nm(f.to, 'to'), text: f.text, kind: f.kind, src: 'flow' });
    });
    out.sort(function (a, b) { return (T.ms(b.at) || 0) - (T.ms(a.at) || 0); });
    return out;
  };

  /* ---------- Overview ---------- */
  function overview(s, el) {
    var w = s.waiting_on_you || [];
    var runs = (s.pipelines || []).filter(function (p) { return p.status === 'running'; });
    var ags = (s.agents || []).filter(function (a) { return a.status === 'working' || a.status === 'stalled'; });
    var h = '<div class="ovg"><div class="ovmain" data-scroll="ovmain">';
    var spent = T.sumUsage && T.sumUsage(s.agents || []);
    if (spent) h += '<p class="mut sm usage ovusage" title="All agents in this view">Spent ' + esc(T.usageLine(spent)) + '</p>';
    if (T.plan) h += T.plan(s);
    if (w.length) h += '<section><h2>Needs you</h2><ul class="needs card">' + w.map(function (x) {
      var pn = T.proj(x.project);
      return '<li>' + T.dot('amber', 'waiting for you') + '<span class="t">' + (x.link ? '<a href="' + esc(x.link) + '" target="_blank" rel="noopener">' + esc(x.text) + '</a>' : T.mdInline(x.text)) + '</span><span class="m">' + esc(pn) + '</span><span class="r">' + T.agoSpan(x.since) + '</span></li>';
    }).join('') + '</ul></section>';
    h += '<section><h2>Running pipelines <span class="n">' + runs.length + '</span></h2>' + (runs.length ? '<div class="grid2">' + runs.map(function (p) {
      var st = p.stages || [], cur = T.curStage(p);
      return '<button class="card rcard" data-act="gorun" data-key="' + esc(p.run) + '"><span class="rhead"><b>' + esc(T.runName(p)) + ' <span class="mut">' + esc(T.runNum(p)) + '</span></b><span class="mut">' + esc(T.proj(p.project)) + '</span><span class="rtime">' + T.span(p.started_at, null) + '</span></span>' +
        '<span class="mini">' + st.map(function (x, i) { return (i ? '<i class="ml ' + (st[i - 1].status === 'passed' ? 'on' : '') + '"></i>' : '') + '<i class="mn ' + esc(x.status) + '" title="' + esc(x.name + ' · ' + x.status) + '"></i>'; }).join('') + '</span>' +
        (cur ? '<span class="mcur">' + esc(cur.name) + '</span>' : '') + '</button>';
    }).join('') + '</div>' : T.empty('No pipelines running')) + '</section>';
    h += '<section><h2>Working agents <span class="n">' + ags.length + '</span></h2>' + (ags.length ? '<div class="grid2">' + ags.map(function (a) { return T.agentCard(a, false).replace('data-act="agent"', 'data-act="goagent"'); }).join('') + '</div>' : T.empty('No agents working')) + '</section>';
    h += '</div><aside class="ovside" data-scroll="ovside">' + activity(s) + '</aside></div>';
    el.innerHTML = h;
  }
  function activity(s) {
    var items = T.messages(s);
    var waits = (s.waiting_on_you || []).map(function (w) { return norm(w.text); });
    var feats = (s.agents || []).map(function (a) { return norm(a.feature); }).filter(Boolean);
    items = items.filter(function (m) {
      if (/^pipeline:/.test(m.fromId) || /^pipeline:/.test(m.toId)) return false;
      var t = norm(m.text);
      if ((m.kind === 'request' || m.kind === 'alert') && waits.some(function (w) { return w.indexOf(t.slice(0, 30)) === 0 || t.indexOf(w.slice(0, 30)) === 0; })) return false;
      if ((m.kind === 'brief' || m.kind === 'report') && feats.some(function (x) { return t.indexOf(x) >= 0; })) return false;
      return true;
    });
    var total = items.length; items = items.slice(0, 80);
    return '<section><h2>Activity <span class="n">' + total + '</span></h2>' + (items.length ? '<ul class="feed">' + items.map(function (x) {
      return '<li><span class="when">' + esc(T.clockTime(x.at)) + '</span><span class="who"><i class="dot" style="background:' + (T.KIND[x.kind] || '#9ca3af') + '" title="' + esc(x.kind) + '"></i>' +
        esc(x.fromName) + ' <span class="arr">→</span> ' + esc(x.toName) + '</span><span class="txt mdt">' + T.mdBox(x.text, 'm:' + x.fromId + x.at) + '</span></li>';
    }).join('') + '</ul>' : T.empty('No activity in this session')) + '</section>';
  }
  T.views.overview = overview;

  /* ---------- render ---------- */
  function scrolls() { var o = {}; [].forEach.call(document.querySelectorAll('[data-scroll]'), function (e) { var k = e.getAttribute('data-scroll'); o[k] = e.scrollTop; o['h:' + k] = e.scrollHeight; }); return o; }
  /* a list with data-anchor grows at the top (newest first): keep what the reader sees where it is */
  function restore(o) {
    [].forEach.call(document.querySelectorAll('[data-scroll]'), function (e) {
      var k = e.getAttribute('data-scroll');
      if (!o[k]) return;
      e.scrollTop = e.hasAttribute('data-anchor') && o['h:' + k] ? o[k] + e.scrollHeight - o['h:' + k] : o[k];
    });
  }
  T.render = function (fstate, rawState) {
    if (rawState) raw = rawState;
    if (fstate) state = fstate;
    if (!state) return;
    var a = document.activeElement, fk = a && a.getAttribute && [a.getAttribute('data-act'), a.getAttribute('data-key'), a.getAttribute('data-i')];
    var sc = scrolls();
    common = computeCommon(state);
    renderNav(); renderCrumb();
    var el = document.getElementById('view'); el.className = 'view v-' + T.filter.view;
    (T.views[T.filter.view] || overview)(state, el);
    restore(sc);
    T.mdFit(el);
    if (document.querySelector('.canvas')) applyZoom();
    if (ui.reveal) {
      var rn = [].filter.call(document.querySelectorAll('.fnode'), function (x) { return x.getAttribute('data-id') === ui.reveal; })[0];
      if (rn) rn.scrollIntoView({ block: 'nearest', inline: 'nearest' });
      ui.reveal = null;
    }
    if (fk && fk[0]) {
      var q = '[data-act="' + fk[0] + '"]' + (fk[1] ? '[data-key="' + (window.CSS && CSS.escape ? CSS.escape(fk[1]) : fk[1]) + '"]' : '') + (fk[2] != null ? '[data-i="' + fk[2] + '"]' : '');
      var n = document.querySelector(q); if (n) n.focus();
    }
  };
  T.renderNavOnly = function (r) { raw = r; };

  T.initUi = function () {
    document.getElementById('nav').addEventListener('click', function (e) {
      var b = e.target.closest('[data-nav]'); if (!b) return;
      var k = b.getAttribute('data-nav'), v = b.getAttribute('data-v');
      if (k === 'view') { ui.fit = true; T.setFilter('view', v); }
      else T.setFilter(k, T.filter[k] === v ? '' : v);
    });
    document.getElementById('view').addEventListener('click', function (e) {
      var b = e.target.closest('[data-act]'); if (!b) return;
      var act = b.getAttribute('data-act'), key = b.getAttribute('data-key');
      if (act === 'run') { ui.selRun = key; ui.drawer = null; ui.fit = true; }
      else if (act === 'stage') { ui.selRun = key; ui.stageSel[key] = +b.getAttribute('data-i'); ui.drawer = 'stage'; }
      else if (act === 'agent') { ui.openAgent = ui.openAgent === key && T.filter.mode === 'list' ? null : key; ui.drawer = 'agent'; }
      else if (act === 'fnode') { ui.fopen[key] = !ui.fopen[key]; if (ui.fopen[key]) { ui.fit = false; ui.zoom = Math.max(ui.zoom, .85); ui.reveal = key; } }
      else if (act === 'fall') { var all = (ui.fids || []).every(function (i) { return ui.fopen[i]; }); ui.fopen = {}; if (!all) (ui.fids || []).forEach(function (i) { ui.fopen[i] = true; }); }
      else if (act === 'edge') { ui.edge = key; ui.drawer = 'edge'; }
      else if (act === 'close') { ui.drawer = null; if (T.filter.mode !== 'list') ui.openAgent = null; ui.edge = null; }
      else if (act === 'earlier') ui.showEarlier = !ui.showEarlier;
      else if (act === 'logf') ui.logFilter = key;
      else if (act === 'logsys') ui.logSys[key] = !ui.logSys[key];
      else if (act === 'logopen') ui.logOpen[key] = !ui.logOpen[key];
      else if (act === 'mdopen') T.mdToggle(key);
      else if (act === 'mode') { T.setFilter('mode', b.getAttribute('data-v')); return; }
      else if (act === 'zoom-in') { ui.fit = false; ui.zoom = Math.min(2, Math.round((ui.zoom + .1) * 10) / 10); }
      else if (act === 'zoom-out') { ui.fit = false; ui.zoom = Math.max(.3, Math.round((ui.zoom - .1) * 10) / 10); }
      else if (act === 'fit') ui.fit = true;
      else if (act === 'gorun') { ui.selRun = key; ui.fit = true; T.setFilter('view', 'pipelines'); return; }
      else if (act === 'goagent') { ui.openAgent = key; ui.drawer = 'agent'; T.setFilter('view', 'agents'); return; }
      else return;
      T.render();
    });
    window.addEventListener('resize', function () { if (document.querySelector('.canvas')) applyZoom(); });
    document.addEventListener('keydown', function (e) {
      if (e.key === 'Escape' && ui.drawer && !/^(INPUT|SELECT)$/.test(document.activeElement.tagName)) { ui.drawer = null; ui.edge = null; if (T.filter.mode !== 'list') ui.openAgent = null; T.render(); }
    });
  };
})();
