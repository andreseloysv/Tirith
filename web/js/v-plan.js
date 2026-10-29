/* Plan card(s) at the top of the Overview: a project's roadmap, step by step. */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  ui.planOpen = ui.planOpen || {};

  var IC = {
    done: '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="7" fill="#2ea44f"/><path d="M4.8 8.2l2.2 2.2 4.2-4.6" fill="none" stroke="#fff" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>',
    active: '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="6.2" fill="#fff" stroke="#2f6fed" stroke-width="1.6"/><path d="M8 3.4a4.6 4.6 0 010 9.2z" fill="#2f6fed"/></svg>',
    todo: '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="6.2" fill="none" stroke="#b4bac4" stroke-width="1.6"/></svg>',
    empty: '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="6.2" fill="none" stroke="#d3d7de" stroke-width="1.6" stroke-dasharray="2 2.4"/></svg>'
  };
  var OK = '<svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"><path d="M2.6 6.3l2.3 2.3 4.5-4.9" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>';
  var OPEN = '<svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"><circle cx="6" cy="6" r="3.6" fill="none" stroke="#b4bac4" stroke-width="1.3"/></svg>';

  function plain(t) { return esc(String(t).replace(/`/g, '')); }
  function rich(t) { return esc(t).replace(/`([^`]+)`/g, '<code>$1</code>'); }

  /* "6 d 4 h left", "5 h 12 min left", "overdue" */
  function left(iso) {
    var t = T.ms(iso); if (t == null) return null;
    var s = Math.floor((t - T.now()) / 1000), d, h, m;
    if (s <= 0) return { txt: 'Overdue', hot: true };
    d = Math.floor(s / 86400); h = Math.floor((s % 86400) / 3600); m = Math.floor((s % 3600) / 60);
    return { hot: s < 86400, txt: (d ? d + ' d ' + h + ' h' : h ? h + ' h ' + m + ' min' : Math.max(1, m) + ' min') + ' left' };
  }
  function countdown(iso) {
    var c = left(iso); if (!c) return '';
    return '<span class="pcd' + (c.hot ? ' hot' : '') + '" data-due="' + esc(iso) + '">' + esc(c.txt) + '</span>';
  }
  function tickCountdowns() {
    var els = document.querySelectorAll('.pcd[data-due]'), i, c;
    for (i = 0; i < els.length; i++) {
      c = left(els[i].getAttribute('data-due')); if (!c) continue;
      els[i].textContent = c.txt; els[i].className = 'pcd' + (c.hot ? ' hot' : '');
    }
  }

  function bar(done, total, cls) {
    return '<span class="bar ' + (cls || '') + '"><i class="' + (done === total && total ? 'green' : 'blue') + '" style="width:' + (total ? Math.round(100 * done / total) : 0) + '%"></i></span>';
  }

  function step(s, p, state) {
    var key = p.id + '|' + s.id, open = !!ui.planOpen[key], live = s.agents && s.agents.length;
    var ava = (s.agents || []).map(function (id) {
      var a = T.agentById(state, id); if (!a) return '';
      return '<button class="pava" data-act="goagent" data-key="' + esc(id) + '" title="' + esc(a.name + ' · ' + (a.feature || '')) + '">' + T.avatar(a, 22) + '</button>';
    }).join('');
    var h = '<li class="pstep ' + esc(s.status) + (open ? ' open' : '') + '"><div class="prow">' +
      '<button class="ptog" data-act="plan" data-key="' + esc(key) + '" aria-expanded="' + open + '">' +
      '<span class="pico' + (live ? ' pulse' : '') + '">' + (IC[s.status] || IC.todo) + '</span>' +
      '<span class="ptit"><span class="pn">' + esc(s.id) + '.</span> ' + plain(s.title) + '</span>' +
      (s.total ? bar(s.done, s.total, 'pbar') + '<span class="pcnt">' + s.done + '/' + s.total + '</span>' : '<span class="pbar"></span><span class="pcnt">–</span>') +
      '</button><span class="pavas">' + ava + '</span></div>';
    if (open) {
      h += '<ul class="pitems">' + (s.items.length ? s.items.map(function (i) {
        return '<li class="' + (i.done ? 'ok' : '') + '"><span class="pi">' + (i.done ? OK : OPEN) + '</span><span>' + rich(i.text) + '</span></li>';
      }).join('') : '<li class="none">No items yet</li>') + '</ul>';
    }
    return h + '</li>';
  }

  function card(p, state, multi) {
    var r = p.roadmap;
    return '<div class="card pcard' + (multi ? ' compact' : '') + '"><div class="phead"><b>' + esc(p.name) + '</b>' +
      bar(r.done, r.total, 'ptotal') + '<span class="pcnt big">' + r.done + ' / ' + r.total + '</span>' + countdown(r.deadline) + '</div>' +
      '<ul class="psteps">' + r.steps.map(function (s) { return step(s, p, state); }).join('') + '</ul></div>';
  }

  /* HTML for the Plan section, or '' when no visible project has a roadmap. */
  T.plan = function (state) {
    var f = T.filter.project, ps = (state.projects || []).filter(function (p) {
      return p.roadmap && p.roadmap.steps && (!f || p.id === f);
    });
    if (!ps.length) return '';
    return '<section class="plan"><h2>Plan</h2>' + ps.map(function (p) { return card(p, state, ps.length > 1); }).join('') + '</section>';
  };

  document.getElementById('view').addEventListener('click', function (e) {
    var b = e.target.closest('[data-act="plan"]'); if (!b) return;
    var k = b.getAttribute('data-key');
    ui.planOpen[k] = !ui.planOpen[k];
    T.render();
  });
  setInterval(tickCountdowns, 30000);
})();
