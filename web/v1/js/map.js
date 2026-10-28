/* Left: flat isometric map (SVG). Tower in the centre, one district tile per project, agents as tokens,
   pipeline stage strips and light dots travelling along curved paths for information flows. */
(function () {
  var T = window.T, esc = T.esc;
  var W = 1200, H = 780, TOWER = { x: 600, y: 392 };
  var svg, scene, fx, tipEl, tips = {}, points = {}, seen = {}, anims = [], raf = null, lastState = null, trailIds = {};
  var map = T.map = {};

  function hash(s) { var h = 0; for (var i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) | 0; return Math.abs(h); }
  function mix(hex, to, k) {
    var n = parseInt(hex.slice(1), 16), r = n >> 16, g = (n >> 8) & 255, b = n & 255;
    var t = to === 'w' ? 255 : 0;
    var f = function (c) { return Math.round(c + (t - c) * k); };
    return 'rgb(' + f(r) + ',' + f(g) + ',' + f(b) + ')';
  }
  function isoBlock(cx, cy, hw, th, top, left, right, stroke) {
    var hh = hw / 2, st = stroke ? ' stroke="' + stroke + '" stroke-width="1" stroke-linejoin="round"' : '';
    return '<polygon points="' + (cx - hw) + ',' + cy + ' ' + cx + ',' + (cy + hh) + ' ' + cx + ',' + (cy + hh + th) + ' ' + (cx - hw) + ',' + (cy + th) + '" fill="' + left + '"' + st + '/>' +
      '<polygon points="' + (cx + hw) + ',' + cy + ' ' + cx + ',' + (cy + hh) + ' ' + cx + ',' + (cy + hh + th) + ' ' + (cx + hw) + ',' + (cy + th) + '" fill="' + right + '"' + st + '/>' +
      '<polygon points="' + cx + ',' + (cy - hh) + ' ' + (cx + hw) + ',' + cy + ' ' + cx + ',' + (cy + hh) + ' ' + (cx - hw) + ',' + cy + '" fill="' + top + '"' + st + '/>';
  }
  function star(cx, cy, r, pts) {
    var p = [];
    for (var i = 0; i < pts * 2; i++) {
      var a = -Math.PI / 2 + i * Math.PI / pts, rr = i % 2 ? r * 0.45 : r;
      p.push((cx + Math.cos(a) * rr).toFixed(1) + ',' + (cy + Math.sin(a) * rr).toFixed(1));
    }
    return '<polygon points="' + p.join(' ') + '"/>';
  }

  /* ---------- layout ---------- */
  var SLOTS = [[-112, -6], [0, -6], [112, -6], [-56, 46], [56, 46], [-170, 40], [170, 40], [-120, 90], [120, 90]];
  function layout(s) {
    var projs = (s.projects || []).slice(), known = {};
    projs.forEach(function (p) { known[p.id] = 1; });
    (s.agents || []).concat(s.pipelines || []).forEach(function (a) {
      if (a.project && !known[a.project]) { known[a.project] = 1; projs.push({ id: a.project, name: a.project, color: '#8b8f98' }); }
    });
    var n = Math.max(1, projs.length), sc = n <= 3 ? 1 : n === 4 ? 0.9 : 0.78;
    var out = [], rx = 370, ry = 245;
    projs.forEach(function (p, i) {
      var ang = (-30 + (360 / n) * i) * Math.PI / 180;
      if (n === 1) ang = 90 * Math.PI / 180;
      var cx = TOWER.x + rx * Math.cos(ang), cy = TOWER.y - 10 + ry * Math.sin(ang);
      var na = (s.agents || []).filter(function (a) { return a.project === p.id; }).length;
      out.push({ p: p, cx: cx, cy: cy, sc: sc, n: na, hw: 195 * sc * (na > 6 ? 1.3 : na > 3 ? 1.1 : 1) });
    });
    out.forEach(function (o) { if (o.cx + o.hw > W - 20) o.cx = (W - 20) - o.hw; if (o.cx - o.hw < 20) o.cx = 20 + o.hw; });
    return out;
  }

  /* ---------- scene ---------- */
  function build(s) {
    tips = {}; points = {};
    var d = layout(s), h = '';
    var waiting = (s.waiting_on_you || []).length;
    h += '<ellipse cx="600" cy="395" rx="585" ry="355" fill="url(#ground)" stroke="#3a3426" stroke-width="1.5"/>';
    h += '<ellipse cx="600" cy="395" rx="550" ry="325" fill="none" stroke="#2c2a22" stroke-dasharray="2 8"/>';
    /* roads */
    d.forEach(function (o) {
      var mx = (TOWER.x + o.cx) / 2, my = (TOWER.y + o.cy) / 2 + 10;
      h += '<path d="M' + TOWER.x + ',' + (TOWER.y + 20) + ' Q' + mx + ',' + my + ' ' + o.cx + ',' + o.cy + '" fill="none" stroke="#2d3329" stroke-width="16" stroke-linecap="round" opacity=".85"/>' +
        '<path d="M' + TOWER.x + ',' + (TOWER.y + 20) + ' Q' + mx + ',' + my + ' ' + o.cx + ',' + o.cy + '" fill="none" stroke="#4a5144" stroke-width="1.5" stroke-dasharray="3 7" opacity=".8"/>';
    });
    /* districts back to front */
    d.slice().sort(function (a, b) { return a.cy - b.cy; }).forEach(function (o) { h += district(s, o); });
    h += tower(s, waiting);
    h += corner(s, waiting);
    /* trails (under tokens) are drawn by trails() into their own string appended later */
    h += '<g id="trails">' + trails(s) + '</g>';
    h += '<g id="tokens">' + d.map(function (o) { return tokens(s, o); }).join('') + '</g>';
    scene.innerHTML = h;
  }

  function district(s, o) {
    var p = o.p, hw = o.hw, hh = hw / 2, cx = o.cx, cy = o.cy, h = '';
    var top = p.color, key = 'proj:' + p.id;
    var nAg = (s.agents || []).filter(function (a) { return a.project === p.id; }).length;
    tips[key] = '<b>' + esc(p.name) + '</b><br><span class="mut">' + nAg + ' agent' + (nAg === 1 ? '' : 's') + (p.architecture && p.architecture.nodes ? ' &middot; ' + p.architecture.nodes.length + ' modules' : '') + '</span>';
    h += '<g data-tip="' + key + '">' + isoBlock(cx, cy, hw, 16, mix(top, 'k', 0.28), mix(top, 'k', 0.5), mix(top, 'k', 0.66), 'rgba(255,255,255,.18)');
    /* plaza inset */
    var iw = hw - 16, ih = iw / 2;
    h += '<polygon points="' + cx + ',' + (cy - ih) + ' ' + (cx + iw) + ',' + cy + ' ' + cx + ',' + (cy + ih) + ' ' + (cx - iw) + ',' + cy + '" fill="rgba(255,255,255,.07)" stroke="rgba(255,255,255,.22)" stroke-dasharray="3 5"/></g>';
    /* name pill */
    var topY = cy - hh, label = p.name, pw = label.length * 8.2 + 26;
    h += '<g data-tip="' + key + '"><rect x="' + (cx - pw / 2) + '" y="' + (topY - 56) + '" width="' + pw + '" height="22" rx="11" fill="' + mix(top, 'k', 0.55) + '" stroke="' + mix(top, 'w', 0.25) + '"/>' +
      '<text x="' + cx + '" y="' + (topY - 41) + '" text-anchor="middle" class="pname">' + esc(label) + '</text></g>';
    /* pipeline strip */
    var pl = T.latestPipeline(s, p.id);
    if (pl) {
      var st = pl.stages || [], sw = st.length * 17 - 3, bw = Math.max(sw + 16, 96), sx = cx - bw / 2, sy = topY - 30;
      var tk = 'run:' + pl.run;
      tips[tk] = '<b>' + esc(pl.run) + '</b> ' + pl.status + '<br>' + st.map(function (x) {
        return '<span class="dot ' + esc(x.status) + '"></span>' + esc(x.name) + ' <span class="mut">' + (x.status === 'running' ? 'running' : x.duration_s != null ? T.dur(x.duration_s) : x.status) + '</span>';
      }).join('<br>');
      h += '<g class="strip" data-act="run" data-id="' + esc(pl.run) + '" data-tip="' + esc(tk) + '"><rect x="' + sx + '" y="' + sy + '" width="' + bw + '" height="23" rx="5" fill="rgba(10,10,8,.82)" stroke="rgba(201,162,74,.55)"/>';
      st.forEach(function (x, i) {
        h += '<rect class="sq ' + esc(x.status) + '" x="' + (sx + 8 + i * 17) + '" y="' + (sy + 4) + '" width="14" height="14" rx="3"/>';
      });
      h += '<text x="' + cx + '" y="' + (sy + 35) + '" text-anchor="middle" class="plabel">' + esc(pl.run) + '</text></g>';
      points['pipeline:' + pl.run] = { x: cx, y: sy + 11 };
    }
    return h;
  }

  function tokens(s, o) {
    var h = '', i = 0;
    (s.agents || []).forEach(function (a) {
      if (a.project !== o.p.id) return;
      var n = o.n, cols = n <= 3 ? n : n <= 6 ? 3 : 4, row = Math.floor(i / cols), rows = Math.ceil(n / cols);
      var inRow = Math.min(cols, n - row * cols);
      var x = o.cx + (i % cols - (inRow - 1) / 2) * 112, y = o.cy + (row - (rows - 1) / 2) * 76 + 4; i++;
      points['agent:' + a.id] = { x: x, y: y };
      var c = T.clsOf(a['class']), live = (a.status === 'working' || a.status === 'stalled') && !a.ended_at;
      var tk = 'agent:' + a.id, el = T.agentElapsed(a);
      tips[tk] = '<b>' + esc(T.agentFull(a)) + '</b><br><span class="gold">' + esc(T.cap(a.rank)) + ' ' + esc(c.label) + '</span><br>' + esc(a.feature || '') +
        '<br><span class="schip ' + esc(a.status) + '">' + esc(a.status) + '</span> ' + T.dur(el) + ' &middot; ' + (a.files_changed || 0) + ' files <span class="add">+' + (a.insertions || 0) + '</span> <span class="del">-' + (a.deletions || 0) + '</span>' +
        (a.summary ? '<br><i>' + esc(a.summary) + '</i>' : '');
      var ring = T.STATUS_COLOR[a.status] || '#888';
      h += '<g class="token ' + esc(a.status) + '" transform="translate(' + x.toFixed(1) + ' ' + y.toFixed(1) + ')" data-act="agent" data-id="' + esc(a.id) + '" data-tip="' + esc(tk) + '">' +
        (a.status === 'working' ? '<circle class="halo" r="22" fill="none" stroke="' + ring + '" stroke-width="2"/>' : '') +
        '<ellipse cx="0" cy="17" rx="19" ry="7" fill="rgba(0,0,0,.35)"/>' +
        '<circle r="22" fill="#14110d" stroke="' + ring + '" stroke-width="3.5"' + (a.status === 'stalled' ? ' stroke-dasharray="5 4"' : '') + '/>' +
        '<circle r="17" fill="' + c.color + '" stroke="rgba(255,255,255,.5)"/>' +
        '<g transform="translate(-10 -10) scale(.83)"><path d="' + c.glyph + '" fill="#fff" fill-rule="evenodd"/></g>' +
        (a.status === 'done' ? '<g transform="translate(9 -26)"><circle r="8" fill="#3fb950" stroke="#0d0f0e"/><path d="M-4 0l3 3 5-6" fill="none" stroke="#fff" stroke-width="2"/></g>' : '') +
        (a.status === 'failed' ? '<g transform="translate(9 -26)"><circle r="8" fill="#f2564f" stroke="#0d0f0e"/><path d="M-3.5 -3.5l7 7m0 -7l-7 7" stroke="#fff" stroke-width="2"/></g>' : '') +
        '<text y="38" text-anchor="middle" class="aname">' + esc(a.name) + '</text>' +
        '<text y="50" text-anchor="middle" class="asub">' + esc(T.cap(a.rank)) + ' &middot; ' +
        (live ? '<tspan class="tick" data-short="1" data-start="' + esc(a.started_at) + '">' + T.durShort(T.elapsed(a.started_at)) + '</tspan>' : '<tspan>' + T.durShort(el) + '</tspan>') + '</text></g>';
    });
    return h;
  }

  function tower(s, waiting) {
    var x = TOWER.x, y = TOWER.y, st = s.steward || {}, h = '';
    var tk = 'tower';
    var note = (st.notes && st.notes.length) ? st.notes[st.notes.length - 1] : null;
    tips[tk] = '<b>' + esc(st.name || 'The Steward') + '</b> <span class="mut">' + esc(st.title || '') + '</span><br>' + esc(st.activity || 'Idle') +
      (st.since ? '<br><span class="mut">since ' + esc(T.clockTime(st.since)) + '</span>' : '') + (note ? '<br><i>Last note: ' + esc(note.text) + '</i>' : '');
    points.steward = { x: x, y: y - 130 };
    h += '<g data-tip="' + tk + '">' +
      isoBlock(x, y + 8, 96, 14, '#8d8670', '#6a6552', '#57533f') +
      isoBlock(x, y - 78, 40, 86, '#f1ecdd', '#d4cdb6', '#b7af97') +
      /* windows */
      '<g transform="translate(' + (x - 28) + ' ' + (y - 40) + ') skewY(26.57)" fill="#3a3326"><rect width="8" height="16" rx="1"/><rect x="14" y="7" width="8" height="16" rx="1"/></g>' +
      '<g transform="translate(' + (x + 6) + ' ' + (y - 17) + ') skewY(-26.57)" fill="#2a251b"><rect width="8" height="16" rx="1"/><rect x="14" y="-7" width="8" height="16" rx="1"/></g>' +
      isoBlock(x, y - 132, 28, 48, '#f6f2e6', '#dcd5be', '#bfb7a0') +
      '<polygon points="' + (x - 34) + ',' + (y - 132) + ' ' + x + ',' + (y - 115) + ' ' + x + ',' + (y - 176) + '" fill="#c9a24a"/>' +
      '<polygon points="' + (x + 34) + ',' + (y - 132) + ' ' + x + ',' + (y - 115) + ' ' + x + ',' + (y - 176) + '" fill="#8f6f26"/>' +
      '</g>';
    h += '<g fill="#fff3c2" stroke="#c9a24a" stroke-width="1">' + star(x, y - 192, 14, 7) + '</g>';
    h += '<g id="beacon" data-act="beacon" data-tip="beacon" class="' + (waiting ? 'on' : '') + '"><circle class="bglow" cx="' + x + '" cy="' + (y - 192) + '" r="30" fill="url(#beaconGrad)"/><circle cx="' + x + '" cy="' + (y - 192) + '" r="6" fill="' + (waiting ? '#ffb02e' : '#5a5648') + '"/></g>';
    tips.beacon = waiting ? '<b>' + waiting + ' thing' + (waiting > 1 ? 's' : '') + ' waiting for you</b><br><span class="mut">click to open the list</span>' : 'Nothing is waiting for you';
    h += '<text x="' + x + '" y="' + (y + 50) + '" text-anchor="middle" class="stname">' + esc(st.name || 'The Steward') + '</text>';
    h += '<text x="' + x + '" y="' + (y + 66) + '" text-anchor="middle" class="stact">' + esc(clip(st.activity || 'Idle', 46)) + '</text>';
    return h;
  }
  function clip(t, n) { return t.length > n ? t.slice(0, n - 1) + '…' : t; }

  /* owner badge, guard badge, room perches (top-left / top-right corners) */
  function corner(s, waiting) {
    var h = '', flows = s.flows || [], needGuard = false, rooms = {};
    flows.forEach(function (f) {
      [f.from, f.to].forEach(function (e) {
        if (e === 'guard') needGuard = true;
        if (e.indexOf('room:') === 0) rooms[e.slice(5)] = 1;
      });
    });
    (s.rooms || []).forEach(function (r) { rooms[r.id] = 1; });
    points.owner = { x: 74, y: 78 };
    var w = (s.waiting_on_you || []);
    tips.owner = '<b>You</b><br>' + (w.length ? w.length + ' waiting for you' : 'Nothing waiting');
    h += '<g class="you' + (waiting ? ' on' : '') + '" data-act="beacon" data-tip="owner" transform="translate(74 78)">' + (waiting ? '<circle class="halo2" r="30" fill="none" stroke="#ffb02e" stroke-width="2"/>' : '') +
      '<circle r="27" fill="#2a2110" stroke="#c9a24a" stroke-width="2.5"/><g transform="translate(-12 -12) scale(1)"><g fill="#f3d99a">' + T.icons.owner.replace(/ d=/g, ' d=') + '</g></g>' +
      (w.length ? '<g transform="translate(20 -20)"><circle r="10" fill="#ff9f1a" stroke="#0d0f0e" stroke-width="2"/><text y="4" text-anchor="middle" class="cbadge">' + w.length + '</text></g>' : '') +
      '<text y="45" text-anchor="middle" class="aname">You</text></g>';
    if (needGuard) {
      points.guard = { x: 74, y: 176 }; tips.guard = '<b>Guard</b><br><span class="mut">commit and repository checks</span>';
      h += '<g transform="translate(74 176)" data-tip="guard"><circle r="19" fill="#1d1f22" stroke="#8b8f98" stroke-width="2"/><g transform="translate(-9 -9) scale(.75)" fill="#cfd3da">' + T.icons.guard.replace(/^<path/, '<path') + '</g><text y="34" text-anchor="middle" class="asub">Guard</text></g>';
    }
    var ids = Object.keys(rooms);
    ids.forEach(function (id, i) {
      var x = W - 66, y = 66 + i * 70, r = null;
      (s.rooms || []).forEach(function (q) { if (q.id === id) r = q; });
      points['room:' + id] = { x: x, y: y };
      tips['room:' + id] = '<b>' + esc(r ? r.title : id) + '</b><br><span class="mut">agent room</span>' + (r && r.summary ? '<br><i>' + esc(r.summary) + '</i>' : '');
      h += '<g transform="translate(' + x + ' ' + y + ')" data-act="room" data-id="' + esc(id) + '" data-tip="room:' + esc(id) + '">' +
        '<rect x="-4" y="-6" width="8" height="32" rx="2" fill="#4a3f26"/><circle r="22" fill="#14202e" stroke="#3b6ea5" stroke-width="2.5"/>' +
        '<g transform="translate(-10 -10) scale(.83)" fill="#8fc0f2">' + T.icons.room + '</g>' +
        '<text y="42" text-anchor="middle" class="asub">' + esc(clip(r ? r.title : id, 18)) + '</text></g>';
    });
    return h;
  }

  /* ---------- flows ---------- */
  function curve(a, b, id) {
    var dx = b.x - a.x, dy = b.y - a.y, len = Math.sqrt(dx * dx + dy * dy) || 1;
    var sign = hash(id) % 2 ? 1 : -1, k = 0.2 * len * sign;
    return { x0: a.x, y0: a.y, x2: b.x, y2: b.y, cx: (a.x + b.x) / 2 - dy / len * k, cy: (a.y + b.y) / 2 + dx / len * k - len * 0.12 };
  }
  function at(c, t) {
    var u = 1 - t;
    return { x: u * u * c.x0 + 2 * u * t * c.cx + t * t * c.x2, y: u * u * c.y0 + 2 * u * t * c.cy + t * t * c.y2 };
  }
  function flowCurve(f) {
    var a = points[f.from], b = points[f.to];
    return a && b ? curve(a, b, f.id || f.from + f.to) : null;
  }
  function trails(s) {
    var fl = (s.flows || []).slice().sort(function (a, b) { return (T.ms(a.at) || 0) - (T.ms(b.at) || 0); }).slice(-12), h = '';
    fl.forEach(function (f, i) {
      var c = flowCurve(f); if (!c) return;
      var col = T.flowColor(f), op = (0.2 + 0.4 * (i + 1) / fl.length).toFixed(2), m = at(c, 0.55), m2 = at(c, 0.6);
      var ang = Math.atan2(m2.y - m.y, m2.x - m.x) * 180 / Math.PI;
      var tk = 'flow:' + f.id;
      tips[tk] = '<span class="kchip" style="--kc:' + col + '">' + esc(f.kind) + '</span> ' + esc(T.clockTime(f.at)) + '<br><b>' + esc(T.endpointName(s, f.from, f, 'from')) + '</b> &rarr; <b>' + esc(T.endpointName(s, f.to, f, 'to')) + '</b><br>' + esc(f.text || '');
      h += '<g class="trail" data-act="flow" data-id="' + esc(f.id) + '" data-tip="' + esc(tk) + '">' +
        '<path d="M' + c.x0 + ',' + c.y0 + ' Q' + c.cx + ',' + c.cy + ' ' + c.x2 + ',' + c.y2 + '" fill="none" stroke="transparent" stroke-width="16"/>' +
        '<path class="vis" d="M' + c.x0 + ',' + c.y0 + ' Q' + c.cx + ',' + c.cy + ' ' + c.x2 + ',' + c.y2 + '" fill="none" stroke="' + col + '" stroke-opacity="' + op + '" stroke-width="2" stroke-dasharray="2 6" stroke-linecap="round"/>' +
        '<polygon class="vis" points="-5,-4 5,0 -5,4" fill="' + col + '" fill-opacity="' + Math.min(1, op * 1.6).toFixed(2) + '" transform="translate(' + m.x.toFixed(1) + ' ' + m.y.toFixed(1) + ') rotate(' + ang.toFixed(1) + ')"/></g>';
    });
    return h;
  }

  map.play = function (f, delay) {
    anims.push({ f: f, t0: performance.now() + (delay || 0), dur: 2600 });
    if (!raf) raf = setTimeout(function () { frame(performance.now()); }, 33);
  };
  function frame(now) {
    var h = '', keep = [];
    anims.forEach(function (a) {
      var t = (now - a.t0) / a.dur;
      if (t < 0) { keep.push(a); return; }
      var c = flowCurve(a.f);
      if (!c) return;
      var col = T.flowColor(a.f);
      if (t > 1.15) return;
      keep.push(a);
      var e = Math.min(1, t); e = e * e * (3 - 2 * e);
      for (var k = 0; k < 7; k++) {
        var tt = Math.max(0, e - k * 0.028), p = at(c, tt);
        h += '<circle cx="' + p.x.toFixed(1) + '" cy="' + p.y.toFixed(1) + '" r="' + (k === 0 ? 6 : 5 - k * 0.55).toFixed(1) + '" fill="' + col + '" opacity="' + (k === 0 ? 1 : (0.55 - k * 0.07)).toFixed(2) + '"' + (k === 0 ? ' filter="url(#glow)"' : '') + '/>';
      }
      if (t >= 0.95) {
        var q = (t - 0.95) / 0.2, pe = at(c, 1);
        h += '<circle cx="' + pe.x.toFixed(1) + '" cy="' + pe.y.toFixed(1) + '" r="' + (8 + q * 22).toFixed(1) + '" fill="none" stroke="' + col + '" stroke-width="2" opacity="' + (1 - q).toFixed(2) + '"/>';
      }
    });
    anims = keep; fx.innerHTML = h;
    raf = anims.length ? setTimeout(function () { frame(performance.now()); }, 33) : null;
  }

  /* ---------- tooltip + events ---------- */
  function onMove(e) {
    var el = e.target.closest ? e.target.closest('[data-tip]') : null;
    if (!el || !tips[el.getAttribute('data-tip')]) { tipEl.style.display = 'none'; return; }
    tipEl.innerHTML = tips[el.getAttribute('data-tip')];
    tipEl.style.display = 'block';
    var r = tipEl.getBoundingClientRect(), x = e.clientX + 16, y = e.clientY + 14;
    if (x + r.width > window.innerWidth - 8) x = e.clientX - r.width - 16;
    if (y + r.height > window.innerHeight - 8) y = e.clientY - r.height - 14;
    tipEl.style.left = x + 'px'; tipEl.style.top = y + 'px';
  }
  function onClick(e) {
    var el = e.target.closest('[data-act]'); if (!el) return;
    var act = el.getAttribute('data-act'), id = el.getAttribute('data-id');
    if (act === 'agent') T.openIn('agents', { type: 'agent', id: id }, { afilter: 'all' });
    else if (act === 'run') T.openIn('pipelines', { type: 'run', id: id }, { pfilter: 'all' });
    else if (act === 'flow') T.openIn('messages', { type: 'flow', id: id });
    else if (act === 'beacon') T.openIn('waiting', null);
    else if (act === 'room') T.openIn('messages', null);
  }

  map.init = function () {
    svg = document.getElementById('map'); tipEl = document.getElementById('tip');
    svg.setAttribute('viewBox', '0 0 ' + W + ' ' + H);
    svg.innerHTML = '<defs>' +
      '<radialGradient id="ground" cx="50%" cy="45%" r="60%"><stop offset="0" stop-color="#2b3329"/><stop offset=".7" stop-color="#181c16"/><stop offset="1" stop-color="#101210"/></radialGradient>' +
      '<radialGradient id="beaconGrad"><stop offset="0" stop-color="#ffc247" stop-opacity=".95"/><stop offset="1" stop-color="#ffc247" stop-opacity="0"/></radialGradient>' +
      '<filter id="glow" x="-100%" y="-100%" width="300%" height="300%"><feGaussianBlur stdDeviation="3" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter></defs>' +
      '<g id="scene"></g><g id="fx" style="pointer-events:none"></g>';
    scene = svg.querySelector('#scene'); fx = svg.querySelector('#fx');
    svg.addEventListener('mousemove', onMove);
    svg.addEventListener('mouseleave', function () { tipEl.style.display = 'none'; });
    svg.addEventListener('click', onClick);
  };

  map.render = function (s, first) {
    lastState = s;
    build(s);
    var fl = (s.flows || []).slice().sort(function (a, b) { return (T.ms(a.at) || 0) - (T.ms(b.at) || 0); });
    var fresh = fl.filter(function (f) { return !seen[f.id]; });
    fl.forEach(function (f) { seen[f.id] = 1; });
    if (first) fresh = fl.slice(-6);          /* first load: replay the latest few */
    fresh.slice(-8).forEach(function (f, i) { map.play(f, 400 + i * 700); });
  };
  map.flows = function () { return lastState ? (lastState.flows || []) : []; };
})();
