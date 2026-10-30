/* First-run screen of the desktop app. A new Mac may lack uv (it runs the collector) and has no project yet:
   this explains it, installs uv in a visible Terminal on a click, offers "Add a project" and a labelled demo.
   Gone as soon as uv is there and a project exists. A page opened in a browser never shows it. */
(function () {
  var T = window.T, esc = T.esc, box = null, info = null, demo = false, note = '';

  function tauri() { var t = window.__TAURI__; return t && t.core && t.core.invoke ? t : null; }
  function close() { if (box) { box.remove(); box = null; } }

  function body() {
    if (!info.uv) {
      return '<p class="ap-p">Tirith runs its collector with <b>uv</b>, a small Python tool, and it is not installed on this Mac yet.</p>' +
        '<p class="ap-p">The button opens Terminal and runs the official installer there, so you see everything it does. Tirith carries on by itself when it finishes.</p>' +
        (note ? '<p class="ap-p">' + esc(note) + '</p>' : '') +
        '<div class="ap-act"><a class="ap-b" href="' + esc(info.installPage) + '">Read about uv</a><button type="button" class="ap-b" data-fr="demo">Look at demo data</button>' +
        '<button type="button" class="ap-b pri" data-fr="uv">Install uv</button></div>';
    }
    return '<p class="ap-p">Add the folder of a project and Tirith starts showing its pipelines, agents and plan within a minute.</p>' +
      (note ? '<p class="ap-p">' + esc(note) + '</p>' : '') +
      '<div class="ap-act"><button type="button" class="ap-b" data-fr="demo">Look at demo data</button><button type="button" class="ap-b pri" data-fr="add">Add a project</button></div>';
  }
  function draw() {
    if (!box) {
      box = document.createElement('div');
      box.className = 'ap-ov'; box.setAttribute('role', 'dialog'); box.setAttribute('aria-modal', 'true'); box.setAttribute('aria-label', 'Welcome to Tirith');
      box.addEventListener('click', onClick);
      document.body.appendChild(box);
    }
    box.innerHTML = '<div class="ap-card"><h3>' + (info.uv ? 'Welcome to Tirith' : 'Tirith needs uv') + '</h3>' + body() + '</div>';
  }

  function loadDemo() {
    delete window.TIRITH_STATE;
    var sc = document.createElement('script');
    sc.src = 'demo/state.demo.js?t=' + Date.now();
    sc.onload = function () { sc.remove(); if (window.TIRITH_STATE && T.showDemo) { demo = true; close(); T.showDemo(window.TIRITH_STATE); } };
    sc.onerror = function () { sc.remove(); note = 'The demo data could not be loaded.'; if (box) draw(); };
    document.head.appendChild(sc);
  }
  function onClick(e) {
    var b = e.target.closest('[data-fr]'); if (!b) return;
    var a = b.getAttribute('data-fr');
    if (a === 'add') T.addProject();
    else if (a === 'demo') loadDemo();
    else if (a === 'uv') {
      tauri().core.invoke('install_uv').then(function () { note = 'Terminal is open. Finish the install there; this window updates by itself.'; draw(); },
        function (err) { note = 'Could not open Terminal: ' + err; draw(); });
    }
  }

  function poll() {
    tauri().core.invoke('first_run').then(function (r) {
      info = r;
      if (r.uv && r.projects > 0) { demo = false; close(); }
      else if (!demo) draw();
    }, function () {});
  }

  if (tauri()) { poll(); setInterval(poll, 2000); }
})();
