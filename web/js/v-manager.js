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
  ui.chatOut = ui.chatOut || [];     /* sent from here, not yet in the state: {cid, id, text, project, at, state: "sending"|"sent"|"failed", err, pics} */
  ui.chatCli = ui.chatCli || null;   /* CLI line to copy (no way to send from here) */
  ui.chatErr = ui.chatErr || null;   /* why the send failed */
  ui.chatPics = ui.chatPics || [];   /* files waiting in the composer: {name, type, size, url, w, h, b64}; url only for a picture */
  ui.chatDrag = false;               /* files are being dragged over the chat */
  ui.chatPicUrl = ui.chatPicUrl || {}; /* saved path -> data URL, for thumbnails the page cannot open by path */
  ui.chatReplyTo = ui.chatReplyTo || null; /* id of the owner item the composer answers */
  ui.chatLast = ui.chatLast || '';   /* the last message sent from here (Up arrow brings it back) */
  ui.chatJump = ui.chatJump || 0;    /* index of the last item "Jump to next" went to */
  ui.chatStick = true;               /* scroll to the newest message after the next render */
  var lastState = null, SEND_WAIT = 30000, PENDING_MAX = 6 * 3600 * 1000, KINDS = { chat: 1, ask: 1, finish: 1, 'autopilot-proposal': 1 };

  /* drafts and unconfirmed sends survive a refresh: localStorage, keyed by project, never trusted to work */
  var DRAFT_KEY = 'tirith.chat.draft.', UNSENT_KEY = 'tirith.chat.unsent';
  function store(op, key, val) {
    try {
      var ls = window.localStorage;
      if (op === 'get') return ls.getItem(key);
      if (val === '' || val == null) ls.removeItem(key); else ls.setItem(key, val);
    } catch (e) { /* private window or blocked storage: the draft just does not survive */ }
    return null;
  }
  function curProject() { return ui.chatProj != null ? ui.chatProj : (T.filter.project || ''); }
  function saveDraft() { store('set', DRAFT_KEY + ui.chatDraftProj, ui.chatDraft); }
  function saveUnsent() {
    store('set', UNSENT_KEY, ui.chatOut.length ? JSON.stringify(ui.chatOut.map(function (o) { return { cid: o.cid, id: o.id, text: o.text, project: o.project, at: o.at }; })) : '');
  }
  /* the project of the composer changed (chip or sidebar): park its draft and bring the other one */
  function syncDraftProject() {
    var p = curProject();
    if (ui.chatDraftProj === p) return;
    if (ui.chatDraftProj != null) saveDraft();
    ui.chatDraftProj = p;
    ui.chatDraft = store('get', DRAFT_KEY + p) || '';
  }

  var SEND = '<svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true"><path d="M8 13V3.2M3.8 7.2L8 3l4.2 4.2" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/></svg>';
  var BUBBLE = '<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M3 3h10a1 1 0 011 1v6a1 1 0 01-1 1H7.5L4.6 13.6V11H3a1 1 0 01-1-1V4a1 1 0 011-1z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/></svg>';

  /* any file can go with a message; these four types show as pictures, the rest as a chip with the name */
  var PIC_MAX = 6, PIC_BYTES = 10 * 1024 * 1024, PIC_TYPES = { 'image/png': 1, 'image/jpeg': 1, 'image/gif': 1, 'image/webp': 1 };
  var NO_SAVE = 'Attach files in the desktop app. A page opened as a file cannot save them for the manager.';
  var DOC = '<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M4 1.8h5l3 3v9.4H4z M9 1.8v3h3" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/></svg>';
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
  function sessionBusy(state) {
    var p = state.manager && state.manager.presence;
    return !!(p && p.session && p.session.busy);
  }
  function inTurn(state, id) {
    return ((state.manager && state.manager.turns) || []).some(function (t) { return (t.turn_requests || []).indexOf(id) >= 0; });
  }
  function messages(state) {
    var out = [], rows = state.inbox || [], busy = sessionBusy(state);
    rows.forEach(function (r) {
      if (!KINDS[r.kind]) return;
      var ctx = ctxOf(r);
      if (r.kind === 'autopilot-proposal') {   /* Tirith's own request to the manager: one short line, not an owner message */
        out.push({ side: 'mgr', text: String(r.text || '').split('\n')[0], at: r.at, ctx: 'Autopilot' + (r.project ? ' · ' + T.proj(r.project) : ''), req: r.id, project: r.project });
      } else {
        out.push({ side: 'me', text: r.kind === 'finish' ? 'Finish this step' : r.text, at: r.at, ctx: ctx, req: r.id, project: r.project, pics: r.attachments,
          queued: busy && r.kind === 'chat' && pending(r) && !inTurn(state, r.id) });
      }
      var run = T.liveForRequest ? T.liveForRequest(state, r.id) : null;   /* what the manager does for it, like the CLI */
      if (run) out.push({ side: 'live', row: run, at: new Date((T.ms(r.at) || 0) + 1).toISOString(), req: r.id });
      (r.answers || []).forEach(function (a) {
        out.push({ side: 'mgr', text: a.text, at: a.at, ctx: who(a) + (a.auto ? ', automatic' : ''), actions: a.actions, req: r.id, project: r.project, pics: a.attachments });
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
    var before = ui.chatOut.length;
    ui.chatOut = ui.chatOut.filter(function (o) {
      var seen = rows.some(function (r) { return r.cid === o.cid; });
      if (seen) {
        clearTimeout(o.timer);
        if (o.restored && ui.chatDraft === o.text) { ui.chatDraft = ''; saveDraft(); }  /* it arrived after all */
      }
      return !seen;
    });
    if (ui.chatOut.length !== before) saveUnsent();
    ui.chatOut.forEach(function (o) {
      var saved = (o.pics || []).map(function (p) { return p.saved; }).filter(Boolean);
      out.push({ side: 'me', text: o.text, at: new Date(o.at).toISOString(), ctx: o.project ? T.proj(o.project) : '', local: o.state, cid: o.cid, err: o.err, unconfirmed: o.unconfirmed, pics: saved });
    });
    out.sort(function (a, b) { return (T.ms(a.at) || 0) - (T.ms(b.at) || 0); });
    var rest = out.filter(function (m) { return !m.open; }).slice(-80);   /* an item that waits for the owner is never cut off */
    return out.filter(function (m) { return m.open || rest.indexOf(m) >= 0; });
  }
  /* ISO time of the oldest message the manager still owes an answer to, or null */
  function waitingSince(state) {
    var ats = (state.inbox || []).filter(pending).map(function (r) { return r.at; });
    ui.chatOut.forEach(function (o) { if (o.state === 'sending' || o.state === 'sent') ats.push(new Date(o.at).toISOString()); });
    ats.sort(function (a, b) { return (T.ms(a) || 0) - (T.ms(b) || 0); });
    return ats[0] || null;
  }
  T.chatPending = function (state) { return !!waitingSince(state || lastState || {}); };

  function picSrc(a) { return ui.chatPicUrl[a.path] || ('file://' + encodeURI(a.path).replace(/#/g, '%23').replace(/\?/g, '%3F')); }
  function size(n) { return n < 1024 ? n + ' B' : n < 1024 * 1024 ? Math.round(n / 1024) + ' KB' : (n / 1048576).toFixed(1) + ' MB'; }
  /* a file that is not a picture: its name and size; `x` adds the remove button of the composer */
  function fileChip(name, bytes, title, x) {
    return '<span class="mc-file" title="' + esc(title) + '">' + DOC + '<span class="mc-fname">' + esc(name) + '</span><span class="mc-fsize">' + esc(size(bytes || 0)) + '</span>' + (x || '') + '</span>';
  }
  function thumbs(pics) {
    if (!pics || !pics.length) return '';
    var shots = pics.filter(function (a) { return PIC_TYPES[a.type]; }), files = pics.filter(function (a) { return !PIC_TYPES[a.type]; });
    return '<div class="mc-pics">' + shots.map(function (a, i) {
      var src = picSrc(a), alt = 'Image ' + (i + 1) + (a.width ? ', ' + a.width + '×' + a.height : '');
      return '<button class="mc-pic" type="button" data-act="mc-zoom" data-src="' + esc(src) + '" title="Open full size" aria-label="' + esc(alt) + '"><img src="' + esc(src) + '" alt="' + esc(alt) + '" loading="lazy"></button>';
    }).join('') + files.map(function (a) {
      return fileChip(a.name || String(a.path).split('/').pop(), a.size, a.path);
    }).join('') + '</div>';
  }
  T.chatThumbs = thumbs;
  function msg(m, state) {
    if (m.item) return turnMsg(m, state);
    if (m.side === 'live') return T.liveBlock(m.row, state);
    var body = m.side === 'me' ? esc(m.text).replace(/\n/g, '<br>') : T.md(m.text, T.chatOpts(state, m.project));
    var meta = (m.ctx ? esc(m.ctx) + ' · ' : '') + esc(T.clockTime(m.at));
    if (m.queued) meta = '<span class="mc-wait" title="The manager is working on an earlier message; this one goes into the same session when that turn ends">Queued</span> · ' + meta;
    if (m.local === 'sending') meta = 'Sending… · ' + meta;
    else if (m.local === 'sent') meta = '<span class="mc-wait" title="Written to the inbox; waiting for the manager to pick it up">Delivered ✓</span> · ' + meta;
    else if (m.local === 'failed') meta = '<b class="mc-bad">' + (m.unconfirmed ? 'Not confirmed' : 'Not sent') + '</b> · <button type="button" class="mc-lnk" data-act="mc-retry" data-cid="' + esc(m.cid) + '">Retry</button> · ' +
      '<button type="button" class="mc-lnk" data-act="mc-copy" data-cid="' + esc(m.cid) + '">Copy text</button>' + (m.err ? '<br><span class="mc-why">' + esc(m.err) + '</span>' : '');
    return '<div class="mc-m ' + m.side + (m.local ? ' local ' + m.local : '') + '"' + (m.cid ? ' data-cid="' + esc(m.cid) + '"' : '') + '>' + thumbs(m.pics) + (m.text ? '<div class="mc-b' + (m.side === 'me' ? '' : ' md') + '">' +
      body + '</div>' : '') +
      (m.actions && m.actions.length ? '<div class="mc-acts">' + T.actions(m.actions, state, m.project) + '</div>' : '') +
      '<div class="mc-meta">' + meta + '</div></div>';
  }
  /* a message that needs the owner (open) or needed it (answered): the text, then options and Run actions */
  function turnMsg(m, state) {
    var w = m.item.w, replying = ui.chatReplyTo === w.id;
    return '<div class="mc-m mgr turn' + (m.open ? ' wait' : '') + (replying ? ' replying' : '') + '" data-wait="' + esc(w.id) + '"><div class="mc-b md">' + T.md(m.text, T.chatOpts(state, m.project)) + '</div>' +
      (w.reason_tag ? '<div class="nacts"><span class="nchip nreason">' + esc(w.reason_tag) + '</span></div>' : '') +
      T.turnBlock(m.item, state) +
      '<div class="mc-meta">' + (m.open ? '<b class="mc-await">Waits for you</b> · ' : '') + esc(m.ctx) + ' · ' + esc(T.clockTime(m.at)) + '</div></div>';
  }
  /* What is really happening to the oldest pending request: the manager is writing, or nobody is. */
  var LIVE_FRESH = 5 * 60 * 1000, LATE = 30 * 60 * 1000;
  function writing(r, state) {
    var live = state.manager && state.manager.live, beat = live ? T.ms(live.heartbeat) : null, at = T.ms(r.at);
    if (beat != null && at != null && beat > at && T.now() - beat < LIVE_FRESH) return true;   /* a live session saw it */
    return (state.agents || []).some(function (a) { return a.request_id === r.id && a.status === 'working'; });   /* a headless lane has it */
  }
  /* the result line when an action the manager gave was run after its last answer: "✗ Install the agent shims failed" */
  function reopened(r) {
    var runs = r.action_runs || [], last = null, lastAns = '';
    (r.answers || []).forEach(function (a) { if (a.by === 'manager' && String(a.at || '') > lastAns) lastAns = String(a.at); });
    runs.forEach(function (x) { if (String(x.at || '') > lastAns && (!last || String(x.at) > String(last.at))) last = x; });
    if (!last) return null;
    var label = last.action_id;
    (r.answers || []).forEach(function (a) { (a.actions || []).forEach(function (x) { if (x.id === last.action_id && x.label) label = x.label; }); });
    return { at: last.at, ok: last.status === 'passed', text: (last.status === 'passed' ? '✓ ' : '✗ ') + label + (last.status === 'passed' ? ' passed' : ' failed') };
  }
  T.waitAge = function (sec) { return sec < 60 ? Math.max(0, sec) + ' s' : sec < 3600 ? Math.floor(sec / 60) + ' min' : Math.floor(sec / 3600) + ' h ' + Math.floor((sec % 3600) / 60) + ' min'; };
  /* "Waiting for the manager · 12 min", then "The manager has not answered yet" (amber class "late") after 30 min */
  T.waitLine = function (startIso) {
    var ms = T.now() - (T.ms(startIso) || T.now()), late = ms >= LATE;
    return { late: late, text: late ? 'The manager has not answered yet · ' + T.waitAge(Math.floor(ms / 1000)) : 'Waiting for the manager · ' + T.waitAge(Math.floor(ms / 1000)) };
  };
  function waitingLine(since) {
    var w = T.waitLine(since);
    return '<div class="mc-wait' + (w.late ? ' late' : '') + '" role="status" data-wait="' + esc(since) + '">' + esc(w.text) + '</div>';
  }
  function typing(since) {
    return '<div class="mc-m mgr"><div class="mc-b mc-typing" role="status" aria-label="Manager is typing"><i></i><i></i><i></i></div>' +
      '<div class="mc-meta">Manager is typing… · <span class="tick" data-start="' + esc(since) + '">' + esc(T.dur(T.elapsed(since))) + '</span></div></div>';
  }
  /* the line under the newest message; never "typing" unless something is writing */
  function status(state) {
    var rows = (state.inbox || []).filter(pending), i, r, re;
    for (i = 0; i < rows.length; i++) {
      if (T.liveWorking && T.liveWorking(T.liveForRequest(state, rows[i].id))) return '';   /* its live block shows the work */
      if (writing(rows[i], state)) return typing(rows[i].at);
    }
    r = rows[0];
    if (r) {
      re = reopened(r);
      if (re) return '<div class="mc-reason' + (re.ok ? '' : ' bad') + '">' + esc(re.text) + '</div>' + waitingLine(re.at);
      return waitingLine(r.at);
    }
    var since = waitingSince(state);   /* only a message not yet in the state */
    return since ? waitingLine(since) : '';
  }
  function projectChip(state) {
    var cur = curProject(), prj = state.projects || [];
    return '<label class="mc-chip" title="Tell the manager which project this is about">' + BUBBLE +
      '<select data-act="mc-proj" data-keep="mc-proj" aria-label="Project of this message"><option value="">No project</option>' + prj.map(function (p) {
        return '<option value="' + esc(p.id) + '"' + (p.id === cur ? ' selected' : '') + '>' + esc(p.name || p.id) + '</option>';
      }).join('') + '</select></label>';
  }
  function picStrip() {
    return ui.chatPics.length ? '<div class="mc-strip">' + ui.chatPics.map(function (p, i) {
      var x = '<button type="button" data-act="mc-unpic" data-i="' + i + '" aria-label="Remove ' + esc(p.name) + '" title="Remove">&times;</button>';
      if (!p.url) return fileChip(p.name, p.size, p.name, x);
      return '<span class="mc-th"><img src="' + esc(p.url) + '" alt="' + esc(p.name) + '" title="' + esc(p.name) + '">' + x + '</span>';
    }).join('') + '</div>' : '';
  }
  T.chatStrip = picStrip;
  function composer(state) {
    var rows = Math.max(1, Math.min(6, ui.chatDraft.split('\n').length));
    var target = ui.chatReplyTo && T.turnFind ? T.turnFind(ui.chatReplyTo) : null;
    if (ui.chatReplyTo && !target) ui.chatReplyTo = null;   /* answered meanwhile */
    return '<div class="mc-comp">' + (target ? '<div class="mc-to"><span>Replying to <b>' + esc(plainHead(target.text)) + '</b></span>' +
      '<button data-act="mc-unreply" type="button" aria-label="Reply to nothing in particular" title="Stop replying to it">&times;</button></div>' : '') + (ui.chatErr ? '<div class="ierr mc-note">' + esc(ui.chatErr) + '</div>' : '') +
      (ui.chatCli ? '<div class="icli mc-note">Tirith cannot send from this page. Run in a terminal:<code>' + esc(ui.chatCli) + '</code></div>' : '') +
      '<div class="mc-box">' + picStrip() + '<textarea data-act="mc-text" data-keep="mc-text" rows="' + rows + '" maxlength="4000" placeholder="' + (target ? 'Your answer…' : 'Message the manager…') + '" aria-label="Message to the manager">' + esc(ui.chatDraft) + '</textarea>' +
      '<div class="mc-row"><button class="mc-clip" type="button" data-act="mc-attach" aria-label="Attach files" title="Attach images or files (or paste, or drop them here)">' + CLIP + '</button>' +
      '<input type="file" data-act="mc-file" multiple hidden>' + projectChip(state) + '<span class="mc-hint">Enter sends · Shift+Enter new line</span>' +
      '<button class="mc-send" data-act="mc-send" type="button" aria-label="Send" title="Send to the manager">' + SEND + '</button></div></div></div>';
  }

  /* who answers: a live terminal session, or the on-call manager Tirith starts when the owner writes */
  function presence(state) {
    var p = state.manager && state.manager.presence;
    if (!p) return '';
    var txt = p.working ? 'Manager is working on your message…' : p.mode === 'live' ? 'Manager: live session' : p.session ? 'Manager: one session, always on' : 'Manager: on call (starts when you write)';
    return '<span class="mc-presence ' + (p.working ? 'work' : p.mode) + '" role="status">' + esc(txt) + '</span>' +
      (p.session && p.session.busy ? '<button class="mc-stop" type="button" data-act="mc-interrupt" title="Stop the current turn (Esc)">Stop</button>' : '');
  }

  function plainHead(t) {
    t = String(t || '').replace(/[`*_#>\[\]]/g, '').replace(/\s+/g, ' ').trim();
    return t.length > 48 ? t.slice(0, 47) + '…' : t;
  }

  /* the chat as HTML; `drawer` adds a close button */
  T.managerChat = function (state, drawer) {
    lastState = state;
    syncDraftProject();
    var list = messages(state), since = waitingSince(state), h;
    var open = T.turnOpen ? T.turnOpen(state) : 0;
    h = '<section class="mchat' + (drawer ? ' in-drawer' : '') + (ui.chatDrag ? ' drop' : '') + '"><div class="mc-head"><h2>Manager' +
      (open ? ' <span class="icount" title="Messages that wait for you">' + open + '</span>' : '') + '</h2>' + presence(state) +
      (open ? '<button class="mc-jump" data-act="mc-jump" type="button">' + (open > 1 ? 'Jump to next' : 'Jump to it') + '</button>' : '') +
      (drawer ? '<button class="mc-x" data-act="mc-close" type="button" aria-label="Close the chat">&times;</button>' : '') + '</div>' +
      '<div class="mc-log" id="mc-log">';
    h += list.length ? list.map(function (m) { return msg(m, state); }).join('') : T.empty('Write to the manager here. Its answers, and what it needs from you, show in this thread.');
    if (since) h += status(state);
    return h + '</div>' + composer(state) + '</section>';
  };

  /* ---------- sending: the app writes the inbox, a browser opens the tirith:// link, else the CLI line is shown ---------- */
  function openLink(url) {
    var a = document.createElement('a'); a.href = url; a.style.display = 'none';
    document.body.appendChild(a); a.click(); document.body.removeChild(a);
  }
  function hasTauri() { var t = window.__TAURI__; return !!(t && t.core && t.core.invoke); }
  /* {name, type, size, b64} -> a composer entry; a picture also gets its data URL and size (a broken one is a file) */
  function picOf(f) {
    var p = { name: f.name || 'file', type: f.type || '', size: f.size, url: null, w: 0, h: 0, b64: f.b64 };
    if (!PIC_TYPES[p.type]) return Promise.resolve(p);
    return new Promise(function (resolve) {
      var url = 'data:' + p.type + ';base64,' + p.b64, im = new Image();
      im.onload = function () { p.url = url; p.w = im.naturalWidth; p.h = im.naturalHeight; resolve(p); };
      im.onerror = function () { resolve(p); };
      im.src = url;
    });
  }
  function readPic(file) {
    return new Promise(function (resolve) {
      var r = new FileReader();
      r.onload = function () {
        var url = String(r.result);
        resolve(picOf({ name: file.name, type: file.type, size: file.size, b64: url.slice(url.indexOf(',') + 1) }));
      };
      r.onerror = function () { resolve(null); };
      r.readAsDataURL(file);
    });
  }
  /* the files that fit: at most 6 per message, 10 MB each, none empty; `bad` says why one was left out */
  function admit(files) {
    var bad = null;
    files = files.filter(function (f) {
      if (f.error) { bad = f.error + '.'; return false; }
      if (!f.size) { bad = (f.name || 'The file') + ' is empty (a folder cannot be attached).'; return false; }
      if (f.size > PIC_BYTES) { bad = f.name + ' is over 10 MB.'; return false; }
      return true;
    });
    if (ui.chatPics.length + files.length > PIC_MAX) { bad = 'At most ' + PIC_MAX + ' files per message.'; files = files.slice(0, Math.max(0, PIC_MAX - ui.chatPics.length)); }
    return { files: files, bad: bad };
  }
  T.chatAdmit = admit;
  function take(pending, bad) {
    ui.chatErr = bad;
    Promise.all(pending).then(function (got) {
      got.forEach(function (p) { if (p && ui.chatPics.length < PIC_MAX) ui.chatPics.push(p); else if (!p) ui.chatErr = 'A file could not be read.'; });
      T.render();
    });
    if (bad) T.render();
  }
  /* add files (picked, pasted or dropped on the page) to the composer; a page opened as a file cannot write the
     attachments folder, so it says so */
  function addPics(files) {
    files = Array.prototype.slice.call(files || []);
    if (!files.length) return;
    ui.chatErr = null;
    if (!hasTauri()) { ui.chatErr = NO_SAVE; T.render(); return; }
    var ok = admit(files);
    take(ok.files.map(readPic), ok.bad);
  }
  /* files dropped on the app window: the app reads them (only those of that drop) and hands over the bytes */
  function addDropped(paths) {
    if (!paths || !paths.length) return;
    ui.chatErr = null;
    window.__TAURI__.core.invoke('attachment_dropped', { paths: paths }).then(function (got) {
      var ok = admit(got || []);
      take(ok.files.map(picOf), ok.bad);
    }, function (err) { ui.chatErr = 'Could not attach: ' + (err && err.message || err); T.render(); });
  }
  function newId() {
    var b = new Uint8Array(4);
    if (window.crypto && crypto.getRandomValues) crypto.getRandomValues(b); else for (var i = 0; i < 4; i++) b[i] = Math.random() * 256;
    return 'in-' + Array.prototype.map.call(b, function (x) { return ('0' + x.toString(16)).slice(-2); }).join('');
  }
  function newCid() { return 'c-' + newId().slice(3) + Date.now().toString(36); }
  function findOut(cid) { return ui.chatOut.filter(function (o) { return o.cid === cid; })[0]; }
  /* link or copy path only: nothing in the state after SEND_WAIT, so delivery is not confirmed; give the owner the text back */
  function arm(o) {
    clearTimeout(o.timer);
    o.timer = setTimeout(function () { fail(o, 'Not confirmed: it was not in the inbox within ' + SEND_WAIT / 1000 + ' s. It may still arrive; Retry is safe.', true); }, SEND_WAIT);
  }
  function fail(o, why, unconfirmed) {
    if (o.state !== 'sending' || !findOut(o.cid)) return;
    clearTimeout(o.timer);
    o.state = 'failed'; o.err = why; o.unconfirmed = !!unconfirmed; ui.chatErr = null; ui.chatStick = true;
    if (!o.restored) {
      o.restored = true;
      ui.chatDraft = ui.chatDraft.trim() && ui.chatDraft !== o.text ? o.text + '\n' + ui.chatDraft : o.text;
      if (o.pics && o.pics.length && !ui.chatPics.length) ui.chatPics = o.pics.filter(function (p) { return p.b64; });
      saveDraft();
    }
    saveUnsent();
    T.render();
  }
  /* the app saves each file under attachments/<id>/ first, then adds the chat row that lists them */
  function transmit(o) {
    var inv = window.__TAURI__.core.invoke;
    return (o.pics || []).reduce(function (chain, p) {
      return chain.then(function () {
        if (p.saved) return;
        return inv('attachment_save', { messageId: o.id, name: p.name, mime: p.type, bytesBase64: p.b64 }).then(function (r) {
          if (p.url) ui.chatPicUrl[r.path] = p.url;
          p.saved = { path: r.path, type: r.type, size: r.size, width: p.w, height: p.h, name: r.name || p.name };
        });
      });
    }, Promise.resolve()).then(function () {
      var atts = (o.pics || []).map(function (p) { return p.saved; });
      return inv('inbox_add', { kind: 'chat', project: o.project, step: '', text: o.text, id: o.id, cid: o.cid, attachments: atts });
    });
  }
  function linkOf(o) {
    var L = lastState && lastState.launch;
    return 'tirith://inbox?kind=chat&project=' + encodeURIComponent(o.project) + '&step=&text=' + encodeURIComponent(o.text) + '&cid=' + encodeURIComponent(o.cid) + '&token=' + encodeURIComponent(L.token);
  }
  /* send (or resend) one message: it stays "Sending…" until the state has it */
  function dispatch(o, viaLink) {
    o.state = 'sending'; o.err = null; o.at = T.now(); ui.chatStick = true;
    if (!findOut(o.cid)) ui.chatOut.push(o);
    o.unconfirmed = false;
    saveUnsent();
    if (viaLink) { arm(o); openLink(linkOf(o)); }
    else transmit(o).then(function () {
      /* the app wrote the inbox row: delivered, however slowly the collector shows it */
      if (o.state === 'sending' && findOut(o.cid)) { o.state = 'sent'; T.render(); }
    }, function (err) { fail(o, 'Could not send: ' + (err && err.message || err)); });
    T.render();
  }
  /* Stop: interrupt the running turn (the session lives on); the app does it, else the CLI line is shown */
  function interrupt() {
    if (hasTauri()) {
      window.__TAURI__.core.invoke('manager_control', { op: 'interrupt' }).then(null, function (err) { ui.chatErr = String(err); T.render(); });
      return;
    }
    ui.chatErr = null; ui.chatCli = 'tirith manager interrupt';
    T.render();
  }
  function lastSent() {
    if (ui.chatLast) return ui.chatLast;
    var mine = ((lastState && lastState.inbox) || []).filter(function (r) { return r.kind === 'chat' && r.text; });
    mine.sort(function (a, b) { return (T.ms(a.at) || 0) - (T.ms(b.at) || 0); });
    return mine.length ? mine[mine.length - 1].text : '';
  }
  function canLink() { var L = lastState && lastState.launch; return !hasTauri() && !!(L && L.token && L.available); }
  function send() {
    var text = ui.chatDraft.trim(), pics = ui.chatPics.slice();
    if (pics.length && !hasTauri()) return;
    if (!text && !pics.length) return;
    var project = curProject();
    ui.chatErr = null; ui.chatCli = null;
    if (ui.chatReplyTo && text && !pics.length) {   /* the answer to an item that waits for the owner: a reply, not a new chat message */
      var to = ui.chatReplyTo;
      ui.chatReplyTo = null; ui.chatDraft = ''; ui.chatStick = true; saveDraft();
      if (T.turnReply(to, text)) return;
      ui.chatDraft = text;
    }
    var o = { cid: newCid(), id: newId(), text: text, project: project, pics: pics };
    ui.chatLast = text;
    ui.chatDraft = ''; ui.chatPics = []; saveDraft();
    if (hasTauri() || canLink()) { dispatch(o, !hasTauri()); return; }
    ui.chatCli = 'tirith inbox add --kind chat' + (project ? ' --project ' + shq(project) : '') + ' --cid ' + o.cid + ' --text ' + shq(text);
    ui.chatStick = true;
    T.render();
  }
  function retry(cid) {
    var o = findOut(cid); if (!o || o.state === 'sending') return;
    if (o.restored && ui.chatDraft === o.text) { ui.chatDraft = ''; saveDraft(); }
    o.restored = false;
    dispatch(o, canLink());
  }
  function copyText(cid) {
    var o = findOut(cid); if (!o) return;
    try { navigator.clipboard.writeText(o.text).catch(function () {}); } catch (e) { /* the text is also back in the composer */ }
  }

  try {
    JSON.parse(store('get', UNSENT_KEY) || '[]').forEach(function (o) {
      /* the page closed while sending: the retry is safe, the collector ignores a client id it already has */
      if (o && o.cid && o.text && !findOut(o.cid)) { o.at = T.now(); o.state = 'sending'; ui.chatOut.push(o); arm(o); }
    });
  } catch (e) { /* unreadable list: nothing to bring back */ }

  /* scroll to the next message that waits for the owner, round and round */
  function jump() {
    var els = document.querySelectorAll('#mc-log .mc-m.turn.wait'); if (!els.length) return;
    ui.chatJump = ui.chatJump >= els.length ? 0 : ui.chatJump;
    var el = els[ui.chatJump]; ui.chatJump = (ui.chatJump + 1) % els.length;
    el.scrollIntoView({ block: 'center', behavior: 'smooth' });
    el.classList.remove('flash'); void el.offsetWidth; el.classList.add('flash');
  }

  /* the box grows with its text up to half the window, then scrolls inside; the message list keeps its bottom in view */
  function fit(t) {
    var log = document.getElementById('mc-log');
    var atEnd = log && log.scrollHeight - log.scrollTop - log.clientHeight < 48;
    t.style.height = 'auto';
    var cap = Math.floor(window.innerHeight * 0.5), h = Math.min(t.scrollHeight, cap);
    t.style.height = h + 'px';
    t.style.overflowY = t.scrollHeight > cap ? 'auto' : 'hidden';
    if (log && atEnd) log.scrollTop = log.scrollHeight;
  }
  window.addEventListener('resize', function () { var t = document.querySelector('textarea[data-act="mc-text"]'); if (t) fit(t); });

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
    var ta = document.querySelector('textarea[data-act="mc-text"]');
    if (ta) { fit(ta); if (nl && (!keep || keep.bottom || ui.chatStick)) nl.scrollTop = nl.scrollHeight; }
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
    else if (act === 'mc-interrupt') interrupt();
    else if (act === 'mc-retry') retry(b.getAttribute('data-cid'));
    else if (act === 'mc-copy') copyText(b.getAttribute('data-cid'));
    else if (act === 'mc-close') { ui.chatOpen = false; T.render(); }
    else if (act === 'mc-attach' && !hasTauri()) { ui.chatErr = NO_SAVE; T.render(); }
    else if (act === 'mc-attach') { var f =view.querySelector('[data-act="mc-file"]'); if (f) f.click(); }
    else if (act === 'mc-unpic') { ui.chatPics.splice(+b.getAttribute('data-i'), 1); T.render(); }
    else if (act === 'mc-zoom') lightbox(b.getAttribute('data-src'));
    else if (act === 'mc-reply') {
      ui.chatReplyTo = b.getAttribute('data-key'); T.render();
      var ta = document.querySelector('[data-act="mc-text"]'); if (ta) ta.focus();
    } else if (act === 'mc-unreply') { ui.chatReplyTo = null; T.render(); }
    else if (act === 'mc-jump') jump();
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
    var files = (e.clipboardData && e.clipboardData.files) || [];
    if (files.length) { e.preventDefault(); addPics(files); }
  });

  /* ---------- drag and drop onto the chat: the page's own events (a browser, and the app, which turns its native
     drop handler off so the page gets the files), plus the app's file-drop events for a window that keeps that
     handler on (the window takes the drop there, so the page never sees the files) ---------- */
  function dragMark(c) {
    Array.prototype.forEach.call(document.querySelectorAll('.mchat.drop'), function (x) { if (x !== c) x.classList.remove('drop'); });
    if (c) c.classList.add('drop');
    ui.chatDrag = !!c;   /* a render while dragging keeps the drop target */
  }
  function hasFiles(e) { return !!(e.dataTransfer && Array.prototype.indexOf.call(e.dataTransfer.types || [], 'Files') >= 0); }
  var ev = window.__TAURI__ && window.__TAURI__.event, appDrop = hasTauri() && !!(ev && ev.listen), dragOff = 0;
  /* a file dropped beside the chat does nothing: without this the browser would leave Tirith to show the file */
  document.addEventListener('dragover', function (e) {
    if (!hasFiles(e)) return;
    e.preventDefault();
    clearTimeout(dragOff);
    var c = e.target.closest ? e.target.closest('.mchat') : null;
    if (e.dataTransfer) e.dataTransfer.dropEffect = c ? 'copy' : 'none';
    dragMark(c);
  });
  /* leaving one element for another also fires dragleave (and Safari never says for which): the next dragover keeps it */
  document.addEventListener('dragleave', function () { clearTimeout(dragOff); dragOff = setTimeout(function () { dragMark(null); }, 100); });
  document.addEventListener('drop', function (e) {
    if (!hasFiles(e)) return;
    e.preventDefault();
    var c = e.target.closest && e.target.closest('.mchat');
    dragMark(null);
    if (c && e.dataTransfer.files && e.dataTransfer.files.length) addPics(e.dataTransfer.files);
    else T.render();
  });
  /* the app's events carry the pointer in physical pixels of the webview */
  function chatAt(pos) {
    var r = window.devicePixelRatio || 1, el = pos ? document.elementFromPoint(pos.x / r, pos.y / r) : null;
    return el && el.closest ? el.closest('.mchat') : null;
  }
  if (appDrop) {
    var over = function (e) { dragMark(chatAt(e.payload && e.payload.position)); };
    ev.listen('tauri://drag-enter', over);
    ev.listen('tauri://drag-over', over);
    ev.listen('tauri://drag-leave', function () { dragMark(null); });
    ev.listen('tauri://drag-drop', function (e) {
      var p = e.payload || {}, c = chatAt(p.position);
      dragMark(null);
      if (c) addDropped(p.paths); else T.render();
    });
  }
  view.addEventListener('input', function (e) {
    var t = e.target, act = t.getAttribute && t.getAttribute('data-act');
    if (act === 'mc-text') { ui.chatDraft = t.value; saveDraft(); fit(t); }
  });
  view.addEventListener('change', function (e) {
    var t = e.target, act = t.getAttribute && t.getAttribute('data-act');
    if (act === 'mc-proj') { saveDraft(); ui.chatProj = t.value; syncDraftProject(); T.render(); }
    else if (act === 'mc-file') { addPics(t.files); t.value = ''; }
  });
  view.addEventListener('keydown', function (e) {
    var t = e.target; if (!t.getAttribute || t.getAttribute('data-act') !== 'mc-text') return;
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); ui.chatDraft = t.value; send(); }
    else if (e.key === 'Escape' && T.managerTurn && T.managerTurn(lastState)) { e.preventDefault(); e.stopPropagation(); interrupt(); }
    else if (e.key === 'ArrowUp' && !t.value && !ui.chatPics.length && lastSent()) { e.preventDefault(); t.value = ui.chatDraft = lastSent(); saveDraft(); fit(t); }
    else if (e.key === 'Escape' && inDrawer()) { e.preventDefault(); e.stopPropagation(); ui.chatOpen = false; T.render(); }
  });
})();
