/* Agents view: Kanban (default) or compact list. */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui, DAY = 86400000;

  function isOld(a) {
    if (a.status !== 'done' && a.status !== 'failed') return false;
    var e = T.ms(a.ended_at); return e != null && T.now() - e > DAY;
  }
  function byStart(a, b) { return (T.ms(b.started_at) || 0) - (T.ms(a.started_at) || 0); }

  function listRow(a) {
    var open = ui.openAgent === a.id, meta = [T.cap(a['class']), T.cap(a.rank)].filter(Boolean).join(' · ');
    var cls = a.status === 'working' ? 'blue pulse' : a.status === 'done' ? 'green' : a.status === 'failed' ? 'red' : 'amber';
    var h = '<li class="row' + (open ? ' open' : '') + '"><button class="head agent" data-act="agent" data-key="' + esc(a.id) + '" aria-expanded="' + open + '">' +
      '<span class="chv"></span>' + T.dot(cls, a.status) +
      '<span class="name" title="' + esc((a.name || '') + (a.epithet ? ' ' + a.epithet : '')) + '">' + esc(a.name) + ' <span class="mut sm">' + esc(meta) + '</span></span>' +
      '<span class="feat">' + esc(a.feature) + '</span><span class="mut proj">' + esc(T.proj(a.project)) + '</span><span class="time">' + T.agentClock(a) + '</span></button>';
    if (open) h += '<div class="detail">' + detail(a) + '</div>';
    return h + '</li>';
  }
  function detail(a) {
    var ch = T.agentFiles(a);
    return (a.summary ? '<p class="sum">' + esc(a.summary) + '</p>' : '') + (ch ? '<p class="mut files">' + ch + '</p>' : '') + T.logBox(a.log_tail);
  }

  function view(s, el) {
    var all = (s.agents || []).slice().sort(byStart);
    var tb = '<div class="toolbar"><div class="seg" role="group" aria-label="Layout"><button data-act="mode" data-v="list" class="' + (T.filter.mode === 'list' ? 'on' : '') + '">List</button><button data-act="mode" data-v="kanban" class="' + (T.filter.mode !== 'list' ? 'on' : '') + '">Kanban</button></div></div>';
    if (!all.length) { el.innerHTML = tb + '<div class="pad">' + T.empty('No agents in this session') + '</div>'; return; }
    var old = all.filter(isOld), vis = ui.showEarlier ? all : all.filter(function (a) { return !isOld(a); });
    var more = old.length ? '<button class="link" data-act="earlier">' + (ui.showEarlier ? 'Hide earlier' : 'Show ' + old.length + ' earlier') + '</button>' : '';
    if (T.filter.mode === 'list') {
      var rank = function (a) { return a.status === 'working' ? 0 : a.status === 'stalled' ? 1 : 2; };
      vis.sort(function (a, b) { return rank(a) - rank(b) || byStart(a, b); });
      el.innerHTML = tb + '<div class="listwrap" data-scroll="alist"><ul class="rows card">' + vis.map(listRow).join('') + '</ul>' + more + '</div>';
      return;
    }
    var cols = [['working', 'Working', 'blue'], ['stalled', 'Stalled', 'amber'], ['done', 'Done', 'green'], ['failed', 'Failed', 'red']];
    var board = cols.map(function (c) {
      var l = vis.filter(function (a) { return a.status === c[0]; });
      return '<section class="col ' + c[2] + '"><header><b>' + c[1] + '</b><span class="n">' + l.length + '</span></header><div class="cbody" data-scroll="col-' + c[0] + '">' +
        (l.length ? l.map(function (a) { return T.agentCard(a, ui.drawer === 'agent' && ui.openAgent === a.id); }).join('') : '') + '</div></section>';
    }).join('');
    var sel = ui.drawer === 'agent' && all.filter(function (a) { return a.id === ui.openAgent; })[0];
    var drawer = sel ? T.drawer(sel.name + (sel.epithet ? ' ' + sel.epithet : ''), detail(sel)) : '';
    el.innerHTML = tb + '<div class="board" >' + board + '</div>' + (more ? '<div class="moreline">' + more + '</div>' : '') + drawer;
  }
  T.views.agents = view;
})();
