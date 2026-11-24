/* Tabs (the install guide's systems, the bots' jobs) and the copy buttons of the commands. Each tablist's
   buttons name their panel with aria-controls; arrows and Home/End move between them, as WAI-ARIA asks. */
(function () {
  function select(list, tab, focus) {
    list.querySelectorAll('[role="tab"]').forEach(function (t) {
      var on = t === tab;
      t.setAttribute('aria-selected', on ? 'true' : 'false');
      t.tabIndex = on ? 0 : -1;
      var panel = document.getElementById(t.getAttribute('aria-controls'));
      if (panel) panel.hidden = !on;
    });
    if (focus) tab.focus();
  }

  function tablist(list) {
    var tabs = Array.prototype.slice.call(list.querySelectorAll('[role="tab"]'));
    tabs.forEach(function (t) {
      t.addEventListener('click', function () { select(list, t, false); });
      t.addEventListener('keydown', function (e) {
        var i = tabs.indexOf(t), next = null;
        if (e.key === 'ArrowRight' || e.key === 'ArrowDown') next = tabs[(i + 1) % tabs.length];
        if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') next = tabs[(i - 1 + tabs.length) % tabs.length];
        if (e.key === 'Home') next = tabs[0];
        if (e.key === 'End') next = tabs[tabs.length - 1];
        if (!next) return;
        e.preventDefault();
        select(list, next, true);
      });
    });
  }

  /* A visitor on Linux or Windows lands on their own system's guide. */
  function ownSystem() {
    var list = document.getElementById('systems');
    if (!list) return;
    var ua = navigator.userAgent;
    var id = /Windows/.test(ua) ? 'tab-windows' : /Linux|X11/.test(ua) && !/Android/.test(ua) ? 'tab-linux' : null;
    var tab = id && document.getElementById(id);
    if (tab) select(list, tab, false);
  }

  function copy(button) {
    var code = button.parentNode.querySelector('code');
    var text = code ? code.textContent : '';
    var done = function (word) {
      button.textContent = word;
      setTimeout(function () { button.textContent = 'Copy'; }, 1600);
    };
    if (navigator.clipboard && window.isSecureContext) {
      navigator.clipboard.writeText(text).then(function () { done('Copied'); }, function () { done('Select it'); });
    } else {
      done('Select it');
    }
  }

  document.addEventListener('DOMContentLoaded', function () {
    document.querySelectorAll('[role="tablist"]').forEach(tablist);
    ownSystem();
    document.querySelectorAll('.cmd button').forEach(function (b) {
      b.addEventListener('click', function () { copy(b); });
    });
  });
})();
