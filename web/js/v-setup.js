/* Setup card(s) in the Overview, under the Plan: pipelines, hooks, rules and `tirith check` of each project.
   Changes (a new stage or rule, running a stage) are sent to the manager through the inbox, kind "setup". */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  ui.setupForm = ui.setupForm || {};   /* project id -> 'stage' | 'rule' while its form is open */
  ui.setupDraft = ui.setupDraft || {}; /* "project|form" -> {field: value} typed so far */
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

  /* ---------- the "+ Stage" and "+ Rule" forms ---------- */
  var FIELDS = {
    stage: [
      { f: 'pipeline', label: 'Pipeline', ph: 'ci', req: 1 }, { f: 'name', label: 'Name', ph: 'lint', req: 1 },
      { f: 'run', label: 'Command', ph: 'make lint', req: 1, wide: 1 },
      { f: 'kind', label: 'Kind', sel: ['', 'build', 'test', 'deploy'] }, { f: 'after', label: 'After', ph: '(last)' }
    ],
    rule: [
      { f: 'id', label: 'Id', ph: 'no-force-push', req: 1 },
      { f: 'effect', label: 'Action', sel: ['block', 'warn'] },
      { f: 'match', label: 'Command glob', ph: 'git push --force*', wide: 1 },
      { f: 'paths', label: 'or edited paths', ph: 'harness/**, *.lock' },
      { f: 'regex', label: 'or regex', ph: 'curl .*\\| *sh' },
      { f: 'message', label: 'Message', ph: 'force pushes rewrite history', req: 1, wide: 1 },
      { f: 'instead', label: 'Instead', ph: 'git push (without --force); ask the owner', req: 1, wide: 1 },
      { f: 'description', label: 'In plain words', ph: 'Nobody may rewrite the shared history.', req: 1, wide: 1 }
    ]
  };
  function form(p, su) {
    var which = ui.setupForm[p.id]; if (!which) return '';
    var d = ui.setupDraft[p.id + '|' + which] || {}, pipes = su.pipelines.map(function (x) { return x.id; });
    var fields = FIELDS[which].map(function (x) {
      var key = p.id + '|' + which + '|' + x.f, v = d[x.f] != null ? d[x.f] : (x.f === 'pipeline' && pipes.length === 1 ? pipes[0] : '');
      var input = x.sel ? '<select data-act="sfield" data-key="' + esc(key) + '" aria-label="' + esc(x.label) + '">' + x.sel.map(function (o) {
        return '<option value="' + esc(o) + '"' + (v === o ? ' selected' : '') + '>' + esc(o || '(auto)') + '</option>';
      }).join('') + '</select>' :
        '<input type="text" data-act="sfield" data-key="' + esc(key) + '" placeholder="' + esc(x.ph || '') + '" value="' + esc(v) + '" aria-label="' + esc(x.label) + '"' + (x.f === 'pipeline' ? ' list="spipes-' + esc(p.id) + '"' : '') + '>';
      return '<label class="sf' + (x.wide ? ' wide' : '') + '"><span>' + esc(x.label) + (x.req ? ' *' : '') + '</span>' + input + '</label>';
    }).join('');
    var list = which === 'stage' ? '<datalist id="spipes-' + esc(p.id) + '">' + pipes.map(function (x) { return '<option value="' + esc(x) + '">'; }).join('') + '</datalist>' : '';
    return '<div class="sform"><div class="sfhead"><b>' + (which === 'stage' ? 'New stage' : 'New rule') + '</b>' +
      '<span class="mut sm">' + (which === 'stage' ? 'Added to tirith.toml by the manager' : 'Set exactly one of command glob, edited paths, regex') + '</span></div>' +
      '<div class="sfields">' + fields + list + '</div><div class="sfoot"><button class="isend" data-act="ssend" data-key="' + esc(p.id) + '">Ask the manager</button>' +
      '<button class="scancel" data-act="sclose" data-key="' + esc(p.id) + '">Cancel</button></div></div>';
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
      (su.manifest ? '' : ' · no tirith.toml') + '</span><span class="sbtns">' +
      '<button class="sadd" data-act="sopen" data-key="' + esc(p.id + '|stage') + '">+ Stage</button>' +
      '<button class="sadd" data-act="sopen" data-key="' + esc(p.id + '|rule') + '">+ Rule</button></span></div>' +
      (!su.gates_active && first ? banner(p, su, state) : '') + checkRow(su) +
      pipelinesRow(p, su) + hooksRow(su) + rulesRow(su) + pending(p, su) + form(p, su) + cliBlock(p) + '</div>';
  }

  /* HTML for the Setup section, or '' when no visible project has setup data. */
  T.setup = function (state) {
    lastState = state;
    var f = T.filter.project, ps = (state.projects || []).filter(function (p) { return p.setup && (!f || p.id === f); });
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

  function sendForm(pid) {
    var which = ui.setupForm[pid], d = ui.setupDraft[pid + '|' + which] || {}, su = (lastState.projects.filter(function (x) { return x.id === pid; })[0] || {}).setup || {};
    var pipes = (su.pipelines || []).map(function (x) { return x.id; });
    if (which === 'stage') {
      d = { pipeline: d.pipeline != null ? d.pipeline : (pipes.length === 1 ? pipes[0] : ''), name: d.name, run: d.run, kind: d.kind, after: d.after };
      if (!(d.pipeline || '').trim() || !(d.name || '').trim() || !(d.run || '').trim()) { ui.setupErr[pid] = 'A stage needs a pipeline, a name and a command.'; T.render(); return; }
      var req = { action: 'add-stage', pipeline: d.pipeline.trim(), name: d.name.trim(), run: d.run.trim() };
      if (d.kind) req.kind = d.kind;
      if ((d.after || '').trim()) req.after = d.after.trim();
      var cli = 'tirith setup add-stage --project ' + shq(pid) + ' --pipeline ' + shq(req.pipeline) + ' --name ' + shq(req.name) + ' --run ' + shq(req.run) +
        (req.kind ? ' --kind ' + req.kind : '') + (req.after ? ' --after ' + shq(req.after) : '');
      done(pid, which); send(pid, req, 'add-stage ' + req.pipeline + '/' + req.name + ': ' + req.run, cli);
      return;
    }
    var n = ['match', 'paths', 'regex'].filter(function (k) { return (d[k] || '').trim(); });
    if (!(d.id || '').trim() || !(d.description || '').trim() || !(d.message || '').trim() || !(d.instead || '').trim() || n.length !== 1) {
      ui.setupErr[pid] = 'A rule needs an id, a description, a message, an instead, and exactly one of command glob, edited paths or regex.'; T.render(); return;
    }
    var rule = { action: 'add-rule', id: d.id.trim(), effect: d.effect || 'block', message: d.message.trim(), instead: d.instead.trim(), description: d.description.trim() };
    rule[n[0]] = d[n[0]].trim();
    var flag = n[0] === 'match' ? '--match' : n[0] === 'regex' ? '--regex' : '--paths';
    var rcli = 'tirith setup add-rule --project ' + shq(pid) + ' --id ' + shq(rule.id) + ' ' + flag + ' ' + shq(rule[n[0]]) + ' --action ' + rule.effect +
      ' --message ' + shq(rule.message) + ' --instead ' + shq(rule.instead) + ' --description ' + shq(rule.description);
    done(pid, which); send(pid, rule, 'add-rule ' + rule.id + ': ' + (rule.match || rule.regex || rule.paths), rcli);
  }
  function done(pid, which) { ui.setupForm[pid] = null; ui.setupDraft[pid + '|' + which] = {}; ui.setupErr[pid] = null; }

  var view = document.getElementById('view');
  view.addEventListener('click', function (e) {
    var b = e.target.closest('[data-act^="s"]'); if (!b) return;
    var act = b.getAttribute('data-act'), key = b.getAttribute('data-key');
    if (act === 'srun') { sendRun(key); return; }
    if (act === 'sopen') {
      var parts = key.split('|'); ui.setupErr[parts[0]] = null; ui.setupCli[parts[0]] = null;
      ui.setupForm[parts[0]] = ui.setupForm[parts[0]] === parts[1] ? null : parts[1]; T.render();
      var first = document.querySelector('.sform input'); if (first) first.focus();
      return;
    }
    if (act === 'sclose') { ui.setupForm[key] = null; ui.setupErr[key] = null; T.render(); return; }
    if (act === 'ssend') sendForm(key);
  });
  function field(t) {
    if (!t.getAttribute || t.getAttribute('data-act') !== 'sfield') return null;
    var p = t.getAttribute('data-key').split('|'); return { pid: p[0], form: p[1], f: p[2] };
  }
  view.addEventListener('input', function (e) {
    var x = field(e.target); if (!x) return;
    var k = x.pid + '|' + x.form; (ui.setupDraft[k] = ui.setupDraft[k] || {})[x.f] = e.target.value;
  });
  view.addEventListener('change', function (e) {
    var x = field(e.target); if (!x) return;
    var k = x.pid + '|' + x.form; (ui.setupDraft[k] = ui.setupDraft[k] || {})[x.f] = e.target.value;
  });
  view.addEventListener('keydown', function (e) {
    var x = field(e.target); if (!x) return;
    if (e.key === 'Enter' && !e.isComposing) { e.preventDefault(); sendForm(x.pid); }
    else if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); ui.setupForm[x.pid] = null; T.render(); }
  });
})();
