/* Setup card(s) in the Overview, under the Plan: pipelines, hooks, rules and `tirith check` of each project.
   Running a stage is sent to the manager through the inbox, kind "setup". */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  ui.setupPend = ui.setupPend || {};   /* "project|summary" -> ms of a send the collector has not shown yet */
  ui.setupCli = ui.setupCli || {};     /* project id -> command to copy (no way to send from here) */
  ui.setupErr = ui.setupErr || {};     /* project id -> why the send failed */
  var lastState = null, PEND_WAIT = 30000;

  var KIND = {
    build: '<svg viewBox="0 0 12 12" width="11" height="11" aria-hidden="true"><path d="M7.6 1.6a2.6 2.6 0 00-2.3 3.5L1.7 8.7a1 1 0 001.4 1.4l3.6-3.6a2.6 2.6 0 003.5-2.3L8.8 5 7 3.2z" fill="none" stroke="currentColor" stroke-width="1.1" stroke-linejoin="round"/></svg>',
    test: '<svg viewBox="0 0 12 12" width="11" height="11" aria-hidden="true"><path d="M4.6 1.5h2.8M5.2 1.5v3.2L2.4 9.6a.9.9 0 00.8 1.4h5.6a.9.9 0 00.8-1.4L6.8 4.7V1.5" fill="none" stroke="currentColor" stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round"/></svg>',
    deploy: '<svg viewBox="0 0 12 12" width="11" height="11" aria-hidden="true"><path d="M6 10.5V2.6M2.9 5.5L6 2.4l3.1 3.1" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/></svg>',
    none: '<svg viewBox="0 0 12 12" width="11" height="11" aria-hidden="true"><circle cx="6" cy="6" r="3.2" fill="none" stroke="currentColor" stroke-width="1.1"/></svg>'
  };
  var PLAY = '<svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true"><path d="M3.4 2.2v7.6L9.6 6z" fill="currentColor"/></svg>';
  var KIND_NAME = { build: 'Build', test: 'Test', deploy: 'Deploy' };

  function shq(t) { return "'" + String(t).replace(/'/g, "'\\''") + "'"; }

  /* "✓ 2 min ago", "✗ 5 min ago", "running", "never run" */
  function lastText(l) {
    if (!l) return '<span class="slast never">never run</span>';
    if (l.status === 'running') return '<span class="slast run">running</span>';
    var ok = l.status === 'passed';
    return '<span class="slast ' + (ok ? 'ok' : 'bad') + '" title="' + esc(l.run || '') + (l.duration_s != null ? ' · ' + esc(T.dur(l.duration_s)) : '') + '">' +
      (ok ? '✓' : '✗') + ' ' + esc(T.ago(l.at)) + (T.ago(l.at) === 'now' ? '' : ' ago') + '</span>';
  }

  function stageChip(p, pipe, s, multi) {
    var cmd = s.run ? ' · <code>' + esc(s.run) + '</code>' : '';
    var key = p.id + '|' + pipe.id + '|' + s.name;
    var busy = (ui.setupPend[key + '|run'] && T.now() - ui.setupPend[key + '|run'] < PEND_WAIT);
    return '<li class="schip ' + (s.last ? esc(s.last.status) : 'none') + '" title="' + esc((KIND_NAME[s.kind] || 'Stage') + ' · ' + (s.run || 'no command')) + '">' +
      '<span class="sk">' + (KIND[s.kind] || KIND.none) + '</span><span class="sname">' + esc(s.name) + '</span>' + cmd + lastText(s.last) +
      (s.run ? '<button class="srun' + (busy ? ' busy' : '') + '" data-act="srun" data-key="' + esc(key) + '"' + (busy ? ' disabled' : '') +
        ' title="Ask the manager to run this stage" aria-label="Run ' + esc(s.name) + '">' + PLAY + '<span>' + (busy ? 'Asked' : 'Run') + '</span></button>' : '') + '</li>';
  }

  function pipelinesRow(p, su, multi) {
    if (!su.pipelines.length) return '<div class="srow"><span class="slabel">Pipelines</span><span class="mut sm">None in tirith.toml yet</span></div>';
    return su.pipelines.map(function (pipe, i) {
      return '<div class="srow"><span class="slabel">' + (i ? '' : 'Pipelines') + '</span><div class="sbody"><div class="spname">' + esc(pipe.name) + '</div>' +
        '<ul class="schips">' + pipe.stages.map(function (s) { return stageChip(p, pipe, s, multi); }).join('') + '</ul></div></div>';
    }).join('');
  }

  function hooksRow(su) {
    if (!su.hooks.length) return '';
    return '<div class="srow"><span class="slabel">Git hooks</span><ul class="shooks sbody">' + su.hooks.map(function (h) {
      return '<li><span class="sev">' + esc(h.event) + '</span><span class="sarr">→</span><code>' + esc(h.run) + '</code>' + lastText(h.last) + '</li>';
    }).join('') + '</ul></div>';
  }

  function ruleRow(r) {
    return '<li class="srule ' + (r.active ? 'on' : 'off') + (r.source !== 'built-in' ? ' custom' : '') + '" title="' + esc(r.description) + '">' +
      '<span class="dot ' + (r.active ? 'green' : 'grey') + '"></span><span class="rname">' + esc(r.description) + '</span>' +
      (r.source !== 'built-in' ? '<span class="rtag">' + esc(r.action || 'block') + '</span>' : '') +
      '<span class="rstate">' + (r.active ? 'Active' : 'Inactive') + '</span></li>';
  }
  function rulesRow(su) {
    var custom = su.rules.filter(function (r) { return r.source !== 'built-in'; });
    var built = su.rules.filter(function (r) { return r.source === 'built-in'; });
    return '<div class="srow"><span class="slabel">Rules</span><div class="sbody"><ul class="srules">' + custom.concat(built).map(ruleRow).join('') + '</ul></div></div>';
  }

  /* What is missing, in one sentence, with Run buttons (owner actions); nothing when all is active. */
  function banner(p, su, state) {
    return '<div class="sban" role="alert"><span>' + esc(su.gates_summary || 'Agents could bypass the rules, because Tirith is not fully installed.') + '</span>' +
      T.actions(su.gates_actions, state, p.id) + '</div>';
  }

  function checkRow(su) {
    if (su.manifest === null && !su.check.problems.length) return '';
    if (su.check.ok) return '<div class="scheck ok"><span class="dot green"></span><b>Ready</b><span class="mut sm"> · tirith check passes</span></div>';
    return '<div class="scheck bad"><span class="dot red"></span><b>' + su.check.problems.length + ' to fix</b><span class="mut sm"> · tirith check</span>' +
      '<ul>' + su.check.problems.map(function (x) {
        return '<li><span class="pmsg">' + esc(x.message) + '</span><span class="pfix"><span class="mut">Fix</span> ' + esc(x.fix) + '</span></li>';
      }).join('') + '</ul></div>';
  }

  /* one line per agent: hooks present in its settings file (machine wide, so only in the first card) */
  function protectRow(su) {
    return (su.protection || []).map(function (x) {
      var at = x.checked_at ? ' · last check ' + esc(T.agoLong(x.checked_at)) : '';
      var healed = x.healed_at ? ' · restored ' + esc(T.agoLong(x.healed_at)) : '';
      return x.ok ? '<div class="scheck ok"><span class="dot green"></span><b>Hooks protected</b><span class="mut sm"> · ' + esc(x.agent) + at + healed + '</span></div>' :
        '<div class="scheck bad"><span class="dot red"></span><b>Hooks missing</b><span class="mut sm"> · ' + esc(x.agent) + at + ' · tirith hook install --agent ' + esc(x.agent) + '</span></div>';
    }).join('');
  }

  function pending(p, su) {
    var h = (su.pending || []).map(function (x) {
      return '<span class="ichip" title="The manager applies it from its inbox">Waiting for the manager · ' + esc(x.summary) + '</span>';
    });
    Object.keys(ui.setupPend).forEach(function (k) {
      var parts = k.split('|'), t = ui.setupPend[k];
      if (parts[0] !== p.id || parts[parts.length - 1] === 'run' || T.now() - t > PEND_WAIT) return;
      if ((su.pending || []).some(function (x) { return x.summary === parts.slice(1).join('|'); })) return;
      h.push('<span class="ichip">Sent · ' + esc(parts.slice(1).join('|')) + '</span>');
    });
    return h.length ? '<div class="spend">' + h.join('') + '</div>' : '';
  }

  function cliBlock(p) {
    var h = '';
    if (ui.setupErr[p.id]) h += '<div class="ierr">' + esc(ui.setupErr[p.id]) + '</div>';
    if (ui.setupCli[p.id]) {
      h += '<div class="icli">Tirith cannot send from this page. Run in a terminal:<code>' + esc(ui.setupCli[p.id]) + '</code>' +
        '<button class="vcopy" data-act="copy" data-key="' + esc(ui.setupCli[p.id]) + '">Copy</button></div>';
    }
    return h;
  }

  function card(p, state, first) {
    var su = p.setup;
    return '<div class="card scard"><div class="phead"><b>' + esc(p.name) + '</b><span class="mut sm">Setup' +
      (su.manifest ? '' : ' · no tirith.toml') + '</span></div>' +
      (!su.gates_active && first ? banner(p, su, state) : '') + checkRow(su) + (first ? protectRow(su) : '') +
      pipelinesRow(p, su) + hooksRow(su) + rulesRow(su) + pending(p, su) + cliBlock(p) + '</div>';
  }

  /* HTML for the Setup section, or '' when no visible project has setup data. */
  T.setup = function (state) {
    lastState = state;
    var ps = (state.projects || []).filter(function (p) { return p.setup; });
    if (!ps.length) return '';
    return '<section class="setup"><h2>Setup</h2>' + ps.map(function (p, i) { return card(p, state, i === 0); }).join('') + '</section>';
  };

  /* ---------- sending: the app writes the inbox, a browser opens the tirith:// link, else the CLI line is shown ---------- */
  function send(pid, req, summary, cli) {
    var L = lastState && lastState.launch, tauri = window.__TAURI__, text = JSON.stringify(req);
    ui.setupErr[pid] = null; ui.setupCli[pid] = null;
    function sent() { ui.setupPend[pid + '|' + summary] = T.now(); T.render(); setTimeout(T.render, PEND_WAIT + 500); }
    if (tauri && tauri.core && tauri.core.invoke) {
      tauri.core.invoke('inbox_add', { kind: 'setup', project: pid, step: 'setup', text: text }).then(sent, function (err) {
        ui.setupErr[pid] = 'Could not send: ' + err; T.render();
      });
      return;
    }
    if (L && L.token && L.available) {
      var a = document.createElement('a');
      a.href = 'tirith://inbox?kind=setup&project=' + encodeURIComponent(pid) + '&step=setup&text=' + encodeURIComponent(text) + '&token=' + encodeURIComponent(L.token);
      a.style.display = 'none'; document.body.appendChild(a); a.click(); document.body.removeChild(a);
      sent();
      return;
    }
    ui.setupCli[pid] = cli;
    T.render();
  }

  function sendRun(key) {
    var parts = key.split('|'), pid = parts[0], pipe = parts[1], name = parts[2];
    var cli = 'tirith run --project ' + shq(pid) + ' --pipeline ' + shq(pipe) + ' --stage ' + shq(name);
    ui.setupPend[key + '|run'] = T.now(); setTimeout(T.render, PEND_WAIT + 500);
    send(pid, { action: 'run-stage', pipeline: pipe, name: name }, 'run ' + pipe + '/' + name, cli);
  }

  var view = document.getElementById('view');
  view.addEventListener('click', function (e) {
    var b = e.target.closest('[data-act^="s"]'); if (!b) return;
    var act = b.getAttribute('data-act'), key = b.getAttribute('data-key');
    if (act === 'srun') sendRun(key);
  });
})();
