/* Settings view: the model and effort of the manager and of the agents it starts (docs/CONTRACT-models.md).
   Saving sends a "setup" inbox request for the project "settings"; `tirith models set` applies it. */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  ui.modDraft = ui.modDraft || {};  /* "scope|field" -> value typed so far; scope: manager, workers, new */
  ui.modPend = ui.modPend || {};    /* summary -> ms of a send the collector has not shown yet */
  ui.modCli = ui.modCli || null;    /* command to copy (no way to send from here) */
  ui.modErr = ui.modErr || null;
  ui.modOther = ui.modOther || {};  /* "scope|field" -> true while the "Other…" text field replaces the list */
  var OTHER = '__other__';
  var lastState = null, PEND_WAIT = 30000;
  var BOLT = '<svg viewBox="0 0 12 12" width="11" height="11" aria-hidden="true"><path d="M7 1.2L2.6 6.8h2.7L4.8 10.8 9.4 5H6.6z" fill="currentColor"/></svg>';

  function shq(t) { return "'" + String(t).replace(/'/g, "'\\''") + "'"; }
  function draft(scope, f, base) {
    var v = ui.modDraft[scope + '|' + f];
    return v !== undefined ? v : (base == null ? '' : base);
  }
  function opts(list, cur, blank, other) {
    var all = list.slice();
    if (cur && all.indexOf(cur) < 0) all.unshift(cur);
    return (blank != null ? '<option value="">' + esc(blank) + '</option>' : '') + all.map(function (o) {
      return '<option value="' + esc(o) + '"' + (o === cur ? ' selected' : '') + '>' + esc(o) + '</option>';
    }).join('') + (other ? '<option value="' + OTHER + '">Other…</option>' : '');
  }
  /* the models offered for the agent chosen in the same scope (every model when it declares no prefix) */
  function modelsFor(scope, M) {
    var agent = scope === 'new' ? draft('workers', 'agent', M.workers.agent) : draft(scope, 'agent', (M[scope] || {}).agent);
    return (M.choices_by_agent && M.choices_by_agent[agent]) || M.choices;
  }
  function input(scope, f, label, cur, M, blank) {
    var key = 'data-act="mod-f" data-key="' + esc(scope + '|' + f) + '" aria-label="' + esc(label) + '"';
    if (f === 'agent') return '<select ' + key + '>' + opts(M.agents, cur, blank) + '</select>';
    var list = f === 'effort' ? M.efforts : modelsFor(scope, M);
    if (ui.modOther[scope + '|' + f] || !list.length) {
      return '<input type="text" ' + key + ' value="' + esc(cur) + '" placeholder="' + esc(blank || label) + '" spellcheck="false">' +
        (list.length ? '<button class="alink" data-act="mod-list" data-key="' + esc(scope + '|' + f) + '">List</button>' : '');
    }
    return '<select ' + key + '>' + opts(list, cur, blank, true) + '</select>';
  }
  function labeled(scope, f, label, base, M, blank) {
    return '<label class="sf"><span>' + esc(label) + '</span>' + input(scope, f, label, draft(scope, f, base), M, blank) + '</label>';
  }

  function pending(M) {
    var h = (M.pending || []).map(function (x) {
      return '<span class="ichip" title="The manager applies it from its inbox">Waiting for the manager · ' + esc(x) + '</span>';
    });
    Object.keys(ui.modPend).forEach(function (k) {
      if (T.now() - ui.modPend[k] > PEND_WAIT || (M.pending || []).indexOf(k) >= 0) return;
      h.push('<span class="ichip">Sent · ' + esc(k) + '</span>');
    });
    return h.length ? '<div class="spend">' + h.join('') + '</div>' : '';
  }

  function managerCard(M) {
    var m = M.manager, held = m.settings, note;
    if (!m.model && held && held.model) m = Object.assign({}, m, { model: held.model });  /* default: what the agent already uses */
    if (!m.agent || !m.model) note = '<span class="mut sm">Choose the agent and the model, save, then apply.</span>';
    else if (!held) note = '<span class="mut sm">The settings file of ' + esc(m.agent) + ' is not readable yet; applying creates it.</span>';
    else if (m.in_sync) note = '<span class="dot green"></span><span class="sm">The settings file already holds <b>' + esc(m.model) + '</b></span>';
    else note = '<span class="dot amber"></span><span class="sm">The settings file holds <b>' + esc(held.model || 'no model') + '</b>; apply to change it to <b>' + esc(m.model) + '</b></span>';
    return '<div class="card scard"><div class="phead"><b>Manager</b><span class="mut sm">The orchestrator session</span></div>' +
      '<div class="sfields mfields">' + labeled('manager', 'agent', 'Agent', m.agent, M, '(choose)') +
      labeled('manager', 'model', 'Model', m.model, M, '(choose)') + labeled('manager', 'effort', 'Effort', m.effort, M, '(none)') + '</div>' +
      '<div class="sfoot"><button class="isend" data-act="mod-save" data-key="manager">Save</button>' +
      '<button class="mapply" data-act="mod-apply"' + (m.model && m.agent ? '' : ' disabled') + ' title="Write the model into the manager agent\'s own settings file">' + BOLT + '<span>Apply to the manager</span></button>' +
      '<span class="mnote">' + note + '</span></div>' +
      (m.model && m.effort && !m.effort_key ? '<p class="mut sm mhint">' + esc(m.agent) + ' has no effort key in its settings file: the effort is kept in config.toml only.</p>' : '') + '</div>';
  }

  function scopeRows(M) {
    var rows = [];
    Object.keys(M.projects).sort().forEach(function (p) { rows.push({ label: T.proj(p) + ' · all steps', v: M.projects[p], key: p }); });
    Object.keys(M.steps).sort().forEach(function (k) {
      var i = k.indexOf('/'); rows.push({ label: T.proj(k.slice(0, i)) + ' · step ' + k.slice(i + 1), v: M.steps[k], key: k });
    });
    return rows.map(function (r) {
      return '<tr><td>' + esc(r.label) + '</td><td>' + esc(r.v.model || '—') + '</td><td>' + esc(r.v.effort || '—') + '</td>' +
        '<td class="mx"><button class="alink" data-act="mod-clear" data-key="' + esc(r.key) + '" aria-label="Remove override">Remove</button></td></tr>';
    }).join('');
  }

  function wfallback(M) {
    var cur = draft('workers', 'fallback', M.workers.fallback || []), main = draft('workers', 'agent', M.workers.agent);
    var left = M.agents.filter(function (a) { return cur.indexOf(a) < 0 && a !== main; });
    var chips = cur.map(function (a, i) {
      return '<span class="fbchip"><b>' + (i + 1) + '</b> ' + esc(a) +
        (i ? '<button class="alink" data-act="mod-fb-up" data-key="' + esc(a) + '" aria-label="Move ' + esc(a) + ' earlier">↑</button>' : '') +
        '<button class="alink" data-act="mod-fb-rm" data-key="' + esc(a) + '" aria-label="Remove ' + esc(a) + '">✕</button></span>';
    }).join('');
    return '<div class="sfallback"><span class="slabel">Fallback</span><span class="mut sm">Used in this order when an agent is out of credits</span>' +
      '<div class="fbrow">' + (chips || '<span class="mut sm">None</span>') +
      (left.length ? '<select data-act="mod-fb-add" data-keep="mod-fb-add" aria-label="Add a fallback agent">' + opts(left, '', 'Add…') + '</select>' : '') + '</div></div>';
  }
  function statusChips(M) {
    var st = M.agent_status || {};
    return '<div class="astat">' + M.agents.map(function (a) {
      var s = st[a] || { state: 'ok' }, out = s.state === 'out';
      return '<span class="achip ' + (out ? 'out' : 'ok') + '"><span class="dot ' + (out ? 'red' : 'green') + '"></span>' + esc(a) + ' · ' +
        (out ? 'out of credits' + (s.since ? ' since ' + esc(T.clockTime(s.since)) : '') : 'ok') + '</span>';
    }).join('') + '</div>';
  }

  function workersCard(M, state) {
    var w = M.workers, projects = (state.projects || []).map(function (p) { return p.id; });
    var rows = scopeRows(M);
    var add = '<tr class="madd"><td><select data-act="mod-f" data-key="new|project" aria-label="Project">' + opts(projects.map(function (p) { return p; }), draft('new', 'project', ''), 'Project…') + '</select>' +
      '<input type="text" data-act="mod-f" data-key="new|step" value="' + esc(draft('new', 'step', '')) + '" placeholder="step (optional)" aria-label="Step" class="mstep"></td>' +
      '<td>' + input('new', 'model', 'Model', draft('new', 'model', ''), M, '(inherit)') + '</td><td>' + input('new', 'effort', 'Effort', draft('new', 'effort', ''), M, '(inherit)') + '</td>' +
      '<td class="mx"><button class="sadd" data-act="mod-add" data-key="new">Add override</button></td></tr>';
    return '<div class="card scard"><div class="phead"><b>Workers</b><span class="mut sm">Agents the manager, dispatcher or Plan button starts</span></div>' +
      '<div class="sfields mfields">' + labeled('workers', 'agent', 'Agent', w.agent, M, '(default)') +
      labeled('workers', 'model', 'Model', w.model, M, '(from the agent)') + labeled('workers', 'effort', 'Effort', w.effort, M, '(from the agent)') + '</div>' +
      wfallback(M) + statusChips(M) +
      '<div class="sfoot"><button class="isend" data-act="mod-save" data-key="workers">Save defaults</button></div>' +
      '<div class="mover"><div class="slabel">Overrides</div><table class="mtab"><thead><tr><th>Where</th><th>Model</th><th>Effort</th><th></th></tr></thead><tbody>' +
      (rows || '<tr><td colspan="4" class="mut sm">None: every project and step uses the defaults above.</td></tr>') + add + '</tbody></table></div></div>';
  }

  function cliBlock() {
    var h = '';
    if (ui.modErr) h += '<div class="ierr">' + esc(ui.modErr) + '</div>';
    if (ui.modCli) h += '<div class="icli">Tirith cannot send from this page. Run in a terminal:<code>' + esc(ui.modCli) + '</code>' +
      '<button class="vcopy" data-act="copy" data-key="' + esc(ui.modCli) + '">Copy</button></div>';
    return h;
  }

  T.views.settings = function (state, el) {
    lastState = state;
    var M = state.models;
    if (!M) { el.innerHTML = '<section>' + T.empty('No settings in this state yet: run the collector again') + '</section>'; return; }
    el.innerHTML = '<section class="msettings">' +
      '<p class="mut sm mlead">Model and effort come from <code>~/.tirith/config.toml</code> under <code>[models]</code>; the manager keeps its own in the agent\'s settings file.</p>' +
      managerCard(M) + workersCard(M, state) + agentsCard() + pending(M) + cliBlock() + '</section>';
  };

  /* Agent CLIs per platform (docs/PLATFORMS.md; read 2026-10-29). "?" = not checked. */
  var PLATFORMS = [
    ['anthropic', 'yes', 'yes', 'yes', 'yes'], ['codex', 'yes', 'yes', 'yes', 'no'],
    ['copilot', 'yes', 'yes', 'yes', 'no'], ['muse', 'yes', '?', '?', 'no']
  ];
  function agentsCard() {
    return '<div class="mover"><div class="slabel">Agents</div><table class="mtab"><thead><tr><th>Agent</th><th>macOS</th><th>Windows</th><th>Linux</th><th>Chat</th></tr></thead><tbody>' +
      PLATFORMS.map(function (r) {
        return '<tr>' + r.map(function (c, i) { return i ? '<td>' + c + '</td>' : '<td><code>' + esc(c) + '</code></td>'; }).join('') + '</tr>';
      }).join('') + '</tbody></table><p class="mut sm">Hooks and shims work for all four. Details: docs/PLATFORMS.md.</p></div>';
  }

  /* ---------- sending: the app writes the inbox, a browser opens the tirith:// link, else the CLI line is shown ---------- */
  function send(req, summary, cli) {
    var L = lastState && lastState.launch, tauri = window.__TAURI__, text = JSON.stringify(req);
    ui.modErr = null; ui.modCli = null;
    function sent() { ui.modPend[summary] = T.now(); T.render(); setTimeout(T.render, PEND_WAIT + 500); }
    if (tauri && tauri.core && tauri.core.invoke) {
      tauri.core.invoke('inbox_add', { kind: 'setup', project: 'settings', step: 'setup', text: text }).then(sent, function (err) {
        ui.modErr = 'Could not send: ' + err; T.render();
      });
      return;
    }
    if (L && L.token && L.available) {
      var a = document.createElement('a');
      a.href = 'tirith://inbox?kind=setup&project=settings&step=setup&text=' + encodeURIComponent(text) + '&token=' + encodeURIComponent(L.token);
      a.style.display = 'none'; document.body.appendChild(a); a.click(); document.body.removeChild(a);
      sent();
      return;
    }
    ui.modCli = cli;
    T.render();
  }
  function flags(o) {
    return ['agent', 'model', 'effort'].filter(function (k) { return o[k]; }).map(function (k) { return ' --' + k + ' ' + shq(o[k]); }).join('') +
      (o.fallback ? ' --fallback ' + shq(o.fallback.join(',')) : '');
  }
  function summary(req) {
    return 'models ' + req.role + (req.project ? ' ' + req.project + (req.step ? '/' + req.step : '') : '') +
      ['agent', 'model', 'effort'].filter(function (k) { return req[k]; }).map(function (k) { return ' ' + k + '=' + req[k]; }).join('') +
      (req.fallback ? ' fallback=' + (req.fallback.join(',') || 'none') : '');
  }
  function set(req) {
    var cli = 'tirith models set ' + req.role + flags(req) + (req.project ? ' --project ' + shq(req.project) : '') + (req.step ? ' --step ' + shq(req.step) : '') + (req.clear ? ' --clear' : '');
    send(Object.assign({ action: 'models-set' }, req), summary(req) + (req.clear ? ' (remove)' : ''), cli);
  }
  function save(scope) {
    var M = lastState.models, base = M[scope], req = { role: scope }, held = scope === 'manager' && base.settings;
    ['agent', 'model', 'effort'].forEach(function (f) {
      var v = String(draft(scope, f, f === 'model' && held && !base.model ? held.model : base[f])).trim(); if (v) req[f] = v;
    });
    if (scope === 'workers' && ui.modDraft['workers|fallback']) req.fallback = ui.modDraft['workers|fallback'].slice();
    if (scope === 'manager' && (!req.agent || !req.model)) { ui.modErr = 'The manager needs an agent and a model.'; T.render(); return; }
    if (!req.model && !req.effort && !req.agent && !req.fallback) { ui.modErr = 'Nothing to save: choose a model, an effort or an agent.'; T.render(); return; }
    ['agent', 'model', 'effort', 'fallback'].forEach(function (f) { delete ui.modDraft[scope + '|' + f]; delete ui.modOther[scope + '|' + f]; });
    set(req);
  }
  function add() {
    var req = { role: 'workers' }, p = String(draft('new', 'project', '')).trim(), st = String(draft('new', 'step', '')).trim();
    if (!p) { ui.modErr = 'Pick the project this override is for.'; T.render(); return; }
    req.project = p; if (st) req.step = st;
    ['model', 'effort'].forEach(function (f) { var v = String(draft('new', f, '')).trim(); if (v) req[f] = v; });
    if (!req.model && !req.effort) { ui.modErr = 'An override needs a model or an effort.'; T.render(); return; }
    ['project', 'step', 'model', 'effort'].forEach(function (f) { delete ui.modDraft['new|' + f]; delete ui.modOther['new|' + f]; });
    set(req);
  }
  function clear(key) {
    var i = key.indexOf('/'), req = { role: 'workers', clear: true };
    if (lastState.models.steps[key]) { req.project = key.slice(0, i); req.step = key.slice(i + 1); } else req.project = key;
    set(req);
  }

  var view = document.getElementById('view');
  view.addEventListener('click', function (e) {
    var b = e.target.closest('[data-act^="mod-"]'); if (!b || b.disabled) return;
    var act = b.getAttribute('data-act'), key = b.getAttribute('data-key');
    if (act === 'mod-save') save(key);
    else if (act === 'mod-add') add();
    else if (act === 'mod-clear') clear(key);
    else if (act === 'mod-list') { delete ui.modOther[key]; delete ui.modDraft[key]; T.render(); }
    else if (act === 'mod-fb-rm' || act === 'mod-fb-up') {
      var l = fbList(), i = l.indexOf(key);
      if (i >= 0 && act === 'mod-fb-rm') l.splice(i, 1);
      else if (i > 0) { l[i] = l[i - 1]; l[i - 1] = key; }
      ui.modDraft['workers|fallback'] = l; T.render();
    }
    else if (act === 'mod-apply') send({ action: 'apply-manager' }, 'apply-manager', 'tirith models apply-manager');
  });
  function fbList() {
    return draft('workers', 'fallback', lastState.models.workers.fallback || []).slice();
  }
  function typed(e) {
    var t = e.target, act = t.getAttribute && t.getAttribute('data-act'), key;
    if (act === 'mod-fb-add') {
      if (e.type === 'change' && t.value) { ui.modErr = null; ui.modDraft['workers|fallback'] = fbList().concat(t.value); T.render(); }
      return;
    }
    if (act !== 'mod-f') return;
    key = t.getAttribute('data-key');
    if (ui.modErr) { ui.modErr = null; var er = view.querySelector('.ierr'); if (er) er.remove(); }  /* a new choice makes the old error stale */
    if (t.tagName === 'SELECT' && t.value === OTHER) {
      ui.modOther[key] = true; ui.modDraft[key] = ''; T.render();
      var box = view.querySelector('input[data-key="' + key + '"]'); if (box) box.focus();
      return;
    }
    ui.modDraft[key] = t.value;
    if (t.tagName === 'SELECT' && key.slice(-6) === '|agent') T.render();  /* the model list depends on the agent */
  }
  view.addEventListener('input', typed);
  view.addEventListener('change', typed);
  view.addEventListener('keydown', function (e) {
    var t = e.target;
    if (e.key !== 'Enter' || e.isComposing || !t.getAttribute || t.getAttribute('data-act') !== 'mod-f') return;
    var scope = t.getAttribute('data-key').split('|')[0];
    e.preventDefault();
    if (scope === 'new') add(); else save(scope);
  });
})();
