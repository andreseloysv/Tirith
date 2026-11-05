/* «Accesos» view: what each agent touched (state.accesses, docs/CONTRACT-access-audit.md). Only for looking things
   up: no alerts, nothing in Your turn, no counter in the sidebar. Grouped by agent, filtered by system and date,
   each group linked to the agent and to the plan point it works on. Agents are shown by alias, never by product. */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui, DAY = 86400000;
  var f = ui.acc = ui.acc || { sys: '', days: '7', open: {} };
  var RANGES = [['1', 'Today'], ['7', '7 days'], ['30', '30 days'], ['', 'All']];

  function inRange(row, days) {
    if (!days) return true;
    var t = T.ms(row.ts); if (t == null) return false;
    var d = new Date(T.now()); d.setHours(0, 0, 0, 0);
    return t >= d.getTime() - (+days - 1) * DAY;
  }
  /* rows kept by the date and system filters, grouped by agent (alias + lane) */
  T.accessGroups = function (s, sys, days) {
    var rows = (s.accesses || []).filter(function (r) { return (!sys || r.system === sys); });
    rows = rows.filter(function (r) { return inRange(r, days); });
    var by = {}, out = [];
    rows.forEach(function (r) {
      var k = (r.agent || '?') + '|' + (r.lane || '');
      if (!by[k]) { by[k] = { key: k, agent: r.agent || null, lane: r.lane || null, project: r.project, plan: r.plan, rows: [] }; out.push(by[k]); }
      by[k].rows.push(r); by[k].plan = r.plan || by[k].plan;
    });
    out.forEach(function (g) { g.last = g.rows[g.rows.length - 1].ts; });
    return out.sort(function (a, b) { return String(b.last).localeCompare(String(a.last)); });
  };
  function chips(list, cur, act) {
    return '<div class="seg" role="group">' + list.map(function (x) {
      return '<button data-act="' + act + '" data-key="' + esc(x[0]) + '" class="' + (cur === x[0] ? 'on' : '') + '">' + esc(x[1]) + '</button>';
    }).join('') + '</div>';
  }
  function group(s, g) {
    var a = g.lane ? T.agentById(s, g.lane) : null, open = !!f.open[g.key];
    var who = a ? '<button class="iag" data-act="goagent" data-key="' + esc(a.id) + '" title="Open this agent">' + T.avatar(a, 18) + '<b>' + esc(a.name) + '</b></button>'
      : '<b>' + esc(g.agent || 'Unknown agent') + '</b>';
    var plan = g.plan ? (g.project ? '<button class="link" data-act="goplan" data-key="' + esc(g.project) + '" title="Open the plan">' + esc(g.plan) + '</button>' : esc(g.plan)) : '<span class="mut">no plan point</span>';
    var h = '<li class="row' + (open ? ' open' : '') + '"><div class="head"><button class="chvb" data-act="accopen" data-key="' + esc(g.key) + '" aria-expanded="' + open + '"><span class="chv"></span></button>' +
      who + ' <span class="mut sm">' + esc(g.agent || '') + '</span><span class="feat">' + plan + '</span>' +
      '<span class="mut proj">' + esc(g.project ? T.proj(g.project) : '') + '</span><span class="mut sm">' + g.rows.length + ' · ' + esc(T.clockTime(g.last)) + '</span></div>';
    if (open) h += '<div class="detail"><table class="mtab"><tbody>' + g.rows.slice().reverse().map(function (r) {
      return '<tr><td class="sm">' + esc(T.clockTime(r.ts)) + '</td><td class="sm">' + esc(r.system || '') + '</td><td>' + esc(r.action || r.tool || '') +
        ((r.external || []).length ? ' <span class="mut sm">→ ' + esc(r.external.join(', ')) + '</span>' : '') + '</td></tr>';
    }).join('') + '</tbody></table></div>';
    return h + '</li>';
  }
  /* what reached outside the machine (a web page, an API, a websocket, ssh, a git remote, a package registry,
     a search, an MCP server), newest first: from the commands and from the connections the processes really opened */
  var KINDS = { web: 'Web', api: 'API', websocket: 'WebSocket', ssh: 'SSH', git: 'Git remote', github: 'GitHub',
    'package': 'Packages', search: 'Search', mcp: 'MCP', https: 'HTTPS', http: 'HTTP', tcp: 'TCP', ftp: 'FTP' };
  T.externalRows = function (s, days) {
    var out = [];
    (s.accesses || []).forEach(function (r) {
      if (!inRange(r, days)) return;
      (r.external || []).forEach(function (x) {
        var sp = String(x).indexOf(' ');
        out.push({ ts: r.ts, kind: sp > 0 ? String(x).slice(0, sp) : String(x), dest: sp > 0 ? String(x).slice(sp + 1) : '',
          row: r, live: r.tool === 'connection' });
      });
    });
    return out.sort(function (a, b) { return String(b.ts).localeCompare(String(a.ts)); });
  };
  function externalSection(s) {
    var rows = T.externalRows(s, f.days);
    var body = rows.length ? '<table class="mtab ext"><tbody>' + rows.slice(0, 200).map(function (x) {
      var a = x.row.lane ? T.agentById(s, x.row.lane) : null;
      var who = a ? '<button class="link" data-act="goagent" data-key="' + esc(a.id) + '">' + esc(T.agentRole ? T.agentRole(a) : a.name) + ' · ' + esc(a.name) + '</button>' : esc(x.row.lane || x.row.agent || '');
      return '<tr><td class="sm">' + esc(T.clockTime(x.ts)) + '</td><td><span class="chip xk ' + esc(x.kind) + '">' + esc(KINDS[x.kind] || x.kind) + '</span></td>' +
        '<td><b>' + esc(x.dest) + '</b></td><td class="sm">' + who + '</td><td class="sm mut" title="' + esc(x.row.action || '') + '">' +
        esc(x.live ? 'connection opened' : (x.row.action || x.row.tool || '')) + '</td></tr>';
    }).join('') + '</tbody></table>' : '<p class="mut sm">Nothing reached outside the machine in this period.</p>';
    return '<section class="pad extsec"><h2>External <span class="mut">' + rows.length + '</span></h2>' + body + '</section>';
  }
  function view(s, el) {
    var all = s.accesses || [], systems = [['', 'All systems']];
    var seen = {}; all.forEach(function (r) { if (r.system && !seen[r.system]) { seen[r.system] = 1; systems.push([r.system, r.system]); } });
    var tb = '<div class="toolbar">' + chips(systems, f.sys, 'accsys') + ' ' + chips(RANGES, f.days, 'accdays') + '</div>';
    var gs = T.accessGroups(s, f.sys, f.days);
    el.innerHTML = tb + externalSection(s) + '<div class="pad"><p class="mut sm">Everything the agents did, recorded by Tirith. Values of secrets are never stored.</p></div>' +
      (gs.length ? '<div class="listwrap" data-scroll="accesos"><ul class="rows card">' + gs.map(function (g) { return group(s, g); }).join('') + '</ul></div>'
        : '<div class="pad">' + T.empty('No accesses recorded for these filters') + '</div>');
  }
  T.views.accesos = view;

  document.getElementById('view').addEventListener('click', function (e) {
    var b = e.target.closest('[data-act]'); if (!b) return;
    var act = b.getAttribute('data-act'), key = b.getAttribute('data-key');
    if (act === 'accsys') f.sys = key;
    else if (act === 'accdays') f.days = key;
    else if (act === 'accopen') f.open[key] = !f.open[key];
    else if (act === 'goplan') { T.filter.project = key; T.setFilter('view', 'overview'); return; }
    else return;
    T.render();
  });
})();
