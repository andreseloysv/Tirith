/* Pipelines view: run list (left) + stage graph (right). */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;

  function sorted(l) { return l.slice().sort(function (a, b) { return (T.ms(b.started_at) || 0) - (T.ms(a.started_at) || 0); }); }

  function runCard(p, sel) {
    var cur = T.curStage(p), pct = T.pct(p), pn = T.proj(p.project), body;
    var col = p.status === 'failed' ? 'red' : p.status === 'running' ? 'blue' : p.status === 'pending' ? 'grey' : 'green';
    if (p.status === 'pending') body = '<span class="mut sm">Declared in tirith.toml, not run yet</span>';
    else if (p.status === 'running') body = '<span class="rline"><span class="rcur">' + esc(cur ? cur.name : '') + '</span><span class="mut">' + pct + '%</span></span>';
    else if (p.status === 'failed') body = '<span class="rerr">' + esc(T.errLine(p)) + '</span><span class="mut sm">' + esc(T.ago(p.ended_at)) + ' ago</span>';
    else body = '<span class="mut sm">' + esc(T.ago(p.ended_at)) + ' ago</span>';
    return '<button class="card runcard' + (sel ? ' sel' : '') + '" data-act="run" data-key="' + esc(p.run) + '" aria-pressed="' + sel + '">' +
      '<span class="rhead"><b>' + esc(T.runName(p)) + ' <span class="mut">' + esc(T.runNum(p)) + '</span></b><span class="mut sm">' + esc(pn) + '</span></span>' +
      '<span class="bar"><i class="' + col + '" style="width:' + (p.status === 'running' ? pct : p.status === 'pending' ? 0 : 100) + '%"></i></span>' + body + '</button>';
  }

  function stageNode(p, st, i, sel) {
    var n = (p.stages || []).length, ic = T.stageIcon(st.name, 20), pend = st.status === 'pending' || st.status === 'skipped';
    var dur = T.stageDur(st);
    return (i ? '<i class="conn ' + (p.stages[i - 1].status === 'passed' ? 'on' : '') + '"></i>' : '') +
      '<button class="node snode s-' + esc(st.status) + (sel ? ' sel' : '') + '" data-act="stage" data-key="' + esc(p.run) + '" data-i="' + i + '" title="' + esc(st.status) + '">' +
      '<span class="gico ' + esc(st.status) + '" style="background:' + ic.bg + '">' + ic.svg + '</span>' +
      '<span class="nt"><span class="ov">Step ' + (i + 1) + ' of ' + n + '</span><span class="tt">' + esc(st.name) + '</span><span class="ds">' + esc(T.stageDesc(st)) + '</span></span>' +
      (dur ? '<span class="dpill">' + T.lineIcon(T.ICON.play, 9, 'currentColor', 0).replace('fill="none"', 'fill="currentColor"') + dur + '</span>' : '') + '</button>';
  }

  function view(s, el) {
    var all = sorted(s.pipelines || []);
    if (!all.length) { el.innerHTML = '<div class="pad">' + T.empty('No pipelines in this session') + '</div>'; return; }
    var sel = all.filter(function (p) { return p.run === ui.selRun; })[0] ||
      all.filter(function (p) { return p.status === 'running'; })[0] || all.filter(function (p) { return p.status === 'failed'; })[0] || all[0];
    ui.selRun = sel.run;
    var groups = [['running', 'Running'], ['failed', 'Failed'], ['passed', 'Passed'], ['pending', 'Not run yet']];
    var list = groups.map(function (g) {
      var l = all.filter(function (p) { return p.status === g[0]; });
      return l.length ? '<h3>' + g[1] + ' <span class="n">' + l.length + '</span></h3>' + l.map(function (p) { return runCard(p, p.run === sel.run); }).join('') : '';
    }).join('');
    var st = sel.stages || [], si = ui.stageSel[sel.run] != null ? ui.stageSel[sel.run] : T.defaultStage(sel);
    var total = sel.status === 'running' ? T.span(sel.started_at, null) : sel.status === 'pending' ? '' : esc(T.dur(T.elapsed(sel.started_at, sel.ended_at)));
    var graph = '<div class="flowcol">' + st.map(function (x, i) { return stageNode(sel, x, i, ui.drawer === 'stage' && i === si); }).join('') + '</div>';
    var drawer = ui.drawer === 'stage' && st[si] ? T.drawer('Log', T.logBox(st[si].log_tail)) : '';
    el.innerHTML = '<div class="pv"><div class="plist" data-scroll="plist">' + list + '</div><div class="stagearea">' +
      T.canvas(graph, '<span class="sepv"></span><span class="tot">Total: ' + total + '</span>') + drawer + '</div></div>';
  }
  T.views.pipelines = view;
})();
