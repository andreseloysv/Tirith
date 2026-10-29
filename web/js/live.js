/* Desktop app only: Cmd/Ctrl+R reloads the page files; a pill offers it when the files on disk changed. */
(function () {
  var tauri = window.__TAURI__ && window.__TAURI__.event ? window.__TAURI__ : null;
  if (!tauri) return;
  function reload() { location.reload(); }   /* the hash stays, so the same view comes back */
  document.addEventListener('keydown', function (e) {
    if ((e.metaKey || e.ctrlKey) && !e.altKey && e.key.toLowerCase() === 'r') { e.preventDefault(); reload(); }
  });
  tauri.event.listen('tirith://web-changed', function () {
    if (document.getElementById('webpill')) return;
    var el = document.createElement('div');
    el.id = 'webpill'; el.className = 'webpill';
    el.innerHTML = '<span>New version of the page</span><button type="button">Reload</button>';
    el.querySelector('button').addEventListener('click', reload);
    document.body.appendChild(el);
  });
})();
