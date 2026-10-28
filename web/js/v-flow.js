/* Flow view: how information passes between you, the Steward, agents, rooms and pipelines. */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  var NW = 184, NH = 58, GX = 96, GY = 22, MAXEXT = 7;
  var STEWARD = T.CLS.steward.color;

  function kindOf(id) {
    if (id === 'owner' || id === 'guard') return 0;
    if (id === 'steward') return 1;
    if (/^(agent|pipeline):/.test(id)) return 2;
    if (/^room:/.test(id)) return 3;
    return 4;
  }
  function nodeLook(s, id, name) {
    if (id === 'steward') return { bg: T.grad(STEWARD), svg: T.fillIcon(T.CLS.steward.glyph, 20), ov: 'Orchestrator' };
    if (id === 'owner') return { bg: 'linear-gradient(135deg,#f2b84b,#e8892b)', svg: T.fillIcon(T.ICON.owner, 20), ov: 'You' };
    if (id === 'guard') return { bg: 'linear-gradient(135deg,#9aa3b2,#6b7280)', svg: T.fillIcon(T.ICON.guard, 20), ov: 'Guard' };
    if (/^room:/.test(id)) return { bg: 'linear-gradient(135deg,#4aa8e8,#3b6bd9)', svg: T.fillIcon(T.ICON.room, 19), ov: 'Room' };
    if (/^pipeline:/.test(id)) return { bg: 'linear-gradient(135deg,#7c83ff,#4f5bd5)', svg: T.fillIcon(T.ICON.pipe, 19), ov: 'Pipeline' };
    if (/^agent:/.test(id)) {
      var a = T.agentById(s, id.slice(6)), c = T.clsOf(a && a['class']);
      return { bg: T.grad(c.color), svg: T.fillIcon(c.glyph, 20), ov: a ? T.cap(a['class']) : 'Agent' };
    }
    return { bg: 'linear-gradient(135deg,#c3c9d4,#9aa3b2)', svg: '<b class="init">' + esc((name || '?').charAt(0).toUpperCase()) + '</b>', ov: 'External' };
  }

  function build(s) {
    var msgs = T.messages(s), nodes = {}, extCount = {};
    msgs.forEach(function (m) { [m.fromId, m.toId].forEach(function (id) { if (/^ext:/.test(id)) extCount[id] = (extCount[id] || 0) + 1; }); });
    var keep = Object.keys(extCount).sort(function (a, b) { return extCount[b] - extCount[a]; }).slice(0, MAXEXT), keepSet = {};
    keep.forEach(function (k) { keepSet[k] = 1; });
    var nOthers = Object.keys(extCount).length - keep.length;
    function map(id) { return /^ext:/.test(id) && !keepSet[id] ? 'ext:*' : id; }
    var edges = {};
    msgs.forEach(function (m) {
      var a = map(m.fromId), b = map(m.toId);
      [[a, m.fromName], [b, m.toName]].forEach(function (x) {
        if (!nodes[x[0]]) nodes[x[0]] = { id: x[0], name: x[0] === 'ext:*' ? nOthers + ' others' : x[0] === 'steward' ? 'The Steward' : x[1], col: kindOf(x[0]) };
      });
      var k = [a, b].sort().join('>'), e = edges[k] || (edges[k] = { key: k, a: a, b: b, msgs: [] });
      e.msgs.push(m);
    });
    return { nodes: nodes, edges: Object.keys(edges).map(function (k) { return edges[k]; }) };
  }

  function view(s, el) {
    var g = build(s), ids = Object.keys(g.nodes);
    if (!ids.length) { el.innerHTML = '<div class="pad">' + T.empty('No information has flowed in this session') + '</div>'; return; }
    var cols = {}, used = [];
    ids.forEach(function (id) { var c = g.nodes[id].col; (cols[c] = cols[c] || []).push(g.nodes[id]); });
    Object.keys(cols).map(Number).sort().forEach(function (c) { used.push(c); });
    var maxN = Math.max.apply(null, used.map(function (c) { return cols[c].length; }));
    var H = maxN * (NH + GY) - GY, W = used.length * NW + (used.length - 1) * GX;
    used.forEach(function (c, ci) {
      var l = cols[c]; l.sort(function (a, b) { return a.name < b.name ? -1 : 1; });
      var top = (H - (l.length * (NH + GY) - GY)) / 2;
      l.forEach(function (n, i) { n.x = ci * (NW + GX); n.y = top + i * (NH + GY); n.ci = ci; });
    });
    var paths = '', pills = '', maxc = 1;
    g.edges.forEach(function (e) { maxc = Math.max(maxc, e.msgs.length); });
    g.edges.sort(function (a, b) { return b.msgs.length - a.msgs.length; });
    if (ui.edge === 'first' && g.edges.length) ui.edge = g.edges[0].key;
    g.edges.forEach(function (e) {
      var A = g.nodes[e.a], B = g.nodes[e.b], x1, x2, y1 = A.y + NH / 2, y2 = B.y + NH / 2, d, mx, my;
      if (A.ci === B.ci) {
        x1 = A.x + NW; x2 = B.x + NW; var o = 70;
        d = 'M' + x1 + ',' + y1 + ' C' + (x1 + o) + ',' + y1 + ' ' + (x2 + o) + ',' + y2 + ' ' + x2 + ',' + y2; mx = x1 + o * .75; my = (y1 + y2) / 2;
      } else {
        if (A.ci < B.ci) { x1 = A.x + NW; x2 = B.x; } else { x1 = A.x; x2 = B.x + NW; }
        var dx = Math.abs(x2 - x1) / 2;
        d = 'M' + x1 + ',' + y1 + ' C' + (x1 + (x2 > x1 ? dx : -dx)) + ',' + y1 + ' ' + (x2 - (x2 > x1 ? dx : -dx)) + ',' + y2 + ' ' + x2 + ',' + y2;
        mx = (x1 + x2) / 2; my = (y1 + y2) / 2;
      }
      var on = ui.edge === e.key, latest = e.msgs[0];
      paths += '<path class="edge' + (on ? ' on' : '') + '" d="' + d + '" style="stroke-width:' + (1.2 + 1.6 * e.msgs.length / maxc).toFixed(2) + '"/>' +
               '<circle class="port' + (on ? ' on' : '') + '" cx="' + x1 + '" cy="' + y1 + '" r="3.5"/><circle class="port' + (on ? ' on' : '') + '" cx="' + x2 + '" cy="' + y2 + '" r="3.5"/>';
      pills += '<button class="epill' + (on ? ' on' : '') + '" data-act="edge" data-key="' + esc(e.key) + '" style="left:' + mx + 'px;top:' + my + 'px" title="' + esc(latest.text) + '"><b>' + e.msgs.length + '</b><span>' + esc(latest.text) + '</span></button>';
    });
    var nodesH = ids.map(function (id) {
      var n = g.nodes[id], lk = nodeLook(s, id, n.name);
      return '<div class="node fnode" style="left:' + n.x + 'px;top:' + n.y + 'px;width:' + NW + 'px;height:' + NH + 'px"><span class="gico" style="background:' + lk.bg + '">' + lk.svg + '</span><span class="nt"><span class="ov">' + esc(lk.ov) + '</span><span class="tt">' + esc(n.name) + '</span></span></div>';
    }).join('');
    var inner = '<div class="fgraph" style="width:' + W + 'px;height:' + H + 'px"><svg width="' + W + '" height="' + H + '" style="position:absolute;left:0;top:0;overflow:visible">' + paths + '</svg>' + nodesH + pills + '</div>';
    var drawer = '';
    if (ui.drawer === 'edge') {
      var e = g.edges.filter(function (x) { return x.key === ui.edge; })[0];
      if (e) drawer = T.drawer(g.nodes[e.a].name + ' ↔ ' + g.nodes[e.b].name, '<ul class="feed">' + e.msgs.map(function (m) {
        return '<li><span class="when">' + esc(T.clockTime(m.at)) + '</span><span class="who"><i class="dot" style="background:' + (T.KIND[m.kind] || '#9ca3af') + '" title="' + esc(m.kind) + '"></i>' + esc(m.fromName) + ' <span class="arr">→</span> ' + esc(m.toName) + '</span><span class="txt">' + esc(m.text) + '</span></li>';
      }).join('') + '</ul>');
    }
    el.innerHTML = '<div class="fv">' + T.canvas(inner) + drawer + '</div>';
  }
  T.views.flow = view;
})();
