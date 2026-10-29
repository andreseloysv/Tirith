/* Flow view: how information passes between you, the Manager, agents, rooms and pipelines. */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  var MAXEXT = 7, MAXDEC = 3;
  var MANAGER = T.CLS.manager.color;

  function kindOf(id) {
    if (id === 'owner' || id === 'guard') return 0;
    if (id === 'manager') return 1;
    if (/^(agent|pipeline):/.test(id)) return 2;
    if (/^room:/.test(id)) return 3;
    return 4;
  }
  function nodeLook(s, id, name) {
    if (id === 'manager') return { bg: T.grad(MANAGER), svg: T.fillIcon(T.CLS.manager.glyph, 20), ov: 'Orchestrator' };
    if (id === 'owner') return { bg: 'linear-gradient(135deg,#f2b84b,#e8892b)', svg: T.fillIcon(T.ICON.owner, 20), ov: 'You' };
    if (id === 'guard') return { bg: 'linear-gradient(135deg,#9aa3b2,#6b7280)', svg: T.fillIcon(T.ICON.guard, 20), ov: 'Guard' };
    if (/^room:/.test(id)) return { bg: 'linear-gradient(135deg,#4aa8e8,#3b6bd9)', svg: T.fillIcon(T.ICON.room, 19), ov: 'Room' };
    if (/^pipeline:/.test(id)) return { bg: 'linear-gradient(135deg,#7c83ff,#4f5bd5)', svg: T.fillIcon(T.ICON.pipe, 19), ov: 'Pipeline' };
    if (/^agent:/.test(id)) {
      var a = T.agentById(s, id.slice(6)), c = T.clsOf(a && a['class']);
      return { bg: T.grad(a ? T.agentColor(a) : c.color), svg: T.fillIcon(c.glyph, 20), ov: a ? T.cap(a['class']) : 'Agent' };
    }
    return { bg: 'linear-gradient(135deg,#c3c9d4,#9aa3b2)', svg: '<b class="init">' + esc((name || '?').charAt(0).toUpperCase()) + '</b>', ov: 'External' };
  }

  function build(s) {
    var msgs = T.messages(s), nodes = {}, extCount = {};
    /* blocked actions: a red edge from the agent to The Guard */
    T.recentViolations(s).forEach(function (v) {
      var a = v.lane && T.agentById(s, v.lane); if (!a) return;
      msgs.push({ at: v.ts, fromId: 'agent:' + a.id, toId: 'guard', fromName: a.name, toName: 'The Guard', text: T.violationText(v), kind: 'violation', src: 'rule' });
    });
    msgs.sort(function (x, y) { return (T.ms(y.at) || 0) - (T.ms(x.at) || 0); });
    msgs.forEach(function (m) { [m.fromId, m.toId].forEach(function (id) { if (/^ext:/.test(id)) extCount[id] = (extCount[id] || 0) + 1; }); });
    var keep = Object.keys(extCount).sort(function (a, b) { return extCount[b] - extCount[a]; }).slice(0, MAXEXT), keepSet = {};
    keep.forEach(function (k) { keepSet[k] = 1; });
    var nOthers = Object.keys(extCount).length - keep.length;
    function map(id) { return /^ext:/.test(id) && !keepSet[id] ? 'ext:*' : id; }
    var edges = {};
    msgs.forEach(function (m) {
      var a = map(m.fromId), b = map(m.toId);
      [[a, m.fromName], [b, m.toName]].forEach(function (x) {
        if (!nodes[x[0]]) nodes[x[0]] = { id: x[0], name: x[0] === 'ext:*' ? nOthers + ' others' : x[0] === 'manager' ? 'The Manager' : x[1], col: kindOf(x[0]) };
      });
      var k = [a, b].sort().join('>'), e = edges[k] || (edges[k] = { key: k, a: a, b: b, msgs: [], dec: [], all: [] });
      e.all.push(m);
      if (m.kind === 'decision') e.dec.push(m); else e.msgs.push(m);
    });
    return { nodes: nodes, edges: Object.keys(edges).map(function (k) { return edges[k]; }) };
  }


  /* ---------- nodes ---------- */
  function pipelinePanel(p) {
    var st = p.stages || [], col = p.status === 'failed' ? 'failed' : p.status === 'running' ? 'active' : p.status === 'pending' ? 'idle' : 'done';
    var time = p.status === 'running' ? T.span(p.started_at, null) : p.status === 'pending' ? '' : esc(T.dur(T.elapsed(p.started_at, p.ended_at)));
    return '<div class="activity"><section class="asec"><h4>Status</h4><div class="astat"><span class="sbadge ' + col + '">' + esc(T.cap(p.status)) + '</span>' + (time ? '<span class="mut">' + time + '</span>' : '') + '</div></section>' +
      '<section class="asec"><h4>Stages <span class="n">' + st.length + '</span></h4>' + (st.length ? '<ul class="srows">' + st.map(function (x) {
        var ic = T.stageIcon(x.name, 12), dur = T.stageDur(x);
        return '<li><span class="sico" style="background:' + ic.bg + '">' + ic.svg + '</span><span class="sn">' + esc(x.name) + '</span><span class="rb ' + esc(x.status) + '">' + esc(x.status) + '</span><span class="mut when">' + dur + '</span></li>';
      }).join('') + '</ul>' : T.empty('No stages yet')) + '</section></div>';
  }
  function nodeHtml(s, n) {
    var id = n.id, lk = nodeLook(s, id, n.name), a = /^agent:/.test(id) && T.agentById(s, id.slice(6)), p, body = '', open = !!ui.fopen[id], ov = esc(lk.ov), sub = '';
    if (a) {
      var st = T.agentState(a);
      ov = '<i class="dot ' + (st === 'active' ? 'green pulse' : 'grey') + '" title="' + st + '"></i>' + ov;
      if (open) body = T.activity(a, { clock: true });
      else if (a.goal) sub = '<span class="gl" title="' + esc(a.goal) + '">' + T.mdInline(a.goal) + '</span>';
    } else if (/^pipeline:/.test(id)) {
      p = (s.pipelines || []).filter(function (x) { return x.run === id.slice(9); })[0];
      if (p && open) body = pipelinePanel(p);
      if (!p) open = false;
    }
    var can = /^(agent|pipeline):/.test(id) && (a || p);
    var head = '<span class="gico" style="background:' + lk.bg + '">' + lk.svg + '</span><span class="nt"><span class="ov">' + ov + '</span><span class="tt">' + esc(n.name) + '</span>' + sub + '</span>' + (can ? '<i class="caret"></i>' : '');
    return '<div class="node fnode' + (can ? ' ex' : '') + (open ? ' open' : '') + '" data-id="' + esc(id) + '">' +
      (can ? '<button class="fhead" data-act="fnode" data-key="' + esc(id) + '" aria-expanded="' + open + '">' + head + '</button>' : '<div class="fhead">' + head + '</div>') +
      (open && body ? '<div class="fbody">' + body + '</div>' : '') + '</div>';
  }

  /* Nodes flow in columns (so an expanded node pushes its neighbours); edges are drawn from the measured boxes. */
  function layoutEdges(el, g) {
    var gr = el.querySelector('.fgraph'); if (!gr) return;
    var box = {}, W = gr.offsetWidth, H = gr.offsetHeight;
    [].forEach.call(gr.querySelectorAll('.fnode'), function (n) {
      var h = n.querySelector('.fhead');
      box[n.getAttribute('data-id')] = { x: n.offsetLeft, w: n.offsetWidth, cy: n.offsetTop + h.offsetHeight / 2 };
    });
    var paths = '', pills = '', maxc = 1;
    g.edges.forEach(function (e) { maxc = Math.max(maxc, e.all.length); });
    if (ui.edge === 'first' && g.edges.length) ui.edge = g.edges[0].key;
    g.edges.forEach(function (e) {
      var A = box[e.a], B = box[e.b]; if (!A || !B) return;
      var x1, x2, y1 = A.cy, y2 = B.cy, d, mx, my, same = g.nodes[e.a].ci === g.nodes[e.b].ci;
      if (same) {
        x1 = A.x + A.w; x2 = B.x + B.w; var o = 70;
        d = 'M' + x1 + ',' + y1 + ' C' + (x1 + o) + ',' + y1 + ' ' + (x2 + o) + ',' + y2 + ' ' + x2 + ',' + y2; mx = x1 + o * .75; my = (y1 + y2) / 2;
      } else {
        if (g.nodes[e.a].ci < g.nodes[e.b].ci) { x1 = A.x + A.w; x2 = B.x; } else { x1 = A.x; x2 = B.x + B.w; }
        var dx = Math.abs(x2 - x1) / 2;
        d = 'M' + x1 + ',' + y1 + ' C' + (x1 + (x2 > x1 ? dx : -dx)) + ',' + y1 + ' ' + (x2 - (x2 > x1 ? dx : -dx)) + ',' + y2 + ' ' + x2 + ',' + y2;
        mx = (x1 + x2) / 2; my = (y1 + y2) / 2;
      }
      var on = ui.edge === e.key, latest = e.msgs[0], bad = latest && latest.kind === 'violation';
      paths += '<path class="edge' + (on ? ' on' : '') + (bad ? ' bad' : '') + '" d="' + d + '" style="stroke-width:' + (1.2 + 1.6 * e.all.length / maxc).toFixed(2) + '"/>' +
               '<circle class="port' + (on ? ' on' : '') + '" cx="' + x1 + '" cy="' + y1 + '" r="3.5"/><circle class="port' + (on ? ' on' : '') + '" cx="' + x2 + '" cy="' + y2 + '" r="3.5"/>';
      if (e.dec.length) {
        /* labels sit above the line, at the agent's end, so labels of different agents never meet */
        var aEnd = /^agent:/.test(e.a) ? [x1, y1, x2] : /^agent:/.test(e.b) ? [x2, y2, x1] : null;
        var lx = aEnd ? aEnd[0] + (aEnd[2] < aEnd[0] ? -10 : 10) : mx, ly = aEnd ? aEnd[1] - 13 : my + (latest ? 17 : 0), side = aEnd ? (aEnd[2] < aEnd[0] ? ' l' : ' r') : '', up = aEnd ? -19 : 19;
        var label = function (cls, act, key, tip, inner, i) {
          return '<button class="dlabel' + cls + side + '" data-act="' + act + '" data-key="' + esc(key) + '" style="left:' + lx + 'px;top:' + (ly + i * up) + 'px" title="' + esc(tip) + '">' + inner + '</button>';
        };
        e.dec.slice(0, MAXDEC).forEach(function (m, i) {
          pills += label(ui.decision === m.id ? ' on' : '', 'decision', m.id, m.text + ' (click for the reason)', '<i></i><span>' + esc(m.text) + '</span>', i);
        });
        if (e.dec.length > MAXDEC) pills += label(' more', 'edge', e.key, 'All decisions', '+' + (e.dec.length - MAXDEC) + ' decisions', MAXDEC);
      }
      if (!latest) return;
      pills += '<button class="epill' + (on ? ' on' : '') + (bad ? ' bad' : '') + '" data-act="edge" data-key="' + esc(e.key) + '" style="left:' + mx + 'px;top:' + my + 'px" title="' + esc(latest.text) + '"><b>' + e.msgs.length + '</b><span>' + esc(latest.text) + '</span></button>';
    });
    var svg = gr.querySelector('.fsvg');
    svg.setAttribute('width', W); svg.setAttribute('height', H); svg.innerHTML = paths;   /* innerHTML on svg is fine in current browsers */
    gr.querySelector('.fpills').innerHTML = pills;
  }

  function view(s, el) {
    var g = build(s), ids = Object.keys(g.nodes);
    if (!ids.length) { el.innerHTML = '<div class="pad">' + T.empty('No information has flowed in this session') + '</div>'; return; }
    var cols = {}, used = [];
    ids.forEach(function (id) { var c = g.nodes[id].col; (cols[c] = cols[c] || []).push(g.nodes[id]); });
    Object.keys(cols).map(Number).sort().forEach(function (c) { used.push(c); });
    used.forEach(function (c, ci) {
      cols[c].sort(function (a, b) { return a.name < b.name ? -1 : 1; });
      cols[c].forEach(function (n) { n.ci = ci; });
    });
    ui.fids = ids.filter(function (id) { return /^(agent|pipeline):/.test(id); });
    var nodesH = '<div class="fcols">' + used.map(function (c) {
      return '<div class="fcol">' + cols[c].map(function (n) { return nodeHtml(s, n); }).join('') + '</div>';
    }).join('') + '</div>';
    var allOpen = ui.fids.length && ui.fids.every(function (id) { return ui.fopen[id]; });
    var bar = ui.fids.length ? '<span class="sepv"></span><button data-act="fall">' + (allOpen ? 'Collapse all' : 'Expand all') + '</button>' : '';
    var inner = '<div class="fgraph"><svg class="fsvg"></svg>' + nodesH + '<div class="fpills"></div></div>';
    var drawer = '';
    if (ui.drawer === 'edge') {
      var e = g.edges.filter(function (x) { return x.key === ui.edge; })[0];
      if (e) drawer = T.drawer(g.nodes[e.a].name + ' ↔ ' + g.nodes[e.b].name, '<ul class="feed">' + e.all.map(function (m) {
        return '<li><span class="when">' + esc(T.clockTime(m.at)) + '</span><span class="who"><i class="dot" style="background:' + (T.KIND[m.kind] || '#9ca3af') + '" title="' + esc(m.kind) + '"></i>' + esc(m.fromName) + ' <span class="arr">→</span> ' + esc(m.toName) + '</span><span class="txt mdt">' + T.mdBox(m.text, 'e:' + m.fromId + m.at) + '</span></li>';
      }).join('') + '</ul>');
    }
    if (ui.drawer === 'decision') {
      var dm = null;
      g.edges.forEach(function (x) { x.dec.forEach(function (m) { if (m.id === ui.decision) dm = m; }); });
      if (dm) drawer = T.drawer('Decision', '<div class="decision"><p class="dtext">' + T.mdInline(dm.text) + '</p>' +
        '<h4>Why</h4><p class="dwhy">' + (dm.why ? T.mdInline(dm.why) : '<span class="quiet">No reason was recorded</span>') + '</p>' +
        '<p class="mut dmeta">' + esc(dm.fromName) + ' <span class="arr">→</span> ' + esc(dm.toName) + ' · ' + esc(T.clockTime(dm.at)) + '</p></div>');
    }
    el.innerHTML = '<div class="fv">' + T.canvas(inner, bar) + drawer + '</div>';
    layoutEdges(el, g);
  }
  T.views.flow = view;
})();
