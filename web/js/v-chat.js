/* Tirith web UI - chat panel. Only in the desktop app, and only when the app reports the chat flag on.
   Talks to Rust over Tauri IPC (chat_start / chat_send / chat_stop and the tirith://chat event). */
(function () {
  var T = window.T, esc = T.esc, tauri = T.tauri;
  if (!tauri) return;
  var core = tauri.core, panel, log, sid = null, cur = null, state = null;

  /* Text carried by one line of the agent's streaming JSON output; '' for lines with no text. */
  function textOf(line) {
    var o; try { o = JSON.parse(line); } catch (e) { return line; }
    if (!o || typeof o !== 'object') return '';
    if (o.type === 'content_block_delta' && o.delta && o.delta.text) return o.delta.text;
    if (o.type === 'stream_event' && o.event) return textOf(JSON.stringify(o.event));
    if (o.type === 'assistant' && o.message && Array.isArray(o.message.content)) {
      return o.message.content.map(function (c) { return c && c.type === 'text' ? c.text : ''; }).join('');
    }
    return '';
  }
  function add(cls, text) {
    var d = document.createElement('div');
    d.className = 'chat-msg ' + cls; d.textContent = text;
    log.appendChild(d); log.scrollTop = log.scrollHeight;
    return d;
  }

  function build(aliases) {
    var prj = ((state && state.projects) || []).map(function (p) { return '<option value="' + esc(p.id) + '">' + esc(p.name) + '</option>'; }).join('');
    var ags = aliases.map(function (a) { return '<option>' + esc(a) + '</option>'; }).join('');
    panel = document.createElement('div');
    panel.id = 'chat'; panel.hidden = true;
    panel.innerHTML = '<div class="chat-head"><select id="chat-p" aria-label="Project">' + prj + '</select><select id="chat-a" aria-label="Agent">' + ags +
      '</select><button id="chat-new" type="button">New chat</button><button id="chat-x" type="button" aria-label="Close">&times;</button></div>' +
      '<div id="chat-log" class="chat-log"></div>' +
      '<form id="chat-f"><input id="chat-i" autocomplete="off" placeholder="Message" aria-label="Message"><button type="submit">Send</button></form>';
    document.body.appendChild(panel);
    log = panel.querySelector('#chat-log');
    panel.querySelector('#chat-x').onclick = function () { panel.hidden = true; };
    panel.querySelector('#chat-new').onclick = function () {
      if (sid) core.invoke('chat_stop', { id: sid });
      sid = null; cur = null; log.innerHTML = '';
    };
    panel.querySelector('#chat-f').onsubmit = function (e) {
      e.preventDefault();
      var inp = panel.querySelector('#chat-i'), txt = inp.value.trim();
      if (!txt) return;
      inp.value = ''; add('me', txt); cur = null;
      var go = sid ? Promise.resolve(sid) : core.invoke('chat_start', { project: panel.querySelector('#chat-p').value, alias: panel.querySelector('#chat-a').value }).then(function (id) { return (sid = id); });
      go.then(function (id) { return core.invoke('chat_send', { id: id, text: txt }); }).catch(function (err) { add('err', String(err)); sid = null; });
    };
    tauri.event.listen('tirith://chat', function (e) {
      var p = e.payload;
      if (p.id !== sid) return;
      if (p.kind === 'exit') { add('sys', 'Chat ended (' + p.line + ')'); sid = null; cur = null; return; }
      if (p.kind === 'err') { add('err', p.line); return; }
      var t = textOf(p.line);
      if (!t) return;
      if (!cur) cur = add('agent', '');
      cur.textContent += t;
      log.scrollTop = log.scrollHeight;
    });
  }

  function init() {
    core.invoke('chat_enabled').then(function (on) {
      if (!on) return;
      return core.invoke('chat_agents').then(function (aliases) {
        var btn = document.createElement('button');
        btn.id = 'chat-open'; btn.type = 'button'; btn.textContent = 'Chat';
        btn.onclick = function () {
          if (!panel) {
            core.invoke('current_state').then(function (s) { state = s; build(aliases); panel.hidden = false; });
          } else panel.hidden = !panel.hidden;
        };
        document.body.appendChild(btn);
      });
    }).catch(function () {});
  }
  init();
})();
