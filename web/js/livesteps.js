/* The live transcript of a headless agent, in CLI style: what the manager (or a lane) is doing, step by step.
   The collector writes agents/<id>.steps.js (steps: [{kind: text|tool|result|todo, name, summary, at, error?, partial?,
   diff?, todos?}]) and gives the row `live` ({steps, done, error, cost_usd, duration_s, elapsed_s, detail: {path, hash}}).
   Rows are agent rows (state.agents) or the turns of the persistent manager session (state.manager.turns, same shape,
   plus `lanes`: the sub-agent lanes started during the turn, drawn as nested blocks).
   Working: the newest steps, the task list (TodoWrite, one checklist updated in place), the text as it is typed, and a
   "<who> is working · 23s" line with Stop. Ended: one line "12 steps · 48s · $0.21 · show", a click opens the whole
   transcript. Edit / MultiEdit / Write steps show a coloured diff (long ones collapsed after 20 lines).
   The chat puts one under the owner message the turn answers (row.turn_requests). */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  ui.liveOpen = ui.liveOpen || {};   /* row id -> the full transcript is open */
  ui.diffOpen = ui.diffOpen || {};   /* "<row id>:<step index>" -> the whole diff is open */
  var SHOWN = 8, DIFF_SHOWN = 20, cache = {};   /* row id -> {hash, steps, loading} */

  /* the steps file of one row: a <script> tag (file:// works) or the desktop app's agent_detail command */
  function load(row) {
    var d = row.live && row.live.detail, c = cache[row.id] || (cache[row.id] = {});
    if (!d || c.loading || c.hash === d.hash) return c;
    var want = d.hash;
    c.loading = true;
    var done = function (data) {
      c.loading = false;
      c.hash = want;   /* a missing file is not retried until the hash changes */
      if (data && Array.isArray(data.steps)) c.steps = data.steps;
      T.render();
    };
    if (T.tauri) {
      T.tauri.core.invoke('agent_detail', { id: row.id + '.steps' }).then(done, function () { done(null); });
      return c;
    }
    var sc = document.createElement('script');
    sc.src = 'file://' + d.path + '?h=' + encodeURIComponent(want);
    sc.onload = function () { var data = window.TIRITH_AGENT; delete window.TIRITH_AGENT; sc.remove(); done(data); };
    sc.onerror = function () { sc.remove(); done(null); };
    document.head.appendChild(sc);
    return c;
  }

  function diff(s, key) {
    var d = s.diff, rows = d.rows || [], open = !!ui.diffOpen[key], shown = open ? rows : rows.slice(0, DIFF_SHOWN), hidden = rows.length - shown.length;
    return '<div class="ls-diff"><div class="ls-dh">' + esc(d.path) + ' <span class="ls-dn">+' + rows.filter(function (r) { return r[0] === '+'; }).length +
      ' −' + rows.filter(function (r) { return r[0] === '-'; }).length + '</span></div>' +
      shown.map(function (r) { return '<div class="ls-dl ' + (r[0] === '+' ? 'add' : 'del') + '"><span class="ls-sg">' + (r[0] === '+' ? '+' : '−') + '</span>' + esc(r[1]) + '</div>'; }).join('') +
      (hidden > 0 || (open && rows.length > DIFF_SHOWN)
        ? '<button type="button" class="ls-more" data-act="diff-toggle" data-key="' + esc(key) + '">' + (open ? 'collapse' : 'show all ' + rows.length + ' lines') + '</button>' : '') + '</div>';
  }
  function todo(s) {
    var mark = { completed: '☑', in_progress: '▶', pending: '☐' };
    return '<li class="ls-todo"><ul class="ls-checks" aria-label="Task list">' + (s.todos || []).map(function (t) {
      return '<li class="ls-ck ' + esc(t.status) + '"><span class="ls-dot">' + (mark[t.status] || '☐') + '</span><span class="ls-t">' + esc(t.content) + '</span></li>';
    }).join('') + '</ul></li>';
  }
  function step(s, key) {
    var text = esc(s.summary || '');
    if (s.kind === 'todo') return todo(s);
    if (s.kind === 'tool') return '<li class="ls-tool"><span class="ls-dot">●</span><span class="ls-t">' + text + (s.diff ? diff(s, key) : '') + '</span></li>';
    if (s.kind === 'result') return '<li class="ls-res' + (s.error ? ' bad' : '') + '"><span class="ls-dot">⎿</span><span class="ls-t">' + text + '</span></li>';
    return '<li class="ls-say' + (s.partial ? ' typing' : '') + '"><span class="ls-t">' + text + (s.partial ? '<span class="ls-cur" aria-hidden="true">▍</span>' : '') + '</span></li>';
  }
  function steps(row, list) { return list.map(function (s, i) { return step(s, row.id + ':' + i); }).join(''); }
  function who(row) { return row.id.indexOf('manager-') === 0 ? 'Manager' : (row.name || 'Agent'); }
  function working(row) { return !row.ended_at && !(row.live && row.live.done) && row.status !== 'failed'; }
  function turnRow(row) { return row.id.indexOf('manager-turn-') === 0; }

  /* the lanes the manager started during a turn: nested blocks with their own live steps */
  function nested(row, state) {
    var ids = row.lanes || [], rows = ((state && state.agents) || []).filter(function (a) { return ids.indexOf(a.id) >= 0; });
    return rows.length ? '<div class="ls-nest">' + rows.map(function (a) {
      var lv = a.live ? T.liveBlock(a, state) : '';
      return '<div class="ls-lane"><div class="ls-lh">↳ ' + esc(a.feature || a.name || a.id) + ' · <span class="ls-who">' + esc(a.name || a.id) + '</span> · ' + esc(a.status) + '</div>' +
        (lv || '<div class="ls-none">no steps yet</div>') + '</div>';
    }).join('') + '</div>' : '';
  }

  /* the block for one row ('' when it has nothing to show yet) */
  T.liveBlock = function (row, state) {
    var lv = row && row.live, run = !!row && working(row);
    if (!lv || (!lv.steps && !(run && turnRow(row)))) return '';
    var c = load(row), all = c.steps || [], open = !!ui.liveOpen[row.id];
    if (run) {
      var pin = all.filter(function (s) { return s.kind === 'todo'; }).slice(-1), tail = all.filter(function (s) { return s.kind !== 'todo'; }).slice(-SHOWN);
      var list = pin.concat(tail);
      return '<div class="livesteps run" data-live="' + esc(row.id) + '">' + (list.length ? '<ul class="ls-list" aria-live="polite">' + steps(row, list) + '</ul>' : '') + nested(row, state) +
        '<div class="ls-line" role="status"><span class="ls-spin" aria-hidden="true"></span>' + esc(who(row)) + ' is working · ' +
        (row.started_at ? T.span(row.started_at) : '<span>' + esc(T.dur(lv.elapsed_s)) + '</span>') +
        (turnRow(row) ? ' <button type="button" class="ls-stop" data-act="mc-interrupt" title="Stop this turn (Esc)">Stop</button>' : '') + '</div></div>';
    }
    var secs = lv.duration_s != null ? lv.duration_s : lv.elapsed_s,
      parts = [lv.steps + (lv.steps === 1 ? ' step' : ' steps'), T.dur(secs)];
    if (lv.cost_usd != null) parts.push('$' + Number(lv.cost_usd).toFixed(2));
    return '<div class="livesteps done' + (lv.error ? ' bad' : '') + (open ? ' open' : '') + '" data-live="' + esc(row.id) + '">' +
      '<button type="button" class="ls-sum" data-act="live-toggle" data-id="' + esc(row.id) + '" aria-expanded="' + open + '">' +
      esc(parts.filter(Boolean).join(' · ')) + (row.interrupted ? ' · stopped' : lv.error ? ' · failed' : '') + ' · <span class="ls-show">' + (open ? 'hide' : 'show') + '</span></button>' +
      (open ? '<ul class="ls-list">' + (all.length ? steps(row, all) : '<li class="ls-say"><span class="ls-t">Loading…</span></li>') + '</ul>' + nested(row, state) : '') + '</div>';
  };

  /* the newest row (agent or manager turn) that answers the request with this id, else null */
  T.liveForRequest = function (state, reqId) {
    var rows = (state.agents || []).concat((state.manager && state.manager.turns) || []).filter(function (a) { return a.live && (a.turn_requests || []).indexOf(reqId) >= 0; });
    rows.sort(function (a, b) { return String(b.started_at || '').localeCompare(String(a.started_at || '')); });
    return rows[0] || null;
  };
  T.liveWorking = function (row) { return !!row && !!row.live && working(row); };
  /* the manager turn that is running now, else null */
  T.managerTurn = function (state) {
    var rows = ((state && state.manager && state.manager.turns) || []).filter(function (t) { return working(t); });
    return rows[rows.length - 1] || null;
  };

  document.addEventListener('click', function (e) {
    var b = e.target.closest && e.target.closest('[data-act="live-toggle"],[data-act="diff-toggle"]');
    if (!b) return;
    if (b.getAttribute('data-act') === 'diff-toggle') { var k = b.getAttribute('data-key'); ui.diffOpen[k] = !ui.diffOpen[k]; }
    else { var id = b.getAttribute('data-id'); ui.liveOpen[id] = !ui.liveOpen[id]; }
    T.render();
  });
})();
