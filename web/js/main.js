/* Loader, hash parameters, filters, status line, ticker. */
(function () {
  var T = window.T, esc = T.esc;
  var raw = null, sig = '', genSeen = null, srcErr = '';
  var filt = T.filter = { session: '', project: '', q: '', view: 'overview', mode: 'kanban' };
  var statePath = '', defaultMode = false;
  var tauri = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.event ? window.__TAURI__ : null;
  T.tauri = tauri;

  /* ---------- hash ---------- */
  function parseHash() {
    var h = location.hash.replace(/^#/, ''), o = {};
    h.split(/&(?=(?:state|session|project|q|open|view|mode)=)/).forEach(function (kv) {
      var i = kv.indexOf('='); if (i < 0) return;
      var k = kv.slice(0, i), v = kv.slice(i + 1);
      if (k !== 'state') { try { v = decodeURIComponent(v.replace(/\+/g, ' ')); } catch (e) {} }
      o[k] = v;
    });
    return o;
  }
  function writeHash() {
    var parts = [];
    if (!defaultMode && !tauri) parts.push('state=' + statePath);
    if (filt.session) parts.push('session=' + encodeURIComponent(filt.session));
    if (filt.project) parts.push('project=' + encodeURIComponent(filt.project));
    if (filt.q) parts.push('q=' + encodeURIComponent(filt.q));
    if (filt.view !== 'overview') parts.push('view=' + filt.view);
    if (filt.mode !== 'kanban') parts.push('mode=' + filt.mode);
    if (filt.view === 'pipelines' && T.ui.pOpen) parts.push('open=' + encodeURIComponent(T.ui.pOpen + pStageSuffix()));
    var h = parts.length ? '#' + parts.join('&') : '';
    if (h !== location.hash) { try { history.replaceState(null, '', location.pathname + location.search + h); } catch (e) {} }
  }
  /* the open run's selected stage, as "::name", so a link can open the same log */
  function pStageSuffix() {
    var r = T.rawState && T.rawState(), run = r && (r.pipelines || []).filter(function (p) { return p.run === T.ui.pOpen; })[0];
    var st = run && T.ui.stageSel[run.run] != null ? run.stages[T.ui.stageSel[run.run]] : null;
    return st ? '::' + st.name : '';
  }
  function applyHash(initial) {
    var o = parseHash();
    filt.session = o.session || ''; filt.project = o.project || ''; filt.q = o.q || '';
    filt.view = /^(overview|pipelines|agents|flow|settings)$/.test(o.view) ? o.view : 'overview';
    filt.mode = o.mode === 'list' || o.mode === 'tree' ? o.mode : 'kanban';
    if (o.open) {
      T.ui.selRun = o.open; T.ui.openAgent = o.open; T.ui.edge = o.open === 'edge' ? 'first' : null;
      T.ui.drawer = filt.view === 'pipelines' ? 'stage' : filt.view === 'agents' ? 'agent' : filt.view === 'flow' ? (o.open === 'edge' ? 'edge' : null) : null;
    }
    if (filt.view === 'pipelines' && o.open) { var po = o.open.split('::'); T.ui.pOpen = po[0]; T.ui.pOpenStage = po[1] || null; }
    var sp = o.state || '';
    defaultMode = !sp && !tauri; statePath = tauri ? 'tauri' : sp || '../fixtures/state.demo.js';
    if (!initial && tauri) { if (raw) renderAll(); }
    else if (!initial) { sig = ''; raw = null; srcErr = ''; load(); }
  }

  /* ---------- filtering ---------- */
  function hay(parts, q) { return parts.join(' \n ').toLowerCase().indexOf(q) >= 0; }
  function projOfEndpoint(s, ep) {
    var i = ep.indexOf(':'); if (i < 0) return null;
    var k = ep.slice(0, i), id = ep.slice(i + 1), a, r, p;
    if (k === 'agent') { a = T.agentById(s, id); return a ? a.project : null; }
    if (k === 'pipeline') { p = (s.pipelines || []).filter(function (x) { return x.run === id; })[0]; return p ? p.project : null; }
    if (k === 'room') { r = (s.rooms || []).filter(function (x) { return x.id === id; })[0]; return r ? r.project : null; }
    return null;
  }
  function applyFilters(s) {
    var q = filt.q.trim().toLowerCase(), ss = filt.session, pj = filt.project, out = {}, k;
    for (k in s) out[k] = s[k];
    var okS = function (x) { return !ss || x.session === ss; };
    var okP = function (pid) { return !pj || pid === pj; };
    out.agents = (s.agents || []).filter(function (a) {
      return okS(a) && okP(a.project) && (!q || hay([a.name, a.epithet, a.feature, a.summary, a['class'], a.rank], q));
    });
    out.pipelines = (s.pipelines || []).filter(function (p) {
      return okS(p) && okP(p.project) && (!q || hay([p.run, p.pipeline].concat((p.stages || []).map(function (x) { return x.name; })), q));
    });
    out.waiting_on_you = (s.waiting_on_you || []).filter(function (w) { return okS(w) && okP(w.project) && (!q || hay([w.text], q)); });
    out.answered_waits = (s.answered_waits || []).filter(function (w) { return okS(w) && okP(w.project) && (!q || hay([w.text], q)); });
    out.flows = (s.flows || []).filter(function (f) {
      if (!okS(f)) return false;
      if (pj && (f.project || projOfEndpoint(s, f.from) || projOfEndpoint(s, f.to)) !== pj) return false;
      return !q || hay([f.text, f.from_name, f.to_name, f.kind], q);
    });
    out.violations = (s.violations || []).filter(function (v) {
      return okP(v.project) && (!q || hay([v.rule, v.blocked, v.message, v.lane, v.agent], q));
    });
    out.rooms = (s.rooms || []).filter(function (r) { return okP(r.project) && (!ss || r.session === ss); }).map(function (r) {
      if (!q) return r;
      var msgs = (r.messages || []).filter(function (m) { return hay([m.text, m.from, r.title], q); });
      if (!msgs.length) return null;
      var c = {}; for (var k2 in r) c[k2] = r[k2]; c.messages = msgs; return c;
    }).filter(Boolean);
    return out;
  }

  /* The project the Overview shows: the sidebar's, else the one with the most recent activity (never "all"). */
  T.effProject = function (s) {
    if (filt.project) return filt.project;
    var r = raw || s || {}, best = '', at = -1;
    function see(pid, iso) { var m = T.ms(iso); if (pid && m != null && m > at) { at = m; best = pid; } }
    (r.agents || []).forEach(function (a) { see(a.project, a.last_activity_at || a.ended_at || a.started_at); });
    (r.pipelines || []).forEach(function (p) { see(p.project, p.ended_at || p.started_at); });
    (r.violations || []).forEach(function (v) { see(v.project, v.ts); });
    (r.waiting_on_you || []).forEach(function (w) { see(w.project, w.since); });
    return best || ((r.projects || [])[0] || {}).id || '';
  };
  /* s limited to one project's rows (Setup, Rules, Plan, Your turn and Pipelines read it) */
  T.scopeProject = function (s, pid) {
    if (!pid) return s;
    var out = {}, k, on = function (x) { return x.project === pid; };
    for (k in s) out[k] = s[k];
    ['agents', 'pipelines', 'waiting_on_you', 'answered_waits', 'violations', 'rooms'].forEach(function (n) { if (s[n]) out[n] = s[n].filter(on); });
    out.projects = (s.projects || []).filter(function (p) { return p.id === pid; });
    return out;
  };

  /* ---------- render ---------- */
  function renderAll() {
    T.render(applyFilters(raw), raw);
    writeHash();
  }

  T.setFilter = function (k, v) {
    filt[k] = v;
    if (raw) renderAll(); else writeHash();
  };

  /* ---------- loading ---------- */
  var chain = [], busy = false;
  function load() {
    if (busy) return; busy = true;
    chain = [statePath];
    if (defaultMode) chain.push('demo/state.demo.js');   /* only the built-in demo falls back; a live path never does */
    tryLoad(0);
  }
  function tryLoad(i) {
    var url = chain[i];
    delete window.TIRITH_STATE;
    var sc = document.createElement('script');
    sc.src = url + (url.indexOf('?') >= 0 ? '&' : '?') + 't=' + Date.now();
    sc.onload = function () {
      sc.remove(); busy = false;
      if (window.TIRITH_STATE) gotState(window.TIRITH_STATE); else fail(url);
    };
    sc.onerror = function () {
      sc.remove();
      if (i + 1 < chain.length) tryLoad(i + 1); else { busy = false; fail(url); }
    };
    document.head.appendChild(sc);
  }
  function fail(url) {
    srcErr = url;
    if (!raw) {
      document.getElementById('view').innerHTML = '<div class="pad"><p class="quiet">No state found at <code>' + esc(url) + '</code>. Point the page at your file with <code>#state=file:///path/to/state.js</code></p></div>';
    }
    srcLine();
  }
  function gotState(s) {
    srcErr = '';
    var gen = T.ms(s.generated_at);
    if (gen !== genSeen) {
      genSeen = gen;
      T.clock.demo = gen != null && Math.abs(Date.now() - gen) > 300000;
      T.clock.offset = T.clock.demo ? gen - Date.now() : 0;
    }
    var k = JSON.stringify(s, function (key, v) { return key === 'generated_at' ? undefined : v; });
    var changed = k !== sig;
    sig = k; raw = s;
    if (changed) renderAll();
    srcLine();
  }
  function isDemo() { return defaultMode || /(^|\/)(fixtures|demo)\//.test(statePath); }
  function srcLine() {
    var el = document.getElementById('src'), demo = isDemo(), txt, cls;
    if (demo) { cls = 'demo'; txt = 'Demo data'; }
    else if (!raw) { cls = 'wait'; txt = 'Waiting for the collector… run ./run'; }
    else {
      var age = Math.max(0, Math.round((Date.now() - T.ms(raw.generated_at)) / 1000));
      cls = age > 60 ? 'red' : (age > 10 || srcErr) ? 'amber' : 'green';
      txt = (srcErr ? 'state unreadable, ' : '') + 'updated ' + age + ' s ago';
    }
    if (el._t !== txt || el._c !== cls) {
      el._t = txt; el._c = cls; el.className = 'src ' + cls;
      el.innerHTML = (demo ? '' : '<i class="dot"></i>') + '<span>' + esc(txt) + '</span>';
    }
  }
  function tick() {
    var i, els = document.querySelectorAll('.tick');
    for (i = 0; i < els.length; i++) els[i].textContent = T.dur(T.elapsed(els[i].getAttribute('data-start')));
    els = document.querySelectorAll('.ago');
    for (i = 0; i < els.length; i++) els[i].textContent = T.ago(els[i].getAttribute('data-at'));
    els = document.querySelectorAll('.agol');
    for (i = 0; i < els.length; i++) els[i].textContent = T.agoLong(els[i].getAttribute('data-at'));
    srcLine();
  }

  /* ---------- init ---------- */
  function init() {
    applyHash(true);
    T.initUi();
    var onF = function () { if (raw) renderAll(); };
    var qi = document.getElementById('f-q'), qt;
    qi.value = filt.q;
    qi.addEventListener('input', function () { clearTimeout(qt); var v = this.value; qt = setTimeout(function () { filt.q = v; onF(); }, 120); });
    qi.addEventListener('keydown', function (e) { if (e.key === 'Escape') { this.value = ''; filt.q = ''; onF(); this.blur(); } });
    document.addEventListener('keydown', function (e) {
      if (((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') || (e.key === '/' && !/^(INPUT|SELECT|TEXTAREA)$/.test(document.activeElement.tagName))) { e.preventDefault(); qi.focus(); qi.select(); }
    });
    window.addEventListener('hashchange', function () {
      var o = parseHash();
      if ((o.session || '') === filt.session && (o.project || '') === filt.project && (o.q || '') === filt.q && (o.view || 'overview') === filt.view && (o.mode === 'list' || o.mode === 'tree' ? o.mode : 'kanban') === filt.mode && (o.state || '') === (defaultMode || tauri ? '' : statePath)) return;
      applyHash(false); qi.value = filt.q;
    });
    srcLine();
    if (tauri) {
      /* desktop app: Rust reads ~/.tirith/state.json and pushes it by event; no polling, no files */
      tauri.core.invoke('current_state').then(function (s) { if (s) gotState(s); });
      tauri.event.listen('tirith://state', function (e) { gotState(e.payload); });
    } else {
      load();
      setInterval(load, 2000);
    }
    setInterval(tick, 1000);
  }
  init();
})();
