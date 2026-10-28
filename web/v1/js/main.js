/* Loader, filters, counters, ticker. */
(function () {
  var T = window.T, esc = T.esc;
  var raw = null, fstate = null, sig = '', genSeen = null, first = true, srcErr = '', replayI = 0;
  var filt = T.filter = { session: '', project: '', q: '' };
  var statePath = '', defaultMode = false;

  /* ---------- hash ---------- */
  function parseHash() {
    var h = location.hash.replace(/^#/, ''), o = {};
    h.split(/&(?=(?:state|session|project|q|tab|open)=)/).forEach(function (kv) {
      var i = kv.indexOf('='); if (i < 0) return;
      var k = kv.slice(0, i), v = kv.slice(i + 1);
      if (k !== 'state') { try { v = decodeURIComponent(v.replace(/\+/g, ' ')); } catch (e) {} }
      o[k] = v;
    });
    return o;
  }
  function writeHash() {
    var parts = [];
    if (!defaultMode) parts.push('state=' + statePath);
    if (filt.session) parts.push('session=' + encodeURIComponent(filt.session));
    if (filt.project) parts.push('project=' + encodeURIComponent(filt.project));
    if (filt.q) parts.push('q=' + encodeURIComponent(filt.q));
    if (T.ui.tab !== 'pipelines') parts.push('tab=' + T.ui.tab);
    var h = parts.length ? '#' + parts.join('&') : '';
    if (h !== location.hash) { try { history.replaceState(null, '', location.pathname + location.search + h); } catch (e) {} }
  }
  function applyHash(initial) {
    var o = parseHash();
    filt.session = o.session || ''; filt.project = o.project || ''; filt.q = o.q || '';
    if (o.tab) T.ui.tab = o.tab;
    if (o.open) { T.ui.openRuns[o.open] = true; T.ui.openAgents[o.open] = true; }
    var sp = o.state || '';
    defaultMode = !sp; statePath = sp || '../fixtures/state.demo.js';
    if (!initial) { sig = ''; raw = null; srcErr = ''; load(); }
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
  function flowProject(s, f) { return f.project || projOfEndpoint(s, f.from) || projOfEndpoint(s, f.to); }

  T.applyFilters = function (s) {
    var q = filt.q.trim().toLowerCase(), ss = filt.session, pj = filt.project;
    var out = {};
    for (var k in s) out[k] = s[k];
    var okS = function (x) { return !ss || x.session === ss; };
    var okP = function (pid) { return !pj || pid === pj; };
    out.agents = (s.agents || []).filter(function (a) {
      return okS(a) && okP(a.project) && (!q || hay([a.name, a.epithet, a.feature, a.summary, a['class'], a.rank], q));
    });
    out.pipelines = (s.pipelines || []).filter(function (p) {
      return okS(p) && okP(p.project) && (!q || hay([p.run, p.pipeline].concat((p.stages || []).map(function (x) { return x.name; })), q));
    });
    out.waiting_on_you = (s.waiting_on_you || []).filter(function (w) {
      return okS(w) && okP(w.project) && (!q || hay([w.text], q));
    });
    out.flows = (s.flows || []).filter(function (f) {
      if (!okS(f)) return false;
      if (pj && flowProject(s, f) !== pj) return false;
      return !q || hay([f.text, f.from_name, f.to_name, f.kind], q);
    });
    out.rooms = (s.rooms || []).filter(function (r) {
      if (pj && r.project !== pj) return false;
      if (ss && !(r.session === ss)) return false;
      return true;
    }).map(function (r) {
      if (!q) return r;
      var msgs = (r.messages || []).filter(function (m) { return hay([m.text, m.from], q); });
      if (!msgs.length && !hay([r.title, r.summary, r.id], q)) return null;
      var c = {}; for (var k2 in r) c[k2] = r[k2]; c.messages = msgs; return c;
    }).filter(Boolean);
    var sessObj = (s.sessions || []).filter(function (x) { return x.id === ss; })[0];
    var live = {};
    out.agents.concat(out.pipelines, out.waiting_on_you).forEach(function (x) { live[x.project] = 1; });
    out.rooms.forEach(function (r) { if (r.project) live[r.project] = 1; });
    if (sessObj) (sessObj.projects || []).forEach(function (id) { live[id] = 1; });
    var narrowing = ss || q;
    out.projects = (s.projects || []).filter(function (p) { return okP(p.id) && (!narrowing || live[p.id]); });
    out.sessions = s.sessions || [];
    return out;
  };

  /* ---------- filter bar ---------- */
  var optSig = '';
  function sessionsList(s) {
    var l = (s.sessions || []).slice(), have = {}, ids = {};
    l.forEach(function (x) { have[x.id] = 1; });
    ['agents', 'pipelines', 'flows', 'waiting_on_you'].forEach(function (k) { (s[k] || []).forEach(function (x) { if (x.session) ids[x.session] = 1; }); });
    Object.keys(ids).forEach(function (id) { if (!have[id]) { l.push({ id: id, title: id }); have[id] = 1; } });
    if (filt.session && !have[filt.session]) l.push({ id: filt.session, title: filt.session });
    return l;
  }
  function projectsList(s) {
    var l = (s.projects || []).slice();
    if (filt.project && !l.some(function (p) { return p.id === filt.project; })) l.push({ id: filt.project, name: filt.project, color: '#8b8f98' });
    return l;
  }
  function buildFilterBar(s) {
    var ses = sessionsList(s), prj = projectsList(s);
    var sg = JSON.stringify([ses.map(function (x) { return x.id + x.title; }), prj.map(function (x) { return x.id + x.name + x.color + x.auto; }), filt.session, filt.project]);
    if (sg !== optSig) {
      optSig = sg;
      var se = document.getElementById('f-session'), pe = document.getElementById('f-project');
      se.innerHTML = '<option value="">All sessions</option>' + ses.map(function (x) {
        var d = x.started_at ? T.ms(x.started_at) : null, ds = d ? new Date(d).toISOString().slice(5, 10) + ' ' : '';
        return '<option value="' + esc(x.id) + '">' + esc(ds + (x.title || x.id)) + (x.started_at && !x.ended_at ? ' (open)' : '') + '</option>';
      }).join('');
      se.value = filt.session;
      pe.innerHTML = '<option value="">All projects</option>' + prj.map(function (p) {
        return '<option value="' + esc(p.id) + '" style="color:' + esc(p.color) + '">● ' + esc(p.name) + (p.auto ? '  · auto' : '') + '</option>';
      }).join('');
      pe.value = filt.project;
      se.disabled = !ses.length;
    }
    var cur = prj.filter(function (p) { return p.id === filt.project; })[0];
    document.getElementById('f-dot').style.background = cur ? cur.color : 'transparent';
    document.getElementById('f-dot').style.borderColor = cur ? cur.color : '#6b6f78';
    document.getElementById('f-clear').style.visibility = (filt.session || filt.project || filt.q) ? 'visible' : 'hidden';
  }
  function showCount(f) {
    var active = filt.session || filt.project || filt.q.trim();
    var n = function (a, b) { return a + ' ' + b + (a === 1 ? '' : 's'); };
    var txt = (f.agents.length + (f.agents.length === 1 ? ' hero' : ' heroes')) + ' · ' + n(f.pipelines.length, 'run') + ' · ' + n(f.flows.length, 'message') + ' · ' + f.waiting_on_you.length + ' waiting';
    if (active) txt = 'Filtered: ' + txt;
    document.getElementById('f-count').textContent = txt;
  }

  /* ---------- counters ---------- */
  function counters(s) {
    var pl = s.pipelines || [], ag = s.agents || [];
    var c = function (st) { return ag.filter(function (a) { return a.status === st; }).length; };
    var items = [
      ['running', pl.filter(function (p) { return p.status === 'running'; }).length, 'pipelines running', 'running', { tab: 'pipelines', pfilter: 'running' }],
      ['failed', pl.filter(T.failed24).length, 'pipelines failed (24 h)', 'failed', { tab: 'pipelines', pfilter: 'failed24' }],
      ['working', c('working'), 'agents working', 'working', { tab: 'agents', afilter: 'working' }],
      ['stalled', c('stalled'), 'agents stalled', 'stalled', { tab: 'agents', afilter: 'stalled' }],
      ['failed', c('failed'), 'agents failed', 'failed', { tab: 'agents', afilter: 'failed' }],
      ['waiting', (s.waiting_on_you || []).length, 'waiting for you', 'waiting', { tab: 'waiting' }]
    ];
    var h = items.map(function (it, i) {
      return '<button class="ctr ' + it[3] + (it[1] === 0 ? ' zero' : '') + '" data-i="' + i + '">' +
        (it[3] === 'waiting' ? '<i class="bdot"></i>' : T.statusIcon(it[3], 18)) + '<b>' + it[1] + '</b><span>' + it[2] + '</span></button>';
    }).join('');
    var br = s.brain || {};
    h += '<span class="ctr brain ' + (br.enabled ? 'on' : 'off') + '" title="' + esc(br.last_error || br.model || '') + '"><i class="bdot"></i><span>brain ' + (br.enabled ? 'on' : 'off') + '</span></span>';
    var el = document.getElementById('counters');
    el.innerHTML = h; el._items = items;
  }

  /* ---------- render ---------- */
  T.refresh = function () {
    if (!fstate) return;
    T.renderPanel(fstate); writeHash();
  };
  function renderAll() {
    var s = raw;
    buildFilterBar(s);
    fstate = T.applyFilters(s);
    showCount(fstate);
    counters(fstate);
    T.map.render(fstate, first);
    first = false;
    T.refresh();
    var e = document.getElementById('empty');
    if (!fstate.projects.length && !fstate.agents.length) { e.hidden = false; e.textContent = 'Nothing matches the current filters.'; } else e.hidden = true;
  }

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
    srcErr = 'Cannot read ' + url;
    if (!raw) {
      var e = document.getElementById('empty'); e.hidden = false;
      e.innerHTML = '<b>Waiting for the collector… run ./run</b><br>No state found at <code>' + esc(url) + '</code><br>Point the page at your file with <code>#state=file:///path/to/state.js</code>';
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
    if (changed) { document.getElementById('empty').hidden = true; renderAll(); }
    srcLine();
  }
  function isDemo() { return defaultMode || /(^|\/)(fixtures|demo)\//.test(statePath); }
  function srcLine() {
    var el = document.getElementById('src'), demo = isDemo();
    document.body.classList.toggle('demo', demo);
    if (demo) { el.className = 'src demo'; el.textContent = 'DEMO DATA — not your real agents'; return; }
    if (!raw || srcErr) { el.className = 'src wait'; el.textContent = 'Waiting for the collector… run ./run'; return; }
    var age = Math.max(0, Math.round((Date.now() - T.ms(raw.generated_at)) / 1000));
    el.className = 'src live' + (age > 60 ? ' red' : age > 10 ? ' amber' : '');
    el.textContent = 'LIVE · updated ' + age + ' s ago' + (age > 60 ? ' — collector stopped?' : '');
  }

  /* ---------- ticker ---------- */
  function tick() {
    var i, els = document.querySelectorAll('.tick');
    for (i = 0; i < els.length; i++) els[i].textContent = (els[i].getAttribute('data-short') ? T.durShort : T.dur)(T.elapsed(els[i].getAttribute('data-start')));
    els = document.querySelectorAll('.agoel');
    for (i = 0; i < els.length; i++) {
      var at = els[i].getAttribute('data-at');
      els[i].textContent = els[i].getAttribute('data-plain') ? T.dur(T.elapsed(at)) : T.ago(at);
    }
    srcLine();
  }

  /* ---------- init ---------- */
  function init() {
    applyHash(true);
    T.initPanel(); T.map.init();
    document.getElementById('legend').innerHTML =
      '<div class="lg"><b>Flows</b>' + ['brief', 'message', 'report', 'request', 'answer'].map(function (k) { return '<span><i style="background:' + T.KIND[k].color + '"></i>' + k + '</span>'; }).join('') + '</div>' +
      '<div class="lg"><b>Agents</b><span><i class="r working"></i>working</span><span><i class="r done"></i>done</span><span><i class="r failed"></i>failed</span><span><i class="r stalled"></i>stalled</span></div>';
    document.getElementById('counters').addEventListener('click', function (e) {
      var b = e.target.closest('.ctr'); if (!b || b.getAttribute('data-i') == null) return;
      var it = this._items[+b.getAttribute('data-i')], o = {}, k;
      for (k in it[4]) if (k !== 'tab') o[k] = it[4][k];
      T.openIn(it[4].tab, null, o);
    });
    var onF = function () { sig = ''; if (raw) renderAll(); };
    document.getElementById('f-session').addEventListener('change', function () { filt.session = this.value; onF(); });
    document.getElementById('f-project').addEventListener('change', function () { filt.project = this.value; onF(); });
    var qi = document.getElementById('f-q'), qt;
    qi.value = filt.q;
    qi.addEventListener('input', function () { clearTimeout(qt); var v = this.value; qt = setTimeout(function () { filt.q = v; onF(); }, 120); });
    document.getElementById('f-clear').addEventListener('click', function () { filt.session = filt.project = filt.q = ''; qi.value = ''; onF(); });
    window.addEventListener('hashchange', function () {
      var o = parseHash(), same = (o.session || '') === filt.session && (o.project || '') === filt.project && (o.q || '') === filt.q &&
        (o.state || '') === (defaultMode ? '' : statePath) && (!o.tab || o.tab === T.ui.tab);
      if (same) return;
      applyHash(false); qi.value = filt.q; optSig = '';
    });
    srcLine();
    load();
    setInterval(load, 2000);
    setInterval(tick, 1000);
    setInterval(function () {      /* demo data never changes: keep the flows alive */
      if (!T.clock.demo || !fstate) return;
      var fl = fstate.flows || []; if (!fl.length) return;
      T.map.play(fl[replayI++ % fl.length], 0);
    }, 4500);
  }
  init();
})();
