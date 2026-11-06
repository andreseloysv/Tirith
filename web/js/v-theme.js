/* Settings view, "Appearance" card: System (default, follows the OS), Light or Dark.
   The choice is kept in localStorage "tirith.theme" ("light" | "dark"; absent = System). index.html applies it before first paint.
   data-theme on <html> is set only for Light and Dark; System leaves it off, and css/style.css follows prefers-color-scheme. */
(function () {
  var T = window.T, KEY = 'tirith.theme';
  var CHOICES = [['system', 'System'], ['light', 'Light'], ['dark', 'Dark']];

  function get() {
    try { var v = localStorage.getItem(KEY); return v === 'light' || v === 'dark' ? v : 'system'; } catch (e) { return 'system'; }
  }
  function apply(mode) {
    if (mode === 'light' || mode === 'dark') document.documentElement.dataset.theme = mode;
    else delete document.documentElement.dataset.theme;
  }
  function set(mode) {
    if (mode !== 'light' && mode !== 'dark') mode = 'system';
    try { if (mode === 'system') localStorage.removeItem(KEY); else localStorage.setItem(KEY, mode); } catch (e) {}
    apply(mode);
  }
  function card() {
    var cur = get();
    return '<div class="card scard themecard"><div class="phead"><b>Appearance</b><span class="mut sm">System follows the light or dark setting of your computer</span></div>' +
      '<div class="seg" role="group" aria-label="Theme" id="theme-seg">' + CHOICES.map(function (c) {
        return '<button type="button" data-theme-pick="' + c[0] + '" aria-pressed="' + (c[0] === cur) + '"' + (c[0] === cur ? ' class="on"' : '') + '>' + c[1] + '</button>';
      }).join('') + '</div></div>';
  }

  apply(get());
  T.theme = { get: get, set: set, apply: apply };

  var base = T.views.settings;
  T.views.settings = function (state, el) {
    base(state, el);
    var host = el.querySelector('.msettings');
    if (host) host.insertAdjacentHTML('afterbegin', card());
    else el.insertAdjacentHTML('afterbegin', '<section class="msettings">' + card() + '</section>');
  };

  document.addEventListener('click', function (e) {
    var b = e.target.closest && e.target.closest('[data-theme-pick]');
    if (!b) return;
    set(b.getAttribute('data-theme-pick'));
    var seg = document.getElementById('theme-seg');
    if (seg) [].forEach.call(seg.children, function (x) {
      var on = x === b; x.classList.toggle('on', on); x.setAttribute('aria-pressed', on);
    });
  });
})();
