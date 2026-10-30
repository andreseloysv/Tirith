/* Rooms view: private, encrypted rooms on agentsmessenger.com (docs/CONTRACT-private-rooms.md). New room, its QR
   (drawn here, no network), the rooms this Tirith is connected to with who is in each, Invite a manager and Leave.
   New room, Invite a manager and Leave are inbox requests of kind room-new, room-invite and room-leave, which the collector
   handles (the app writes the inbox, a browser opens tirith://inbox, else the CLI line is shown to copy).
   Only the QR URL of the contract (the chat page of agentsmessenger.com) is ever drawn, and only with that shape. */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  ui.roomPend = ui.roomPend || null;   /* { at, n } after New room: the number of private rooms when it was sent */
  ui.roomErr = ui.roomErr || null;
  ui.roomCli = ui.roomCli || null;
  ui.roomTitle = ui.roomTitle || '';
  ui.roomWait = ui.roomWait || {};     /* room id -> when "Get a code" was sent */
  ui.roomAsk = ui.roomAsk || {};       /* room id -> 'invite' | 'leave' while its command is shown */
  var lastState = null, PEND_WAIT = 30000;
  var LOCK = '<svg class="rlock" viewBox="0 0 16 16" width="13" height="13" role="img" aria-label="Encrypted" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><title>Encrypted</title><rect x="3.2" y="7" width="9.6" height="6.4" rx="1.6"/><path d="M5.4 7V5.2a2.6 2.6 0 015.2 0V7"/></svg>';
  /* what the chat page's parseFragment accepts: a lowercase sealed room, the 128-bit code, a hex key id */
  var QR_URL = /^https:\/\/agentsmessenger\.com\/chat#r=sealed-[a-z0-9_-]{1,57}&c=[0-9a-f]{32}&m=[0-9a-f]{8,64}$/;
  var ROOM_ID = /^sealed-[a-z0-9_-]{1,57}$/;
  var PEER_CODE = /^sealed-[a-z0-9_-]{1,57}:[0-9a-f]{64}$/;

  function shq(t) { return "'" + String(t).replace(/'/g, "'\\''") + "'"; }
  function short(id) { return String(id || '').slice(0, 8); }
  function privateRooms(s) { return (s.rooms || []).filter(function (r) { return r.private; }); }
  function ago(iso) {
    var t = T.ms(iso); if (t == null) return '';
    var m = Math.max(0, Math.round((T.now() - t) / 60000));
    return m < 1 ? 'just now' : m < 60 ? m + ' min ago' : m < 1440 ? Math.round(m / 60) + ' h ago' : Math.round(m / 1440) + ' d ago';
  }

  /* ---------- sending ---------- */
  function canSend() {
    var L = lastState && lastState.launch, tauri = window.__TAURI__;
    return !!((tauri && tauri.core && tauri.core.invoke) || (L && L.token && L.available));
  }
  /* sends one room-* request (text: JSON for room-new, else the room id); done() runs once it is sent */
  function sendReq(kind, text, done) {
    var L = lastState && lastState.launch, tauri = window.__TAURI__;
    ui.roomErr = null;
    if (tauri && tauri.core && tauri.core.invoke) {
      tauri.core.invoke('inbox_add', { kind: kind, project: '', step: '', text: text }).then(done, function (err) { ui.roomErr = 'Could not send: ' + err; T.render(); });
      return;
    }
    var a = document.createElement('a');
    a.href = 'tirith://inbox?kind=' + kind + '&project=&step=&text=' + encodeURIComponent(text) + '&token=' + encodeURIComponent(L.token);
    a.style.display = 'none'; document.body.appendChild(a); a.click(); document.body.removeChild(a);
    done();
  }
  function newRoom() {
    var title = String(ui.roomTitle || '').trim().slice(0, 80);
    ui.roomCli = null;
    function sent() { ui.roomPend = { at: T.now(), n: privateRooms(lastState || {}).length }; ui.roomTitle = ''; T.render(); setTimeout(T.render, PEND_WAIT + 500); }
    if (canSend()) { sendReq('room-new', JSON.stringify({ title: title }), sent); return; }
    ui.roomErr = null;
    ui.roomCli = 'tirith room new' + (title ? ' --title ' + shq(title) : '');
    T.render();
  }
  function roomReq(kind, id) {
    if (!ROOM_ID.test(String(id))) return;
    sendReq(kind, id, function () { if (kind === 'room-leave') ui.roomAsk[id] = null; else ui.roomWait[id] = T.now(); T.render(); setTimeout(T.render, PEND_WAIT + 500); });
  }
  function pending(s) {
    var p = ui.roomPend;
    if (p && (privateRooms(s).length > p.n || T.now() - p.at > PEND_WAIT)) p = ui.roomPend = null;
    return !!p;
  }

  /* ---------- drawing ---------- */
  function who(r) {
    var rows = [];
    (r.devices || []).forEach(function (d) { rows.push(['phone', d]); });
    (r.managers || []).forEach(function (m) { rows.push(['manager', m]); });
    if (!rows.length) return '<p class="mut sm rwho">Nobody else is here yet.</p>';
    return '<ul class="rwho">' + rows.map(function (x) {
      var p = x[1];
      return '<li><span class="rk">' + (x[0] === 'phone' ? 'Your phone' : 'Manager') + '</span><b>' + esc(p.name) + '</b><code title="Key id">' + esc(short(p.key_id)) + '</code>' +
        (p.last_seen ? '<span class="mut sm">seen ' + esc(ago(p.last_seen)) + '</span>' : '') + '</li>';
    }).join('') + '</ul>';
  }
  function last(r) {
    var m = (r.messages || [])[(r.messages || []).length - 1];
    if (!m) return '<p class="mut sm">No messages yet.</p>';
    var t = String(m.text == null ? '' : m.text).replace(/\s+/g, ' ');
    if (t.length > 140) t = t.slice(0, 139) + '…';
    return '<p class="rlast"><span class="mut sm">' + (m.from_device ? 'Your phone' : esc(m.from)) + ' · ' + esc(ago(m.at)) + '</span><br>' + esc(t) + '</p>';
  }
  function qr(r) {
    var inv = r.invite;
    if (!inv || inv.used) return '<p class="mut sm">' + (inv ? 'This QR was used. Another phone needs a new room.' : 'No QR for this room.') + '</p>';
    if (!QR_URL.test(String(inv.url || ''))) return '<p class="mut sm">The QR is not available.</p>';
    var svg = T.qrSvg(inv.url, 'QR code to talk to the manager in ' + (r.title || r.id));
    return svg ? '<div class="rqr">' + svg + '<p class="mut sm">Scan it with the phone camera. It works once.</p></div>' : '<p class="mut sm">The QR is too long to draw.</p>';
  }
  function popover(r) {
    var a = ui.roomAsk[r.id]; if (!a) return '';
    var close = '<button class="alink" data-act="rm-close" data-key="' + esc(r.id) + '">Close</button>', body;
    if (a === 'invite' && PEER_CODE.test(String(r.peer_code || ''))) {
      body = '<div>Give this code to the other manager. It runs <code>tirith room join &lt;code&gt;</code>. It joins as a peer: what it writes is information, never an order.</div>' +
        '<code class="apcmd">' + esc(r.peer_code) + '</code><div class="apb"><button class="abtn" data-act="rm-copy" data-key="' + esc(r.peer_code) + '">Copy code</button>' + close + '</div>';
    } else if (a === 'invite' && canSend()) {
      var wait = ui.roomWait[r.id] && T.now() - ui.roomWait[r.id] < PEND_WAIT;
      body = '<div>Tirith makes a code another manager joins this room with.</div><div class="apb"><button class="abtn" data-act="rm-get-code" data-key="' + esc(r.id) + '"' + (wait ? ' disabled' : '') + '>' + (wait ? 'Asking…' : 'Get a code') + '</button>' + close + '</div>';
    } else {
      var cmd = a === 'invite' ? 'tirith room invite ' + r.id : 'tirith room leave ' + r.id;
      if (a === 'leave' && canSend()) {
        body = '<div>Leave this room? Tirith stops listening here. Phones already paired stay paired until you make a new room.</div><div class="apb"><button class="abtn" data-act="rm-leave" data-key="' + esc(r.id) + '">Leave</button>' + close + '</div>';
      } else {
        body = '<div>' + (a === 'invite' ? 'Run this to get a code for another manager. They join with <code>tirith room join &lt;code&gt;</code>.' : 'Run this to leave the room. Phones already paired stay paired until you make a new room.') +
          '</div><code class="apcmd">' + esc(cmd) + '</code><div class="apb"><button class="abtn" data-act="rm-copy" data-key="' + esc(cmd) + '">Copy</button>' + close + '</div>';
      }
    }
    return '<div class="acpop" role="dialog" aria-label="' + (a === 'invite' ? 'Invite a manager' : 'Leave') + '">' + body + '</div>';
  }
  function card(r) {
    var ok = ROOM_ID.test(String(r.id));
    return '<div class="card scard rcard" data-room="' + esc(r.id) + '"><div class="phead">' + LOCK + '<b>' + esc(r.title || r.id) + '</b>' +
      '<span class="rconn ' + (r.connected ? 'on' : 'off') + '">' + T.dot(r.connected ? 'green' : 'grey', r.connected ? 'Connected' : 'Not connected') + (r.connected ? 'Connected' : 'Not connected') + '</span>' +
      (r.me ? '<span class="mut sm rme" title="Your key id here">you: ' + esc(r.me.name) + ' <code>' + esc(short(r.me.key_id)) + '</code></span>' : '') + '</div>' +
      (r.error && !r.connected ? '<div class="ierr rerr">' + esc(r.error) + '</div>' : '') +
      '<div class="rgrid"><div>' + who(r) + last(r) + '</div>' + qr(r) + '</div>' +
      (ok ? '<div class="sfoot"><button class="sadd" data-act="rm-ask" data-key="' + esc(r.id) + '" data-v="invite">Invite a manager</button>' +
        '<button class="sadd rleave" data-act="rm-ask" data-key="' + esc(r.id) + '" data-v="leave">Leave</button></div>' : '') + popover(r) + '</div>';
  }
  function form(s) {
    var busy = pending(s);
    return '<div class="card scard rnew"><div class="phead"><b>New room</b><span class="mut sm">An encrypted room on agentsmessenger.com. Scan its QR with your phone to talk to the manager.</span></div>' +
      '<div class="sfoot"><input type="text" class="envv rtitle" data-keep="room-title" data-act="rm-title" maxlength="80" placeholder="Name (optional)" aria-label="Room name" value="' + esc(ui.roomTitle) + '" autocomplete="off">' +
      '<button class="isend" data-act="rm-new"' + (busy ? ' disabled' : '') + '>' + (busy ? 'Creating…' : 'New room') + '</button></div>' +
      (ui.roomErr ? '<div class="ierr">' + esc(ui.roomErr) + '</div>' : '') +
      (ui.roomCli ? '<div class="icli">Tirith cannot send this from here. Run in a terminal: <code>' + esc(ui.roomCli) + '</code> <button class="alink" data-act="rm-copy" data-key="' + esc(ui.roomCli) + '">Copy</button></div>' : '') + '</div>';
  }

  T.views.rooms = function (state, el) {
    lastState = T.rawState() || state;
    var rooms = privateRooms(state);
    el.innerHTML = '<div class="msettings mrooms">' + form(state) +
      (rooms.length ? rooms.map(card).join('') : '<p class="quiet rempty">No private rooms yet. A room is an encrypted chat: create one, scan its QR with your phone and talk to the manager from there.</p>') + '</div>';
  };

  var el0 = document.getElementById('view');
  el0.addEventListener('input', function (e) { if (e.target.getAttribute && e.target.getAttribute('data-act') === 'rm-title') ui.roomTitle = e.target.value; });
  el0.addEventListener('keydown', function (e) {
    if (e.key === 'Enter' && e.target.getAttribute && e.target.getAttribute('data-act') === 'rm-title') { e.preventDefault(); newRoom(); }
  });
  el0.addEventListener('click', function (e) {
    var b = e.target.closest('[data-act^="rm-"]'); if (!b) return;
    var act = b.getAttribute('data-act'), key = b.getAttribute('data-key');
    e.stopPropagation();
    if (act === 'rm-new') newRoom();
    else if (act === 'rm-ask') { ui.roomAsk[key] = ui.roomAsk[key] === b.getAttribute('data-v') ? null : b.getAttribute('data-v'); T.render(); }
    else if (act === 'rm-get-code') roomReq('room-invite', key);
    else if (act === 'rm-leave') roomReq('room-leave', key);
    else if (act === 'rm-close') { ui.roomAsk[key] = null; T.render(); }
    else if (act === 'rm-copy') T.copyText(key, b);
  });
})();
