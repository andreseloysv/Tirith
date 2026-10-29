/* Commands for the owner: each action is a row with a "Run" button (docs/CONTRACT-actions.md).
   The page never sends command text, only the action id: the app (run_action) or the tirith://run link. */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  ui.actConfirm = ui.actConfirm || {};  /* action id -> true while its confirm popover is open */
  ui.actPend = ui.actPend || {};        /* action id -> ms of a click the collector has not shown yet */
  ui.actOut = ui.actOut || {};          /* action id -> true while its output is shown */
  ui.actErr = ui.actErr || {};          /* action id -> why starting failed */
  var lastState = null, PEND_WAIT = 15000;

  var PLAY = '<svg viewBox="0 0 12 12" width="11" height="11" aria-hidden="true"><path d="M3.2 2.1l6.2 3.9-6.2 3.9z" fill="currentColor"/></svg>';
  var OKI = '<svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"><path d="M2.6 6.3l2.3 2.3 4.5-4.9" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>';
  var BAD = '<svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"><path d="M3 3l6 6M9 3l-6 6" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>';

  function canRun() {
    var L = lastState && lastState.launch, tauri = window.__TAURI__;
    return !!((tauri && tauri.core && tauri.core.invoke) || (L && L.token && L.available));
  }
  function running(a) {
    var r = a.last_run, p = ui.actPend[a.id];
    if (p && r && (T.ms(r.at) || 0) >= p - 2000) { delete ui.actPend[a.id]; p = null; }
    if (p && T.now() - p > PEND_WAIT) { delete ui.actPend[a.id]; p = null; }
    return !!p || !!(r && r.status === 'running');
  }
  function where(a, pid) {
    return a.cwd ? esc(a.cwd) : 'the folder of ' + esc(pid ? T.proj(pid) : 'the project');
  }
  function log(lines) {
    return lines && lines.length ? '<pre class="alog">' + esc(lines.join('\n')) + '</pre>' : '';
  }

  function row(a, pid, fold) {
    var r = a.last_run, busy = running(a), can = canRun(), h;
    /* a passed action of a plan answer is done: one line, nothing to run again */
    if (fold && !busy && r && r.status === 'passed') return '<li class="act acdone" data-aid="' + esc(a.id) + '"><span class="aok" title="Finished at ' + esc(T.clockTime(r.at)) + '">' + OKI + ' ' + esc(a.label) + '</span></li>';
    h = '<li class="act" data-aid="' + esc(a.id) + '"><div class="arow"><span class="alab">' + esc(a.label) + '</span>' +
      '<code class="acmd" title="' + esc(a.command) + '">' + esc(a.command) + '</code><span class="astat">';
    if (busy) {
      h += '<span class="aspin" role="status" aria-label="Running"></span><span class="mut">Running…</span>';
    } else {
      if (r && r.status === 'passed') h += '<span class="aok" title="Finished at ' + esc(T.clockTime(r.at)) + '">' + OKI + ' Passed</span>';
      if (r && r.status === 'failed') h += '<span class="abad" title="Finished at ' + esc(T.clockTime(r.at)) + '">' + BAD + ' Failed · exit ' + esc(r.exit_code) + '</span>';
      if (r && r.log_tail && r.log_tail.length) h += '<button class="alink" data-act="ac-out" data-key="' + esc(a.id) + '" aria-expanded="' + !!ui.actOut[a.id] + '">' + (ui.actOut[a.id] ? 'Hide output' : 'Show output') + '</button>';
      h += can
        ? '<button class="abtn' + (r ? ' again' : '') + '" data-act="ac-ask" data-key="' + esc(a.id) + '" aria-label="Run: ' + esc(a.label) + '">' + (r ? 'Run again' : PLAY + '<span>Run</span>') + '</button>'
        : '<button class="abtn again" data-act="ac-copy" data-key="' + esc(a.id) + '" data-cmd="' + esc(a.command) + '" title="Copy the command to run it in a terminal">Copy</button>';
    }
    h += '</span></div>';
    if (ui.actErr[a.id]) h += '<div class="ierr">' + esc(ui.actErr[a.id]) + '</div>';
    if (ui.actConfirm[a.id] && !busy) {
      h += '<div class="acpop" role="dialog" aria-label="Confirm"><div>Run this command in <b>' + where(a, pid) + '</b>?</div><code class="apcmd">' + esc(a.command) + '</code>' +
        '<div class="apb"><button class="abtn" data-act="ac-go" data-key="' + esc(a.id) + '">Run</button><button class="alink" data-act="ac-cancel" data-key="' + esc(a.id) + '">Cancel</button></div></div>';
    }
    if (busy && r && r.status === 'running') h += log(r.log_tail);
    else if (!busy && ui.actOut[a.id] && r) h += log(r.log_tail);
    if (!can && !r) h += '<div class="anote">Tirith cannot run it from this page. Copy it into a terminal, or install the link handler once (<code>tirith url install</code>).</div>';
    return h + '</li>';
  }

  /* The rows of a list of actions; pid names the project for the default folder. */
  T.actions = function (list, state, pid, fold) {
    if (state) lastState = state;
    if (!list || !list.length) return '';
    return '<ul class="acts">' + list.map(function (a) { return row(a, pid, fold); }).join('') + '</ul>';
  };

  function openLink(url) {
    var a = document.createElement('a'); a.href = url; a.style.display = 'none';
    document.body.appendChild(a); a.click(); document.body.removeChild(a);
  }
  function start(id) {
    var tauri = window.__TAURI__, L = lastState && lastState.launch;
    ui.actErr[id] = null; ui.actConfirm[id] = false;
    function pend() { ui.actPend[id] = T.now(); T.render(); setTimeout(T.render, PEND_WAIT + 500); }
    if (tauri && tauri.core && tauri.core.invoke) {
      tauri.core.invoke('run_action', { actionId: id }).then(pend, function (err) { ui.actErr[id] = 'Could not run: ' + err; T.render(); });
    } else if (L && L.token && L.available) {
      openLink('tirith://run?action=' + encodeURIComponent(id) + '&token=' + encodeURIComponent(L.token));
      pend();
    } else T.render();
  }

  document.getElementById('view').addEventListener('click', function (e) {
    var b = e.target.closest('[data-act^="ac-"]'); if (!b) return;
    var act = b.getAttribute('data-act'), id = b.getAttribute('data-key');
    e.stopPropagation();
    if (act === 'ac-ask') { ui.actConfirm[id] = true; T.render(); }
    else if (act === 'ac-cancel') { ui.actConfirm[id] = false; T.render(); }
    else if (act === 'ac-go') start(id);
    else if (act === 'ac-out') { ui.actOut[id] = !ui.actOut[id]; T.render(); }
    else if (act === 'ac-copy') T.copyText(b.getAttribute('data-cmd'), b);
  });
  document.getElementById('view').addEventListener('keydown', function (e) {
    if (e.key !== 'Escape') return;
    var open = Object.keys(ui.actConfirm).filter(function (k) { return ui.actConfirm[k]; });
    if (open.length) { open.forEach(function (k) { ui.actConfirm[k] = false; }); T.render(); }
  });
})();
