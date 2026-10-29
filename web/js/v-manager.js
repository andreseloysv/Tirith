/* Chat with the manager: the right column of the Overview, and a drawer from the header in every other view.
   One thread, built only from state.inbox: the owner's chat messages (kind "chat"), the questions and finish
   requests of the Plan, and every answer to them (markdown, Run actions). docs/CONTRACT-inbox.md, "Chat". */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  ui.chatOpen = !!ui.chatOpen;       /* the drawer (other views) */
  ui.chatDraft = ui.chatDraft || ''; /* text typed so far */
  ui.chatProj = ui.chatProj == null ? null : ui.chatProj; /* project chip: null follows the sidebar filter */
  ui.chatOut = ui.chatOut || [];     /* sent from here, not yet in the state: {text, project, at} */
  ui.chatCli = ui.chatCli || null;   /* CLI line to copy (no way to send from here) */
  ui.chatErr = ui.chatErr || null;   /* why the send failed */
  ui.chatStick = true;               /* scroll to the newest message after the next render */
  var lastState = null, OUT_MAX = 30000, PENDING_MAX = 6 * 3600 * 1000, KINDS = { chat: 1, ask: 1, finish: 1 };

  var SEND = '<svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true"><path d="M8 13V3.2M3.8 7.2L8 3l4.2 4.2" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>';
  var BUBBLE = '<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M3 3h10a1 1 0 011 1v6a1 1 0 01-1 1H7.5L4.6 13.6V11H3a1 1 0 01-1-1V4a1 1 0 011-1z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/></svg>';

  function shq(t) { return "'" + String(t).replace(/'/g, "'\\''") + "'"; }
  function pending(r) {
    if (!KINDS[r.kind] || r.status === 'done') return false;
    var p = r.manager_pending != null ? r.manager_pending : !(r.answers || []).some(function (a) { return a.by === 'manager'; });
    var at = T.ms(r.at);
    return !!p && (at == null || T.now() - at < PENDING_MAX);
  }
  function who(a) { return a.by === 'brain' ? 'Muse' : a.by === 'tirith' ? 'Tirith' : 'Manager'; }
  function ctxOf(r) {
    var parts = [];
    if (r.project) parts.push('on ' + T.proj(r.project));
    if (r.kind === 'ask' || r.kind === 'finish') parts.push('step ' + r.step);
    return parts.join(' · ');
  }

  /* [{side, text, at, ctx, actions, req}] oldest first, with the local sends not yet in the state */
  function messages(state) {
    var out = [], rows = state.inbox || [];
    rows.forEach(function (r) {
      if (!KINDS[r.kind]) return;
      var ctx = ctxOf(r);
      out.push({ side: 'me', text: r.kind === 'finish' ? 'Finish this step' : r.text, at: r.at, ctx: ctx, req: r.id, project: r.project });
      (r.answers || []).forEach(function (a) {
        out.push({ side: 'mgr', text: a.text, at: a.at, ctx: who(a) + (a.auto ? ', automatic' : ''), actions: a.actions, req: r.id, project: r.project });
      });
    });
    var now = T.now();
    ui.chatOut = ui.chatOut.filter(function (o) {
      return now - o.at < OUT_MAX && !rows.some(function (r) { return r.kind === 'chat' && r.text === o.text && (T.ms(r.at) || 0) >= o.at - 2000; });
    });
    ui.chatOut.forEach(function (o) {
      out.push({ side: 'me', text: o.text, at: new Date(o.at).toISOString(), ctx: o.project ? T.proj(o.project) : '', local: true });
    });
    out.sort(function (a, b) { return (T.ms(a.at) || 0) - (T.ms(b.at) || 0); });
    return out.slice(-80);
  }
  /* ISO time of the oldest message the manager still owes an answer to, or null */
  function waitingSince(state) {
    var ats = (state.inbox || []).filter(pending).map(function (r) { return r.at; });
    ui.chatOut.forEach(function (o) { ats.push(new Date(o.at).toISOString()); });
    ats.sort(function (a, b) { return (T.ms(a) || 0) - (T.ms(b) || 0); });
    return ats[0] || null;
  }
  T.chatPending = function (state) { return !!waitingSince(state || lastState || {}); };

  function msg(m, state) {
    return '<div class="mc-m ' + m.side + (m.local ? ' local' : '') + '"><div class="mc-b' + (m.side === 'me' ? '' : ' md') + '">' +
      (m.side === 'me' ? esc(m.text).replace(/\n/g, '<br>') : T.md(m.text)) + '</div>' +
      (m.actions && m.actions.length ? '<div class="mc-acts">' + T.actions(m.actions, state, m.project) + '</div>' : '') +
      '<div class="mc-meta">' + (m.ctx ? esc(m.ctx) + ' · ' : '') + esc(T.clockTime(m.at)) + '</div></div>';
  }
  function typing(since) {
    return '<div class="mc-m mgr"><div class="mc-b mc-typing" role="status" aria-label="Manager is typing"><i></i><i></i><i></i></div>' +
      '<div class="mc-meta">Manager is typing… · <span class="tick" data-start="' + esc(since) + '">' + esc(T.dur(T.elapsed(since))) + '</span></div></div>';
  }
  function projectChip(state) {
    var cur = ui.chatProj != null ? ui.chatProj : (T.filter.project || ''), prj = state.projects || [];
    return '<label class="mc-chip" title="Tell the manager which project this is about">' + BUBBLE +
      '<select data-act="mc-proj" aria-label="Project of this message"><option value="">No project</option>' + prj.map(function (p) {
        return '<option value="' + esc(p.id) + '"' + (p.id === cur ? ' selected' : '') + '>' + esc(p.name || p.id) + '</option>';
      }).join('') + '</select></label>';
  }
  function composer(state) {
    var rows = Math.max(1, Math.min(6, ui.chatDraft.split('\n').length));
    return '<div class="mc-comp">' + (ui.chatErr ? '<div class="ierr mc-note">' + esc(ui.chatErr) + '</div>' : '') +
      (ui.chatCli ? '<div class="icli mc-note">Tirith cannot send from this page. Run in a terminal:<code>' + esc(ui.chatCli) + '</code></div>' : '') +
      '<div class="mc-box"><textarea data-act="mc-text" rows="' + rows + '" maxlength="4000" placeholder="Message the manager…" aria-label="Message to the manager">' + esc(ui.chatDraft) + '</textarea>' +
      '<div class="mc-row">' + projectChip(state) + '<span class="mc-hint">Enter sends · Shift+Enter new line</span>' +
      '<button class="mc-send" data-act="mc-send" type="button" aria-label="Send" title="Send to the manager">' + SEND + '</button></div></div></div>';
  }

  /* the chat as HTML; `drawer` adds a close button */
  T.managerChat = function (state, drawer) {
    lastState = state;
    var list = messages(state), since = waitingSince(state), h;
    h = '<section class="mchat' + (drawer ? ' in-drawer' : '') + '"><div class="mc-head"><h2>Manager</h2>' +
      (drawer ? '<button class="mc-x" data-act="mc-close" type="button" aria-label="Close the chat">&times;</button>' : '') + '</div>' +
      '<div class="mc-log" id="mc-log">';
    h += list.length ? list.map(function (m) { return msg(m, state); }).join('') : T.empty('Write to the manager here. Its answers, and the answers to your Plan questions, show in this thread.');
    if (since) h += typing(since);
    return h + '</div>' + composer(state) + '</section>';
  };

  /* ---------- sending: the app writes the inbox, a browser opens the tirith:// link, else the CLI line is shown ---------- */
  function openLink(url) {
    var a = document.createElement('a'); a.href = url; a.style.display = 'none';
    document.body.appendChild(a); a.click(); document.body.removeChild(a);
  }
  function send() {
    var text = ui.chatDraft.trim(), st = lastState, L = st && st.launch, tauri = window.__TAURI__;
    if (!text) return;
    var project = ui.chatProj != null ? ui.chatProj : (T.filter.project || '');
    ui.chatErr = null; ui.chatCli = null;
    function sent() {
      ui.chatDraft = ''; ui.chatStick = true;
      ui.chatOut.push({ text: text, project: project, at: T.now() });
      T.render(); setTimeout(T.render, OUT_MAX + 500);
    }
    if (tauri && tauri.core && tauri.core.invoke) {
      tauri.core.invoke('inbox_add', { kind: 'chat', project: project, step: '', text: text }).then(sent, function (err) {
        ui.chatErr = 'Could not send: ' + err; T.render();
      });
      return;
    }
    if (L && L.token && L.available) {
      openLink('tirith://inbox?kind=chat&project=' + encodeURIComponent(project) + '&step=&text=' + encodeURIComponent(text) + '&token=' + encodeURIComponent(L.token));
      sent();
      return;
    }
    ui.chatCli = 'tirith inbox add --kind chat' + (project ? ' --project ' + shq(project) : '') + ' --text ' + shq(text);
    ui.chatDraft = ''; ui.chatStick = true;
    T.render();
  }

  /* ---------- the drawer and the header button ---------- */
  var view = document.getElementById('view'), btn = document.getElementById('mc-btn'), base = T.render;
  function inDrawer() { return T.filter.view !== 'overview'; }
  T.render = function (fstate) {
    if (fstate) lastState = fstate;  /* the filtered state; later calls without arguments reuse it */
    var log = document.getElementById('mc-log'), a = document.activeElement;
    var keep = log ? { top: log.scrollTop, bottom: log.scrollHeight - log.scrollTop - log.clientHeight < 48 } : null;
    var typingNow = a && a.getAttribute && a.getAttribute('data-act') === 'mc-text';
    var sel = typingNow ? [a.selectionStart, a.selectionEnd] : null;
    base.apply(this, arguments);
    if (!lastState) return;
    if (inDrawer() && ui.chatOpen) view.insertAdjacentHTML('beforeend', '<aside class="drawer mdrawer" aria-label="Chat with the manager">' + T.managerChat(lastState, true) + '</aside>');
    var nl = document.getElementById('mc-log');
    if (nl) nl.scrollTop = !keep || keep.bottom || ui.chatStick ? nl.scrollHeight : keep.top;
    ui.chatStick = false;
    if (sel) {
      var ta = document.querySelector('[data-act="mc-text"]');
      if (ta) { ta.focus(); try { ta.setSelectionRange(sel[0], sel[1]); } catch (e) {} }
    }
    if (btn) {
      btn.hidden = !inDrawer();
      btn.classList.toggle('on', ui.chatOpen);
      btn.classList.toggle('wait', T.chatPending(lastState));
    }
  };
  if (btn) btn.addEventListener('click', function () {
    ui.chatOpen = !ui.chatOpen; ui.chatStick = true; T.render();
    if (ui.chatOpen) { var ta = document.querySelector('[data-act="mc-text"]'); if (ta) ta.focus(); }
  });
  view.addEventListener('click', function (e) {
    var b = e.target.closest('[data-act^="mc-"]'); if (!b) return;
    var act = b.getAttribute('data-act');
    if (act === 'mc-send') send();
    else if (act === 'mc-close') { ui.chatOpen = false; T.render(); }
  });
  view.addEventListener('input', function (e) {
    var t = e.target, act = t.getAttribute && t.getAttribute('data-act');
    if (act === 'mc-text') { ui.chatDraft = t.value; t.rows = Math.max(1, Math.min(6, t.value.split('\n').length)); }
  });
  view.addEventListener('change', function (e) {
    var t = e.target; if (t.getAttribute && t.getAttribute('data-act') === 'mc-proj') ui.chatProj = t.value;
  });
  view.addEventListener('keydown', function (e) {
    var t = e.target; if (!t.getAttribute || t.getAttribute('data-act') !== 'mc-text') return;
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); ui.chatDraft = t.value; send(); }
    else if (e.key === 'Escape' && inDrawer()) { e.preventDefault(); e.stopPropagation(); ui.chatOpen = false; T.render(); }
  });
})();
