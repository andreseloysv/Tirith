/* What an agent is doing: goal, status, tests, tools, files, live events. Shared by Flow and the Agents drawer. */
(function () {
  var T = window.T, esc = T.esc, MAXFILES = 12, MAXEVENTS = 12;
  var CH = { added: 'A', modified: 'M', deleted: 'D', renamed: 'R', untracked: 'U' };
  var RES = { passed: 'passed', failed: 'failed', running: 'running' };

  /* "12 s ago", ticking through the .agol class (see main.js tick) */
  T.agoLong = function (iso) { var s = T.ago(iso); return !s ? '' : s === 'now' ? 'just now' : s + ' ago'; };
  var agoLong = function (iso) { return iso ? '<span class="agol" data-at="' + esc(iso) + '">' + esc(T.agoLong(iso)) + '</span>' : ''; };

  /* state of the agent for the dot and the status line: active / idle / done / failed */
  T.agentState = function (a) {
    if (a.status === 'done' || a.status === 'failed') return a.status;
    return a.active ? 'active' : 'idle';
  };

  /* usage: "$0.50 · 2.0k out · 100 in · 4.3k cached · alpha-1 · 12 turns" (cost omitted when unknown) */
  function k(n) { n = n || 0; return n >= 1e6 ? (n / 1e6).toFixed(1) + 'M' : n >= 1e3 ? (n / 1e3).toFixed(n >= 1e4 ? 0 : 1) + 'k' : String(n); }
  T.sumUsage = function (agents) {
    var rows = agents.map(function (a) { return a.usage; }).filter(Boolean);
    if (!rows.length) return null;
    var t = { input_tokens: 0, output_tokens: 0, cache_tokens: 0, cost_usd: null, model: null, turns: 0 }, models = {};
    rows.forEach(function (u) {
      t.input_tokens += u.input_tokens; t.output_tokens += u.output_tokens; t.cache_tokens += u.cache_tokens; t.turns += u.turns;
      if (u.cost_usd != null) t.cost_usd = (t.cost_usd || 0) + u.cost_usd;
      if (u.model) models[u.model] = 1;
    });
    t.turns = 0;  /* turns do not add up across agents in a way worth showing */
    return t;
  };
  /* "model · effort" an agent ran with: the usage record's model, else the one in its command */
  T.ranText = function (a) {
    var r = a && a.ran, m = (a && a.usage && a.usage.model) || (r && r.model), e = r && r.effort;
    return [m, e ? e + ' effort' : ''].filter(Boolean).join(' · ');
  };
  T.usageLine = function (u, a) {
    if (!u) return '';
    var p = [], ran = a && a.ran && a.ran.effort ? a.ran.effort + ' effort' : '';
    if (u.cost_usd != null) p.push('$' + u.cost_usd.toFixed(2));
    p.push(k(u.input_tokens) + ' in', k(u.output_tokens) + ' out');
    if (u.cache_tokens) p.push(k(u.cache_tokens) + ' cached');
    if (u.model) p.push(u.model + (ran ? ' · ' + ran : ''));
    else if (ran) p.push(ran);
    if (u.turns) p.push(u.turns + (u.turns === 1 ? ' turn' : ' turns'));
    return p.join(' · ');
  };

  function sec(title, body, n) {
    return '<section class="asec"><h4>' + esc(title) + (n != null ? ' <span class="n">' + n + '</span>' : '') + '</h4>' + body + '</section>';
  }
  function none(t) { return '<p class="quiet sm">' + esc(t) + '</p>'; }

  function status(a, o) {
    var st = T.agentState(a), h = '<div class="astat"><span class="sbadge ' + st + '">' + (st === 'active' ? '<i class="pdot2"></i>' : '') + T.cap(st) + '</span>';
    if (o.clock) h += '<span class="mut">' + (a.status === 'working' ? 'running ' : 'took ') + T.agentClock(a) + '</span>';
    if (a.last_activity_at && st !== 'done' && st !== 'failed') h += '<span class="mut">last activity ' + agoLong(a.last_activity_at) + '</span>';
    /* the report is the full answer; the one-line summary only shows when it says something else */
    var text = o.sum === false ? '' : a.report || (T.sameText(a.summary, a.report) ? '' : a.summary);
    h += '</div>';
    if (a.usage) h += '<p class="mut sm usage" title="Tokens and cost from the lane log; model and effort it ran with">' + esc(T.usageLine(a.usage, a)) + '</p>';
    else if (T.ranText(a)) h += '<p class="mut sm usage" title="Model and effort it was started with">' + esc(T.ranText(a)) + '</p>';
    return h + (text ? '<div class="sum">' + T.mdBox(text, a.id + ':status') + '</div>' : '');
  }
  /* true when one text starts like (or sits inside) the other, ignoring markdown marks and spacing */
  function flat(t) { return String(t || '').toLowerCase().replace(/[`*_#>~\[\]()|-]/g, '').replace(/\s+/g, ' ').trim(); }
  T.sameText = function (a, b) {
    var x = flat(a), y = flat(b);
    if (!x || !y) return false;
    return y.indexOf(x.slice(0, 60)) >= 0 || x.indexOf(y.slice(0, 60)) >= 0;
  };
  /* the owner's request, on the root lane of a tree only (children inherit it from the collector) */
  function asked(a) {
    var s = T.rawState && T.rawState();
    if (!a.request || !a.request.text) return '';
    if (a.parent && s && T.agentById(s, a.parent)) return '';
    return sec('Asked by you', T.mdBox(a.request.text, a.id + ':asked'));
  }
  function tests(a) {
    var l = (a.tests || []).slice().reverse();
    if (!l.length) return none('No tests yet');
    return '<ul class="trows">' + l.map(function (t) {
      var r = RES[t.result] || 'unknown';
      var cnt = t.passed != null ? t.passed + ' passed' + (t.failed ? ', ' + t.failed + ' failed' : '') : '';
      return '<li><span class="rb ' + r + '">' + (r === 'unknown' ? 'no result' : r) + '</span><code title="' + esc(t.full || t.command) + '">' + esc(t.command) + '</code>' +
        (cnt ? '<span class="mut tcnt">' + esc(cnt) + '</span>' : '') + '<span class="mut when">' + esc(T.clockTime(t.at)) + '</span></li>';
    }).join('') + '</ul>';
  }
  function tools(a) {
    var l = a.tools || [];
    if (!l.length) return none('No tools used yet');
    return '<div class="chips">' + l.map(function (t) { return '<span class="chip">' + esc(t.name) + ' <b>' + t.count + '</b></span>'; }).join('') + '</div>';
  }
  /* the full patch of a finished lane: loaded on demand from diffs/<lane>.js next to state.js */
  var diffOpen = {};
  function diffHtml(a) {
    var all = window.TIRITH_DIFFS || {}, text = all[a.id];
    if (text == null) return '<p class="quiet sm">Loading the diff…</p>';
    var lines = text.split('\n'), MAX = 4000;
    var h = lines.slice(0, MAX).map(function (ln) {
      var c = ln.indexOf('+++') === 0 || ln.indexOf('---') === 0 || ln.indexOf('diff ') === 0 || ln.indexOf('index ') === 0 || ln.indexOf('new file') === 0 ? 'dm' :
        ln.charAt(0) === '+' ? 'da' : ln.charAt(0) === '-' ? 'dd' : ln.charAt(0) === '@' ? 'dh' : '';
      return '<span class="dl ' + c + '">' + esc(ln) + '</span>';
    }).join('');
    return '<pre class="diff" tabindex="0">' + h + '</pre>' + (lines.length > MAX ? '<p class="mut sm">and ' + (lines.length - MAX) + ' more lines, open the patch file for all</p>' : '');
  }
  function diffBox(a) {
    if (!a.diff || !a.diff.size) return '';
    var open = !!diffOpen[a.id];
    return '<p class="sm"><button class="link" data-act="diff" data-key="' + esc(a.id) + '">' + (open ? 'Hide diff' : 'View diff') + '</button> <span class="mut">' + k(a.diff.size) + 'B patch</span></p>' + (open ? diffHtml(a) : '');
  }
  function files(a) {
    var l = a.files || [];
    if (!l.length) return diffBox(a) || none('No files changed yet');
    var h = diffBox(a) + '<ul class="frows">' + l.slice(0, MAXFILES).map(function (f) {
      var k = CH[f.change] || '?', ins = f.insertions || 0, del = f.deletions || 0, i = f.path.lastIndexOf('/');
      return '<li><span class="cb ' + esc(f.change) + '" title="' + esc(f.change) + '">' + k + '</span><span class="fp" title="' + esc(f.path) + '">' +
        (i >= 0 ? '<span class="mut">' + esc(f.path.slice(0, i + 1)) + '</span>' : '') + esc(f.path.slice(i + 1)) + '</span>' +
        (ins || del ? '<span class="fn"><span class="plus">+' + ins + '</span> <span class="minus">−' + del + '</span></span>' : '') + '</li>';
    }).join('') + '</ul>';
    if (l.length > MAXFILES) h += '<p class="mut sm">and ' + (l.length - MAXFILES) + ' more</p>';
    return h;
  }
  function events(a) {
    var l = (a.events || []).slice(-MAXEVENTS).reverse();
    if (!l.length) return none('No activity yet');
    return '<ul class="erows">' + l.map(function (e) {
      return '<li><span class="mut t">' + esc(T.clockTime(e.ts)) + '</span><span class="tool">' + esc(e.tool || e.event) + '</span><span class="what" title="' + esc(e.summary) + '">' + esc(e.summary) + '</span></li>';
    }).join('') + '</ul>';
  }

  /* The state carries a trimmed agent (see docs/CONTRACT-agents.md); its full log, events, files, tests and
     report sit in agents/<id>.js, loaded when the drawer opens: a <script> tag (file:// works) or, in the
     desktop app, the agent_detail command. Returns the agent with the detail merged in once it is there. */
  var detailCache = {};
  function loadDetail(a, c) {
    var want = a.detail.hash;
    c.loading = true;
    var done = function (data) {
      c.loading = false;
      if (data && data.id === a.id) c.data = data;
      c.hash = want;   /* a missing file is not retried until the hash changes */
      T.render();
    };
    if (T.tauri) {
      T.tauri.core.invoke('agent_detail', { id: a.id }).then(done, function () { done(null); });
      return;
    }
    var sc = document.createElement('script');
    sc.src = 'file://' + a.detail.path + '?h=' + encodeURIComponent(want);
    sc.onload = function () { var d = window.TIRITH_AGENT; delete window.TIRITH_AGENT; sc.remove(); done(d); };
    sc.onerror = function () { sc.remove(); done(null); };
    document.head.appendChild(sc);
  }
  T.agentFull = function (a) {
    if (!a || !a.detail || !a.detail_more) return a;
    var c = detailCache[a.id] || (detailCache[a.id] = {});
    if (c.hash !== a.detail.hash && !c.loading) loadDetail(a, c);
    if (!c.data) return a;
    var out = {}, k;
    for (k in a) out[k] = a[k];
    for (k in c.data) if (k !== 'id') out[k] = c.data[k];
    out.report_clipped = false;
    return out;
  };

  /* opts.goal: show the Goal section (off when the card beside it already shows the same text);
     opts.sum: show the one-line summary (off when the card beside it shows it);
     opts.clock: show the running time (off when the card beside it already shows it) */
  T.activity = function (a, opts) {
    var o = opts || {}, h = '';
    a = T.agentFull(a);
    h += asked(a);
    if (T.agentRules) h += T.agentRules(a);
    if (o.goal !== false && a.goal) h += sec('Goal', '<div class="agoal">' + T.mdBox(a.goal, a.id + ':goal') + '</div>');
    h += sec('Status', status(a, o));
    if (T.agentDecisions) h += T.agentDecisions(a);
    if (o.log !== false) h += sec('Log', T.logView(a));
    h += sec('Tests', tests(a), (a.tests || []).length || null);
    h += sec('Tools', tools(a));
    h += sec('Files', files(a), (a.files || []).length || null);
    h += sec('Live activity', events(a));
    return '<div class="activity">' + h + '</div>';
  };

  document.addEventListener('click', function (e) {
    var b = e.target.closest && e.target.closest('[data-act="diff"]'); if (!b) return;
    var id = b.getAttribute('data-key'), s = T.rawState && T.rawState(), a = s && T.agentById(s, id);
    diffOpen[id] = !diffOpen[id];
    if (diffOpen[id] && a && a.diff && !(window.TIRITH_DIFFS || {})[id]) {
      var sc = document.createElement('script');
      sc.src = 'file://' + a.diff.path.replace(/\.patch$/, '.js');
      sc.onload = sc.onerror = function () {
        sc.remove();
        window.TIRITH_DIFFS = window.TIRITH_DIFFS || {};
        if (window.TIRITH_DIFFS[id] == null) window.TIRITH_DIFFS[id] = '(the patch file could not be loaded)';
        T.render();
      };
      document.head.appendChild(sc);
    }
    T.render();
  });
})();
