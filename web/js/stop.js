/* "Stop" for a running agent: a small button on the card, in the drawer and by the avatar in the Plan,
   each opening a confirm popover first. The page sends the lane id only: the app (lane_stop, which checks
   the id against the running lanes in the state) or the tirith://stop link; both run `tirith lane stop`.
   See docs/CONTRACT-gates.md. */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui, PEND_WAIT = 20000;
  ui.stopConfirm = ui.stopConfirm || {};  /* lane id -> true while its confirm popover is open */
  ui.stopPend = ui.stopPend || {};        /* lane id -> ms of a stop the collector has not shown yet */
  ui.stopErr = ui.stopErr || {};          /* lane id -> why sending failed */
  ui.stopCli = ui.stopCli || {};          /* lane id -> CLI line to copy (no way to send from here) */

  var SQUARE = '<svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true"><rect x="2.5" y="2.5" width="7" height="7" rx="1.4" fill="currentColor"/></svg>';

  function stoppable(a) { return a && a.source !== 'github' && (a.status === 'working' || a.status === 'stalled'); }
  function pending(a) {
    var p = ui.stopPend[a.id];
    if (p && T.now() - p > PEND_WAIT) { delete ui.stopPend[a.id]; p = null; }
    return !!p;
  }

  /* where: 'card' (top right of a working card), 'row' (drawer / list), 'ava' (Plan step) */
  T.stopBtn = function (a, where) {
    if (!stoppable(a)) return '';
    var id = esc(a.id), h;
    if (pending(a)) return '<span class="stopb stopping ' + where + '" role="status">Stopping…</span>';
    h = '<span class="stopb ' + where + '" role="button" tabindex="0" data-act="stop-ask" data-key="' + id + '" title="Stop ' + esc(a.name) + ': it ends as failed and its worktree is kept">' + SQUARE + '<span>Stop</span></span>';
    if (ui.stopConfirm[a.id]) {
      h += '<span class="stoppop ' + where + '" role="dialog" aria-label="Confirm stop"><span class="sptxt">Stop <b>' + esc(a.name) + '</b>? It ends as failed and its worktree is kept.</span>' +
        '<span class="spb"><span class="stopgo" role="button" tabindex="0" data-act="stop-go" data-key="' + id + '">Stop agent</span>' +
        '<span class="alink" role="button" tabindex="0" data-act="stop-cancel" data-key="' + id + '">Cancel</span></span></span>';
    }
    if (ui.stopErr[a.id]) h += '<span class="ierr stoperr">' + esc(ui.stopErr[a.id]) + '</span>';
    if (ui.stopCli[a.id]) h += '<span class="icli stoperr">Tirith cannot send from this page. Run in a terminal:<code>' + esc(ui.stopCli[a.id]) + '</code></span>';
    return h;
  };

  function openLink(url) {
    var a = document.createElement('a'); a.href = url; a.style.display = 'none';
    document.body.appendChild(a); a.click(); document.body.removeChild(a);
  }
  function send(id) {
    var st = T.rawState && T.rawState(), L = st && st.launch, tauri = window.__TAURI__;
    ui.stopErr[id] = null; ui.stopCli[id] = null; ui.stopConfirm[id] = false;
    function pend() { ui.stopPend[id] = T.now(); T.render(); setTimeout(T.render, PEND_WAIT + 500); }
    if (tauri && tauri.core && tauri.core.invoke) {
      tauri.core.invoke('lane_stop', { laneId: id }).then(pend, function (err) { ui.stopErr[id] = 'Could not stop: ' + err; T.render(); });
    } else if (L && L.token && L.available) {
      openLink('tirith://stop?lane=' + encodeURIComponent(id) + '&token=' + encodeURIComponent(L.token));
      pend();
    } else {
      ui.stopCli[id] = 'tirith lane stop ' + id + ' --by owner';
      T.render();
    }
  }

  function act(e) {
    var b = e.target.closest('[data-act^="stop-"]'); if (!b) return false;
    var a = b.getAttribute('data-act'), id = b.getAttribute('data-key');
    e.stopPropagation(); e.preventDefault();
    if (a === 'stop-ask') { ui.stopConfirm[id] = !ui.stopConfirm[id]; T.render(); }
    else if (a === 'stop-cancel') { ui.stopConfirm[id] = false; T.render(); }
    else if (a === 'stop-go') send(id);
    return true;
  }
  var view = document.getElementById('view');
  view.addEventListener('click', function (e) { act(e); }, true);  /* capture: before the card's own click */
  view.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') {
      var open = Object.keys(ui.stopConfirm).filter(function (k) { return ui.stopConfirm[k]; });
      if (open.length) { open.forEach(function (k) { ui.stopConfirm[k] = false; }); T.render(); }
    } else if ((e.key === 'Enter' || e.key === ' ') && e.target.closest && e.target.closest('[data-act^="stop-"]')) act(e);
  }, true);
})();
