/* The health dot in the header: green, amber while Tirith heals itself, red only when a fix failed.
   A click lists the checks in plain words (state.health, built by collector/tirith/selfcheck.py).
   docs/CONTRACT-selfcheck.md. */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  var btn = document.getElementById('health-btn'), panel = document.getElementById('health-panel'), base = T.render;
  var TITLE = { green: 'All is well', amber: 'Tirith is fixing something', red: 'Something needs a look' };
  var LEVEL = { ok: 'green', healing: 'amber', failed: 'red' };
  ui.healthOpen = !!ui.healthOpen;
  function draw(h) {
    if (!btn || !panel) return;
    btn.hidden = !h;
    if (!h) { panel.hidden = true; return; }
    btn.className = 'health-btn ' + h.level;
    btn.setAttribute('aria-label', 'Health: ' + (TITLE[h.level] || h.level));
    btn.title = TITLE[h.level] || '';
    panel.hidden = !ui.healthOpen;
    panel.innerHTML = '<b>' + esc(TITLE[h.level] || '') + '</b>' + (h.checks || []).map(function (c) {
      return '<div class="hc"><i class="dot ' + (LEVEL[c.level] || 'grey') + '"></i><span><b>' + esc(c.label) + '</b> ' + esc(c.text) + '</span></div>';
    }).join('');
  }
  T.render = function (fstate, raw) {
    var out = base.apply(this, arguments), s = raw || fstate;
    draw(s && s.health ? s.health : null);
    return out;
  };
  if (btn) btn.addEventListener('click', function () { ui.healthOpen = !ui.healthOpen; panel.hidden = !ui.healthOpen; });
  document.addEventListener('keydown', function (e) { if (e.key === 'Escape' && ui.healthOpen) { ui.healthOpen = false; if (panel) panel.hidden = true; } });
})();
