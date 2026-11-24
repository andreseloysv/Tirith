// Cmd+R (Ctrl+R elsewhere) reloads the page, also in the desktop app, whose window has no menu that would do it
// (the owner: «Cmd+R no recarga el programa»). In a browser it does just what the browser would; Shift+Cmd+R is
// left to the browser.
addEventListener('keydown', (e) => {
  if (!(e.metaKey || e.ctrlKey) || e.shiftKey || e.altKey || e.key.toLowerCase() !== 'r') return;
  e.preventDefault();
  location.reload();
});
