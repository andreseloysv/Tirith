/* Add a project from the sidebar "+": the desktop app opens the native folder picker, then a tiny form
   (name, color) that runs `tirith project add`. A page opened as a file cannot do that, so it says so and
   offers the manager. */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  var COLORS = ['#2f7d5b', '#3b6fd8', '#8a5cd0', '#d0663b', '#c2417c', '#b8860b', '#0e8aa0', '#64748b'];
  var box = null, st = null;

  function tauri() { var t = window.__TAURI__; return t && t.core && t.core.invoke ? t : null; }
  function baseName(p) { var m = String(p).replace(/[\\/]+$/, '').split(/[\\/]/); return m[m.length - 1] || ''; }
  function close() { if (box) { box.remove(); box = null; } st = null; document.removeEventListener('keydown', onKey, true); }
  function onKey(e) { if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); close(); } }

  function swatches() {
    return COLORS.map(function (c) {
      return '<button type="button" class="ap-sw' + (st.color === c ? ' on' : '') + '" data-ap="color" data-c="' + c + '" style="background:' + c + '" aria-label="Color ' + c + '" aria-pressed="' + (st.color === c) + '"></button>';
    }).join('');
  }
  function body() {
    if (st.step === 'browser') {
      return '<p class="ap-p">Adding a project needs the desktop app, which can open a folder picker. Here you can ask the manager to do it.</p>' +
        '<div class="ap-act"><button type="button" class="ap-b" data-ap="close">Cancel</button><button type="button" class="ap-b pri" data-ap="ask">Ask the manager</button></div>';
    }
    if (st.step === 'done') {
      var r = st.result;
      return '<p class="ap-p">' + (r.created ? 'Added ' : 'Already added: ') + '<b>' + esc(st.name || r.id) + '</b></p><p class="ap-folder">' + esc(r.folder) + '</p>' +
        (r.hasToml ? '' : '<p class="ap-p">This folder has no <code>tirith.toml</code> yet, so it has no pipelines.</p>') +
        (st.err ? '<div class="ierr">' + esc(st.err) + '</div>' : '') +
        '<div class="ap-act"><button type="button" class="ap-b' + (r.hasToml ? ' pri' : '') + '" data-ap="close">' + (r.hasToml ? 'Done' : 'Not now') + '</button>' +
        (r.hasToml ? '' : '<button type="button" class="ap-b pri" data-ap="setup">Set up pipelines</button>') + '</div>';
    }
    return '<p class="ap-folder" title="' + esc(st.folder) + '">' + esc(st.folder) + '</p>' +
      '<label class="ap-l" for="ap-name">Name</label><input id="ap-name" class="ap-in" data-ap="name" maxlength="80" value="' + esc(st.name) + '" autocomplete="off">' +
      '<div class="ap-l">Color</div><div class="ap-sws">' + swatches() + '</div>' +
      (st.err ? '<div class="ierr">' + esc(st.err) + '</div>' : '') +
      '<div class="ap-act"><button type="button" class="ap-b" data-ap="close">Cancel</button><button type="button" class="ap-b pri" data-ap="add"' + (st.busy ? ' disabled' : '') + '>' + (st.busy ? 'Adding…' : 'Add') + '</button></div>';
  }
  function draw() {
    if (!box) {
      box = document.createElement('div');
      box.className = 'ap-ov'; box.setAttribute('role', 'dialog'); box.setAttribute('aria-modal', 'true'); box.setAttribute('aria-label', 'Add project');
      box.addEventListener('click', onClick);
      box.addEventListener('input', function (e) { if (e.target.getAttribute('data-ap') === 'name') st.name = e.target.value; });
      box.addEventListener('keydown', function (e) { if (e.key === 'Enter' && e.target.getAttribute('data-ap') === 'name') { e.preventDefault(); add(); } });
      document.body.appendChild(box);
      document.addEventListener('keydown', onKey, true);
    }
    box.innerHTML = '<div class="ap-card"><h3>Add project</h3>' + body() + '</div>';
    var f = box.querySelector('.ap-in, .ap-b.pri'); if (f) f.focus();
  }

  function add() {
    if (!st || st.busy) return;
    st.busy = true; st.err = null; draw();
    tauri().core.invoke('project_add', { folder: st.folder, name: st.name.trim(), color: st.color }).then(function (r) {
      st.busy = false; st.step = 'done'; st.result = r;
      T.setFilter('project', r.id);
      draw();
    }, function (err) { st.busy = false; st.err = String(err); draw(); });
  }
  function setup() {
    var r = st.result;
    tauri().core.invoke('inbox_add', { kind: 'chat', project: r.id, step: '', text: 'Set up pipelines for the project "' + (st.name || r.id) + '" (folder ' + r.folder + '): it has no tirith.toml. Create one and delegate it.' })
      .then(close, function (err) { st.err = 'Could not send: ' + err; draw(); });
  }
  function ask() {
    close();
    ui.chatDraft = 'Add the project in folder: '; ui.chatOpen = true; ui.chatStick = true; T.render();
    var ta = document.querySelector('[data-act="mc-text"]');
    if (ta) { ta.focus(); ta.setSelectionRange(ta.value.length, ta.value.length); }
  }
  function onClick(e) {
    if (e.target === box) return close();
    var b = e.target.closest('[data-ap]'); if (!b || !st) return;
    var a = b.getAttribute('data-ap');
    if (a === 'close') close();
    else if (a === 'color') { st.color = b.getAttribute('data-c'); draw(); }
    else if (a === 'add') add();
    else if (a === 'setup') setup();
    else if (a === 'ask') ask();
  }

  function open() {
    close();
    st = { step: 'browser', folder: '', name: '', color: COLORS[Math.floor(Math.random() * COLORS.length)], err: null };
    var t = tauri();
    if (!t) return draw();
    t.core.invoke('plugin:dialog|open', { options: { directory: true, multiple: false, title: 'Choose the project folder' } }).then(function (picked) {
      var folder = picked && typeof picked === 'object' ? picked.path : picked;
      if (!folder || !st) return close();
      st.step = 'form'; st.folder = String(folder); st.name = baseName(folder);
      draw();
    }, function (err) { st.step = 'form'; st.err = 'Could not open the folder picker: ' + err; draw(); });
  }

  T.addProject = open;
  document.getElementById('nav').addEventListener('click', function (e) {
    if (e.target.closest('[data-addp]')) open();
  });
})();
