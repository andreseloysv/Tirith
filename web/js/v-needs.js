/* "Your turn" items, as the chat sees them: every thing that needs the owner is a manager message (v-manager.js),
   with its one-click options, command, link and Run actions under it. This file builds the items from the state
   and sends the replies; the thread itself is v-manager.js. docs/CONTRACT-asks.md, "Web". */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  ui.needsCli = ui.needsCli || {};      /* id -> CLI line to copy (no way to send from here) */
  ui.needsErr = ui.needsErr || {};      /* id -> why the send failed */
  var lastState = null, STORE = 'tirith.needs.sent', sent = load();
  var DEFAULTS = {
    approval: [{ label: 'Approve', value: 'approve' }, { label: 'Decline', value: 'decline' }],
    info: [{ label: 'Got it', value: 'ack' }]
  };

  /* what the owner already sent, kept across reloads so an item never comes back after the click */
  function load() { try { return JSON.parse(localStorage.getItem(STORE) || '{}') || {}; } catch (e) { return {}; } }
  function save() { try { localStorage.setItem(STORE, JSON.stringify(sent)); } catch (e) {} }

  function shq(t) { return "'" + String(t).replace(/'/g, "'\\''") + "'"; }
  function cliCmd(w, value, text) {
    return 'tirith inbox reply --wait ' + shq(w.id) + (w.project ? ' --project ' + shq(w.project) : '') +
      (value ? ' --value ' + shq(value) : '') + (text ? ' --text ' + shq(text) : '');
  }
  function find(id) {
    return ((lastState || {}).waiting_on_you || []).filter(function (w) { return w.id === id && w.status !== 'answered'; })[0];
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

  /* [{w, done, answer}] oldest first: the open items, then the answered ones (the last 10 the collector lists,
     plus those answered from this page that the collector has not shown yet) */
  T.turnItems = function (s) {
    lastState = s;
    var out = [], seen = {}, answered = (s.answered_waits || []).slice();
    answered.forEach(function (w) { delete sent[w.id]; });
    (s.waiting_on_you || []).filter(function (w) { return w.status !== 'answered'; }).forEach(function (w) {
      var k = (w.project || '') + '\n' + String(w.text || '').replace(/\s+/g, ' ').trim().toLowerCase();
      if (seen[k]) return;   /* the same text for the same project is one item */
      seen[k] = 1;
      if (sent[w.id]) answered.push({ id: w.id, project: w.project, text: w.text, since: w.since, actions: w.actions, status: 'answered', answer: { at: sent[w.id].at, label: sent[w.id].label, text: sent[w.id].text } });
      else out.push({ w: w, done: false });
    });
    answered.forEach(function (w) { out.push({ w: w, done: true, answer: w.answer || {} }); });
    return out;
  };
  T.turnOpen = function (s) { return T.turnItems(s).filter(function (it) { return !it.done; }).length; };
  T.turnFind = function (id) { return find(id); };

  /* what the owner chose, as the "You:" line */
  T.turnAnswer = function (a) {
    var label = String(a.label || '').replace(/^✓\s*/, '');
    return [label, a.text].filter(Boolean).join(' · ') || a.value || 'Done';
  };

  /* Free text typed in the chat composer as the reply to item `id`. */
  T.turnReply = function (id, text) {
    var w = find(id); if (!w || !text) return false;
    reply(w, '', '', text);
    return true;
  };

  /* The part under the message of an open item: options, command, link, Run actions, errors. Nothing for an answered one
     (its Run actions stay, as they show what ran). */
  T.turnBlock = function (it, state) {
    var w = it.w, h = '';
    if (state) lastState = state;
    if (it.done) return w.actions && w.actions.length ? '<div class="mc-acts">' + T.actions(w.actions, lastState, w.project, true) + '</div>' : '';
    h += '<div class="nacts">';
    opts(w).forEach(function (o, i) {
      h += '<button class="nbtn' + (i === 0 ? ' primary' : '') + '" data-act="nopt" data-key="' + esc(w.id) + '" data-i="' + i + '">' + esc(o.label) + '</button>';
    });
    var dup = (w.actions || []).some(function (a) { return a.command === w.command; });   /* the Run row already has it */
    if (w.command && !dup) h += '<button class="nbtn" data-act="copy" data-key="' + esc(w.command) + '" title="Copy the command">Copy</button><code class="ncmd" title="' + esc(w.command) + '">' + esc(w.command) + '</code>';
    if (w.link) h += '<a class="nbtn" href="' + esc(w.link) + '" target="_blank" rel="noopener">Open</a>';
    h += '<button class="nreply" data-act="mc-reply" data-key="' + esc(w.id) + '" type="button" title="Write your answer in the chat box">Reply</button></div>';
    if (w.actions && w.actions.length) h += '<div class="mc-acts">' + T.actions(w.actions, lastState, w.project) + '</div>';
    if (ui.needsErr[w.id]) h += '<div class="ierr">' + esc(ui.needsErr[w.id]) + '</div>';
    if (ui.needsCli[w.id]) h += '<div class="icli">Tirith cannot send from this page. Run in a terminal:<code>' + esc(ui.needsCli[w.id]) + '</code></div>';
    return h;
  };

  document.getElementById('view').addEventListener('click', function (e) {
    var b = e.target.closest('[data-act="nopt"]'); if (!b) return;
    var key = b.getAttribute('data-key'), w = find(key); if (!w) return;
    var o = opts(w)[+b.getAttribute('data-i')]; if (!o) return;
    var text = ui.chatReplyTo === key ? (ui.chatDraft || '').trim() : '';   /* a draft aimed at this item goes along */
    if (text) { ui.chatDraft = ''; ui.chatReplyTo = null; }
    reply(w, o.value, o.label, text);
  });
})();
