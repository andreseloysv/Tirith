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
  /* the card beside the drawer already shows the feature and the clock; the file list replaces the totals line */
  function detail(a) {
    var ch = a.files && a.files.length ? '' : T.agentFiles(a);
    return T.ghLink(a) + T.activity(a, { goal: !!a.goal && a.goal !== a.feature, clock: false }) + (ch ? '<p class="mut files">' + ch + '</p>' : '') ;
  }

  function view(s, el) {
    var all = (s.agents || []).slice().sort(byStart);
    var mode = T.filter.mode;
    var tb = '<div class="toolbar"><div class="seg" role="group" aria-label="Layout">' +
      [['list', 'List'], ['kanban', 'Kanban'], ['tree', 'Tree']].map(function (m) {
        return '<button data-act="mode" data-v="' + m[0] + '" class="' + (mode === m[0] ? 'on' : '') + '">' + m[1] + '</button>';
      }).join('') + '</div></div>';
    if (!all.length) { el.innerHTML = tb + '<div class="pad">' + T.empty('No agents in this session') + '</div>'; return; }
    var old = all.filter(isOld), vis = ui.showEarlier ? all : all.filter(function (a) { return !isOld(a); });
    var more = old.length ? '<button class="link" data-act="earlier">' + (ui.showEarlier ? 'Hide earlier' : 'Show ' + old.length + ' earlier') + '</button>' : '';
    if (T.filter.mode === 'list') {
      var rank = function (a) { return a.status === 'working' ? 0 : a.status === 'stalled' ? 1 : 2; };
      vis.sort(function (a, b) { return rank(a) - rank(b) || byStart(a, b); });
      el.innerHTML = tb + '<div class="listwrap" data-scroll="alist"><ul class="rows card">' + vis.map(listRow).join('') + '</ul>' + more + '</div>';
      return;
    }
    var drawerFor = function () {
      var sel = ui.drawer === 'agent' && all.filter(function (a) { return a.id === ui.openAgent; })[0];
      return sel ? T.drawer(sel.name + (sel.epithet ? ' ' + sel.epithet : ''), detail(sel)) : '';
    };
    if (mode === 'tree') {
      el.innerHTML = tb + '<div class="listwrap treewrap" data-scroll="atree">' + tree(vis) + '</div>' + (more ? '<div class="moreline">' + more + '</div>' : '') + drawerFor();
      return;
    }
    var cols = [['working', 'Working', 'blue'], ['stalled', 'Stalled', 'amber'], ['done', 'Done', 'green'], ['failed', 'Failed', 'red']];
    var board = cols.map(function (c) {
      var l = vis.filter(function (a) { return a.status === c[0]; });
      return '<section class="col ' + c[2] + '"><header><b>' + c[1] + '</b><span class="n">' + l.length + '</span></header><div class="cbody" data-scroll="col-' + c[0] + '">' +
        (l.length ? l.map(function (a) { return T.agentCard(a, ui.drawer === 'agent' && ui.openAgent === a.id); }).join('') : '') + '</div></section>';
    }).join('');
    el.innerHTML = tb + '<div class="board" >' + board + '</div>' + (more ? '<div class="moreline">' + more + '</div>' : '') + drawerFor();
  }

  /* Org chart: the steward on top, each agent under the agent that spawned it. An agent whose
     parent is not shown (filtered out, dropped, unknown) hangs from the steward. */
  function tree(list) {
    var ids = {}, kids = {};
    list.forEach(function (a) { ids[a.id] = true; });
    list.forEach(function (a) {
      var p = a.parent && a.parent !== a.id && ids[a.parent] ? a.parent : 'steward';
      (kids[p] = kids[p] || []).push(a);
    });
    var seen = {};
    function branch(a) {
      if (seen[a.id]) return '';
      seen[a.id] = true;
      var sub = (kids[a.id] || []).slice().sort(byStartAsc).map(branch).join('');
      return '<li>' + T.agentCard(a, ui.drawer === 'agent' && ui.openAgent === a.id) + (sub ? '<ul>' + sub + '</ul>' : '') + '</li>';
    }
    var roots = (kids.steward || []).slice().sort(byStartAsc).map(branch).join('');
    list.forEach(function (a) { if (!seen[a.id]) roots += branch(a); });  /* parent cycles */
    return '<ul class="orgtree"><li><span class="orgroot">Steward</span><ul>' + roots + '</ul></li></ul>';
  }
  function byStartAsc(a, b) { return byStart(b, a); }
  T.views.agents = view;
})();
