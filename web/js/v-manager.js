/* Chat with the manager: the right column of the Overview, and a drawer from the header in every other view.
   One thread, built only from state.inbox: the owner's chat messages (kind "chat"), the questions and finish
   requests of the Plan, and every answer to them (markdown, Run actions). The things that need the owner ("Your
   turn", v-needs.js) and the manager's plain messages (state.says) are manager messages in the same thread.
   docs/CONTRACT-inbox.md, "Chat". */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  ui.chatOpen = !!ui.chatOpen;       /* the drawer (other views) */
  ui.chatDraft = ui.chatDraft || ''; /* text typed so far */
  ui.chatProj = ui.chatProj == null ? null : ui.chatProj; /* project chip: null follows the sidebar filter */
  ui.chatOut = ui.chatOut || [];     /* sent from here, not yet in the state: {text, project, at} */
  ui.chatCli = ui.chatCli || null;   /* CLI line to copy (no way to send from here) */
  ui.chatErr = ui.chatErr || null;   /* why the send failed */
  ui.chatReplyTo = ui.chatReplyTo || null; /* id of the owner item the composer answers */
  ui.chatJump = ui.chatJump || 0;    /* index of the last item "Jump to next" went to */
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
    (state.says || []).forEach(function (s) {
      out.push({ side: 'mgr', text: s.text, at: s.at, ctx: 'Manager' + (s.project ? ' · ' + T.proj(s.project) : ''), project: s.project });
    });
    var items = T.turnItems ? T.turnItems(state) : [];
    items.forEach(function (it) {
      var w = it.w, ctx = 'Manager' + (w.project ? ' · ' + T.proj(w.project) : '');
      out.push({ side: 'mgr', text: w.text, at: w.since, ctx: ctx, project: w.project, item: it, open: !it.done });
      if (it.done) out.push({ side: 'me', text: T.turnAnswer(it.answer), at: it.answer.at || w.since, ctx: 'You', project: w.project, req: w.id });
    });
    var now = T.now();
    ui.chatOut = ui.chatOut.filter(function (o) {
      return now - o.at < OUT_MAX && !rows.some(function (r) { return r.kind === 'chat' && r.text === o.text && (T.ms(r.at) || 0) >= o.at - 2000; });
    });
    ui.chatOut.forEach(function (o) {
      out.push({ side: 'me', text: o.text, at: new Date(o.at).toISOString(), ctx: o.project ? T.proj(o.project) : '', local: true });
    });
    out.sort(function (a, b) { return (T.ms(a.at) || 0) - (T.ms(b.at) || 0); });
    var rest = out.filter(function (m) { return !m.open; }).slice(-80);   /* an item that waits for the owner is never cut off */
    return out.filter(function (m) { return m.open || rest.indexOf(m) >= 0; });
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
    if (m.item) return turnMsg(m, state);
    return '<div class="mc-m ' + m.side + (m.local ? ' local' : '') + '"><div class="mc-b' + (m.side === 'me' ? '' : ' md') + '">' +
      (m.side === 'me' ? esc(m.text).replace(/\n/g, '<br>') : T.md(m.text)) + '</div>' +
      (m.actions && m.actions.length ? '<div class="mc-acts">' + T.actions(m.actions, state, m.project) + '</div>' : '') +
      '<div class="mc-meta">' + (m.ctx ? esc(m.ctx) + ' · ' : '') + esc(T.clockTime(m.at)) + '</div></div>';
  }
  /* a message that needs the owner (open) or needed it (answered): the text, then options and Run actions */
  function turnMsg(m, state) {
    var w = m.item.w, replying = ui.chatReplyTo === w.id;
    return '<div class="mc-m mgr turn' + (m.open ? ' wait' : '') + (replying ? ' replying' : '') + '" data-wait="' + esc(w.id) + '"><div class="mc-b md">' + T.md(m.text) + '</div>' +
      T.turnBlock(m.item, state) +
      '<div class="mc-meta">' + (m.open ? '<b class="mc-await">Waits for you</b> · ' : '') + esc(m.ctx) + ' · ' + esc(T.clockTime(m.at)) + '</div></div>';
  }
  function typing(since) {
    return '<div class="mc-m mgr"><div class="mc-b mc-typing" role="status" aria-label="Manager is typing"><i></i><i></i><i></i></div>' +
      '<div class="mc-meta">Manager is typing… · <span class="tick" data-start="' + esc(since) + '">' + esc(T.dur(T.elapsed(since))) + '</span></div></div>';
  }
  function projectChip(state) {
    var cur = ui.chatProj != null ? ui.chatProj : (T.filter.project || ''), prj = state.projects || [];
    return '<label class="mc-chip" title="Tell the manager which project this is about">' + BUBBLE +
      '<select data-act="mc-proj" data-keep="mc-proj" aria-label="Project of this message"><option value="">No project</option>' + prj.map(function (p) {
        return '<option value="' + esc(p.id) + '"' + (p.id === cur ? ' selected' : '') + '>' + esc(p.name || p.id) + '</option>';
      }).join('') + '</select></label>';
  }
  function composer(state) {
    var rows = Math.max(1, Math.min(6, ui.chatDraft.split('\n').length));
    var target = ui.chatReplyTo && T.turnFind ? T.turnFind(ui.chatReplyTo) : null;
    if (ui.chatReplyTo && !target) ui.chatReplyTo = null;   /* answered meanwhile */
    return '<div class="mc-comp">' + (target ? '<div class="mc-to"><span>Replying to <b>' + esc(plainHead(target.text)) + '</b></span>' +
      '<button data-act="mc-unreply" type="button" aria-label="Reply to nothing in particular" title="Stop replying to it">&times;</button></div>' : '') + (ui.chatErr ? '<div class="ierr mc-note">' + esc(ui.chatErr) + '</div>' : '') +
      (ui.chatCli ? '<div class="icli mc-note">Tirith cannot send from this page. Run in a terminal:<code>' + esc(ui.chatCli) + '</code></div>' : '') +
      '<div class="mc-box"><textarea data-act="mc-text" data-keep="mc-text" rows="' + rows + '" maxlength="4000" placeholder="' + (target ? 'Your answer…' : 'Message the manager…') + '" aria-label="Message to the manager">' + esc(ui.chatDraft) + '</textarea>' +
      '<div class="mc-row">' + projectChip(state) + '<span class="mc-hint">Enter sends · Shift+Enter new line</span>' +
      '<button class="mc-send" data-act="mc-send" type="button" aria-label="Send" title="Send to the manager">' + SEND + '</button></div></div></div>';
  }

  function plainHead(t) {
    t = String(t || '').replace(/[`*_#>\[\]]/g, '').replace(/\s+/g, ' ').trim();
    return t.length > 48 ? t.slice(0, 47) + '…' : t;
  }

  /* the chat as HTML; `drawer` adds a close button */
  T.managerChat = function (state, drawer) {
    lastState = state;
    var list = messages(state), since = waitingSince(state), h;
    var open = T.turnOpen ? T.turnOpen(state) : 0;
    h = '<section class="mchat' + (drawer ? ' in-drawer' : '') + '"><div class="mc-head"><h2>Manager' +
      (open ? ' <span class="icount" title="Messages that wait for you">' + open + '</span>' : '') + '</h2>' +
      (open ? '<button class="mc-jump" data-act="mc-jump" type="button">' + (open > 1 ? 'Jump to next' : 'Jump to it') + '</button>' : '') +
      (drawer ? '<button class="mc-x" data-act="mc-close" type="button" aria-label="Close the chat">&times;</button>' : '') + '</div>' +
      '<div class="mc-log" id="mc-log">';
    h += list.length ? list.map(function (m) { return msg(m, state); }).join('') : T.empty('Write to the manager here. Its answers, and what it needs from you, show in this thread.');
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
    if (ui.chatReplyTo) {   /* the answer to an item that waits for the owner: a reply, not a new chat message */
      var to = ui.chatReplyTo;
      ui.chatReplyTo = null; ui.chatDraft = ''; ui.chatStick = true;
      if (T.turnReply(to, text)) return;
      ui.chatDraft = text;
    }
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

  /* scroll to the next message that waits for the owner, round and round */
  function jump() {
    var els = document.querySelectorAll('#mc-log .mc-m.turn.wait'); if (!els.length) return;
    ui.chatJump = ui.chatJump >= els.length ? 0 : ui.chatJump;
    var el = els[ui.chatJump]; ui.chatJump = (ui.chatJump + 1) % els.length;
    el.scrollIntoView({ block: 'center', behavior: 'smooth' });
    el.classList.remove('flash'); void el.offsetWidth; el.classList.add('flash');
  }

  /* ---------- the drawer and the header button ---------- */
  var view = document.getElementById('view'), btn = document.getElementById('mc-btn'), base = T.render;
  function inDrawer() { return T.filter.view !== 'overview'; }
  T.render = function (fstate) {
    if (fstate) lastState = fstate;  /* the filtered state; later calls without arguments reuse it */
    var log = document.getElementById('mc-log'), a = document.activeElement;
    var keep = log ? { top: log.scrollTop, bottom: log.scrollHeight - log.scrollTop - log.clientHeight < 48 } : null;
    var kept = T.keepCapture();
    base.apply(this, arguments);
    if (!lastState) return;
    if (inDrawer() && ui.chatOpen) view.insertAdjacentHTML('beforeend', '<aside class="drawer mdrawer" aria-label="Chat with the manager">' + T.managerChat(lastState, true) + '</aside>');
    var nl = document.getElementById('mc-log');
    if (nl) nl.scrollTop = !keep || keep.bottom || ui.chatStick ? nl.scrollHeight : keep.top;
    ui.chatStick = false;
    T.keepRestore(kept);
    if (btn) {
      btn.hidden = !inDrawer();
      btn.classList.toggle('on', ui.chatOpen);
      btn.classList.toggle('wait', T.chatPending(lastState) || (T.turnOpen ? T.turnOpen(lastState) > 0 : false));
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
    else if (act === 'mc-reply') {
      ui.chatReplyTo = b.getAttribute('data-key'); T.render();
      var ta = document.querySelector('[data-act="mc-text"]'); if (ta) ta.focus();
    } else if (act === 'mc-unreply') { ui.chatReplyTo = null; T.render(); }
    else if (act === 'mc-jump') jump();
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
