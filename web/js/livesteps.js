/* The live transcript of a headless agent, in CLI style: what the manager (or a lane) is doing, step by step.
   The collector writes agents/<id>.steps.js (steps: [{kind: text|tool|result, name, summary, at, error?}]) and
   gives the agent row `live` ({steps, done, error, cost_usd, duration_s, elapsed_s, detail: {path, hash}}).
   Working: the newest steps and a "<who> is working · 23s" line. Ended: one line "12 steps · 48s · $0.21 · show",
   a click opens the whole transcript. The chat puts one under the owner message the turn answers
   (row.turn_requests); the agent detail view has the same block. */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  ui.liveOpen = ui.liveOpen || {};   /* agent id -> the full transcript is open */
  var SHOWN = 8, cache = {};          /* agent id -> {hash, steps, loading} */

  /* the steps file of one agent: a <script> tag (file:// works) or the desktop app's agent_detail command */
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

  function step(s) {
    var text = esc(s.summary || '');
    if (s.kind === 'tool') return '<li class="ls-tool"><span class="ls-dot">●</span><span class="ls-t">' + text + '</span></li>';
    if (s.kind === 'result') return '<li class="ls-res' + (s.error ? ' bad' : '') + '"><span class="ls-dot">⎿</span><span class="ls-t">' + text + '</span></li>';
    return '<li class="ls-say"><span class="ls-t">' + text + '</span></li>';
  }
  function who(row) { return row.id.indexOf('manager-') === 0 ? 'Manager' : (row.name || 'Agent'); }
  function working(row) { return !row.ended_at && !(row.live && row.live.done) && row.status !== 'failed'; }

  /* the block for one agent row ('' when it has no steps yet) */
  T.liveBlock = function (row) {
    var lv = row && row.live;
    if (!lv || !lv.steps) return '';
    var c = load(row), steps = c.steps || [], run = working(row), open = !!ui.liveOpen[row.id];
    if (run) {
      var tail = steps.slice(-SHOWN);
      return '<div class="livesteps run" data-live="' + esc(row.id) + '"><ul class="ls-list" aria-live="polite">' + tail.map(step).join('') + '</ul>' +
        '<div class="ls-line" role="status"><span class="ls-spin" aria-hidden="true"></span>' + esc(who(row)) + ' is working · ' +
        (row.started_at ? T.span(row.started_at) : '<span>' + esc(T.dur(lv.elapsed_s)) + '</span>') + '</div></div>';
    }
    var secs = lv.duration_s != null ? lv.duration_s : lv.elapsed_s,
      parts = [lv.steps + (lv.steps === 1 ? ' step' : ' steps'), T.dur(secs)];
    if (lv.cost_usd != null) parts.push('$' + Number(lv.cost_usd).toFixed(2));
    return '<div class="livesteps done' + (lv.error ? ' bad' : '') + (open ? ' open' : '') + '" data-live="' + esc(row.id) + '">' +
      '<button type="button" class="ls-sum" data-act="live-toggle" data-id="' + esc(row.id) + '" aria-expanded="' + open + '">' +
      esc(parts.filter(Boolean).join(' · ')) + (lv.error ? ' · failed' : '') + ' · <span class="ls-show">' + (open ? 'hide' : 'show') + '</span></button>' +
      (open ? '<ul class="ls-list">' + (steps.length ? steps.map(step).join('') : '<li class="ls-say"><span class="ls-t">Loading…</span></li>') + '</ul>' : '') + '</div>';
  };

  /* the newest agent that answers the request with this id (the on-call manager's turn), else null */
  T.liveForRequest = function (state, reqId) {
    var rows = (state.agents || []).filter(function (a) { return a.live && (a.turn_requests || []).indexOf(reqId) >= 0; });
    rows.sort(function (a, b) { return String(b.started_at || '').localeCompare(String(a.started_at || '')); });
    return rows[0] || null;
  };
  T.liveWorking = function (row) { return !!row && !!row.live && working(row); };

  document.addEventListener('click', function (e) {
    var b = e.target.closest && e.target.closest('[data-act="live-toggle"]');
    if (!b) return;
    var id = b.getAttribute('data-id');
    ui.liveOpen[id] = !ui.liveOpen[id];
    T.render();
  });
})();
