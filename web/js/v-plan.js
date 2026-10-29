/* Plan card(s) at the top of the Overview: a project's roadmap, step by step. */
(function () {
  var T = window.T, esc = T.esc, ui = T.ui;
  ui.planOpen = ui.planOpen || {};
  ui.launching = ui.launching || {};   /* "project|step" -> ms of the click, until the agent shows up */
  ui.launchHint = ui.launchHint || null;
  ui.inboxAsk = ui.inboxAsk || {};     /* "project|step" -> true while its question box is open */
  ui.inboxDraft = ui.inboxDraft || {}; /* "project|step" -> text typed so far */
  ui.inboxPend = ui.inboxPend || {};   /* "project|step" -> ms of a send the collector has not shown yet */
  ui.inboxCli = ui.inboxCli || {};     /* "project|step" -> CLI command to copy (no way to send from here) */
  ui.inboxErr = ui.inboxErr || {};     /* "project|step" -> why the send failed */
  ui.inboxOpen = ui.inboxOpen || {};   /* request id -> true/false: answers shown (default: not for done requests) */
  var lastState = null, LAUNCH_WAIT = 90000, PEND_WAIT = 10000, ESCALATE = 120, INSTALL = 'tirith url install';

  var IC = {
    done: '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="7" fill="#2ea44f"/><path d="M4.8 8.2l2.2 2.2 4.2-4.6" fill="none" stroke="#fff" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>',
    active: '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="6.2" fill="#fff" stroke="#2f6fed" stroke-width="1.6"/><path d="M8 3.4a4.6 4.6 0 010 9.2z" fill="#2f6fed"/></svg>',
    todo: '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="6.2" fill="none" stroke="#b4bac4" stroke-width="1.6"/></svg>',
    empty: '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="6.2" fill="none" stroke="#d3d7de" stroke-width="1.6" stroke-dasharray="2 2.4"/></svg>'
  };
  var OK = '<svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"><path d="M2.6 6.3l2.3 2.3 4.5-4.9" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>';
  var OPEN = '<svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"><circle cx="6" cy="6" r="3.6" fill="none" stroke="#b4bac4" stroke-width="1.3"/></svg>';

  var ROCKET = '<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M9.6 2.2c2.1-.4 3.8-.3 4.2-.2.1.4.2 2.1-.2 4.2-.3 1.5-1.3 2.9-2.5 3.8l-.9.7-2.6-2.6.7-.9c.9-1.2 2.3-2.2 3.8-2.5z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/><circle cx="10.9" cy="5.1" r="1" fill="currentColor"/><path d="M5.9 6.5l-2.2.4-1.5 2.4 2.5.1M9.5 10.1l-.4 2.2-2.4 1.5-.1-2.5M4.6 11.4c-.8.2-1.6 1-1.9 1.9.9-.3 1.7-1.1 1.9-1.9z" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/></svg>';

  var FLAG = '<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M4 14V2.6M4 3.2c2.3-1.3 3.8 1.3 6.3 0 .9-.5 1.6-.6 2-.5v5.3c-.4-.1-1.1 0-2 .5-2.5 1.3-4-1.3-6.3 0" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>';
  var BUBBLE = '<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M3 3h10a1 1 0 011 1v6a1 1 0 01-1 1H7.5L4.6 13.6V11H3a1 1 0 01-1-1V4a1 1 0 011-1z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/></svg>';
  var CHEV = '<svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true"><path d="M3 4.5l3 3 3-3" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>';

  function plain(t) { return esc(String(t).replace(/`/g, '')); }
  function rich(t) { return esc(t).replace(/`([^`]+)`/g, '<code>$1</code>'); }

  /* "6 d 4 h left", "5 h 12 min left", "overdue" */
  function left(iso) {
    var t = T.ms(iso); if (t == null) return null;
    var s = Math.floor((t - T.now()) / 1000), d, h, m;
    if (s <= 0) return { txt: 'Overdue', hot: true };
    d = Math.floor(s / 86400); h = Math.floor((s % 86400) / 3600); m = Math.floor((s % 3600) / 60);
    return { hot: s < 86400, txt: (d ? d + ' d ' + h + ' h' : h ? h + ' h ' + m + ' min' : Math.max(1, m) + ' min') + ' left' };
  }
  function countdown(iso) {
    var c = left(iso); if (!c) return '';
    return '<span class="pcd' + (c.hot ? ' hot' : '') + '" data-due="' + esc(iso) + '">' + esc(c.txt) + '</span>';
  }
  function tickCountdowns() {
    var els = document.querySelectorAll('.pcd[data-due]'), i, c;
    for (i = 0; i < els.length; i++) {
      c = left(els[i].getAttribute('data-due')); if (!c) continue;
      els[i].textContent = c.txt; els[i].className = 'pcd' + (c.hot ? ' hot' : '');
    }
  }

  function bar(done, total, cls) {
    return '<span class="bar ' + (cls || '') + '"><i class="' + (done === total && total ? 'green' : 'blue') + '" style="width:' + (total ? Math.round(100 * done / total) : 0) + '%"></i></span>';
  }

  /* The "Finish with an agent" button: not for done steps or steps an agent is already working on. */
  function launchBtn(s, p, state, long) {
    var L = state.launch, key = p.id + '|' + s.id, t = ui.launching[key];
    if (!L || !L.token || s.status === 'done' || s.status === 'empty' || (s.agents && s.agents.length)) {
      if (t && s.agents && s.agents.length) delete ui.launching[key];
      return '';
    }
    var starting = t && T.now() - t < LAUNCH_WAIT, label = starting ? 'Starting…' : 'Start an agent now';
    var h = '<span class="plw"><button class="plaunch' + (long ? ' long' : '') + (starting ? ' busy' : '') + '" data-act="launch" data-key="' + esc(key) + '"' +
      (starting ? ' disabled' : '') + ' title="' + (L.available ? 'Start a sub-agent that finishes this step' : 'Install the link handler first: ' + INSTALL) + '" aria-label="' + label + '">' +
      ROCKET + (long || starting ? '<span>' + label + '</span>' : '') + '</button>';
    if (ui.launchHint === key && !L.available) {
      h += '<span class="pltip" role="tooltip">Tirith needs its link handler once. Run in a terminal:<code>' + INSTALL + '</code>then click again.</span>';
    }
    return h + '</span>';
  }

  /* ---------- the owner's inbox: "Finish it", "Ask", the chip and the answers ---------- */
  function reqsFor(state, pid, sid) {
    return (state.inbox || []).filter(function (r) { return r.project === pid && String(r.step) === String(sid); });
  }
  function shq(t) { return "'" + String(t).replace(/'/g, "'\\''") + "'"; }
  function cliCmd(kind, pid, sid, text) {
    return 'tirith inbox add --kind ' + kind + ' --project ' + shq(pid) + ' --step ' + shq(sid) + (text ? ' --text ' + shq(text) : '');
  }
  function inboxBtns(key) {
    return '<span class="iacts"><button class="ibtn" data-act="ifinish" data-key="' + esc(key) + '" title="Tirith starts an agent on the open tasks of this step, or tells you who already has it" aria-label="Finish this step">' + FLAG + '<span>Finish this step</span></button>' +
      '<button class="ibtn' + (ui.inboxAsk[key] ? ' on' : '') + '" data-act="iask" data-key="' + esc(key) + '" title="Ask what is going on with this step; the answer shows here" aria-label="Ask">' + BUBBLE + '<span>Ask</span></button></span>';
  }
  var AST = { working: 'blue pulse', done: 'green', failed: 'red', stalled: 'amber' };
  function tbadge(a) {
    var l = a.tests || [], t = l[l.length - 1], txt;
    if (!t || !t.result) return '';
    txt = t.result === 'running' ? 'running' : t.result === 'failed' ? (t.failed != null ? t.failed + ' failed' : 'failed') : (t.passed != null ? t.passed + ' passed' : 'passed');
    return ' <span class="rb ' + esc(t.result) + '" title="Last test run: ' + esc(t.command || '') + '">' + esc(txt) + '</span>';
  }
  /* "Handled by [avatar] name · feature" with the agent's live status */
  function handled(id, state) {
    var a = T.agentById(state, id);
    if (!a) return '<li class="ie"><i class="ido"></i><div class="ib">Handled by <b>' + esc(id) + '</b></div></li>';
    return '<li class="ie"><i class="ido ' + (a.status === 'working' ? 'live' : a.status === 'failed' ? 'bad' : 'ok') + '"></i><div class="ib">Handled by ' +
      '<button class="iag" data-act="goagent" data-key="' + esc(a.id) + '" title="Open this agent">' + T.avatar(a, 18) + '<b>' + esc(a.name) + '</b></button>' +
      (a.feature ? ' <span class="mut">· ' + plain(a.feature) + '</span>' : '') +
      ' <span class="ist">' + T.dot(AST[a.status] || 'amber', a.status) + esc(a.status) + '</span>' + tbadge(a) +
      (a.status === 'working' && a.summary ? '<div class="inow">' + T.mdInline(a.summary) + '</div>' : '') + '</div></li>';
  }
  function answerLabel(a) {
    return a.by === 'brain' ? (a.auto ? 'Muse Spark answered automatically' : 'Muse Spark answered') : 'Steward answered';
  }
  /* One request as a timeline: asked -> answered -> handled by an agent -> done. Only from state.inbox. */
  function thread(r, state) {
    var open = r.id in ui.inboxOpen ? ui.inboxOpen[r.id] : r.status !== 'done', answers = r.answers || [], lanes = r.lanes || [], h;
    h = '<div class="ithread" data-req="' + esc(r.id) + '"><ol class="itl"><li class="ie"><i class="ido me"></i><div class="ib"><b>' +
      (r.kind === 'ask' ? 'You asked: ' + plain(r.text) : 'You asked to finish this step') + '</b> <span class="iwhen">· ' + esc(T.clockTime(r.at)) + '</span></div></li>';
    answers.forEach(function (a) {
      h += '<li class="ie"><i class="ido ok"></i><div class="ib ians' + (open ? ' open' : '') + '"><button class="iahead" data-act="iatog" data-key="' + esc(r.id) + '" aria-expanded="' + open + '">' + CHEV +
        '<b>' + esc(answerLabel(a)) + '</b> <span class="iwhen">· ' + esc(T.clockTime(a.at)) + '</span></button>' +
        (open ? '<div class="ia md">' + T.md(a.text) + '</div>' + T.actions(a.actions, state, r.project) : '') + '</div></li>';
    });
    lanes.forEach(function (id) {
      var ag = T.agentById(state, id), named = answers.some(function (a) { return a.lane === id; });
      if (!named && ag && ag.started_at) h += '<li class="ie"><i class="ido ok"></i><div class="ib"><b>Tirith started an agent</b> <span class="iwhen">· ' + esc(T.clockTime(ag.started_at)) + '</span></div></li>';
      h += handled(id, state);
    });
    if (!answers.length && !lanes.length && r.status === 'open') {
      h += '<li class="ie"><i class="ido wait"></i><div class="ib iwait">Waiting for the steward · <span class="tick" data-start="' + esc(r.at) + '">' + esc(T.dur(T.elapsed(r.at))) + '</span>' +
        (T.elapsed(r.at) >= ESCALATE ? '<div class="inow">No answer yet. Tirith will escalate this.</div>' : '') + '</div></li>';
    }
    if (r.status === 'done') h += '<li class="ie"><i class="ido ok"></i><div class="ib"><b>Done</b></div></li>';
    return h + '</ol></div>';
  }
  function inboxBelow(s, p, state, key) {
    var reqs = reqsFor(state, p.id, s.id), pend = ui.inboxPend[key], h = '';
    if (pend && reqs.some(function (r) { return (T.ms(r.at) || 0) >= pend - 2000; })) { delete ui.inboxPend[key]; pend = null; }
    if (pend && T.now() - pend < PEND_WAIT) h += '<div class="ichip-row"><span class="ichip" title="Sent; it shows here as soon as the collector has it">Sent · waiting for the steward</span></div>';
    if (ui.inboxErr[key]) h += '<div class="ierr">' + esc(ui.inboxErr[key]) + '</div>';
    if (ui.inboxCli[key]) {
      h += '<div class="icli">Tirith cannot send from this page. Run in a terminal:<code>' + esc(ui.inboxCli[key]) + '</code></div>';
    }
    if (ui.inboxAsk[key]) {
      h += '<div class="iask"><input class="iin" type="text" data-act="itext" data-key="' + esc(key) + '" maxlength="1000" placeholder="Ask about this step…" aria-label="Question about this step" value="' + esc(ui.inboxDraft[key] || '') + '">' +
        '<button class="isend" data-act="isend" data-key="' + esc(key) + '">Send</button><span class="ihint">Enter sends · Esc closes</span></div>';
    }
    reqs.slice().sort(function (a, b) { return (T.ms(b.at) || 0) - (T.ms(a.at) || 0); }).slice(0, 3).forEach(function (r) { h += thread(r, state); });
    return h;
  }
  function openCount(ps, state) {
    var ids = {}; ps.forEach(function (p) { ids[p.id] = 1; });
    return (state.inbox || []).filter(function (r) { return r.status === 'open' && ids[r.project]; }).length;
  }

  /* Record a request: the app writes the inbox itself, a browser opens the tirith:// link, else the CLI line is shown. */
  function sendInbox(kind, key, text) {
    var parts = key.split('|'), pid = parts[0], sid = parts[1], st = lastState, L = st && st.launch, tauri = window.__TAURI__;
    ui.inboxErr[key] = null; ui.inboxCli[key] = null;
    function sent() { ui.inboxPend[key] = T.now(); T.render(); setTimeout(T.render, PEND_WAIT + 500); }
    if (tauri && tauri.core && tauri.core.invoke) {
      tauri.core.invoke('inbox_add', { kind: kind, project: pid, step: sid, text: text || '' }).then(sent, function (err) {
        ui.inboxErr[key] = 'Could not send: ' + err; T.render();
      });
      return;
    }
    if (L && L.token && L.available) {
      openLink('tirith://inbox?kind=' + kind + '&project=' + encodeURIComponent(pid) + '&step=' + encodeURIComponent(sid) +
        '&text=' + encodeURIComponent(text || '') + '&token=' + encodeURIComponent(L.token));
      sent();
      return;
    }
    ui.inboxCli[key] = cliCmd(kind, pid, sid, text);
    T.render();
  }
  function focusAsk(key) {
    var els = document.querySelectorAll('[data-act="itext"]'), i, n;
    for (i = 0; i < els.length; i++) {
      if (els[i].getAttribute('data-key') === key) { n = els[i]; n.focus(); n.setSelectionRange(n.value.length, n.value.length); }
    }
  }
  function submitAsk(key) {
    var text = (ui.inboxDraft[key] || '').trim();
    if (!text) return;
    ui.inboxDraft[key] = ''; ui.inboxAsk[key] = false;
    sendInbox('ask', key, text);
  }

  function step(s, p, state) {
    var key = p.id + '|' + s.id, open = !!ui.planOpen[key], live = s.agents && s.agents.length;
    var ava = (s.agents || []).map(function (id) {
      var a = T.agentById(state, id); if (!a) return '';
      return '<button class="pava" data-act="goagent" data-key="' + esc(id) + '" title="' + esc(a.name + ' · ' + (a.feature || '')) + '">' + T.avatar(a, 22) + '</button>';
    }).join('');
    var h = '<li class="pstep ' + esc(s.status) + (open ? ' open' : '') + '"><div class="prow">' +
      '<button class="ptog" data-act="plan" data-key="' + esc(key) + '" aria-expanded="' + open + '">' +
      '<span class="pico' + (live ? ' pulse' : '') + '">' + (IC[s.status] || IC.todo) + '</span>' +
      '<span class="ptit"><span class="pn">' + esc(s.id) + '.</span> ' + plain(s.title) + '</span>' +
      (s.total ? bar(s.done, s.total, 'pbar') + '<span class="pcnt">' + s.done + '/' + s.total + '</span>' : '<span class="pbar"></span><span class="pcnt">–</span>') +
      '</button><span class="pavas">' + ava + '</span>' + inboxBtns(key) + '</div>' + inboxBelow(s, p, state, key);
    if (open) {
      h += '<ul class="pitems">' + (s.items.length ? s.items.map(function (i) {
        return '<li class="' + (i.done ? 'ok' : '') + '"><span class="pi">' + (i.done ? OK : OPEN) + '</span><span>' + rich(i.text) + '</span></li>';
      }).join('') : '<li class="none">No items yet</li>') + '</ul>';
      h += launchBtn(s, p, state, true).replace('class="plw"', 'class="plw pbody"');
    }
    return h + '</li>';
  }

  function card(p, state, multi) {
    var r = p.roadmap;
    return '<div class="card pcard' + (multi ? ' compact' : '') + '"><div class="phead"><b>' + esc(p.name) + '</b>' +
      bar(r.done, r.total, 'ptotal') + '<span class="pcnt big">' + r.done + ' / ' + r.total + '</span>' + countdown(r.deadline) + '</div>' +
      '<ul class="psteps">' + r.steps.map(function (s) { return step(s, p, state); }).join('') + '</ul></div>';
  }

  /* HTML for the Plan section, or '' when no visible project has a roadmap. */
  T.plan = function (state) {
    lastState = state;
    var f = T.filter.project, ps = (state.projects || []).filter(function (p) {
      return p.roadmap && p.roadmap.steps && (!f || p.id === f);
    });
    if (!ps.length) return '';
    var n = openCount(ps, state);
    return '<section class="plan"><h2>Plan' + (n ? ' <span class="icount" title="Requests the steward has not answered yet">' + n + ' open</span>' : '') + '</h2>' + ps.map(function (p) { return card(p, state, ps.length > 1); }).join('') + '</section>';
  };

  function openLink(url) {
    var a = document.createElement('a'); a.href = url; a.style.display = 'none';
    document.body.appendChild(a); a.click(); document.body.removeChild(a);
  }

  document.getElementById('view').addEventListener('click', function (e) {
    var l = e.target.closest('[data-act="launch"]');
    if (l) {
      var lk = l.getAttribute('data-key'), parts = lk.split('|'), st = lastState;
      var L = st && st.launch; if (!L) return;
      if (!L.available) { ui.launchHint = ui.launchHint === lk ? null : lk; T.render(); return; }
      ui.launchHint = null; ui.launching[lk] = T.now();
      openLink('tirith://launch?project=' + encodeURIComponent(parts[0]) + '&step=' + encodeURIComponent(parts[1]) + '&token=' + encodeURIComponent(L.token));
      T.render();
      setTimeout(T.render, LAUNCH_WAIT + 500);
      return;
    }
    var ib = e.target.closest('[data-act^="i"]'), act = ib && ib.getAttribute('data-act'), ik = ib && ib.getAttribute('data-key');
    if (act === 'ifinish') { sendInbox('finish', ik, ''); return; }
    if (act === 'iask') { ui.inboxAsk[ik] = !ui.inboxAsk[ik]; T.render(); if (ui.inboxAsk[ik]) focusAsk(ik); return; }
    if (act === 'isend') { submitAsk(ik); return; }
    if (act === 'iatog') {
      var cur = (lastState.inbox || []).filter(function (r) { return r.id === ik; })[0];
      ui.inboxOpen[ik] = !(ik in ui.inboxOpen ? ui.inboxOpen[ik] : !(cur && cur.status === 'done'));
      T.render(); return;
    }
    var b = e.target.closest('[data-act="plan"]'); if (!b) return;
    var k = b.getAttribute('data-key');
    ui.planOpen[k] = !ui.planOpen[k];
    T.render();
  });
  document.getElementById('view').addEventListener('input', function (e) {
    var t = e.target; if (t.getAttribute && t.getAttribute('data-act') === 'itext') ui.inboxDraft[t.getAttribute('data-key')] = t.value;
  });
  document.getElementById('view').addEventListener('keydown', function (e) {
    var t = e.target; if (!t.getAttribute || t.getAttribute('data-act') !== 'itext') return;
    var key = t.getAttribute('data-key');
    if (e.key === 'Enter' && !e.isComposing) { e.preventDefault(); ui.inboxDraft[key] = t.value; submitAsk(key); }
    else if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); ui.inboxAsk[key] = false; T.render(); }
  });
  setInterval(tickCountdowns, 30000);
})();
