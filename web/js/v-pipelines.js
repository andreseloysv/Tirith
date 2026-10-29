/* Pipelines view: a run table like GitLab's pipeline list. One row per run, stages as a row of status circles,
   a tooltip per stage, the stage log one click away. "Retry" and "Run" go through the Setup view's inbox path (data-act="srun"). */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  ui.pOpen = ui.pOpen || null;      /* run id whose detail is open below its row */
  ui.pOpenStage = ui.pOpenStage || null; /* stage name from the URL hash, resolved on the next render */

  var PLAY = '<svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true"><path d="M3.2 2.1l6.2 3.9-6.2 3.9z" fill="currentColor"/></svg>';
  var SVG = {
    passed: '<path d="M3.2 6.4l2.1 2.1 3.6-4.2"/>', failed: '<path d="M3.6 3.6l4.8 4.8M8.4 3.6L3.6 8.4"/>',
    warning: '<path d="M6 2.8v3.6M6 8.6v.1"/>', stalled: '<path d="M6 2.8v3.6M6 8.6v.1"/>',
    skipped: '<path d="M2.6 3.4l2.6 2.6-2.6 2.6M6.2 3.4l2.6 2.6-2.6 2.6"/>', canceled: '<path d="M3.4 8.6l5.2-5.2"/>'
  };
  var LABEL = { passed: 'Passed', failed: 'Failed', running: 'Running', pending: 'Pending', canceled: 'Canceled', warning: 'Warning', stalled: 'Stalled', skipped: 'Skipped' };

  function raw() { return T.rawState() || {}; }
  function projectOf(pid) { return (raw().projects || []).filter(function (x) { return x.id === pid; })[0] || null; }
  function startMs(p) { return T.ms(p.started_at) || 0; }

  /* the declared command of a stage (tirith.toml), or of a git hook */
  function stageCmd(p, st) {
    var su = (projectOf(p.project) || {}).setup, hit = '';
    if (!su) return '';
    (su.pipelines || []).forEach(function (pl) {
      if (pl.id !== p.pipeline) return;
      (pl.stages || []).forEach(function (s) { if (s.name === st.name && s.run) hit = s.run; });
    });
    if (!hit && p.pipeline === 'git-hooks') (su.hooks || []).forEach(function (h) { if (h.event === st.name) hit = h.run; });
    return hit;
  }

  /* who started it: an agent (avatar + name), the manager, the owner, a git hook or GitHub */
  function trigger(p) {
    var a = p.lane ? T.agentById(raw(), p.lane) : null, name, av = '';
    if (a) { name = a.name; av = T.avatar(a, 20); }
    else if (p.source === 'github') { name = 'GitHub'; av = '<span class="ptw-ic">' + T.ghMark(p) + '</span>'; }
    else if (p.pipeline === 'git-hooks') { name = 'git hook'; av = '<span class="ptw-ic">' + T.lineIcon(T.ICON.pipe, 12, 'currentColor', 2) + '</span>'; }
    else if (p.lane === 'owner') { name = 'owner'; av = '<span class="ptw-ic">' + T.lineIcon(T.ICON.owner, 12, 'currentColor', 2) + '</span>'; }
    else { name = p.lane && p.lane !== 'steward' && p.lane !== 'manager' ? p.lane : 'manager'; av = '<span class="ptw-ic">' + T.lineIcon(T.ICON.guard, 12, 'currentColor', 2) + '</span>'; }
    return '<span class="ptw">' + av + '<b>' + esc(name) + '</b></span>' + (p.branch ? '<span class="pbr" title="' + esc(p.branch) + '">' + esc(p.branch) + '</span>' : '');
  }

  function glyph(status, size) {
    if (status === 'running') return '<i class="pspin"></i>';
    var d = SVG[status];
    return d ? '<svg viewBox="0 0 12 12" width="' + size + '" height="' + size + '" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' + d + '</svg>' : '';
  }
  function circle(p, st, i, sel) {
    return (i ? '<i class="pln ' + (p.stages[i - 1].status === 'passed' ? 'on' : '') + '"></i>' : '') +
      '<button class="pc22 s-' + esc(st.status) + (sel ? ' sel' : '') + '" data-act="pstage" data-key="' + esc(p.run) + '" data-i="' + i + '" aria-label="' + esc(st.name + ': ' + (LABEL[st.status] || st.status)) + '">' + glyph(st.status, 12) + '</button>';
  }

  function badge(p) {
    var s = p.status, d = s === 'pending' ? '' : s === 'running' ? T.span(p.started_at, null) : esc(T.dur(T.elapsed(p.started_at, p.ended_at)));
    return '<span class="pbadge s-' + esc(s) + '">' + glyph(s, 11) + '<span>' + esc(LABEL[s] || T.cap(s)) + '</span>' + (d ? '<span class="pdur">' + d + '</span>' : '') + '</span>';
  }

  function chip(pid) {
    var n = T.proj(pid), pr = projectOf(pid);
    return n ? '<span class="pchip"><i style="background:' + esc((pr && pr.color) || '#9aa1ac') + '"></i>' + esc(n) + '</span>' : '';
  }

  function retry(p) {
    var f = T.curStage(p);
    if (p.status !== 'failed' || !f || f.status !== 'failed' || p.pipeline === 'git-hooks') return '';
    var key = p.project + '|' + p.pipeline + '|' + f.name, at = ui.setupPend && ui.setupPend[key + '|run'];
    var busy = at && T.now() - at < 30000;
    return '<button class="srun pact' + (busy ? ' busy' : '') + '" data-act="srun" data-key="' + esc(key) + '"' + (busy ? ' disabled' : '') +
      ' title="Ask the manager to run ' + esc(f.name) + ' again" aria-label="Retry ' + esc(f.name) + '">' + PLAY + '<span>' + (busy ? 'Asked' : 'Retry') + '</span></button>';
  }

  function runRow(p, open) {
    var st = p.stages || [];
    return '<div class="prow2 s-' + esc(p.status) + (open ? ' open' : '') + '" data-run="' + esc(p.run) + '">' +
      '<span class="pc">' + badge(p) + '</span>' +
      '<span class="pc pid"><button class="plink" data-act="prun" data-key="' + esc(p.run) + '" aria-expanded="' + !!open + '">' + T.ghMark(p) + esc(T.runName(p)) + ' <span class="mut">' + esc(T.runNum(p)) + '</span></button>' + chip(p.project) + '</span>' +
      '<span class="pc ptr">' + trigger(p) + '</span>' +
      '<span class="pc pmini">' + st.map(function (x, i) { return circle(p, x, i, open && (ui.stageSel[p.run] != null ? ui.stageSel[p.run] : T.defaultStage(p)) === i); }).join('') + '</span>' +
      '<span class="pc pwhen" title="' + esc(T.clockTime(p.started_at)) + '">' + T.agoSpan(p.started_at) + (T.ago(p.started_at) === 'now' ? '' : '<span class="mut"> ago</span>') + '</span>' +
      '<span class="pc pacts">' + retry(p) + '<button class="pact ghost" data-act="plogs" data-key="' + esc(p.run) + '">Logs</button></span></div>';
  }

  function detail(p) {
    var st = p.stages || [], si = ui.stageSel[p.run] != null ? ui.stageSel[p.run] : T.defaultStage(p), cur = st[si] || st[0];
    if (!cur) return '';
    var list = st.map(function (x, i) {
      return '<button class="psl s-' + esc(x.status) + (i === si ? ' on' : '') + '" data-act="pstage" data-key="' + esc(p.run) + '" data-i="' + i + '"><span class="pc22 s-' + esc(x.status) + '">' + glyph(x.status, 12) + '</span>' +
        '<span class="psn">' + esc(x.name) + '</span><span class="mut sm">' + (T.stageDur(x) || '') + '</span></button>';
    }).join('');
    var cmd = stageCmd(p, cur), ran = cur.status !== 'pending' && cur.status !== 'skipped';
    var desc = T.stageDesc(cur);
    return '<div class="pdet"><div class="psls">' + list + '</div><div class="pdr"><header><b>' + esc(cur.name) + '</b><span class="pbadge s-' + esc(cur.status) + '">' + glyph(cur.status, 11) + '<span>' + esc(LABEL[cur.status] || cur.status) + '</span></span>' +
      (T.stageDur(cur) ? '<span class="mut sm">' + T.stageDur(cur) + '</span>' : '') + T.ghLink({ source: p.source, html_url: cur.html_url }) +
      '<button class="x" data-act="pclose" aria-label="Close">' + T.lineIcon(T.ICON.close, 13, 'currentColor', 2) + '</button></header>' +
      (cmd ? '<code class="pcmd">' + esc(cmd) + '</code>' : desc ? '<span class="mut sm">' + esc(desc) + '</span>' : '') +
      (ran ? T.logBox(cur.log_tail) : '<p class="quiet">' + (cur.status === 'skipped' ? 'Skipped, an earlier stage failed' : 'Not run yet') + '</p>') + '</div></div>';
  }

  function dayKey(p) { var d = new Date(startMs(p)); return d.getFullYear() * 400 + d.getMonth() * 32 + d.getDate(); }
  function dayLabel(p) {
    var d = new Date(startMs(p)), n = new Date(T.now()), y = new Date(T.now() - 86400000);
    if (d.toDateString() === n.toDateString()) return 'Today';
    if (d.toDateString() === y.toDateString()) return 'Yesterday';
    return ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'][d.getDay()] + ', ' +
      ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'][d.getMonth()] + ' ' + d.getDate();
  }

  function summary(runs) {
    var today = new Date(T.now()).toDateString(), done = runs.filter(function (p) { return p.status === 'passed' || p.status === 'failed'; });
    var ok = done.filter(function (p) { return p.status === 'passed'; }).length;
    var secs = done.map(function (p) { return T.elapsed(p.started_at, p.ended_at); }).filter(function (x) { return x != null; }).sort(function (a, b) { return a - b; });
    var med = secs.length ? (secs.length % 2 ? secs[(secs.length - 1) / 2] : Math.round((secs[secs.length / 2 - 1] + secs[secs.length / 2]) / 2)) : null;
    var run = runs.filter(function (p) { return p.status === 'running'; }).length;
    var failed = done.length - ok;
    function stat(v, l, cls) { return '<div class="pstat ' + (cls || '') + '"><b>' + v + '</b><span>' + l + '</span></div>'; }
    return '<div class="psum card">' +
      stat(runs.filter(function (p) { return new Date(startMs(p)).toDateString() === today; }).length, 'runs today') +
      stat(done.length ? Math.round(100 * ok / done.length) + '%' : '-', 'pass rate' + (failed ? ' · ' + failed + ' failed' : ''), failed ? 'bad' : 'good') +
      stat(med == null ? '-' : esc(T.dur(med)), 'median duration') +
      stat(run, 'running now', run ? 'live' : '') + '</div>';
  }

  /* declared in tirith.toml, never run: a Run button per stage (the Setup view's inbox path) */
  function never(list) {
    if (!list.length) return '';
    return '<section class="pnever"><h3>Declared, never run <span class="n">' + list.length + '</span></h3>' + list.map(function (p) {
      var chips = (p.stages || []).map(function (s) {
        var cmd = stageCmd(p, s), key = p.project + '|' + p.pipeline + '|' + s.name, at = ui.setupPend && ui.setupPend[key + '|run'], busy = at && T.now() - at < 30000;
        return '<li class="schip none"><span class="sname">' + esc(s.name) + '</span>' + (cmd ? '<code title="' + esc(cmd) + '">' + esc(cmd) + '</code>' : '') +
          (cmd ? '<button class="srun' + (busy ? ' busy' : '') + '" data-act="srun" data-key="' + esc(key) + '"' + (busy ? ' disabled' : '') + ' aria-label="Run ' + esc(s.name) + '">' + PLAY + '<span>' + (busy ? 'Asked' : 'Run') + '</span></button>' : '') + '</li>';
      }).join('');
      return '<div class="pnrow"><span class="pnname"><b>' + esc(T.runName(p)) + '</b>' + chip(p.project) + '</span><ul class="schips">' + chips + '</ul></div>';
    }).join('') + '</section>';
  }

  /* when a Retry/Run cannot be sent from this page the Setup code keeps the command to copy */
  function notices(runs) {
    var out = '';
    Object.keys(ui.setupCli || {}).forEach(function (pid) {
      if (!ui.setupCli[pid] || !runs.some(function (p) { return p.project === pid; })) return;
      out += '<div class="icli pnote"><span>Tirith cannot send it from this page. Run it in a terminal:</span><code>' + esc(ui.setupCli[pid]) + '</code>' +
        '<button class="alink" data-act="copy" data-key="' + esc(ui.setupCli[pid]) + '">Copy</button><button class="alink" data-act="pdismiss" data-key="' + esc(pid) + '">Dismiss</button></div>';
    });
    Object.keys(ui.setupErr || {}).forEach(function (pid) {
      if (ui.setupErr[pid] && runs.some(function (p) { return p.project === pid; })) out += '<div class="ierr">' + esc(ui.setupErr[pid]) + '</div>';
    });
    return out;
  }

  function view(s, el) {
    var all = (s.pipelines || []).slice().sort(function (a, b) { return startMs(b) - startMs(a); });
    if (!all.length) { el.innerHTML = '<div class="pad">' + T.empty('No pipelines in this session') + '</div>'; return; }
    var runs = all.filter(function (p) { return p.status !== 'pending'; }), pend = all.filter(function (p) { return p.status === 'pending'; });
    if (ui.pOpenStage) {   /* the URL named a stage */
      var op = all.filter(function (p) { return p.run === ui.pOpen; })[0];
      if (op) (op.stages || []).forEach(function (x, i) { if (x.name === ui.pOpenStage) ui.stageSel[op.run] = i; });
      ui.pOpenStage = null;
    }
    var h = '', last = null;
    runs.forEach(function (p) {
      var k = dayKey(p);
      if (k !== last) {
        last = k;
        h += '<h3 class="pday">' + esc(dayLabel(p)) + ' <span class="n">' + runs.filter(function (x) { return dayKey(x) === k; }).length + '</span></h3>';
      }
      var open = ui.pOpen === p.run;
      h += runRow(p, open) + (open ? detail(p) : '');
    });
    el.innerHTML = '<div class="pgl" data-scroll="pgl">' + summary(runs) + notices(all) +
      (runs.length ? '<div class="ptable"><div class="phead"><span>Status</span><span>Run</span><span>Triggered by</span><span>Stages</span><span>Started</span><span></span></div>' + h + '</div>'
        : '<div class="pad">' + T.empty('No runs yet') + '</div>') + never(pend) + '</div>';
  }
  T.views.pipelines = view;

  /* ---------- tooltip: one floating box, placed by script so no scroll area clips it ---------- */
  var tip = null;
  function showTip(b) {
    var run = b.getAttribute('data-key'), i = +b.getAttribute('data-i');
    var p = (raw().pipelines || []).filter(function (x) { return x.run === run; })[0], st = p && p.stages[i];
    if (!st) return;
    var cmd = stageCmd(p, st), d = T.stageSecs(st), lines = st.status === 'failed' ? (st.log_tail || []).map(T.stripAnsi).filter(function (x) { return String(x).trim(); }).slice(-3) : [];
    if (!tip) { tip = document.createElement('div'); tip.className = 'ptip'; tip.setAttribute('role', 'tooltip'); document.body.appendChild(tip); }
    tip.innerHTML = '<div class="pth">Stage: ' + esc(st.name) + '</div>' + (cmd ? '<code class="ptc">' + esc(cmd) + '</code>' : '') +
      '<div class="ptl"><span class="pc22 s-' + esc(st.status) + '">' + glyph(st.status, 12) + '</span>' + esc(st.name) + ': ' + esc((LABEL[st.status] || st.status).toLowerCase()) + (d != null && st.status !== 'pending' && st.status !== 'skipped' ? ' · ' + esc(T.dur(d)) : '') + '</div>' +
      (lines.length ? '<pre class="ptlog">' + esc(lines.join('\n')) + '</pre>' : '');
    tip.style.display = 'block';
    var r = b.getBoundingClientRect(), w = tip.offsetWidth, ht = tip.offsetHeight;
    var x = Math.max(8, Math.min(window.innerWidth - w - 8, r.left + r.width / 2 - w / 2));
    var y = r.bottom + 8 + ht > window.innerHeight - 8 ? r.top - ht - 8 : r.bottom + 8;
    tip.style.left = x + 'px'; tip.style.top = Math.max(8, y) + 'px';
  }
  function hideTip() { if (tip) tip.style.display = 'none'; }
  var v = document.getElementById('view');
  v.addEventListener('mouseover', function (e) { var b = e.target.closest && e.target.closest('.pmini [data-act="pstage"]'); if (b) showTip(b); });
  v.addEventListener('mouseout', function (e) { if (e.target.closest && e.target.closest('.pmini [data-act="pstage"]')) hideTip(); });
  v.addEventListener('focusin', function (e) { var b = e.target.closest && e.target.closest('.pmini [data-act="pstage"]'); if (b) showTip(b); });
  v.addEventListener('focusout', hideTip);

  /* ---------- clicks: a row, a circle, Logs, close ---------- */
  function go() { hideTip(); T.setFilter('view', 'pipelines'); }
  v.addEventListener('click', function (e) {
    var b = e.target.closest('[data-act]'), row = e.target.closest('.prow2'), act = b && b.getAttribute('data-act'), key = b && b.getAttribute('data-key');
    if (!row && act !== 'pclose' && act !== 'pstage' && act !== 'pdismiss') return;
    if (act === 'pstage') { ui.pOpen = key; ui.stageSel[key] = +b.getAttribute('data-i'); ui.drawer = null; }
    else if (act === 'plogs') { ui.pOpen = key; delete ui.stageSel[key]; }
    else if (act === 'pclose') ui.pOpen = null;
    else if (act === 'pdismiss') { ui.setupCli[key] = null; ui.setupErr[key] = null; }
    else if (act === 'prun' || (!b && row && !e.target.closest('a'))) { var rk = row.getAttribute('data-run'); ui.pOpen = ui.pOpen === rk ? null : rk; }
    else return;
    go();
  });
  document.addEventListener('keydown', function (e) {
    if (e.key === 'Escape' && ui.pOpen && T.filter.view === 'pipelines' && !/^(INPUT|SELECT|TEXTAREA)$/.test(document.activeElement.tagName)) { ui.pOpen = null; go(); }
  });
})();
