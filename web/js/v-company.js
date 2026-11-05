/* Company view: the timeline of <home>/company.jsonl (state.company), every promotion or retirement with when,
   which role, the rule with the value measured against its threshold, and why. Newest first.
   docs/CONTRACT-manifest.md, "roles". */
(function () {
  var T = window.T, esc = T.esc;
  var EVENT = { promote: 'Promoted', retire: 'Retired' };
  var RULE = {
    task_size: 'Task too big for one lane', lane_diff: 'Lane diff grew', parallel_items: 'Items in parallel',
    subagents: 'Task asks for sub-agents', workers: 'Workers running', idle_seconds: 'Idle time'
  };
  function when(iso) {
    var t = Date.parse(iso); if (!t) return '';
    var m = Math.max(0, Math.round((T.now() - t) / 60000));
    return m < 1 ? 'just now' : m < 60 ? m + ' min ago' : m < 1440 ? Math.round(m / 60) + ' h ago' : Math.round(m / 1440) + ' d ago';
  }
  function row(r) {
    var ev = r.event === 'retire' ? 'retire' : 'promote';
    return '<li class="co-row co-' + ev + '"><div class="co-when sm mut" title="' + esc(r.ts || '') + '">' + esc(when(r.ts)) + '</div>' +
      '<div><b>' + esc(EVENT[ev]) + '</b> <span class="chip">' + esc(r.role || '') + '</span> ' +
      '<span class="mut sm">' + esc(RULE[r.rule] || r.rule || '') + ': ' + esc(r.measured) + ' against ' + esc(r.threshold) + '</span>' +
      (r.reason ? '<div class="sm">' + esc(r.reason) + '</div>' : '') + '</div></li>';
  }
  T.views.company = function (state, el) {
    var rows = (state.company || []).slice().sort(function (a, b) { return String(b.ts).localeCompare(String(a.ts)); });
    el.innerHTML = '<div class="listwrap" data-scroll="company"><section class="pad"><h2>Company <span class="mut">' + rows.length + '</span></h2>' +
      '<p class="mut sm">Every time Tirith promoted or retired a worker, manager or CEO, when and why.</p>' +
      (rows.length ? '<ol class="co-list" style="list-style:none;padding:0;margin:0">' + rows.map(row).join('') + '</ol>'
        : '<p class="quiet mut sm co-empty">Nothing yet. Promotions and retirements appear here as the work needs them.</p>') +
      '</section></div>';
  };
})();
