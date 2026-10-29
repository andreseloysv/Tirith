/* Settings view, "Environment" card: the variables every lane, stage run and Run action of the selected project
   gets (docs/CONTRACT-projenv.md). Values are shown in full on purpose. Saving replaces the whole set:
   the app calls `project_env_set`; a browser opens tirith://env-set; else the CLI line is shown. */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui, KEY_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;
  ui.envDraft = ui.envDraft || {};  /* project -> [{ k, v }] while the table has unsaved changes */
  ui.envSent = ui.envSent || {};    /* project -> ms of a save the state has not shown yet */
  ui.envErr = ui.envErr || null;
  ui.envCli = ui.envCli || null;
  var lastState = null, SENT_WAIT = 30000;

  function shq(t) { return "'" + String(t).replace(/'/g, "'\\''") + "'"; }
  function rowsOf(env) { return Object.keys(env || {}).map(function (k) { return { k: k, v: env[k] }; }); }
  function projectRow(s, pid) { return (s.projects || []).filter(function (p) { return p.id === pid; })[0]; }
  function sameAs(rows, env) {
    var o = toObject(rows), a = Object.keys(o), b = Object.keys(env || {});
    return a.length === b.length && a.every(function (k) { return env[k] === o[k]; });
  }
  function toObject(rows) {
    var o = {}; rows.forEach(function (r) { if (r.k.trim() !== '' || r.v !== '') o[r.k.trim()] = r.v; }); return o;
  }
  /* what the table shows: the draft while it differs from the state, else the state itself */
  function current(pid, p) {
    var d = ui.envDraft[pid];
    if (d && ui.envSent[pid] && sameAs(d, p.env)) { delete ui.envDraft[pid]; delete ui.envSent[pid]; d = null; }
    return d || rowsOf(p.env);
  }
  function problem(rows) {
    var seen = {}, i, k;
    for (i = 0; i < rows.length; i++) {
      k = rows[i].k.trim();
      if (k === '' && rows[i].v === '') continue;
      if (!KEY_RE.test(k)) return (k === '' ? 'A row has a value but no name' : '"' + k + '" is not a valid name') + ': use letters, digits and _, not starting with a digit.';
      if (seen[k]) return k + ' appears twice.';
      seen[k] = 1;
    }
    return '';
  }

  function table(pid, rows) {
    var body = rows.map(function (r, i) {
      return '<tr><td><input type="text" class="envk" data-act="env-f" data-key="k|' + i + '" value="' + esc(r.k) + '" placeholder="NAME" spellcheck="false" autocomplete="off" aria-label="Name"></td>' +
        '<td><input type="text" class="envv" data-act="env-f" data-key="v|' + i + '" value="' + esc(r.v) + '" placeholder="value" spellcheck="false" autocomplete="off" aria-label="Value of ' + esc(r.k || 'row ' + (i + 1)) + '"></td>' +
        '<td class="mx"><button class="alink" data-act="env-rm" data-key="' + i + '" aria-label="Delete ' + esc(r.k || 'row') + '">Delete</button></td></tr>';
    }).join('');
    return '<table class="mtab envtab"><thead><tr><th>Name</th><th>Value</th><th></th></tr></thead><tbody>' +
      (body || '<tr><td colspan="3" class="mut sm">No variables: every run of ' + esc(T.projectName(lastState, pid)) + ' gets only the inherited environment.</td></tr>') + '</tbody></table>';
  }

  function card(state) {
    var pid = T.effProject(state), p = pid && projectRow(state, pid);
    if (!p) return '';
    var rows = current(pid, p), dirty = !!ui.envDraft[pid] && !sameAs(rows, p.env);
    var waiting = ui.envSent[pid] && T.now() - ui.envSent[pid] < SENT_WAIT && ui.envDraft[pid];
    var h = '<div class="card scard envcard" data-project="' + esc(pid) + '"><div class="phead"><b>Environment</b><span class="mut sm">' + esc(T.projectName(state, pid)) +
      ' · given to every lane, stage run and Run action, on top of the inherited environment</span></div>' + table(pid, rows) +
      '<div class="sfoot"><button class="sadd" data-act="env-add">Add variable</button>' +
      '<button class="isend" data-act="env-save"' + (dirty ? '' : ' disabled') + '>Save</button>' +
      (waiting ? '<span class="ichip">Sent · waiting for the state</span>' : dirty ? '<span class="mut sm">Unsaved changes</span>' : '') + '</div>';
    if (ui.envErr) h += '<div class="ierr">' + esc(ui.envErr) + '</div>';
    if (ui.envCli) h += '<div class="icli">Tirith cannot send from this page. Run in a terminal:<code>' + esc(ui.envCli) + '</code>' +
      '<button class="vcopy" data-act="copy" data-key="' + esc(ui.envCli) + '">Copy</button></div>';
    return h + '</div>';
  }

  var base = T.views.settings;
  T.views.settings = function (state, el) {
    lastState = state;
    base(state, el);
    var html = card(state), lead = el.querySelector('.msettings .mlead');
    if (html && lead) lead.insertAdjacentHTML('afterend', html);
  };

  function edit(pid, fn) {
    var p = projectRow(lastState, pid), rows = current(pid, p).map(function (r) { return { k: r.k, v: r.v }; });
    fn(rows); ui.envDraft[pid] = rows; delete ui.envSent[pid]; ui.envErr = null; ui.envCli = null;
  }
  function send(pid, env) {
    var tauri = window.__TAURI__, L = lastState.launch, json = JSON.stringify(env);
    ui.envErr = null; ui.envCli = null;
    function sent() { ui.envSent[pid] = T.now(); T.render(); setTimeout(T.render, SENT_WAIT + 500); }
    if (tauri && tauri.core && tauri.core.invoke) {
      tauri.core.invoke('project_env_set', { project: pid, env: env }).then(sent, function (err) { ui.envErr = 'Could not save: ' + err; T.render(); });
      return;
    }
    if (L && L.token && L.available) {
      var a = document.createElement('a');
      a.href = 'tirith://env-set?project=' + encodeURIComponent(pid) + '&env=' + encodeURIComponent(json) + '&token=' + encodeURIComponent(L.token);
      a.style.display = 'none'; document.body.appendChild(a); a.click(); document.body.removeChild(a);
      sent();
      return;
    }
    ui.envCli = 'printf %s ' + shq(json) + ' | tirith project env set --project ' + shq(pid) + ' --replace-stdin';
    T.render();
  }
  function save(pid) {
    var rows = ui.envDraft[pid]; if (!rows) return;
    var bad = problem(rows);
    if (bad) { ui.envErr = bad; T.render(); return; }
    send(pid, toObject(rows));
  }

  var view = document.getElementById('view');
  view.addEventListener('click', function (e) {
    var b = e.target.closest('[data-act^="env-"]'), c = b && b.closest('.envcard'); if (!b || b.disabled || !c) return;
    var act = b.getAttribute('data-act'), pid = c.getAttribute('data-project');
    if (act === 'env-save') save(pid);
    else if (act === 'env-rm') { edit(pid, function (rows) { rows.splice(+b.getAttribute('data-key'), 1); }); T.render(); }
    else if (act === 'env-add') {
      var n; edit(pid, function (rows) { rows.push({ k: '', v: '' }); n = rows.length - 1; }); T.render();
      var box = view.querySelector('input[data-key="k|' + n + '"]'); if (box) box.focus();
    }
  });
  view.addEventListener('input', function (e) {
    var t = e.target, c = t.closest && t.closest('.envcard');
    if (!c || t.getAttribute('data-act') !== 'env-f') return;
    var pid = c.getAttribute('data-project'), key = t.getAttribute('data-key').split('|');
    edit(pid, function (rows) { rows[+key[1]][key[0]] = t.value; });
    var save = c.querySelector('[data-act="env-save"]'); if (save) save.disabled = false;
    var er = c.querySelector('.ierr'); if (er) er.remove();
  });
  view.addEventListener('keydown', function (e) {
    var t = e.target;
    if (e.key !== 'Enter' || e.isComposing || !t.getAttribute || t.getAttribute('data-act') !== 'env-f') return;
    var c = t.closest('.envcard'); e.preventDefault(); save(c.getAttribute('data-project'));
  });
})();
