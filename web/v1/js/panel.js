/* Right panel: tabs Pipelines / Agents / Messages / Waiting. */
(function () {
  var T = window.T, esc = T.esc;
  var ui = T.ui = { tab: 'pipelines', pfilter: 'all', afilter: 'all', openRuns: {}, openAgents: {}, stageSel: {}, focus: null, scrollFocus: false };

  var TABS = [['pipelines', 'Pipelines'], ['agents', 'Agents'], ['messages', 'Messages'], ['waiting', 'Waiting']];

  function failed24(p) {
    var t = T.ms(p.ended_at || p.started_at);
    return p.status === 'failed' && t != null && T.now() - t < 86400000;
  }
  T.failed24 = failed24;

  function counts(s) {
    var pl = s.pipelines || [], ag = s.agents || [];
    return {
      pipelines: pl.length, agents: ag.length, messages: (s.flows || []).length, waiting: (s.waiting_on_you || []).length
    };
  }

  function statusChip(st) {
    var label = { running: 'RUNNING', passed: 'PASSED', failed: 'FAILED', working: 'WORKING', done: 'DONE', stalled: 'STALLED', canceled: 'CANCELED' }[st] || String(st).toUpperCase();
    return '<span class="schip ' + esc(st) + '">' + T.statusIcon(st, 14) + label + '</span>';
  }

  function stageDots(p) {
    return '<span class="sdots">' + (p.stages || []).map(function (st) { return '<i class="sd ' + esc(st.status) + '"></i>'; }).join('') + '</span>';
  }

  /* ---------- Pipelines ---------- */
  var STAGE = {
    build: ['Compile the app.', 'building'], tests: ['Run the automated tests.', 'running the tests'], test: ['Run the automated tests.', 'running the tests'],
    lint: ['Check the code and the project rules.', 'checking the code'], '16kb': ['Google Play memory-page check (16 KB pages).', 'checking memory pages'],
    sign: ['Sign the package with the release key.', 'signing'], device: ['Try the app on a real phone.', 'testing on a phone'],
    store: ['Upload the build to the store.', 'uploading'], upload: ['Upload the build to the store.', 'uploading'],
    review: ["The store's review of the release.", 'waiting for the store review'], export: ['Export the project.', 'exporting'],
    archive: ['Build the signed archive.', 'archiving']
  };
  function stageDesc(st) { return st.description || (STAGE[String(st.name).toLowerCase()] || [''])[0]; }
  function stageDoing(st) { return (STAGE[String(st.name).toLowerCase()] || [0, 'running ' + st.name])[1]; }
  function errLine(st) {
    var l = (st.log_tail || []).filter(function (x) { return String(x).trim(); });
    for (var i = l.length - 1; i >= 0; i--) if (/error|fail|exception|denied|no .*found/i.test(l[i])) return l[i];
    return l.length ? l[l.length - 1] : '';
  }
  function runSummary(s, p) {
    var st = p.stages || [], n = st.length, done = st.filter(function (x) { return x.status === 'passed'; }).length;
    var proj = T.projectById(s, p.project).name, i;
    if (p.status === 'running') {
      var cur = st.filter(function (x) { return x.status === 'running'; })[0];
      return 'Running ' + esc(p.pipeline) + ' for ' + esc(proj) + ': ' + done + ' of ' + n + ' steps done' +
        (cur ? ', ' + esc(stageDoing(cur)) + ' now (' + T.tickSpan(cur.started_at, null, '') + ')' : '');
    }
    if (p.status === 'failed') {
      var f = st.filter(function (x) { return x.status === 'failed'; })[0], rest = [];
      if (!f) return 'Failed (' + done + ' of ' + n + ' steps done).';
      for (i = st.indexOf(f) + 1; i < n; i++) rest.push(st[i].name);
      var e = errLine(f);
      return 'Failed at ' + esc(f.name) + (e ? ': ' + esc(e) : '') + (rest.length ? ' \u2014 ' + esc(rest.join(', ')) + ' did not run' : '');
    }
    if (p.status === 'passed') return 'All ' + n + ' steps passed in ' + T.dur(T.elapsed(p.started_at, p.ended_at)) + '.';
    return esc(T.cap(p.status)) + ': ' + done + ' of ' + n + ' steps done.';
  }
  var LEGEND = '<div class="plegend"><span><b class="lk ok">\u2713</b> passed (time it took)</span><span><b class="lk run"></b> spinning = running (time so far)</span>' +
    '<span><b class="lk pend"></b> dashed = not started</span><span><b class="lk bad">\u2717</b> failed (error below)</span><span><b class="lk clk">00:00</b> big clock = total time of the run</span></div>';
  function pipelinesHtml(s) {
    var runs = (s.pipelines || []).slice().sort(function (a, b) { return (T.ms(b.started_at) || 0) - (T.ms(a.started_at) || 0); });
    var shown = runs.filter(function (p) { return ui.pfilter === 'all' || (ui.pfilter === 'failed24' ? failed24(p) : p.status === ui.pfilter); });
    var h = LEGEND + filterBar([['all', 'All'], ['running', 'Running'], ['failed24', 'Failed 24h'], ['passed', 'Passed']], ui.pfilter, 'pf', shown.length + ' of ' + runs.length + ' runs');
    if (!shown.length) h += '<div class="empty">No pipeline runs here.</div>';
    shown.forEach(function (p) {
      var open = !!ui.openRuns[p.run], foc = ui.focus && ui.focus.type === 'run' && ui.focus.id === p.run;
      var hash = p.run.lastIndexOf('#'), name = hash > 0 ? p.run.slice(0, hash) : p.pipeline, num = hash > 0 ? p.run.slice(hash) : '';
      h += '<div class="row run ' + p.status + (foc ? ' focus' : '') + '" data-id="run:' + esc(p.run) + '">' +
        '<div class="rowhead" data-act="toggle-run" data-id="' + esc(p.run) + '">' +
        '<div class="ricon">' + T.statusIcon(p.status, 28) + '</div>' +
        '<div class="rmain"><div class="rtitle"><b>' + esc(name) + '</b> <span class="num">' + esc(num) + '</span></div>' +
        '<div class="rplain">' + runSummary(s, p) + '</div>' +
        '<div class="rsub">' + T.projectChip(s, p.project) + '<span class="mut">' + esc(T.clockTime(p.started_at)) + '</span><span class="sep"></span>' + T.agoSpan(p.started_at) + '</div></div>' +
        stageDots(p) + statusChip(p.status) +
        '<div class="dbox">' + T.tickSpan(p.started_at, p.ended_at, 'mono') + '</div>' +
        '<div class="chev">' + (open ? '&#9662;' : '&#9656;') + '</div></div>';
      if (open) h += runDetail(p);
      h += '</div>';
    });
    return h;
  }

  function defaultStage(p) {
    var st = p.stages || [], i;
    for (i = 0; i < st.length; i++) if (st[i].status === 'running') return i;
    for (i = 0; i < st.length; i++) if (st[i].status === 'failed') return i;
    for (i = st.length - 1; i >= 0; i--) if (st[i].status === 'passed') return i;
    return 0;
  }
  function runDetail(p) {
    var sel = ui.stageSel[p.run] != null ? ui.stageSel[p.run] : defaultStage(p);
    var h = '<div class="rundetail"><div class="stages">';
    (p.stages || []).forEach(function (st, i) {
      var d;
      if (st.status === 'running') d = T.tickSpan(st.started_at, null, 'mono');
      else if (st.status === 'pending' || st.status === 'skipped') d = '<span class="mono">--</span>';
      else d = '<span class="mono">' + T.dur(st.duration_s != null ? st.duration_s : T.elapsed(st.started_at, st.ended_at)) + '</span>';
      if (i) h += '<span class="join ' + esc(p.stages[i - 1].status) + '"></span>';
      h += '<button title="' + esc(stageDesc(st)) + '" class="pill ' + esc(st.status) + (i === sel ? ' sel' : '') + '" data-act="stage" data-run="' + esc(p.run) + '" data-i="' + i + '">' +
        T.statusIcon(st.status, 20) + '<span class="pn"><b>' + esc(st.name) + '</b>' + d + '</span></button>';
    });
    h += '</div>';
    var cur = (p.stages || [])[sel];
    if (cur) {
      var lines = cur.log_tail && cur.log_tail.length ? cur.log_tail.join('\n') : '(no log output)';
      h += '<div class="loghead">' + esc(p.pipeline) + ' / ' + esc(cur.name) + ' &middot; ' + esc(cur.status) + '</div>' + (stageDesc(cur) ? '<div class="sdesc"><b>' + esc(cur.name) + ':</b> ' + esc(stageDesc(cur)) + '</div>' : '') + '<pre class="log">' + esc(lines) + '</pre>';
    }
    return h + '</div>';
  }

  /* ---------- Agents ---------- */
  function agentsHtml(s) {
    var order = { working: 0, stalled: 1, failed: 2, done: 3 };
    var ag = (s.agents || []).slice().sort(function (a, b) {
      return (order[a.status] - order[b.status]) || ((T.ms(b.started_at) || 0) - (T.ms(a.started_at) || 0));
    });
    var shown = ag.filter(function (a) { return ui.afilter === 'all' || a.status === ui.afilter; });
    var h = filterBar([['all', 'All'], ['working', 'Working'], ['stalled', 'Stalled'], ['failed', 'Failed'], ['done', 'Done']], ui.afilter, 'af', shown.length + ' of ' + ag.length + ' heroes');
    if (!shown.length) h += '<div class="empty">No agents here.</div>';
    shown.forEach(function (a) {
      var open = !!ui.openAgents[a.id], foc = ui.focus && ui.focus.type === 'agent' && ui.focus.id === a.id;
      var live = (a.status === 'working' || a.status === 'stalled') && !a.ended_at;
      var timer = live ? T.tickSpan(a.started_at, null, 'mono') : '<span class="mono">' + T.dur(T.agentElapsed(a)) + '</span>';
      h += '<div class="row agent ' + a.status + (foc ? ' focus' : '') + '" data-id="agent:' + esc(a.id) + '">' +
        '<div class="rowhead" data-act="toggle-agent" data-id="' + esc(a.id) + '">' +
        '<div class="ricon">' + T.badge(a['class'], 44) + '</div>' +
        '<div class="rmain"><div class="rtitle"><b>' + esc(a.name) + '</b> <span class="epi">' + esc(a.epithet || '') + '</span></div>' +
        '<div class="rsub"><span class="rank">' + esc(T.cap(a.rank)) + ' ' + esc(T.clsOf(a['class']).label) + '</span><span class="sep"></span>' + T.projectChip(s, a.project) + '<span class="sep"></span><span class="mut">' + esc(a.feature || '') + '</span></div>' +
        '<div class="rsum">' + esc(a.summary || '') + '</div></div>' +
        '<div class="aside">' + statusChip(a.status) + '<div class="dbox big">' + timer + '</div>' +
        '<div class="files mono">' + (a.files_changed || 0) + ' files <span class="add">+' + (a.insertions || 0) + '</span> <span class="del">&minus;' + (a.deletions || 0) + '</span></div></div></div>';
      if (open) {
        h += '<div class="rundetail"><div class="loghead">started ' + esc(T.clockTime(a.started_at)) + ' &middot; ' + T.agoSpan(a.started_at) + ' &middot; renown ' + (a.renown || 0) + '</div>';
        if (a.report) h += '<div class="report"><b>Report</b> ' + esc(a.report) + '</div>';
        h += '<pre class="log">' + esc((a.log_tail && a.log_tail.length ? a.log_tail : ['(no log output)']).join('\n')) + '</pre></div>';
      }
      h += '</div>';
    });
    return h;
  }

  /* ---------- Messages ---------- */
  function messagesHtml(s) {
    var fl = (s.flows || []).slice().sort(function (a, b) { return (T.ms(b.at) || 0) - (T.ms(a.at) || 0); });
    var h = '';
    (s.rooms || []).forEach(function (r) {
      var need = (r.needs_owner || []).length;
      h += '<div class="room" data-id="room:' + esc(r.id) + '"><div class="rtop">' + T.endpointIcon(s, 'room:' + r.id, 22) + '<b>' + esc(r.title || r.id) + '</b>' +
        (r.project ? T.projectChip(s, r.project) : '') + '<span class="mut">' + (r.activity_per_hour || 0) + ' msg/h &middot; ' + (r.participants || []).length + ' present</span>' +
        (need ? '<span class="chip warn">' + need + ' need you</span>' : '') + '</div><div class="rsum">' + esc(r.summary || '') + '</div></div>';
    });
    h += '<div class="toolbar"><span class="mut">Information flow, newest first</span><span class="mut">' + fl.length + ' messages</span></div>';
    if (!fl.length) h += '<div class="empty">Nothing has flowed yet.</div>';
    fl.forEach(function (f) {
      var col = T.flowColor(f), foc = ui.focus && ui.focus.type === 'flow' && ui.focus.id === f.id;
      h += '<div class="row flow' + (foc ? ' focus' : '') + '" data-id="flow:' + esc(f.id) + '" style="--kc:' + col + '">' +
        '<div class="ftime"><b>' + esc(T.clockTime(f.at)) + '</b>' + T.agoSpan(f.at) + '</div>' +
        '<div class="fbody"><div class="fwho">' + T.endpointIcon(s, f.from, 20) + '<span>' + esc(T.endpointName(s, f.from, f, 'from')) + '</span><span class="arrow">&rarr;</span>' +
        T.endpointIcon(s, f.to, 20) + '<span>' + esc(T.endpointName(s, f.to, f, 'to')) + '</span><span class="kchip">' + esc(f.kind) + '</span></div>' +
        '<div class="ftext">' + esc(f.text || '') + '</div></div></div>';
    });
    return h;
  }

  /* ---------- Waiting ---------- */
  function waitingHtml(s) {
    var w = (s.waiting_on_you || []).slice().sort(function (a, b) { return (T.ms(a.since) || 0) - (T.ms(b.since) || 0); });
    var h = '<div class="toolbar"><span class="mut">Things only you can do, oldest first</span><span class="mut">' + w.length + ' open</span></div>';
    if (!w.length) h += '<div class="empty ok">Nothing is waiting for you.</div>';
    w.forEach(function (x) {
      h += '<div class="row wait"><div class="ricon">' + T.ico('owner', 26, '#c9a24a') + '</div><div class="rmain"><div class="wtext">' + esc(x.text) + '</div>' +
        '<div class="rsub">' + T.projectChip(s, x.project) + '<span class="mut">since ' + esc(T.clockTime(x.since)) + '</span><span class="sep"></span>' +
        (x.source ? '<span class="mut">' + esc(x.source) + '</span><span class="sep"></span>' : '') +
        (x.link ? '<a class="lnk" target="_blank" rel="noopener" href="' + esc(x.link) + '">open</a>' : '') + '</div></div>' +
        '<div class="dbox"><span class="agoel mono" data-at="' + esc(x.since) + '" data-plain="1">' + esc(T.dur(T.elapsed(x.since))) + '</span></div></div>';
    });
    return h;
  }

  function filterBar(opts, cur, key, right) {
    return '<div class="toolbar"><span class="filters">' + opts.map(function (o) {
      return '<button class="fchip' + (o[0] === cur ? ' on' : '') + '" data-act="' + key + '" data-v="' + o[0] + '">' + o[1] + '</button>';
    }).join('') + '</span><span class="mut">' + esc(right) + '</span></div>';
  }

  /* ---------- render ---------- */
  var tabsEl, bodyEl;
  T.initPanel = function () {
    tabsEl = document.getElementById('tabs'); bodyEl = document.getElementById('tabbody');
    document.getElementById('panel').addEventListener('click', onClick);
  };
  T.renderPanel = function (s) {
    if (!s) return;
    var c = counts(s), top = bodyEl.scrollTop;
    tabsEl.innerHTML = TABS.map(function (t) {
      return '<button class="tab' + (ui.tab === t[0] ? ' on' : '') + (t[0] === 'waiting' && c.waiting ? ' alert' : '') + '" data-act="tab" data-v="' + t[0] + '">' + t[1] + ' <span class="cnt">' + c[t[0]] + '</span></button>';
    }).join('');
    bodyEl.innerHTML = ({ pipelines: pipelinesHtml, agents: agentsHtml, messages: messagesHtml, waiting: waitingHtml })[ui.tab](s);
    bodyEl.scrollTop = top;
    if (ui.scrollFocus) {
      ui.scrollFocus = false;
      var f = ui.focus && bodyEl.querySelector('[data-id="' + ui.focus.type + ':' + (window.CSS && CSS.escape ? CSS.escape(ui.focus.id) : ui.focus.id) + '"]');
      if (f) bodyEl.scrollTop = Math.max(0, f.offsetTop - 70);
    }
  };

  /* open something from the map or the header */
  T.openIn = function (tab, focus, opts) {
    ui.tab = tab; ui.focus = focus || null; ui.scrollFocus = !!focus;
    if (opts) for (var k in opts) ui[k] = opts[k];
    if (focus && focus.type === 'run') ui.openRuns[focus.id] = true;
    if (focus && focus.type === 'agent') ui.openAgents[focus.id] = true;
    T.refresh();
  };

  function onClick(e) {
    var el = e.target.closest('[data-act]'); if (!el) return;
    var act = el.getAttribute('data-act'), v = el.getAttribute('data-v'), id = el.getAttribute('data-id');
    if (act === 'tab') { ui.tab = v; ui.focus = null; bodyEl.scrollTop = 0; }
    else if (act === 'pf') ui.pfilter = v;
    else if (act === 'af') ui.afilter = v;
    else if (act === 'toggle-run') { ui.openRuns[id] = !ui.openRuns[id]; ui.focus = null; }
    else if (act === 'toggle-agent') { ui.openAgents[id] = !ui.openAgents[id]; ui.focus = null; }
    else if (act === 'stage') { ui.stageSel[el.getAttribute('data-run')] = +el.getAttribute('data-i'); }
    T.refresh();
  }
})();
