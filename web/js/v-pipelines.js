/* Pipelines view: a compact GitLab-like run list. One ~36 px row per run (Running first, then by day), the stage
   chips in order with their names, the failing stage named in red. A click opens the run in place (accordion):
   stages, commands and the last log lines. "Run again" and "Run" go through the Setup view's inbox path (data-act="srun"). */
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
  /* the stage mark: ✓ ✗ » ○ and a pulsing dot */
  function mark(status) {
    return status === 'running' ? '<i class="pdot"></i>' : status === 'pending' ? '<i class="pring"></i>' : glyph(status, 11);
  }
  function chip(p, st, i, sel) {
    return '<button class="pchp s-' + esc(st.status) + (sel ? ' sel' : '') + '" data-act="pstage" data-key="' + esc(p.run) + '" data-i="' + i + '" title="' + esc(st.name + ': ' + (LABEL[st.status] || st.status)) + '">' + mark(st.status) + '<span>' + esc(st.name) + '</span></button>';
  }
  function failing(p) {
    var f = (p.stages || []).filter(function (x) { return x.status === 'failed'; })[0];
    return f ? '<span class="pfail" title="' + esc(T.errLine(p)) + '">' + (f.reason === 'interrupted' ? 'interrupted at ' : 'failed at ') + '<b>' + esc(f.name) + '</b></span>' : '';
  }
  function projChip(pid) {
    var n = T.proj(pid), pr = projectOf(pid);
    return n ? '<span class="pchip"><i style="background:' + esc((pr && pr.color) || '#9aa1ac') + '"></i>' + esc(n) + '</span>' : '';
  }
  function selIndex(p) { return ui.stageSel[p.run] != null ? ui.stageSel[p.run] : T.defaultStage(p); }

  function runRow(p, open) {
    var st = p.stages || [], si = open ? selIndex(p) : -1, dur = p.status === 'running' ? T.span(p.started_at, null) : esc(T.dur(T.elapsed(p.started_at, p.ended_at)));
    return '<div class="prow2 s-' + esc(p.status) + (open ? ' open' : '') + '" data-run="' + esc(p.run) + '">' +
      '<span class="pst s-' + esc(p.status) + '" title="' + esc(LABEL[p.status] || p.status) + '">' + (p.status === 'pending' ? '<i class="pring"></i>' : glyph(p.status, 12)) + '</span>' +
      '<span class="pc pid"><button class="plink" data-act="prun" data-key="' + esc(p.run) + '" aria-expanded="' + !!open + '">' + T.ghMark(p) + esc(T.runName(p)) + ' <span class="mut">' + esc(T.runNum(p)) + '</span></button>' + projChip(p.project) + '</span>' +
      '<span class="pc ptr">' + trigger(p) + '</span>' +
      '<span class="pc pmini">' + st.map(function (x, i) { return chip(p, x, i, i === si); }).join('') + failing(p) + '</span>' +
      '<span class="pc pdur">' + (p.status === 'pending' ? '' : dur) + '</span>' +
      '<span class="pc pwhen" title="' + esc(T.clockTime(p.started_at)) + '">' + T.agoSpan(p.started_at) + (T.ago(p.started_at) === 'now' ? '' : '<span class="mut"> ago</span>') + '</span></div>';
  }
  T.pipeRow = runRow;

  function runAgain(p, cur) {
    if (!cur || p.source === 'github' || p.pipeline === 'git-hooks' || !stageCmd(p, cur)) return '';
    var key = p.project + '|' + p.pipeline + '|' + cur.name, at = ui.setupPend && ui.setupPend[key + '|run'];
    var busy = at && T.now() - at < 30000;
    return '<button class="srun pact' + (busy ? ' busy' : '') + '" data-act="srun" data-key="' + esc(key) + '"' + (busy ? ' disabled' : '') +
      ' title="Ask the manager to run ' + esc(cur.name) + ' again">' + PLAY + '<span>' + (busy ? 'Asked' : 'Run again') + '</span></button>';
  }

  /* the accordion body: the stages (name, command, time, status) and the last log lines of the selected one */
  function detail(p) {
    var st = p.stages || [], si = selIndex(p), cur = st[si] || st[0];
    if (!cur) return '<div class="pdet"><p class="quiet">No stages</p></div>';
    var list = st.map(function (x, i) {
      var cmd = stageCmd(p, x);
      return '<button class="psl s-' + esc(x.status) + (i === si ? ' on' : '') + '" data-act="pstage" data-key="' + esc(p.run) + '" data-i="' + i + '">' + mark(x.status) +
        '<span class="psn">' + esc(x.name) + '</span><code class="psc">' + esc(cmd) + '</code><span class="mut sm psd">' + (T.stageDur(x) || '') + '</span>' +
        '<span class="pstt s-' + esc(x.status) + '">' + esc(x.reason === 'interrupted' ? 'Interrupted' : LABEL[x.status] || x.status) + '</span></button>';
    }).join('');
    var ran = cur.status !== 'pending' && cur.status !== 'skipped';
    var lines = (cur.log_tail || []).slice(-20);
    return '<div class="pdet"><div class="psls">' + list + '</div><div class="pdr"><header><b>' + esc(cur.name) + '</b>' +
      (ran ? '<span class="mut sm">last ' + lines.length + ' log lines</span>' : '') + T.ghLink({ source: p.source, html_url: cur.html_url }) +
      '<span class="pdacts">' + (ran ? '<button class="pact ghost" data-act="pfull" data-key="' + esc(p.run) + '">Full log</button>' : '') + runAgain(p, cur) + '</span></header>' +
      (ran ? T.logBox(lines) : '<p class="quiet">' + (cur.status === 'skipped' ? 'Skipped, nothing ran after an earlier stage ended the run' : 'Not run yet') + '</p>') + '</div></div>';
  }

  function dayKey(p) { var d = new Date(startMs(p)); return d.getFullYear() * 400 + d.getMonth() * 32 + d.getDate(); }
  function dayLabel(p) {
    var d = new Date(startMs(p)), n = new Date(T.now()), y = new Date(T.now() - 86400000);
    if (d.toDateString() === n.toDateString()) return 'Today';
    if (d.toDateString() === y.toDateString()) return 'Yesterday';
    return ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'][d.getDay()] + ', ' +
      ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'][d.getMonth()] + ' ' + d.getDate();
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
      return '<div class="pnrow"><span class="pnname"><b>' + esc(T.runName(p)) + '</b>' + projChip(p.project) + '</span><ul class="schips">' + chips + '</ul></div>';
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

  function group(title, rows) {
    return '<h3 class="pday">' + esc(title) + ' <span class="n">' + rows.length + '</span></h3>' + rows.map(function (p) {
      return runRow(p, ui.pOpen === p.run) + (ui.pOpen === p.run ? detail(p) : '');
    }).join('');
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
    var live = runs.filter(function (p) { return p.status === 'running'; }), h = '', last = null, bucket = [];
    function flush() { if (bucket.length) h += group(dayLabel(bucket[0]), bucket); bucket = []; }
    if (live.length) h += group('Running', live);
    runs.filter(function (p) { return p.status !== 'running'; }).forEach(function (p) {
      var k = dayKey(p);
      if (k !== last) { flush(); last = k; }
      bucket.push(p);
    });
    flush();
    el.innerHTML = '<div class="pgl" data-scroll="pgl">' + notices(all) +
      (runs.length ? '<div class="ptable">' + h + '</div>' : '<div class="pad">' + T.empty('No runs yet') + '</div>') + never(pend) + '</div>';
    var full = ui.drawer === 'pfull' && ui.pFull && all.filter(function (p) { return p.run === ui.pFull; })[0];
    if (full) {
      var cur = (full.stages || [])[selIndex(full)];
      if (cur) el.insertAdjacentHTML('beforeend', T.drawer(T.runName(full) + ' ' + T.runNum(full) + ' · ' + cur.name, T.logBox(cur.log_tail)));
    }
  }
  T.views.pipelines = view;

  /* ---------- clicks: a row (opens in place, one at a time), a stage, Full log, close ---------- */
  var v = document.getElementById('view');
  function go() { T.setFilter('view', 'pipelines'); }
  v.addEventListener('click', function (e) {
    var b = e.target.closest('[data-act]'), row = e.target.closest('.prow2'), act = b && b.getAttribute('data-act'), key = b && b.getAttribute('data-key');
    if (!row && act !== 'pstage' && act !== 'pdismiss' && act !== 'pfull') return;
    if (act === 'pstage') { ui.pOpen = key; ui.stageSel[key] = +b.getAttribute('data-i'); ui.drawer = null; }
    else if (act === 'pfull') { ui.pFull = key; ui.drawer = 'pfull'; }
    else if (act === 'pdismiss') { ui.setupCli[key] = null; ui.setupErr[key] = null; }
    else if (act === 'prun' || (!b && row && !e.target.closest('a'))) { var rk = row.getAttribute('data-run'); ui.pOpen = ui.pOpen === rk ? null : rk; ui.drawer = null; }
    else return;
    go();
  });
  document.addEventListener('keydown', function (e) {
    if (e.key === 'Escape' && ui.pOpen && !ui.drawer && T.filter.view === 'pipelines' && !/^(INPUT|SELECT|TEXTAREA)$/.test(document.activeElement.tagName)) { ui.pOpen = null; go(); }
  });
})();
