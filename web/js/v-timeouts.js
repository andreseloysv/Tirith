/* Settings view, "Timeouts" card: how long each kind of command may run before Tirith stops it, and for agent
   runs and stages how long it may go without progress (docs/CONTRACT-timeouts.md). Read-only: the owner fixes a value in config.toml under [timeouts]. */
(function () {
  var T = window.T, esc = T.esc;
  var WHERE = {
    default: 'Tirith\'s starting value',
    learned: 'Learned from how long it usually takes',
    agent: 'Set by an agent until its lane ends',
    owner: 'Set by you',
    config: 'Fixed by you in config.toml'
  };

  function secs(n) {
    n = Math.round(+n || 0);
    if (n >= 3600 && n % 3600 === 0) return n / 3600 + ' h';
    if (n >= 60 && n % 60 === 0) return n / 60 + ' min';
    return n + ' s';
  }
  function ago(iso) {
    var t = Date.parse(iso); if (!t) return '';
    var m = Math.max(0, Math.round((T.now() - t) / 60000));
    return m < 1 ? 'just now' : m < 60 ? m + ' min ago' : m < 1440 ? Math.round(m / 60) + ' h ago' : Math.round(m / 1440) + ' d ago';
  }

  function idle(r) {
    var i = r.idle;
    if (!i) return '';
    return esc(secs(i.seconds)) + ' <span class="mut sm">without progress (' + esc((WHERE[i.source] || i.source).toLowerCase()) + ')</span><br>';
  }

  function row(r) {
    var last = r.last ? esc(ago(r.last.at)) + ' · ' + esc(secs(r.last.seconds)) + (r.last.command ? ' · <span class="mut">' + esc(r.last.command) + '</span>' : '') : '<span class="mut">none</span>';
    var why = esc(WHERE[r.source] || r.source) + (r.reason ? ' (' + esc(r.reason) + ')' : '');
    return '<tr><td><b>' + esc(r.label) + '</b><div class="mut sm">' + esc(r.words) + '</div></td>' +
      '<td>' + idle(r) + esc(secs(r.seconds)) + (r.idle ? ' <span class="mut sm">hard cap</span>' : '') + '</td><td class="sm">' + why + '</td>' +
      '<td class="sm">' + esc(secs(r.min)) + ' to ' + esc(secs(r.max)) + '</td><td class="sm">' + last + '</td></tr>';
  }

  function card(state) {
    var rows = state.timeouts;
    if (!rows || !rows.length) return '';
    return '<div class="card scard tmocard"><div class="phead"><b>Timeouts</b><span class="mut sm">' +
      'Every command Tirith starts has a time limit; past it, the command and everything it started are stopped. Agent runs and stages are stopped only when nothing happens for the idle limit (no output, file change or CPU)</span></div>' +
      '<table class="mtab"><thead><tr><th>Kind of command</th><th>Limit now (idle, then hard cap)</th><th>Where it comes from</th><th>Allowed range</th><th>Last time it ran out</th></tr></thead><tbody>' +
      rows.map(row).join('') + '</tbody></table>' +
      '<p class="mut sm mhint">To fix a limit yourself, write it under [timeouts] in config.toml, for example <code>git-commit = 300</code>. Idle limits: <code>agent-turn-idle = 1800</code>. Agents may raise or lower a limit within the allowed range, with a reason (<code>tirith timeout set --class agent-turn --idle N</code>).</p></div>';
  }

  var base = T.views.settings;
  T.views.settings = function (state, el) {
    base(state, el);
    var html = card(state), host = el.querySelector('.msettings');
    if (html && host) host.insertAdjacentHTML('beforeend', html);
  };
})();
