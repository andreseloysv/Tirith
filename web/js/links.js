/* Web links in the desktop app. The app's webview ignores target="_blank", so a click on an http(s) link
   (chat markdown, a needs item's Open, Open on GitHub) asks the app (open_url) to open it in the default
   browser. A plain browser keeps its own behaviour; tirith://, file links (data-act="fopen") and the "#"
   links of the page itself are left alone. */
(function () {
  var T = window.T = window.T || {};

  /* the URL a click on `a` should hand to the browser, or null */
  T.webLink = function (a) {
    var href = a && a.getAttribute && a.getAttribute('href');
    return href && /^https?:\/\//i.test(href) ? href : null;
  };

  /* capture: before the view's own click handlers; the click still reaches them (a link in a card stays a click on the card) */
  document.addEventListener('click', function (e) {
    var tauri = window.__TAURI__;
    if (e.defaultPrevented || !(tauri && tauri.core && tauri.core.invoke)) return;
    var a = e.target.closest && e.target.closest('a[href]'), url = T.webLink(a); if (!url) return;
    e.preventDefault();
    tauri.core.invoke('open_url', { url: url }).catch(function (err) {
      if (T.note) T.note('Could not open ' + url + ': ' + err, true);
    });
  }, true);
})();
