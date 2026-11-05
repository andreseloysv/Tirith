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
  /* the agent's own program (its model API, telemetry): marked by the collector, or an older row whose program
     is named by its version ("2.1.318 connected to …") */
  function runtime(r) { return r.tool === 'connection' && (r.runtime === true || /^\d+\.\d+\.\d+ connected/.test(r.action || '')); }
  T.externalRows = function (s, days, sys) {
    var out = [];
    (s.accesses || []).forEach(function (r) {
      if (!inRange(r, days) || (sys && r.system !== sys)) return;
      (r.external || []).forEach(function (x) {
        var sp = String(x).indexOf(' '), dest = sp > 0 ? String(x).slice(sp + 1) : '';
        if (r.domain && r.address && dest.indexOf(r.address) === 0) dest = r.domain + dest.slice(r.address.length);
        out.push({ ts: r.ts, kind: sp > 0 ? String(x).slice(0, sp) : String(x), dest: dest,
          row: r, live: r.tool === 'connection', runtime: runtime(r) });
      });
    });
    return out.sort(function (a, b) { return String(b.ts).localeCompare(String(a.ts)); });
  };
  /* the agents' own traffic, one line per destination: how many connections, which roles, the last one */
  function runtimeLines(s, rows) {
    var by = {}, keys = [];
    rows.forEach(function (x) {
      if (!by[x.dest]) { by[x.dest] = { dest: x.dest, n: 0, last: x.ts, who: {}, org: x.row.org || '' }; keys.push(x.dest); }
      var g = by[x.dest], a = x.row.lane ? T.agentById(s, x.row.lane) : null;
      g.n++; g.who[a && T.agentRole ? T.agentRole(a) : 'Agent'] = 1;
    });
    return keys.map(function (k) {
      var g = by[k];
      return '<tr class="rt"><td class="sm">' + esc(T.clockTime(g.last)) + '</td><td><span class="chip xk rt">Agent runtime</span></td><td><b>' +
        esc(g.dest) + '</b>' + (g.org ? ' <span class="mut sm">' + esc(g.org) + '</span>' : '') + '</td><td class="sm">' +
        esc(Object.keys(g.who).join(', ')) + '</td><td class="sm mut">' + g.n + (g.n === 1 ? ' connection' : ' connections') +
        ' · opened by the agent program itself, not by a command it ran</td><td class="sm">' + blockBtn(s, g.dest) + '</td></tr>';
    }).join('');
  }
  /* blocking: a setup request (action net-block / net-unblock) that Tirith applies at once */
  ui.netPend = ui.netPend || {}; ui.netErr = ui.netErr || null; ui.netCli = ui.netCli || null;
  function hostOf(dest) { var d = String(dest || ''); return d.indexOf(':') > 0 && d.split(':').length === 2 ? d.split(':')[0] : d; }
  function blockedBy(s, host) {
    var h = String(host || '').toLowerCase();
    return (s.netblock || []).filter(function (r) { var m = r.match; return h === m || h.slice(-(m.length + 1)) === '.' + m; })[0] || null;
  }
  function netSend(action, match) {
    var st = T.rawState && T.rawState(), L = st && st.launch, tauri = window.__TAURI__;
    var text = JSON.stringify({ action: action, match: match });
    ui.netErr = null; ui.netCli = null;
    function sent() { ui.netPend[match] = action; T.render(); setTimeout(function () { delete ui.netPend[match]; T.render(); }, 8000); }
    if (tauri && tauri.core && tauri.core.invoke) {
      tauri.core.invoke('inbox_add', { kind: 'setup', project: 'all', step: 'setup', text: text }).then(sent, function (err) { ui.netErr = 'Could not send: ' + err; T.render(); });
    } else if (L && L.token && L.available) {
      var a = document.createElement('a');
      a.href = 'tirith://inbox?kind=setup&project=all&step=setup&text=' + encodeURIComponent(text) + '&token=' + encodeURIComponent(L.token);
      a.style.display = 'none'; document.body.appendChild(a); a.click(); document.body.removeChild(a); sent();
    } else { ui.netCli = 'tirith net ' + (action === 'net-block' ? 'block ' : 'unblock ') + match; T.render(); }
  }
  function blockBtn(s, dest) {
    var host = hostOf(dest), rule = blockedBy(s, host), pend = ui.netPend[host] || (rule && ui.netPend[rule.match]);
    if (pend) return '<span class="mut sm">' + (pend === 'net-block' ? 'Blocking…' : 'Unblocking…') + '</span>';
    return rule ? '<span class="chip xk blk">Blocked</span> <button class="link sm" data-act="netunblock" data-key="' + esc(rule.match) + '">Unblock</button>'
      : '<button class="link sm" data-act="netblock" data-key="' + esc(host) + '" title="No agent may reach ' + esc(host) + ' or its subdomains">Block</button>';
  }
  function blockList(s) {
    var rules = s.netblock || [];
    var h = rules.length ? '<p class="sm"><b>Blocked:</b> ' + rules.map(function (r) {
      return '<span class="chip xk blk">' + esc(r.match) + '</span> <button class="link sm" data-act="netunblock" data-key="' + esc(r.match) + '">Unblock</button>';
    }).join(' ') + '</p>' : '';
    if (ui.netErr) h += '<p class="ierr">' + esc(ui.netErr) + '</p>';
    if (ui.netCli) h += '<p class="icli">Tirith cannot send from this page. Run in a terminal: <code>' + esc(ui.netCli) + '</code></p>';
    return h;
  }
  function destinations(list) { var d = {}; list.forEach(function (x) { d[x.dest] = 1; }); return Object.keys(d).length; }
  function externalSection(s) {
    var all = T.externalRows(s, f.days, f.sys);
    var own = all.filter(function (x) { return x.runtime; }), rows = all.filter(function (x) { return !x.runtime; });
    var body = all.length ? '<table class="mtab ext"><tbody>' + runtimeLines(s, own) + rows.slice(0, 200).map(function (x) {
      var a = x.row.lane ? T.agentById(s, x.row.lane) : null;
      var who = a ? '<button class="link" data-act="goagent" data-key="' + esc(a.id) + '">' + esc(T.agentRole ? T.agentRole(a) : a.name) + ' · ' + esc(a.name) + '</button>' : esc(x.row.lane || x.row.agent || '');
      return '<tr><td class="sm">' + esc(T.clockTime(x.ts)) + '</td><td><span class="chip xk ' + esc(x.kind) + '">' + esc(KINDS[x.kind] || x.kind) + '</span></td>' +
        '<td><b>' + esc(x.dest) + '</b>' + (x.row.org ? ' <span class="mut sm">' + esc(x.row.org) + '</span>' : '') +
          (x.row.address && x.dest.indexOf(x.row.address) < 0 ? ' <span class="mut sm" title="' +
          esc(x.row.reverse || '') + '">' + esc(x.row.address) + '</span>' : '') + '</td><td class="sm">' + who + '</td><td class="sm mut" title="' + esc(x.row.action || '') + '">' +
        esc(x.row.blocked ? x.row.action : x.live ? 'connection opened' : (x.row.action || x.row.tool || '')) + '</td><td class="sm">' +
        (x.kind === 'search' || x.kind === 'mcp' || x.dest === 'remote' ? '' : blockBtn(s, x.dest)) + '</td></tr>';
    }).join('') + '</tbody></table>' : '<p class="mut sm">Nothing reached outside the machine in this period.</p>';
    var lines = destinations(own) + Math.min(rows.length, 200);
    return '<section class="pad extsec"><h2>External <span class="mut">' + lines + (lines === 1 ? ' line' : ' lines') +
      (all.length !== lines ? ' · ' + all.length + ' accesses' : '') + '</span></h2>' + blockList(s) + body + '</section>';
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
    else if (act === 'netblock') { netSend('net-block', key); return; }
    else if (act === 'netunblock') { netSend('net-unblock', key); return; }
    else if (act === 'goplan') { T.filter.project = key; T.setFilter('view', 'overview'); return; }
    else return;
    T.render();
  });
})();
