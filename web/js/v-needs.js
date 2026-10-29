/* "Your turn" on the Overview: one card per thing that needs the owner, each with a clear call to action.
   One-click options, a command to copy, a link to open, and a box to write what you want. */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  ui.needsOpen = ui.needsOpen || {};    /* id -> true: title shown in full */
  ui.needsDraft = ui.needsDraft || {};  /* id -> text typed so far */
  ui.needsCli = ui.needsCli || {};      /* id -> CLI line to copy (no way to send from here) */
  ui.needsErr = ui.needsErr || {};      /* id -> why the send failed */
  ui.needsFold = ui.needsFold || {};    /* 'older' | 'answered' -> true while open */
  var lastState = null, OLD_MS = 48 * 3600 * 1000, STORE = 'tirith.needs.sent', sent = load();
  var DEFAULTS = {
    approval: [{ label: 'Approve', value: 'approve' }, { label: 'Decline', value: 'decline' }],
    info: [{ label: 'Got it', value: 'ack' }]
  };

  /* what the owner already sent, kept across reloads so a card never comes back after the click */
  function load() { try { return JSON.parse(localStorage.getItem(STORE) || '{}') || {}; } catch (e) { return {}; } }
  function save() { try { localStorage.setItem(STORE, JSON.stringify(sent)); } catch (e) {} }

  function shq(t) { return "'" + String(t).replace(/'/g, "'\\''") + "'"; }
  function cliCmd(w, value, text) {
    return 'tirith inbox reply --wait ' + shq(w.id) + (w.project ? ' --project ' + shq(w.project) : '') +
      (value ? ' --value ' + shq(value) : '') + (text ? ' --text ' + shq(text) : '');
  }
  function find(id) {
    var s = lastState || {}, all = (s.waiting_on_you || []);
    return all.filter(function (w) { return w.id === id; })[0];
  }
  function opts(w) {
    if (w.options && w.options.length) return w.options;
    if (w.kind === 'action') return [{ label: 'Done', value: 'done' }];
    return DEFAULTS[w.kind] || [];
  }

  /* Send the reply: the app writes the inbox itself, a browser opens the tirith:// link, else the CLI line is shown. */
  function reply(w, value, label, text) {
    var id = w.id, st = lastState, L = st && st.launch, tauri = window.__TAURI__;
    ui.needsErr[id] = null; ui.needsCli[id] = null;
    function ok() { sent[id] = { at: new Date().toISOString(), label: label || '', text: text || '' }; save(); T.render(); }
    if (tauri && tauri.core && tauri.core.invoke) {
      tauri.core.invoke('inbox_reply', { wait: id, project: w.project || '', value: value || '', text: text || '' }).then(ok, function (err) {
        ui.needsErr[id] = 'Could not send: ' + err; T.render();
      });
      return;
    }
    if (L && L.token && L.available) {
      var a = document.createElement('a');
      a.href = 'tirith://inbox?kind=reply&wait=' + encodeURIComponent(id) + '&project=' + encodeURIComponent(w.project || '') +
        '&value=' + encodeURIComponent(value || '') + '&text=' + encodeURIComponent(text || '') + '&token=' + encodeURIComponent(L.token);
      a.style.display = 'none'; document.body.appendChild(a); a.click(); document.body.removeChild(a);
      ok();
      return;
    }
    ui.needsCli[id] = cliCmd(w, value, text);
    T.render();
  }

  function age(w) { var t = T.ms(w.since); return t == null ? 0 : T.now() - t; }
  function isOld(w) { return age(w) > OLD_MS && !(w.options && w.options.length) && !w.command && !w.link; }
  function long(w) { return w.text.length > 150 || w.text.split('\n').length > 2; }

  function card(w, done) {
    var open = !!ui.needsOpen[w.id], pn = w.project ? T.proj(w.project) : '', h;
    h = '<li class="ncard' + (done ? ' done' : '') + '"><div class="nhead">' +
      '<div class="ntitle' + (open ? ' open' : '') + '"' + (long(w) ? ' data-act="nopen" data-key="' + esc(w.id) + '" role="button" tabindex="0" title="' + (open ? 'Show less' : 'Show all') + '"' : '') + '>' + T.md(w.text) + '</div>' +
      (pn ? '<span class="nchip">' + esc(pn) + '</span>' : '') + '<span class="nage">' + T.agoSpan(w.since) + '</span></div>';
    if (done) {
      var a = w.answer || {};
      return h + '<div class="nsent"><svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"><path d="M2.6 6.3l2.3 2.3 4.5-4.9" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>Sent to the steward' +
        (a.label || a.text ? ' · <b>' + esc(a.label || '') + '</b>' + (a.label && a.text ? ' · ' : '') + esc(a.text || '') : '') + '</div></li>';
    }
    h += '<div class="nacts">';
    opts(w).forEach(function (o, i) {
      h += '<button class="nbtn' + (i === 0 ? ' primary' : '') + '" data-act="nopt" data-key="' + esc(w.id) + '" data-i="' + i + '">' + esc(o.label) + '</button>';
    });
    if (w.command) h += '<button class="nbtn" data-act="copy" data-key="' + esc(w.command) + '" title="Copy the command">Copy</button><code class="ncmd" title="' + esc(w.command) + '">' + esc(w.command) + '</code>';
    if (w.link) h += '<a class="nbtn" href="' + esc(w.link) + '" target="_blank" rel="noopener">Open</a>';
    h += '</div><div class="nwrite"><input class="iin" type="text" data-act="ntext" data-key="' + esc(w.id) + '" maxlength="1000" placeholder="Or write what you need…" aria-label="Your reply" value="' + esc(ui.needsDraft[w.id] || '') + '"></div>';
    if (ui.needsErr[w.id]) h += '<div class="ierr">' + esc(ui.needsErr[w.id]) + '</div>';
    if (ui.needsCli[w.id]) h += '<div class="icli">Tirith cannot send from this page. Run in a terminal:<code>' + esc(ui.needsCli[w.id]) + '</code></div>';
    return h + '</li>';
  }

  function fold(key, label, items, done) {
    if (!items.length) return '';
    var open = !!ui.needsFold[key];
    return '<div class="nfold"><button class="nfoldh" data-act="nfold" data-key="' + key + '" aria-expanded="' + open + '"><svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true" style="transform:rotate(' + (open ? 0 : -90) + 'deg)"><path d="M3 4.5l3 3 3-3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>' + label + '<span class="n">' + items.length + '</span></button>' +
      (open ? '<ul class="nlist">' + items.map(function (w) { return card(w, done); }).join('') + '</ul>' : '') + '</div>';
  }

  T.needs = function (s) {
    lastState = s;
    var answeredIds = {}, waits = (s.waiting_on_you || []).filter(function (w) { return w.status !== 'answered'; }), mine = [], older = [], answered;
    answered = (s.answered_waits || []).slice();
    answered.forEach(function (w) { answeredIds[w.id] = 1; delete sent[w.id]; });
    waits.slice().sort(function (a, b) { return (T.ms(b.since) || 0) - (T.ms(a.since) || 0); }).forEach(function (w) {
      if (sent[w.id]) {   /* sent from this page, the collector has not shown the answer yet */
        answered.unshift({ id: w.id, project: w.project, text: w.text, since: w.since, status: 'answered', answer: { label: sent[w.id].label, text: sent[w.id].text } });
      } else (isOld(w) ? older : mine).push(w);
    });
    var h = '<section class="needs-sec"><h2>Your turn' + (mine.length ? ' <span class="icount">' + mine.length + '</span>' : '') + '</h2>';
    h += mine.length ? '<ul class="nlist">' + mine.map(function (w) { return card(w, false); }).join('') + '</ul>'
      : T.empty('Nothing needs you. The steward will ask here.');
    return h + fold('older', 'Older', older, false) + fold('answered', 'Answered', answered, true) + '</section>';
  };

  var view = document.getElementById('view');
  view.addEventListener('click', function (e) {
    var b = e.target.closest('[data-act="nopt"],[data-act="nopen"],[data-act="nfold"]'); if (!b) return;
    var act = b.getAttribute('data-act'), key = b.getAttribute('data-key');
    if (act === 'nfold') { ui.needsFold[key] = !ui.needsFold[key]; T.render(); return; }
    if (act === 'nopen') { ui.needsOpen[key] = !ui.needsOpen[key]; T.render(); return; }
    var w = find(key); if (!w) return;
    var o = opts(w)[+b.getAttribute('data-i')]; if (!o) return;
    var text = (ui.needsDraft[key] || '').trim(); ui.needsDraft[key] = '';
    reply(w, o.value, o.label, text);
  });
  view.addEventListener('input', function (e) {
    var t = e.target; if (t.getAttribute && t.getAttribute('data-act') === 'ntext') ui.needsDraft[t.getAttribute('data-key')] = t.value;
  });
  view.addEventListener('keydown', function (e) {
    var t = e.target; if (!t.getAttribute) return;
    var act = t.getAttribute('data-act'), key = t.getAttribute('data-key');
    if (act === 'nopen' && (e.key === 'Enter' || e.key === ' ')) { e.preventDefault(); ui.needsOpen[key] = !ui.needsOpen[key]; T.render(); return; }
    if (act !== 'ntext' || e.key !== 'Enter' || e.isComposing) return;
    e.preventDefault();
    var text = t.value.trim(), w = find(key); if (!text || !w) return;
    ui.needsDraft[key] = '';
    reply(w, '', '', text);
  });
})();
