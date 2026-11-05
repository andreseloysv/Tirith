/* "Running Tirith": what the manager and the CEO are doing, the way the worker cards show what a lane is doing.
   Manager: the turn running now (what it handles, its live steps), else that it waits; its last decisions (its
   answers in state.inbox) and today's turns and cost (state.manager.turns). CEO: state.ceo (last and next run, the
   short report, open findings, directives, proposals for the owner); docs/CONTRACT-ceo.md. Both read only. */
(function () {
  var T = window.T, esc = T.esc;
  var DECISIONS = 5, TEXT = 140;
  var KIND = { chat: 'your message', ask: 'your question', finish: 'finish a step', 'autopilot-proposal': 'autopilot',
    'plan-refresh': 'plan refresh', 'plan-new': 'new plan', 'plan-edit': 'plan edit', setup: 'setup', reply: 'your reply',
    directive: 'CEO directive' };

  function clip(t, n) { t = String(t || '').replace(/\s+/g, ' ').trim(); return t.length > n ? t.slice(0, n - 1) + '…' : t; }
  function today(iso) { return String(iso || '').slice(0, 10) === new Date().toISOString().slice(0, 10); }
  function label(r) {
    var k = KIND[r.kind] || r.kind || 'request';
    return k + (r.kind === 'autopilot-proposal' && r.step ? ' step ' + r.step : '') + (r.project ? ' · ' + T.proj(r.project) : '');
  }
  function managerAgent(state) {
    var rows = (state.agents || []).filter(function (a) { return String(a.id || '').indexOf('manager-') === 0; });
    rows.sort(function (a, b) { return String(b.started_at || '').localeCompare(String(a.started_at || '')); });
    return rows.filter(function (a) { return a.status === 'working'; })[0] || rows[0] || null;
  }
  function decisions(state) {
    var out = [];
    (state.inbox || []).forEach(function (r) {
      (r.answers || []).forEach(function (a) { if (a.by === 'manager') out.push({ at: a.at, text: a.text, req: r }); });
    });
    out.sort(function (a, b) { return String(b.at || '').localeCompare(String(a.at || '')); });
    return out.slice(0, DECISIONS);
  }
  function list(items) { return items.length ? '<ul class="lead-list">' + items.join('') + '</ul>' : ''; }
  function head(a, title, pill) {
    return '<span class="ahead">' + (a ? T.avatar(a, 36) : '<span class="lead-mark">' + esc(title.charAt(0)) + '</span>') +
      '<span class="aname"><b>' + esc(title) + '</b>' + (pill ? '<span class="pill">' + esc(pill) + '</span>' : '') + '</span></span>';
  }

  function managerCard(state) {
    var m = state.manager || {}, a = managerAgent(state), turn = T.managerTurn ? T.managerTurn(state) : null;
    var byId = {}; (state.inbox || []).forEach(function (r) { byId[r.id] = r; });
    var ran = a && a.ran ? [a.ran.model, a.ran.effort].filter(Boolean).join(' · ') : '';
    var now;
    if (turn) {
      var handled = (turn.turn_requests || []).map(function (id) { return byId[id]; }).filter(Boolean);
      now = '<span class="anow live"><i class="pdot2"></i>Working on ' + (handled.length ?
        handled.map(function (r) { return esc(label(r)) + (r.text ? ': ' + esc(clip(r.text, 60)) : ''); }).join('; ') :
        'Tirith\'s own work (queue, reviews)') + '</span>' + (T.liveBlock ? T.liveBlock(turn, state) : '');
    } else {
      var last = decisions(state)[0];
      now = '<span class="anow">' + (m.presence && m.presence.working ? 'Waiting for the next message' : 'Not running: it starts when something is pending') +
        (last ? ' · last answer ' + esc(T.clockTime(last.at)) : '') + '</span>';
    }
    var turns = (m.turns || []).filter(function (t) { return today(t.started_at); });
    var cost = turns.reduce(function (s, t) { return s + ((t.live && t.live.cost_usd) || 0); }, 0);
    var ctx = m.presence && m.presence.session && m.presence.session.context_tokens;
    var made = decisions(state).map(function (d) {
      return '<li><span class="mut">' + esc(T.clockTime(d.at)) + ' · ' + esc(label(d.req)) + '</span> ' + T.mdInline(T.boldLead ? T.boldLead(clip(d.text, TEXT)) : clip(d.text, TEXT)) + '</li>';
    });
    return '<div class="card acard lead">' + head(a, 'Manager', a && a.name ? a.name : '') +
      (ran ? '<span class="mut">' + esc(ran) + '</span>' : '') + now +
      (made.length ? '<span class="lead-h">Last decisions</span>' + list(made) : '') +
      '<span class="afoot mut">Today: ' + turns.length + (turns.length === 1 ? ' turn' : ' turns') +
      (cost ? ' · $' + cost.toFixed(2) : '') + (ctx ? ' · context ' + Math.round(ctx / 1000) + 'k tokens' : '') + '</span></div>';
  }

  function ceoCard(state) {
    var c = state.ceo;
    if (!c || !c.enabled) {
      return '<div class="card acard lead">' + head(null, 'CEO', 'Supervises the manager') +
        '<span class="anow">Not running yet. A few times a day it will check how the manager runs the work, from the data, ' +
        'and send it directives or ask you (docs/CONTRACT-ceo.md).</span></div>';
    }
    var now = c.running ? '<span class="anow live"><i class="pdot2"></i>Reviewing how the work is run</span>' :
      '<span class="anow">' + (c.last_at ? 'Last review ' + esc(T.clockTime(c.last_at)) + (c.last_result ? ' · ' + esc(c.last_result) : '') : 'No review yet') +
      (c.next_at ? ' · next ' + esc(T.clockTime(c.next_at)) : '') + '</span>' +
      (c.last_result === 'failed' && c.last_error ? '<span class="mut">Its last review gave no usable result: ' + esc(clip(c.last_error, TEXT)) + '</span>' : '');
    var report = (c.report || []).map(function (l) { return '<li>' + T.mdInline(clip(l, TEXT)) + '</li>'; });
    var dirs = (c.directives || []).map(function (d) {
      return '<li><span class="mut">' + esc(d.status || 'open') + '</span> ' + T.mdInline(T.boldLead ? T.boldLead(clip(d.text, TEXT)) : clip(d.text, TEXT)) + '</li>';
    });
    var asks = (c.proposals || []).filter(function (p) { return !p.decided; }).map(function (p) { return '<li>' + T.mdInline(T.boldLead ? T.boldLead(clip(p.text, TEXT)) : clip(p.text, TEXT)) + '</li>'; });
    return '<div class="card acard lead">' + head(null, 'CEO', 'Supervises the manager') + now +
      (report.length ? '<span class="lead-h">Report</span>' + list(report) : '') +
      (dirs.length ? '<span class="lead-h">Directives to the manager</span>' + list(dirs) : '') +
      (asks.length ? '<span class="lead-h">Waiting for you</span>' + list(asks) : '') +
      '<span class="afoot mut">' + (c.findings_open || 0) + ' open findings' + (c.cost_today ? ' · today $' + Number(c.cost_today).toFixed(2) : '') + '</span></div>';
  }

  /* the section for the Overview (the Agents tab shows the manager and the CEO as cards of its board) */
  T.leaders = function (state) {
    return '<section class="leaders"><h2>Running Tirith</h2><div class="grid2">' + managerCard(state) + ceoCard(state) + '</div></section>';
  };
})();
