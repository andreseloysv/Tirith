/* Rules broken (blocked actions) and decisions: the Overview section and the agent-panel lists. */
(function () {
  var T = window.T, esc = T.esc, DAY = 86400000;
  var RULE = {
    'subagent-tool': 'Sub-agent tool', 'agent-cli': 'Agent without lane run', 'no-verify': 'Skipped git hooks',
    'hooks-path': 'Changed hooks path', 'git-hooks': 'Edited git hooks', 'raw-stage': 'Raw pipeline stage',
    'settings-edit': 'Edited agent settings', 'hooks-removed': 'Hooks removed',
    'lane-brief': 'Lane without brief', 'project-check': 'Project check failing'
  };
  T.ruleLabel = function (id) { return RULE[id] || T.cap(String(id || 'rule').replace(/[-_]+/g, ' ')); };

  /* violations of the last 24 hours, newest first */
  T.recentViolations = function (s) {
    var t = T.now();
    return (s.violations || []).filter(function (v) { var m = T.ms(v.ts); return m != null && t - m <= DAY; })
      .sort(function (a, b) { return (T.ms(b.ts) || 0) - (T.ms(a.ts) || 0); });
  };
  T.violationsOf = function (s, laneId) {
    return (s.violations || []).filter(function (v) { return v.lane === laneId; })
      .sort(function (a, b) { return (T.ms(b.ts) || 0) - (T.ms(a.ts) || 0); });
  };
  T.violationText = function (v) { return T.ruleLabel(v.rule) + ': ' + (v.blocked || v.message); };

  /* command with a copy button */
  function doThis(v) {
    if (!v.instead) return '';
    return '<span class="vdo"><span class="mut">Do this instead</span><code title="' + esc(v.instead) + '">' + esc(v.instead) + '</code>' +
      '<button class="vcopy" data-act="copy" data-key="' + esc(v.instead) + '" aria-label="Copy command" title="Copy command">Copy</button></span>';
  }
  function who(s, v) {
    var a = v.lane && T.agentById(s, v.lane);
    if (a) return '<button class="vwho" data-act="goagent" data-key="' + esc(a.id) + '" title="Open ' + esc(a.name) + '">' + T.avatar(a, 22) + '<b>' + esc(a.name) + '</b></button>';
    return '<span class="vwho"><b>' + esc(v.lane || v.agent || 'An agent') + '</b></span>';
  }

  /* Overview: one row per rule that fired in the last 24 hours; the blocked commands are behind the row */
  T.ui.ruleOpen = T.ui.ruleOpen || {};  /* rule id -> true while its blocked commands are shown */
  function describe(s, id) {
    var r = (s.rules || []).filter(function (x) { return x.id === id; })[0];
    return r && r.description ? r.description : T.ruleLabel(id);
  }
  function byWho(s, v) {
    var a = v.lane && T.agentById(s, v.lane);
    if (a) return '<span class="vwho">' + T.avatar(a, 18) + '<b>' + esc(a.name) + '</b></span>';
    return '<span class="vwho"><b>' + esc(v.lane || v.agent || 'An agent') + '</b></span>';
  }
  T.rulesSection = function (s) {
    var list = T.recentViolations(s), order = [], by = {};
    list.forEach(function (v) { if (!by[v.rule]) { by[v.rule] = []; order.push(v.rule); } by[v.rule].push(v); });
    if (!order.length) return '';
    return '<section><h2>Rules <span class="n bad">' + list.length + '</span></h2><ul class="vlist card">' + order.map(function (id) {
      var vs = by[id], last = vs[0], open = !!T.ui.ruleOpen[id];
      return '<li class="rrow' + (open ? ' open' : '') + '"><button class="rhead" data-act="ruletoggle" data-key="' + esc(id) + '" aria-expanded="' + open + '">' +
        '<span class="rdesc">' + esc(describe(s, id)) + '</span>' +
        '<span class="rmeta">' + vs.length + (vs.length === 1 ? ' time' : ' times') + ' today · last ' + esc(T.clockTime(last.ts)) + ' by ' + byWho(s, last) + '</span>' +
        '<span class="rchev" aria-hidden="true">' + (open ? '▾' : '▸') + '</span></button>' +
        (open ? '<ul class="rblocked">' + vs.map(function (v) {
          return '<li><span class="vt">' + esc(T.clockTime(v.ts)) + '</span>' + byWho(s, v) +
            '<span class="vblocked" title="' + esc(v.blocked || v.message) + '">' + esc(v.blocked || v.message) + '</span></li>';
        }).join('') + '</ul>' : '') + '</li>';
    }).join('') + '</ul></section>';
  };
  document.getElementById('view').addEventListener('click', function (e) {
    var b = e.target.closest('[data-act="ruletoggle"]'); if (!b) return;
    var id = b.getAttribute('data-key'); T.ui.ruleOpen[id] = !T.ui.ruleOpen[id]; T.render();
  });

  /* agent panel: red list of what this agent was not allowed to do */
  T.agentRules = function (a) {
    var s = T.rawState && T.rawState(), list = s ? T.violationsOf(s, a.id) : [];
    if (!list.length) return '';
    return '<section class="asec rules"><h4>Rules broken <span class="n bad">' + list.length + '</span></h4><ul class="vrows">' + list.map(function (v) {
      return '<li><span class="vline"><span class="vrule">' + esc(T.ruleLabel(v.rule)) + '</span><span class="mut">' + esc(T.clockTime(v.ts)) + '</span></span>' +
        '<span class="vblocked">' + esc(v.blocked || v.message) + '</span>' + doThis(v) + '</li>';
    }).join('') + '</ul></section>';
  };

  /* agent panel: decisions this agent took, with the reason */
  T.agentDecisions = function (a) {
    var s = T.rawState && T.rawState();
    var list = s ? (s.flows || []).filter(function (f) { return f.kind === 'decision' && f.from === 'agent:' + a.id; }).reverse() : [];
    if (!list.length) return '';
    return '<section class="asec"><h4>Decisions <span class="n">' + list.length + '</span></h4><ul class="drows">' + list.map(function (f) {
      return '<li><span class="dtext">' + T.mdInline(f.text) + '</span>' + (f.why ? '<span class="dwhy">' + T.mdInline(f.why) + '</span>' : '') + '</li>';
    }).join('') + '</ul></section>';
  };

  T.copyText = function (text, btn) {
    var done = function () {
      if (!btn) return;
      btn.textContent = 'Copied';
      setTimeout(function () { btn.textContent = 'Copy'; }, 1400);
    };
    var fallback = function () {
      var ta = document.createElement('textarea');
      ta.value = text; ta.setAttribute('readonly', ''); ta.style.cssText = 'position:fixed;opacity:0';
      document.body.appendChild(ta); ta.select();
      try { document.execCommand('copy'); done(); } catch (e) {}
      document.body.removeChild(ta);
    };
    if (navigator.clipboard && navigator.clipboard.writeText) navigator.clipboard.writeText(text).then(done, fallback);
    else fallback();
  };
})();
