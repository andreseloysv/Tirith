/* File references in the chat (docs/CONTRACT-files.md). The collector lists, per project, the paths named in
   chat messages that exist (state.file_refs); T.fileOpts gives T.md those, and md.js links them. A click
   sends only the project, the path and the line: the app (open_file) or the tirith://open link resolve it
   again and refuse anything outside the registered project folders. */
(function () {
  var T = window.T;

  /* the opts for T.md(text, opts) of a message of `project` ('' = none), or null when nothing is known */
  T.fileOpts = function (state, project) {
    var list = state && state.file_refs && state.file_refs[project || ''];
    if (!list || !list.length) return null;
    var set = {};
    list.forEach(function (p) { set[p] = true; });
    return { project: project || '', file: function (p) { return set[p] === true; } };
  };

  function openLink(url) {
    var a = document.createElement('a'); a.href = url; a.style.display = 'none';
    document.body.appendChild(a); a.click(); document.body.removeChild(a);
  }
  /* one line at the bottom of the window for a few seconds (the chat re-renders under it) */
  var toast = null, toastTimer = 0;
  function note(text, bad) {
    if (!toast) { toast = document.createElement('div'); toast.className = 'ftoast'; toast.setAttribute('role', 'status'); document.body.appendChild(toast); }
    toast.textContent = text; toast.classList.toggle('bad', !!bad); toast.hidden = false;
    clearTimeout(toastTimer); toastTimer = setTimeout(function () { toast.hidden = true; }, 4000);
  }
  function open(a) {
    var tauri = window.__TAURI__, st = T.rawState && T.rawState(), L = st && st.launch;
    var project = a.getAttribute('data-project') || '', path = a.getAttribute('data-path') || '';
    var line = +a.getAttribute('data-line') || null, col = +a.getAttribute('data-col') || null;
    if (tauri && tauri.core && tauri.core.invoke) {
      tauri.core.invoke('open_file', { project: project, path: path, line: line, col: col })
        .catch(function (err) { note('Could not open ' + path + ': ' + err, true); });
    } else if (L && L.token && L.available) {
      openLink('tirith://open?project=' + encodeURIComponent(project) + '&path=' + encodeURIComponent(path) +
        (line ? '&line=' + line : '') + (col ? '&col=' + col : '') + '&token=' + encodeURIComponent(L.token));
    } else {
      T.copyText(path);
      note('Copied ' + path + '. To open files from this page, install the link handler once: tirith url install');
    }
  }
  /* capture: before the view's own click handlers, and the "#" never reaches the hash */
  document.addEventListener('click', function (e) {
    var a = e.target.closest && e.target.closest('a[data-act="fopen"]'); if (!a) return;
    e.preventDefault(); e.stopPropagation();
    open(a);
  }, true);
})();
