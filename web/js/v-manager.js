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
  ui.chatPics = ui.chatPics || [];   /* images waiting in the composer: {name, type, size, url, w, h, b64} */
  ui.chatPicUrl = ui.chatPicUrl || {}; /* saved path -> data URL, for thumbnails the page cannot open by path */
  ui.chatStick = true;               /* scroll to the newest message after the next render */
  var lastState = null, OUT_MAX = 30000, PENDING_MAX = 6 * 3600 * 1000, KINDS = { chat: 1, ask: 1, finish: 1 };

  var SEND = '<svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true"><path d="M8 13V3.2M3.8 7.2L8 3l4.2 4.2" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>';
  var BUBBLE = '<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M3 3h10a1 1 0 011 1v6a1 1 0 01-1 1H7.5L4.6 13.6V11H3a1 1 0 01-1-1V4a1 1 0 011-1z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/></svg>';

  var PIC_MAX = 6, PIC_BYTES = 10 * 1024 * 1024, PIC_TYPES = { 'image/png': 1, 'image/jpeg': 1, 'image/gif': 1, 'image/webp': 1 };
  var CLIP = '<svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true"><path d="M13.2 7.6L8 12.8a3 3 0 01-4.2-4.2l5.5-5.5a2 2 0 012.9 2.9L6.7 11.4a1 1 0 01-1.4-1.4l4.8-4.8" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>';

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
      out.push({ side: 'me', text: r.kind === 'finish' ? 'Finish this step' : r.text, at: r.at, ctx: ctx, req: r.id, project: r.project, pics: r.attachments });
      (r.answers || []).forEach(function (a) {
        out.push({ side: 'mgr', text: a.text, at: a.at, ctx: who(a) + (a.auto ? ', automatic' : ''), actions: a.actions, req: r.id, project: r.project, pics: a.attachments });
      });
    });
    var now = T.now();
    ui.chatOut = ui.chatOut.filter(function (o) {
      return now - o.at < OUT_MAX && !rows.some(function (r) { return r.kind === 'chat' && r.text === o.text && (r.attachments || []).length === (o.pics || []).length && (T.ms(r.at) || 0) >= o.at - 2000; });
    });
    ui.chatOut.forEach(function (o) {
      out.push({ side: 'me', text: o.text, at: new Date(o.at).toISOString(), ctx: o.project ? T.proj(o.project) : '', local: true, pics: o.pics });
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

  function picSrc(a) { return ui.chatPicUrl[a.path] || ('file://' + encodeURI(a.path).replace(/#/g, '%23').replace(/\?/g, '%3F')); }
  function thumbs(pics) {
    return pics && pics.length ? '<div class="mc-pics">' + pics.map(function (a, i) {
      var src = picSrc(a), alt = 'Image ' + (i + 1) + (a.width ? ', ' + a.width + '×' + a.height : '');
      return '<button class="mc-pic" type="button" data-act="mc-zoom" data-src="' + esc(src) + '" title="Open full size" aria-label="' + esc(alt) + '"><img src="' + esc(src) + '" alt="' + esc(alt) + '" loading="lazy"></button>';
    }).join('') + '</div>' : '';
  }
  function msg(m, state) {
    var body = m.side === 'me' ? esc(m.text).replace(/\n/g, '<br>') : T.md(m.text);
    return '<div class="mc-m ' + m.side + (m.local ? ' local' : '') + '">' + thumbs(m.pics) + (m.text ? '<div class="mc-b' + (m.side === 'me' ? '' : ' md') + '">' +
      body + '</div>' : '') +
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
      '<select data-act="mc-proj" data-keep="mc-proj" aria-label="Project of this message"><option value="">No project</option>' + prj.map(function (p) {
        return '<option value="' + esc(p.id) + '"' + (p.id === cur ? ' selected' : '') + '>' + esc(p.name || p.id) + '</option>';
      }).join('') + '</select></label>';
  }
  function picStrip() {
    return ui.chatPics.length ? '<div class="mc-strip">' + ui.chatPics.map(function (p, i) {
      return '<span class="mc-th"><img src="' + esc(p.url) + '" alt="' + esc(p.name) + '"><button type="button" data-act="mc-unpic" data-i="' + i + '" aria-label="Remove ' + esc(p.name) + '" title="Remove">&times;</button></span>';
    }).join('') + '</div>' : '';
  }
  function composer(state) {
    var rows = Math.max(1, Math.min(6, ui.chatDraft.split('\n').length));
    return '<div class="mc-comp">' + (ui.chatErr ? '<div class="ierr mc-note">' + esc(ui.chatErr) + '</div>' : '') +
      (ui.chatCli ? '<div class="icli mc-note">Tirith cannot send from this page. Run in a terminal:<code>' + esc(ui.chatCli) + '</code></div>' : '') +
      '<div class="mc-box">' + picStrip() + '<textarea data-act="mc-text" data-keep="mc-text" rows="' + rows + '" maxlength="4000" placeholder="Message the manager…" aria-label="Message to the manager">' + esc(ui.chatDraft) + '</textarea>' +
      '<div class="mc-row"><button class="mc-clip" type="button" data-act="mc-attach" aria-label="Attach images" title="Attach images (or paste, or drop them here)">' + CLIP + '</button>' +
      '<input type="file" data-act="mc-file" accept="image/png,image/jpeg,image/gif,image/webp" multiple hidden>' + projectChip(state) + '<span class="mc-hint">Enter sends · Shift+Enter new line</span>' +
      '<button class="mc-send" data-act="mc-send" type="button" aria-label="Send" title="Send to the manager">' + SEND + '</button></div></div></div>';
  }

  /* who answers: a live terminal session, or the on-call manager Tirith starts when the owner writes */
  function presence(state) {
    var p = state.manager && state.manager.presence;
    if (!p) return '';
    var txt = p.working ? 'Manager is working on your message…' : p.mode === 'live' ? 'Manager: live session' : 'Manager: on call (starts when you write)';
    return '<span class="mc-presence ' + (p.working ? 'work' : p.mode) + '" role="status">' + esc(txt) + '</span>';
  }

  /* the chat as HTML; `drawer` adds a close button */
  T.managerChat = function (state, drawer) {
    lastState = state;
    var list = messages(state), since = waitingSince(state), h;
    h = '<section class="mchat' + (drawer ? ' in-drawer' : '') + '"><div class="mc-head"><h2>Manager</h2>' + presence(state) +
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
  function hasTauri() { var t = window.__TAURI__; return !!(t && t.core && t.core.invoke); }
  function readPic(file) {
    return new Promise(function (resolve) {
      var r = new FileReader();
      r.onload = function () {
        var url = String(r.result), im = new Image();
        im.onload = function () { resolve({ name: file.name || 'image', type: file.type, size: file.size, url: url, w: im.naturalWidth, h: im.naturalHeight, b64: url.slice(url.indexOf(',') + 1) }); };
        im.onerror = function () { resolve(null); };
        im.src = url;
      };
      r.onerror = function () { resolve(null); };
      r.readAsDataURL(file);
    });
  }
  /* add files to the composer; a page opened as a file cannot write the attachments folder, so it says so */
  function addPics(files) {
    files = Array.prototype.filter.call(files || [], function (f) { return /^image\//.test(f.type); });
    if (!files.length) return;
    ui.chatErr = null;
    if (!hasTauri()) { ui.chatErr = 'Attach images in the desktop app. A page opened as a file cannot save them for the manager.'; T.render(); return; }
    var bad = null;
    files = files.filter(function (f) {
      if (!PIC_TYPES[f.type]) { bad = f.name + ' is not a PNG, JPEG, GIF or WebP image.'; return false; }
      if (f.size > PIC_BYTES) { bad = f.name + ' is over 10 MB.'; return false; }
      return true;
    });
    if (ui.chatPics.length + files.length > PIC_MAX) { bad = 'At most ' + PIC_MAX + ' images per message.'; files = files.slice(0, PIC_MAX - ui.chatPics.length); }
    if (bad) ui.chatErr = bad;
    Promise.all(files.map(readPic)).then(function (got) {
      got.forEach(function (p) { if (p && ui.chatPics.length < PIC_MAX) ui.chatPics.push(p); });
      T.render();
    });
    if (bad) T.render();
  }
  function newId() {
    var b = new Uint8Array(4);
    if (window.crypto && crypto.getRandomValues) crypto.getRandomValues(b); else for (var i = 0; i < 4; i++) b[i] = Math.random() * 256;
    return 'in-' + Array.prototype.map.call(b, function (x) { return ('0' + x.toString(16)).slice(-2); }).join('');
  }
  /* the app saves each image under attachments/<id>/ first, then adds the chat row that lists them */
  function sendWithPics(text, project, pics) {
    var inv = window.__TAURI__.core.invoke, id = newId();
    ui.chatErr = null; ui.chatCli = null;
    return pics.reduce(function (chain, p) {
      return chain.then(function (acc) {
        return inv('attachment_save', { messageId: id, name: p.name, bytesBase64: p.b64 }).then(function (r) {
          ui.chatPicUrl[r.path] = p.url;
          acc.push({ path: r.path, type: r.type, size: r.size, width: p.w, height: p.h });
          return acc;
        });
      });
    }, Promise.resolve([])).then(function (atts) {
      return inv('inbox_add', { kind: 'chat', project: project, step: '', text: text, id: id, attachments: atts }).then(function () { return atts; });
    }).then(function (atts) {
      ui.chatDraft = ''; ui.chatPics = []; ui.chatStick = true;
      ui.chatOut.push({ text: text, project: project, at: T.now(), pics: atts });
      T.render(); setTimeout(T.render, OUT_MAX + 500);
    }, function (err) { ui.chatErr = 'Could not send: ' + err; T.render(); });
  }
  function send() {
    var text = ui.chatDraft.trim(), st = lastState, L = st && st.launch, tauri = window.__TAURI__;
    if (ui.chatPics.length) {
      if (hasTauri()) sendWithPics(text, ui.chatProj != null ? ui.chatProj : (T.filter.project || ''), ui.chatPics.slice());
      return;
    }
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
    else if (act === 'mc-attach' && !hasTauri()) { ui.chatErr = 'Attach images in the desktop app. A page opened as a file cannot save them for the manager.'; T.render(); }
    else if (act === 'mc-attach') { var f =view.querySelector('[data-act="mc-file"]'); if (f) f.click(); }
    else if (act === 'mc-unpic') { ui.chatPics.splice(+b.getAttribute('data-i'), 1); T.render(); }
    else if (act === 'mc-zoom') lightbox(b.getAttribute('data-src'));
  });
  /* full size: one overlay on the body, closed by a click or Esc */
  function lightbox(src) {
    var box = document.createElement('div');
    box.className = 'mc-lb'; box.setAttribute('role', 'dialog'); box.setAttribute('aria-label', 'Image');
    box.innerHTML = '<img src="' + esc(src) + '" alt="Attached image"><button type="button" class="mc-lbx" aria-label="Close">&times;</button>';
    function close() { box.remove(); document.removeEventListener('keydown', onKey, true); }
    function onKey(e) { if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); close(); } }
    box.addEventListener('click', close);
    document.addEventListener('keydown', onKey, true);
    document.body.appendChild(box);
    box.querySelector('button').focus();
  }
  view.addEventListener('paste', function (e) {
    var t = e.target; if (!t.closest || !t.closest('.mc-comp')) return;
    var files = Array.prototype.filter.call((e.clipboardData && e.clipboardData.files) || [], function (f) { return /^image\//.test(f.type); });
    if (files.length) { e.preventDefault(); addPics(files); }
  });
  view.addEventListener('dragover', function (e) {
    var c = e.target.closest && e.target.closest('.mchat');
    if (c && e.dataTransfer && Array.prototype.indexOf.call(e.dataTransfer.types || [], 'Files') >= 0) { e.preventDefault(); c.classList.add('drop'); }
  });
  view.addEventListener('dragleave', function (e) { var c = e.target.closest && e.target.closest('.mchat'); if (c && !c.contains(e.relatedTarget)) c.classList.remove('drop'); });
  view.addEventListener('drop', function (e) {
    var c = e.target.closest && e.target.closest('.mchat'); if (!c) return;
    c.classList.remove('drop');
    if (e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files.length) { e.preventDefault(); addPics(e.dataTransfer.files); }
  });
  view.addEventListener('input', function (e) {
    var t = e.target, act = t.getAttribute && t.getAttribute('data-act');
    if (act === 'mc-text') { ui.chatDraft = t.value; t.rows = Math.max(1, Math.min(6, t.value.split('\n').length)); }
  });
  view.addEventListener('change', function (e) {
    var t = e.target, act = t.getAttribute && t.getAttribute('data-act');
    if (act === 'mc-proj') ui.chatProj = t.value;
    else if (act === 'mc-file') { addPics(t.files); t.value = ''; }
  });
  view.addEventListener('keydown', function (e) {
    var t = e.target; if (!t.getAttribute || t.getAttribute('data-act') !== 'mc-text') return;
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); ui.chatDraft = t.value; send(); }
    else if (e.key === 'Escape' && inDrawer()) { e.preventDefault(); e.stopPropagation(); ui.chatOpen = false; T.render(); }
  });
})();
