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
  ui.planView = ui.planView || {};     /* project id -> id of the plan shown (default: the current one) */
  ui.planDoneOpen = ui.planDoneOpen || {}; /* project id -> true while the finished plans are unfolded */
  ui.planForm = ui.planForm || {};     /* project id -> true while the "Start a new plan" form is open */
  ui.planDraft = ui.planDraft || {};   /* project id -> {title, goal, deadline, steps} typed so far */
  ui.planFormErr = ui.planFormErr || {};
  ui.planRefreshAt = ui.planRefreshAt || {}; /* project id -> ms of the "Update the plan" click, until the request shows up */
  ui.inboxEarlier = ui.inboxEarlier || {}; /* "project|step" -> true while its older messages are unfolded */
  ui.planHist = ui.planHist || {};     /* "project|step" -> true while the step's History is unfolded */
  ui.pedit = ui.pedit || null;         /* {pid, plan, sid, text (null: a new item), draft}: the item being edited or added */
  ui.pdel = ui.pdel || null;           /* {pid, plan, sid, text}: the item whose "Delete?" popover is open */
  ui.ppend = ui.ppend || [];           /* plan changes sent, until the plan file shows them: {pid, plan, sid, kind, text, to, at, err} */
  ui.autoWant = ui.autoWant || null;   /* { on, at }: the switch the owner just sent, until the collector shows it */
  var lastState = null, LAUNCH_WAIT = 90000, PEND_WAIT = 10000, ESCALATE = 120, INSTALL = 'tirith url install';

  var IC = {
    done: '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="7" fill="#2ea44f"/><path d="M4.8 8.2l2.2 2.2 4.2-4.6" fill="none" stroke="#fff" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>',
    active: '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="6.2" fill="#fff" stroke="#2f6fed" stroke-width="1.6"/><path d="M8 3.4a4.6 4.6 0 010 9.2z" fill="#2f6fed"/></svg>',
    todo: '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="6.2" fill="none" stroke="#b4bac4" stroke-width="1.6"/></svg>',
    empty: '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="6.2" fill="none" stroke="#d3d7de" stroke-width="1.6" stroke-dasharray="2 2.4"/></svg>',
    ready: '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="6.2" fill="none" stroke="#2f6fed" stroke-width="1.6" stroke-dasharray="2 2.4"/><path d="M6.6 5.2l4 2.8-4 2.8z" fill="#2f6fed"/></svg>',
    'blocked-owner': '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><circle cx="8" cy="8" r="7" fill="#f0b429"/><circle cx="8" cy="6.2" r="2" fill="#fff"/><path d="M4.2 12c.3-2 1.9-3 3.8-3s3.5 1 3.8 3z" fill="#fff"/></svg>',
    'blocked-date': '<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"><rect x="2.4" y="3.4" width="11.2" height="10.2" rx="2" fill="none" stroke="#8a93a3" stroke-width="1.5"/><path d="M2.4 6.8h11.2M5.6 2v2.6M10.4 2v2.6" stroke="#8a93a3" stroke-width="1.5" stroke-linecap="round"/></svg>'
  };
  var OK = '<svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"><path d="M2.6 6.3l2.3 2.3 4.5-4.9" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>';
  var OPEN = '<svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true"><circle cx="6" cy="6" r="3.6" fill="none" stroke="#b4bac4" stroke-width="1.3"/></svg>';

  var ROCKET = '<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M9.6 2.2c2.1-.4 3.8-.3 4.2-.2.1.4.2 2.1-.2 4.2-.3 1.5-1.3 2.9-2.5 3.8l-.9.7-2.6-2.6.7-.9c.9-1.2 2.3-2.2 3.8-2.5z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/><circle cx="10.9" cy="5.1" r="1" fill="currentColor"/><path d="M5.9 6.5l-2.2.4-1.5 2.4 2.5.1M9.5 10.1l-.4 2.2-2.4 1.5-.1-2.5M4.6 11.4c-.8.2-1.6 1-1.9 1.9.9-.3 1.7-1.1 1.9-1.9z" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/></svg>';

  var FLAG = '<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M4 14V2.6M4 3.2c2.3-1.3 3.8 1.3 6.3 0 .9-.5 1.6-.6 2-.5v5.3c-.4-.1-1.1 0-2 .5-2.5 1.3-4-1.3-6.3 0" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>';
  var BUBBLE = '<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M3 3h10a1 1 0 011 1v6a1 1 0 01-1 1H7.5L4.6 13.6V11H3a1 1 0 01-1-1V4a1 1 0 011-1z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/></svg>';
  var REFRESH = '<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M13.2 8a5.2 5.2 0 11-1.6-3.7M13.4 2.4v3.1h-3.1" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></svg>';
  var PLUS = '<svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M8 3.2v9.6M3.2 8h9.6" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>';
  var PEN = '<svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true"><path d="M10.6 2.8l2.6 2.6-7.4 7.4-3.1.5.5-3.1z" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round"/></svg>';
  var TRASH = '<svg viewBox="0 0 16 16" width="13" height="13" aria-hidden="true"><path d="M3 4.4h10M6.4 4.4V3h3.2v1.4M4.2 4.4l.6 8.4h6.4l.6-8.4M6.7 7v3.8M9.3 7v3.8" fill="none" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/></svg>';
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

  /* The "Review and merge" button: the newest lane `tirith launch` started for this step that is done and not merged. */
  function reviewLane(s, p, state) {
    var rows = (state.agents || []).filter(function (a) {
      return a.project === p.id && String(a.step) === String(s.id) && a.review && a.review.status !== 'dropped' && a.status === 'done';
    });
    return rows.length ? rows[rows.length - 1] : null;
  }
  function reviewBtn(s, p, state) {
    var L = state.launch, a = reviewLane(s, p, state);
    if (!L || !L.token || !a) return '';
    var r = a.review, key = 'review|' + a.id, t = ui.launching[key];
    if (r.status !== 'pending' && r.status !== 'ready') return '<span class="plw pbody"></span>';
    var busy = t && T.now() - t < LAUNCH_WAIT && r.status === 'pending';
    if (t && r.status !== 'pending') delete ui.launching[key];
    var h = '<span class="plw pbody">';
    h += '<button class="plaunch long' + (busy ? ' busy' : '') + '" data-act="review" data-key="' + esc(a.id) + '"' + (busy ? ' disabled' : '') +
      ' title="Run the tests, then merge the work of ' + esc(a.name) + ' into the project branch">' + ROCKET +
      '<span>' + (busy ? 'Testing…' : r.status === 'pending' ? 'Review and merge' : 'Merge') + '</span></button>';
    if (ui.launchHint === key && !L.available) {
      h += '<span class="pltip" role="tooltip">Tirith needs its link handler once. Run in a terminal:<code>' + INSTALL + '</code>then click again.</span>';
    }
    return h + '</span>';
  }
  /* A merged lane, for the step's History. Failures and dropped lanes never show in the Plan. */
  function reviewLine(s, p, state) {
    var a = reviewLane(s, p, state), r = a && a.review;
    if (!r || r.status !== 'merged') return '';
    return '<div class="prv ok">Merged <b>' + esc(a.name) + '</b> · ' + esc(T.clockTime(r.at)) +
      (r.files != null ? ' <span class="mut">· ' + esc(r.files) + ' files' + (r.passed != null ? ', ' + esc(r.passed) + ' tests passed' : '') + '</span>' : '') + '</div>';
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
      (a.fallback_from ? ' <span class="mut">· continued from ' + esc(a.fallback_from) + '</span>' : '') +
      ' <span class="ist">' + T.dot(AST[a.status] || 'amber', a.status) + esc(a.status) + '</span>' + tbadge(a) +
      (a.status === 'working' && a.summary ? '<div class="inow">' + T.mdInline(a.summary) + '</div>' : '') + '</div></li>';
  }
  function answerLabel(a) {
    return a.by === 'brain' ? (a.auto ? 'Muse Spark answered automatically' : 'Muse Spark answered') : a.by === 'tirith' ? 'Tirith answered' : 'Manager answered';
  }
  /* One request as a timeline: asked -> answered -> handled by an agent -> done. Only from state.inbox. */
  function thread(r, state, older) {
    var open = r.id in ui.inboxOpen ? ui.inboxOpen[r.id] : r.status !== 'done' && !older, answers = r.answers || [], lanes = r.lanes || [], h;
    /* a "finish" click needs no row of its own (the answers below say what happened); a question keeps its text */
    h = '<div class="ithread" data-req="' + esc(r.id) + '"><ol class="itl">' + (r.kind === 'ask' ?
      '<li class="ie"><i class="ido me"></i><div class="ib"><b>You asked: ' + plain(r.text) + '</b> <span class="iwhen">· ' + esc(T.clockTime(r.at)) + '</span></div></li>' :
      r.kind === 'plan-refresh' ? '<li class="ie"><i class="ido me"></i><div class="ib"><b>You asked to update the plan</b> <span class="iwhen">· ' + esc(T.clockTime(r.at)) + '</span></div></li>' :
      r.kind === 'plan-edit' ? '<li class="ie"><i class="ido me"></i><div class="ib"><b>' + editWords(r) + '</b> <span class="iwhen">· ' + esc(T.clockTime(r.at)) + '</span></div></li>' :
      r.kind === 'plan-new' ? '<li class="ie"><i class="ido me"></i><div class="ib"><b>You started a new plan' + (r.plan && r.plan.title ? ': ' + plain(r.plan.title) : '') + '</b> <span class="iwhen">· ' + esc(T.clockTime(r.at)) + '</span></div></li>' : '');
    answers.forEach(function (a) {
      h += '<li class="ie"><i class="ido ok"></i><div class="ib ians' + (open ? ' open' : '') + '"><button class="iahead" data-act="iatog" data-key="' + esc(r.id) + '" aria-expanded="' + open + '">' + CHEV +
        '<b>' + esc(answerLabel(a)) + '</b> <span class="iwhen">· ' + esc(T.clockTime(a.at)) + '</span></button>' +
        (open ? '<div class="ia md">' + T.md(a.text) + '</div>' + T.actions(a.actions, state, r.project, true) : '') + '</div></li>';
    });
    lanes.forEach(function (id) {
      var ag = T.agentById(state, id), named = answers.some(function (a) { return a.lane === id; });
      if (!named && ag && ag.started_at) h += '<li class="ie"><i class="ido ok"></i><div class="ib"><b>Tirith started an agent</b> <span class="iwhen">· ' + esc(T.clockTime(ag.started_at)) + '</span></div></li>';
      h += handled(id, state);
    });
    if (!answers.length && !lanes.length && r.status === 'open') {
      h += '<li class="ie"><i class="ido wait"></i><div class="ib iwait">Waiting for the manager · <span class="tick" data-start="' + esc(r.at) + '">' + esc(T.dur(T.elapsed(r.at))) + '</span>' +
        (T.elapsed(r.at) >= ESCALATE ? '<div class="inow">No answer yet. Tirith will escalate this.</div>' : '') + '</div></li>';
    }
    if (r.status === 'done') h += '<li class="ie"><i class="ido ok"></i><div class="ib"><b>Done</b></div></li>';
    return h + '</ol></div>';
  }
  /* The "sent" chip, the send error and the CLI fallback of a key ("project|step"). */
  function notes(key, reqs) {
    var pend = ui.inboxPend[key], h = '';
    if (pend && reqs.some(function (r) { return (T.ms(r.at) || 0) >= pend - 2000; })) { delete ui.inboxPend[key]; pend = null; }
    if (pend && T.now() - pend < PEND_WAIT) h += '<div class="ichip-row"><span class="ichip" title="Sent; it shows here as soon as the collector has it">Sent · waiting for the manager</span></div>';
    if (ui.inboxErr[key]) h += '<div class="ierr">' + esc(ui.inboxErr[key]) + '</div>';
    if (ui.inboxCli[key]) {
      h += '<div class="icli">Tirith cannot send from this page. Run in a terminal:<code>' + esc(ui.inboxCli[key]) + '</code></div>';
    }
    return h;
  }
  function inboxBelow(s, p, state, key) {
    var reqs = reqsFor(state, p.id, s.id), h = notes(key, reqs);
    if (ui.inboxAsk[key]) {
      h += '<div class="iask"><input class="iin" type="text" data-act="itext" data-key="' + esc(key) + '" maxlength="1000" placeholder="Ask about this step…" aria-label="Question about this step" value="' + esc(ui.inboxDraft[key] || '') + '">' +
        '<button class="isend" data-act="isend" data-key="' + esc(key) + '">Send</button><span class="ihint">Enter sends · Esc closes</span></div>';
    }
    return h;
  }
  /* "Autopilot started <agent>" with the agent's live status */
  function autoStarted(rec, state) {
    var a = T.agentById(state, rec.lane);
    return '<div class="ithread"><ol class="itl"><li class="ie"><i class="ido ' + (a ? (a.status === 'working' ? 'live' : a.status === 'failed' ? 'bad' : 'ok') : 'ok') + '"></i><div class="ib"><b>Autopilot started</b> ' +
      (a ? '<button class="iag" data-act="goagent" data-key="' + esc(a.id) + '" title="Open this agent">' + T.avatar(a, 18) + '<b>' + esc(a.name) + '</b></button>' : '<b>' + esc(rec.lane) + '</b>') +
      ' <span class="iwhen">· ' + esc(T.clockTime(rec.at)) + '</span>' +
      (a ? ' <span class="ist">' + T.dot(AST[a.status] || 'amber', a.status) + esc(a.status) + '</span>' + tbadge(a) +
        (a.status === 'working' && a.summary ? '<div class="inow">' + T.mdInline(a.summary) + '</div>' : '') : '') + '</div></li></ol></div>';
  }
  /* The thread under a step: the latest entry stays open, older ones fold into "N earlier messages". */
  function entries(s, p, state, key, reqs, all) {
    var list = reqs.map(function (r) {
      var last = (r.answers || []).length ? r.answers[r.answers.length - 1].at : r.at;
      return { at: T.ms(last) || 0, r: r };
    });
    ((state.autopilot && state.autopilot.started) || []).forEach(function (rec) {
      if (rec.project === p.id && String(rec.step) === String(s.id)) list.push({ at: T.ms(rec.at) || 0, rec: rec });
    });
    list.sort(function (a, b) { return b.at - a.at; });
    if (!list.length) return '';
    function html(e, older) { return e.r ? thread(e.r, state, older) : autoStarted(e.rec, state); }
    if (all) return list.map(function (e) { return html(e, false); }).join('');
    var h = html(list[0], false), rest = list.slice(1, 9), n = 0, body = '', open = !!ui.inboxEarlier[key];
    rest.forEach(function (e) {
      var t = html(e, true); n += t.split('<li class="ie">').length - 1;
      if (open) body += t;
    });
    if (!rest.length) return h;
    return h + '<div class="iearlier"><button class="iahead" data-act="iearlier" data-key="' + esc(key) + '" aria-expanded="' + open + '">' + CHEV +
      '<span>' + n + ' earlier message' + (n === 1 ? '' : 's') + '</span></button></div>' + (open ? '<div class="iearlier-body">' + body + '</div>' : '');
  }
  function openCount(ps, state) {
    var ids = {}; ps.forEach(function (p) { ids[p.id] = 1; });
    return (state.inbox || []).filter(function (r) { return r.status === 'open' && r.kind !== 'setup' && r.kind !== 'autopilot' && ids[r.project]; }).length;
  }

  /* Record a request: the app writes the inbox itself, a browser opens the tirith:// link, else the CLI line is shown. */
  function sendInbox(kind, key, text, opt) {
    opt = opt || {};
    var parts = key.split('|'), pid = parts[0], sid = parts[1], st = lastState, L = st && st.launch, tauri = window.__TAURI__;
    ui.inboxErr[key] = null; ui.inboxCli[key] = null;
    function sent() { if (!opt.quiet) ui.inboxPend[key] = T.now(); T.render(); setTimeout(T.render, PEND_WAIT + 500); }
    if (tauri && tauri.core && tauri.core.invoke) {
      tauri.core.invoke('inbox_add', { kind: kind, project: pid, step: sid, text: text || '' }).then(sent, function (err) {
        ui.inboxErr[key] = 'Could not send: ' + err; if (opt.fail) opt.fail(); T.render();
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
    if (opt.fail) opt.fail();
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

  function focusForm(pid) {
    var els = document.querySelectorAll('[data-act="pnf"][data-field="title"]'), i;
    for (i = 0; i < els.length; i++) if (els[i].getAttribute('data-key') === pid) els[i].focus();
  }
  function submitPlan(pid) {
    var d = ui.planDraft[pid] || {}, title = (d.title || '').trim();
    if (!title) { ui.planFormErr[pid] = 'Give the plan a title first.'; T.render(); focusForm(pid); return; }
    ui.planDraft[pid] = {}; ui.planForm[pid] = false; ui.planFormErr[pid] = null;
    sendInbox('plan-new', pid + '|plan', JSON.stringify({ title: title, goal: (d.goal || '').trim(), deadline: (d.deadline || '').trim(), steps: (d.steps || '').trim() }));
  }

  var MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  function shortDate(iso) {
    var m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso || '');
    return m ? MONTHS[+m[2] - 1] + ' ' + (+m[3]) : String(iso || '');
  }
  /* ---------- the quiet step: open items and one state line; the rest is behind "History" ---------- */
  function dateless(t) { return t.replace(/\s*\((?:desde|from)\s+\d{4}-\d{2}-\d{2}[^)]*\)?/i, ''); }
  /* ---------- editing the plan: Edit / Delete on an open item, Add item; Tirith applies it and re-checks the plan ---------- */
  function clip(t, n) { t = String(t || ''); return t.length > n ? t.slice(0, n - 1) + '…' : t; }
  function editWords(r) {
    var e = r.edit || {};
    return e.delete ? 'You deleted: ' + plain(clip(e.item_text, 90)) : e.add ? 'You added: ' + plain(clip(e.add, 90)) :
      'You edited: ' + plain(clip(e.item_text, 60)) + ' → ' + plain(clip(e.new_text, 60));
  }
  function same(a, c) { return !!a && a.pid === c.pid && a.plan === c.plan && String(a.sid) === String(c.sid); }
  function ctxAttrs(c, text) {
    return ' data-pid="' + esc(c.pid) + '" data-plan="' + esc(c.plan) + '" data-sid="' + esc(c.sid) + '"' + (text != null ? ' data-text="' + esc(text) + '"' : '');
  }
  function ctxOf(el) { return { pid: el.getAttribute('data-pid'), plan: el.getAttribute('data-plan'), sid: el.getAttribute('data-sid'), text: el.getAttribute('data-text') }; }
  function tool(act, cls, ic, label, c, text) {
    return '<button class="pedbtn ' + cls + '" data-act="' + act + '"' + ctxAttrs(c, text) + ' title="' + label + '" aria-label="' + label + '">' + ic + '</button>';
  }
  /* Changes sent and not yet in the plan file; one that landed (or was refused) is settled here. */
  function pending(c, s, state) {
    var items = s.items.map(function (i) { return i.text; });
    ui.ppend = ui.ppend.filter(function (r) {
      if (!same(r, c)) return true;
      if (r.kind === 'add' ? items.indexOf(r.to) >= 0 : items.indexOf(r.text) < 0) return false;
      if (!r.err) {
        (state.inbox || []).forEach(function (q) {
          var e = q.edit || {}, a = (q.answers || [])[0];
          if (q.kind === 'plan-edit' && !r.err && q.project === r.pid && String(q.step) === String(r.sid) && (T.ms(q.at) || 0) >= r.at - 2000 &&
              (e.item_text || '') === (r.text || '') && (e.add || e.new_text || '') === (r.to || '') && a && /^Could not/.test(a.text)) r.err = a.text;
        });
      }
      return !!r.err || T.now() - r.at < 120000;
    });
    return ui.ppend.filter(function (r) { return same(r, c); });
  }
  function editInput(c, e, key) {
    return '<li class="pediting"><span class="pi">' + OPEN + '</span><input class="iin pedin" type="text" data-act="pe-in" data-key="' + esc(key) + '" maxlength="500" aria-label="' + (e.text === null ? 'New item' : 'Edit item') +
      '" placeholder="' + (e.text === null ? 'What needs doing?' : '') + '" value="' + esc(e.draft) + '"><span class="ihint">Enter saves · Esc cancels</span></li>';
  }
  function itemRow(i, c, pend, quiet) {
    var key = c.pid + '|' + c.plan + '|' + c.sid, e = same(ui.pedit, c) && ui.pedit.text === i.text ? ui.pedit : null, p = null, txt = i.text, h, extra;
    if (e) return editInput(c, e, key);
    pend.forEach(function (r) { if (r.kind !== 'add' && r.text === i.text) p = r; });
    if (p && p.kind === 'edit') txt = p.to;
    extra = p ? '<span class="pwhen saving">' + (p.err ? 'Not saved' : 'Saving…') + '</span>' :
      i.who === 'owner' && i.owner_item && quiet ? '<button class="pitbtn" data-act="goowner" data-key="' + esc(i.owner_item) + '" title="Open what Tirith needs from you">Open</button>' :
      i.who === 'date' && i.until && quiet ? '<span class="pwhen">from ' + esc(shortDate(i.until)) + '</span>' : '';
    h = '<li class="' + (p ? 'psav' + (p.kind === 'delete' ? ' gone' : '') : '') + (i.done ? ' ok' : '') + '"><span class="pi">' + (i.done ? OK : OPEN) + '</span><span class="pit">' + rich(quiet && i.who === 'date' ? dateless(txt) : txt) + '</span>' + extra;
    if (!i.done && !p && c.edit) h += '<span class="pedtools">' + tool('pe-edit', '', PEN, 'Edit', c, i.text) + tool('pe-del', 'del', TRASH, 'Delete', c, i.text) + '</span>';
    if (same(ui.pdel, c) && ui.pdel.text === i.text) {
      h += '<span class="pdelpop" role="alertdialog" aria-label="Delete this item from the plan?"><span>Delete this item from the plan?</span>' +
        '<button class="isend danger" data-act="pe-yes"' + ctxAttrs(c, i.text) + '>Delete</button><button class="ibtn" data-act="pe-no">Cancel</button></span>';
    }
    return h + '</li>';
  }
  /* The rows after the open items: items being added, the add box, the "Add item" link, a refused change. */
  function itemTail(c, pend) {
    var key = c.pid + '|' + c.plan + '|' + c.sid, h = '';
    pend.forEach(function (r) {
      if (r.kind === 'add') h += '<li class="psav"><span class="pi">' + OPEN + '</span><span class="pit">' + rich(r.to) + '</span><span class="pwhen saving">' + (r.err ? 'Not saved' : 'Saving…') + '</span></li>';
    });
    if (same(ui.pedit, c) && ui.pedit.text === null) h += editInput(c, ui.pedit, key);
    else h += '<li class="padd"><button class="pedadd" data-act="pe-add"' + ctxAttrs(c) + ' title="Add an item to this step">' + PLUS + '<span>Add item</span></button></li>';
    pend.forEach(function (r) {
      if (r.err) h += '<li class="perr"><div class="ierr">' + esc(r.err) + ' <button class="link" data-act="pe-dismiss"' + ctxAttrs(c, r.text) + '>Dismiss</button></div></li>';
    });
    return h;
  }
  function openItems(s, c, state) {
    var l = s.items.filter(function (i) { return !i.done && i.who !== 'out'; }), pend = c.edit ? pending(c, s, state) : [];
    if (!l.length && !(same(ui.pedit, c) && ui.pedit.text === null) && !pend.length) return '';
    return '<ul class="pitems pquiet">' + l.map(function (i) { return itemRow(i, c, pend, true); }).join('') + (c.edit ? itemTail(c, pend) : '') + '</ul>';
  }
  function allItems(s, c, state) {
    var pend = c.edit ? pending(c, s, state) : [];
    return '<ul class="pitems">' + (s.items.length ? s.items.map(function (i) { return itemRow(i, c, pend, false); }).join('') : '<li class="none">No items yet</li>') +
      (c.edit ? itemTail(c, pend) : '') + '</ul>';
  }
  /* The ONE current state of an unfinished step, or ''. */
  function stateLine(s, p, state) {
    var rl = reviewLane(s, p, state), r = rl && rl.review, auto = state.autopilot, on = auto ? autoOn(state) : true, a, owner, h, skip;
    if (r && (r.status === 'tests-failed' || r.status === 'merge-failed' || r.status === 'claims-failed' || r.status === 'failed')) {
      return '<div class="pstate wait">The manager is resolving a merge</div>';
    }
    if (r && r.status === 'running') {
      return '<div class="pstate live"><i class="pulse"></i><button class="link" data-act="gorun" data-key="' + esc(r.run || '') + '"' + (r.run ? '' : ' disabled') +
        ' title="Open the test run">Testing <b>' + esc(rl.name) + '</b>…</button></div>';
    }
    if (s.agents && s.agents.length) {
      a = T.agentById(state, s.agents[0]);
      if (a) {
        return '<div class="pstate live">Working: <button class="iag" data-act="goagent" data-key="' + esc(a.id) + '" title="Open this agent">' + T.avatar(a, 18) + '<b>' + esc(a.name) + '</b></button>' +
          (s.agents.length > 1 ? ' <span class="mut">+' + (s.agents.length - 1) + '</span>' : '') +
          ' <span class="mut">·</span> ' + (a.summary ? T.mdInline(a.summary) : '<span class="mut">' + plain(a.feature || 'starting') + '</span>') + '</div>';
      }
    }
    if (s.status === 'blocked-owner') {
      owner = (s.items.filter(function (i) { return i.who === 'owner' && !i.done && i.owner_item; })[0] || {}).owner_item;
      return '<div class="pstate wait">Waiting for you: <button class="pitbtn" data-act="goowner" data-key="' + esc(owner || '') + '"' + (owner ? '' : ' disabled') + ' title="Open what Tirith needs from you">See what</button></div>';
    }
    if (s.status === 'blocked-date') return '<div class="pstate">From ' + esc(shortDate(s.until)) + '</div>';
    if (s.status === 'ready' && on && (skip = skipOf(state, p, s))) return '<div class="pstate">Not started: <span class="mut">' + esc(skip.why) + '</span></div>';
    if (s.status === 'ready') return '<div class="pstate ready">' + (on ? 'Queued for the autopilot' : 'Waiting for the autopilot <span class="mut">(paused)</span>') + '</div>';
    return '';
  }
  /* A request is pending until the manager answered it and every action it proposed has passed. */
  function unpassed(r) {
    var todo = [];
    (r.answers || []).forEach(function (a) {
      (a.actions || []).forEach(function (x) { if (!x.last_run || x.last_run.status !== 'passed') todo.push(x); });
    });
    return todo;
  }
  function unanswered(r) { return !(r.answers || []).length && !(r.lanes || []).length && r.status === 'open'; }
  function isPending(r) { return unanswered(r) || unpassed(r).length > 0; }
  /* Only pending requests are visible, one line each; answered and closed threads live in History. */
  function needsLines(s, p, state, reqs) {
    var h = '';
    reqs.forEach(function (r) {
      var todo = unpassed(r);
      if (unanswered(r) && r.kind !== 'plan-edit') {   /* Tirith applies a plan change itself: the item shows "Saving…" */
        h += '<div class="pstate wait">Waiting for the manager · <span class="tick" data-start="' + esc(r.at) + '">' + esc(T.dur(T.elapsed(r.at))) + '</span>' +
          (T.elapsed(r.at) >= ESCALATE ? ' <span class="mut">No answer yet. Tirith will escalate this.</span>' : '') + '</div>';
      }
      if (todo.length) h += '<div class="pneed">' + T.actions(todo, state, r.project) + '</div>';
    });
    return h;
  }
  function historyBlock(s, p, state, key, reqs) {
    var rl = reviewLane(s, p, state), r = rl && rl.review, open = !!ui.planHist[key], body = entries(s, p, state, key, reqs.filter(function (r) { return !isPending(r); }), true), n = (body.match(/<li class="ie">/g) || []).length, done, past;
    past = r && (r.status === 'merged' || r.status === 'dropped') ? reviewLine(s, p, state) : '';
    if (past) n++;
    if (!n) return '';
    done = '<div class="phist"><button class="iahead" data-act="phist" data-key="' + esc(key) + '" aria-expanded="' + open + '">' + CHEV + '<span>History (' + n + ')</span></button></div>';
    return done + (open ? '<div class="phistbody">' + past + body + '</div>' : '');
  }

  function step(s, p, state, ro) {
    var key = (ro ? ro + '|' : '') + p.id + '|' + s.id, open = !!ui.planOpen[key], live = s.agents && s.agents.length, fin = s.status === 'done';
    var ava = fin ? '' : (s.agents || []).map(function (id) {
      var a = T.agentById(state, id); if (!a) return '';
      return '<button class="pava" data-act="goagent" data-key="' + esc(id) + '" title="' + esc(a.name + ' · ' + (a.feature || '')) + '">' + T.avatar(a, 22) + '</button>' + (T.stopBtn ? T.stopBtn(a, 'ava') : '');
    }).join('');
    var reqs = ro ? [] : reqsFor(state, p.id, s.id), c = { pid: p.id, plan: (plansOf(p)[0] || {}).id || '', sid: s.id, edit: !ro && !fin };
    var h = '<li class="pstep ' + esc(s.status) + (open ? ' open' : '') + '"><div class="prow">' +
      '<button class="ptog" data-act="plan" data-key="' + esc(key) + '" aria-expanded="' + open + '">' +
      '<span class="pico' + (live ? ' pulse' : '') + '">' + (IC[s.status] || IC.todo) + '</span>' +
      '<span class="ptit"><span class="pn">' + esc(s.id) + '.</span> ' + plain(s.title) + '</span>' +
      (fin ? '' : s.total ? bar(s.done, s.total, 'pbar') : '<span class="pbar"></span>') +
      (s.total ? '<span class="pcnt">' + s.done + '/' + s.total + '</span>' : '<span class="pcnt">–</span>') +
      '</button><span class="pavas">' + ava + '</span>' + (c.edit ? '<span class="pedtools pstit">' + tool('pe-add', '', PLUS, 'Add item', c) + '</span>' : '') + (ro || fin ? '' : inboxBtns(key)) + '</div>';
    if (!ro) {
      h += inboxBelow(s, p, state, key, reqs);
      if (!fin) h += (open ? '' : openItems(s, c, state)) + stateLine(s, p, state) + needsLines(s, p, state, reqs);
    }
    if (open) {
      h += allItems(s, c, state);
      if (!ro && !fin) h += reviewBtn(s, p, state) || launchBtn(s, p, state, true).replace('class="plw"', 'class="plw pbody"');
    }
    if (!ro) h += historyBlock(s, p, state, key, reqs);
    return h + '</li>';
  }

  function autoOn(state) {
    var w = ui.autoWant, on = !!(state.autopilot && state.autopilot.enabled);
    if (w && w.on === on) { ui.autoWant = null; return on; }
    return w && T.now() - w.at < PEND_WAIT ? w.on : on;
  }
  /* the autopilot's reason for not starting a ready step ({project, step, why}), or undefined */
  function skipOf(state, p, s) {
    return (((state.autopilot || {}).skipped) || []).filter(function (k) { return k.project === p.id && String(k.step) === String(s.id); })[0];
  }
  /* "Autopilot on · 2 agents working · 3 queued" with the pause / resume switch. */
  function autoBar(ps, state) {
    if (!state.autopilot) return '';
    var on = autoOn(state), n = state.autopilot.working || 0, queued = state.autopilot.queued || 0, bits = [];
    bits.push('<b>Autopilot ' + (on ? 'on' : 'paused') + '</b>');
    if (n) bits.push(n + (n === 1 ? ' agent' : ' agents') + ' working');
    if (queued) bits.push(queued + ' queued');
    var skipped = state.autopilot.skipped || [];
    if (skipped.length) bits.push(skipped.length + ' not started');
    if (!n && !queued) bits.push('nothing to start');
    return '<div class="pauto' + (on ? ' on' : '') + '"><i class="pdot3"></i><span>' + bits.join(' <span class="mut">·</span> ') + '</span>' +
      '<button class="ibtn" data-act="iauto" data-key="' + (on ? 'off' : 'on') + '" title="' + (on ? 'Stop starting agents by itself; running ones finish' : 'Start agents by itself on every step that is ready') + '">' +
      (on ? '<svg viewBox="0 0 12 12" width="11" height="11" aria-hidden="true"><path d="M3.5 2.5v7M8.5 2.5v7" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/></svg>' : '<svg viewBox="0 0 12 12" width="11" height="11" aria-hidden="true"><path d="M3.5 2.2l6 3.8-6 3.8z" fill="currentColor"/></svg>') +
      '<span>' + (on ? 'Pause' : 'Resume') + '</span></button>' + (ui.inboxErr['all|autopilot'] ? '<span class="ierr">' + esc(ui.inboxErr['all|autopilot']) + '</span>' : '') + '</div>';
  }

  /* ---------- the plans of a project: switcher, "Update the plan", "Start a new plan" ---------- */
  function plansOf(p) {
    var r = p.roadmap;
    return p.plans && p.plans.length ? p.plans : [{ id: '', title: '', current: true, finished: false, file: r.file, deadline: r.deadline, done: r.done, total: r.total, steps: r.steps }];
  }
  function shown(p) {
    var all = plansOf(p), want = ui.planView[p.id];
    return all.filter(function (x) { return x.id === want; })[0] || all[0];
  }
  function planReqs(state, pid) {
    return reqsFor(state, pid, 'plan').filter(function (r) { return r.kind === 'plan-refresh' || r.kind === 'plan-new'; });
  }
  function refreshing(state, pid) {
    var t = ui.planRefreshAt[pid];
    if (t && planReqs(state, pid).some(function (r) { return r.kind === 'plan-refresh' && (T.ms(r.at) || 0) >= t - 2000; })) { delete ui.planRefreshAt[pid]; t = 0; }
    return !!(t && T.now() - t < PEND_WAIT) || planReqs(state, pid).some(function (r) {
      return r.kind === 'plan-refresh' && r.status !== 'done' && (r.lanes || []).some(function (id) { var a = T.agentById(state, id); return a && a.status === 'working'; });
    });
  }
  function planBtns(p, state) {
    var key = p.id + '|plan', busy = refreshing(state, p.id), open = !!ui.planForm[p.id];
    return '<span class="iacts pacts">' +
      '<button class="ibtn' + (busy ? ' busy' : '') + '" data-act="prefresh" data-key="' + esc(key) + '"' + (busy ? ' disabled' : '') + ' title="An agent compares every item of the plan with the code, the merged work and your requests, ticks what is done and adds what is missing" aria-label="Update the plan">' + REFRESH + '<span>' + (busy ? 'Updating the plan…' : 'Update the plan') + '</span></button>' +
      '<button class="ibtn' + (open ? ' on' : '') + '" data-act="pnew" data-key="' + esc(p.id) + '" title="Describe a new goal; an agent writes the plan and it joins the plan switcher" aria-label="Start a new plan">' + PLUS + '<span>Start a new plan</span></button></span>';
  }
  function switcher(p, cur) {
    var all = plansOf(p), live = all.filter(function (x) { return !x.finished || x === cur; }), fin = all.filter(function (x) { return x.finished && x !== cur; });
    if (all.length < 2) return '';
    function chip(x) {
      return '<button class="pchip' + (x === cur ? ' on' : '') + (x.finished ? ' fin' : '') + '" data-act="pview" data-key="' + esc(p.id) + '" data-plan="' + esc(x.id) + '" aria-pressed="' + (x === cur) + '">' +
        plain(x.title || x.id) + (x.current ? ' <em>current</em>' : '') + ' <span class="pcnt">' + x.done + '/' + x.total + '</span></button>';
    }
    var h = '<div class="pswitch" role="group" aria-label="Plans of ' + esc(p.name) + '">' + live.map(chip).join('');
    if (fin.length) {
      h += '<button class="pchip fold" data-act="pdonetog" data-key="' + esc(p.id) + '" aria-expanded="' + !!ui.planDoneOpen[p.id] + '">' + CHEV + ' ' + fin.length + ' done</button>';
      if (ui.planDoneOpen[p.id]) h += fin.map(chip).join('');
    }
    return h + '</div>';
  }
  function newPlanForm(p) {
    var d = ui.planDraft[p.id] || {}, id = esc(p.id);
    function f(label, ctl) { return '<label class="pfl"><span>' + label + '</span>' + ctl + '</label>'; }
    return '<div class="pform" data-pform="' + id + '">' +
      f('Title', '<input class="iin" type="text" data-act="pnf" data-key="' + id + '" data-field="title" maxlength="120" placeholder="What is this plan called?" value="' + esc(d.title || '') + '">') +
      f('Goal', '<textarea class="iin" rows="3" data-act="pnf" data-key="' + id + '" data-field="goal" maxlength="1500" placeholder="One paragraph: what must be true when this plan is done?">' + esc(d.goal || '') + '</textarea>') +
      f('Deadline <i>optional</i>', '<input class="iin" type="date" data-act="pnf" data-key="' + id + '" data-field="deadline" value="' + esc(d.deadline || '') + '">') +
      f('First steps <i>optional</i>', '<textarea class="iin" rows="3" data-act="pnf" data-key="' + id + '" data-field="steps" maxlength="1500" placeholder="Free text, one idea per line. The agent turns it into steps.">' + esc(d.steps || '') + '</textarea>') +
      (ui.planFormErr[p.id] ? '<div class="ierr pferr">' + esc(ui.planFormErr[p.id]) + '</div>' : '') +
      '<div class="pfa"><button class="isend" data-act="pnsend" data-key="' + id + '">Start the plan</button><button class="ibtn" data-act="pncancel" data-key="' + id + '">Cancel</button></div></div>';
  }
  function planHistory(key, reqs, state) {
    var open = !!ui.planHist[key], body = reqs.filter(function (r) { return !isPending(r); })
      .sort(function (a, b) { return (T.ms(b.at) || 0) - (T.ms(a.at) || 0); }).map(function (r) { return thread(r, state, true); }).join(''),
      n = (body.match(/<li class="ie">/g) || []).length;
    if (!n) return '';
    return '<div class="phist"><button class="iahead" data-act="phist" data-key="' + esc(key) + '" aria-expanded="' + open + '">' + CHEV + '<span>History (' + n + ')</span></button></div>' +
      (open ? '<div class="phistbody">' + body + '</div>' : '');
  }
  function planBelow(p, state) {
    var key = p.id + '|plan', reqs = planReqs(state, p.id), h = notes(key, reqs);
    if (ui.planForm[p.id]) h += newPlanForm(p);
    h += needsLines(null, p, state, reqs);
    h += planHistory(key, reqs, state);
    return h ? '<div class="pbelow">' + h + '</div>' : '';
  }

  function card(p, state, multi) {
    var cur = shown(p), ro = cur.current ? '' : cur.id;
    return '<div class="card pcard' + (multi ? ' compact' : '') + '"><div class="phead"><b>' + esc(p.name) + '</b>' +
      bar(cur.done, cur.total, 'ptotal') + '<span class="pcnt big">' + cur.done + ' / ' + cur.total + '</span>' + countdown(cur.deadline) + planBtns(p, state) + '</div>' +
      switcher(p, cur) + planBelow(p, state) +
      '<ul class="psteps">' + cur.steps.map(function (s) { return step(s, p, state, ro); }).join('') + '</ul></div>';
  }

  /* HTML for the Plan section, or '' when no visible project has a roadmap. */
  T.plan = function (state) {
    lastState = state;
    var f = T.filter.project, ps = (state.projects || []).filter(function (p) {
      return p.roadmap && p.roadmap.steps && (!f || p.id === f);
    });
    if (!ps.length) return '';
    var n = openCount(ps, state);
    return '<section class="plan"><h2>Plan' + (n ? ' <span class="icount" title="Requests the manager has not answered yet">' + n + ' open</span>' : '') + '</h2>' + autoBar(ps, state) + ps.map(function (p) { return card(p, state, ps.length > 1); }).join('') + '</section>';
  };

  function openLink(url) {
    var a = document.createElement('a'); a.href = url; a.style.display = 'none';
    document.body.appendChild(a); a.click(); document.body.removeChild(a);
  }

  function focusPe() {
    var n = document.querySelector('[data-act="pe-in"]');
    if (n) { n.focus(); n.setSelectionRange(n.value.length, n.value.length); }
  }
  function submitPe() {
    var e = ui.pedit, v = e && (e.draft || '').replace(/\s+/g, ' ').trim();
    if (!e || !v) return;
    if (e.text !== null && v === e.text) { ui.pedit = null; T.render(); return; }
    queuePe(e, e.text === null ? { kind: 'add', to: v } : { kind: 'edit', text: e.text, to: v },
      e.text === null ? { plan: e.plan, add: v } : { plan: e.plan, item_text: e.text, new_text: v });
  }
  /* Send one change; the Plan shows it as "Saving…" until the plan file has it. */
  function queuePe(c, rec, payload) {
    rec.pid = c.pid; rec.plan = c.plan; rec.sid = c.sid; rec.text = rec.text || ''; rec.at = T.now();
    ui.ppend.push(rec); ui.pedit = null; ui.pdel = null;
    sendInbox('plan-edit', c.pid + '|' + c.sid, JSON.stringify(payload), { quiet: true, fail: function () { ui.ppend = ui.ppend.filter(function (r) { return r !== rec; }); } });
    T.render();
  }
  function planEditClick(e) {
    var b = e.target.closest('[data-act^="pe-"]'), act = b && b.getAttribute('data-act'), c;
    if (!b) return false;
    c = ctxOf(b);
    if (act === 'pe-edit') { ui.pdel = null; ui.pedit = { pid: c.pid, plan: c.plan, sid: c.sid, text: c.text, draft: c.text }; T.render(); focusPe(); }
    else if (act === 'pe-add') { ui.pdel = null; ui.pedit = { pid: c.pid, plan: c.plan, sid: c.sid, text: null, draft: '' }; T.render(); focusPe(); }
    else if (act === 'pe-del') { ui.pedit = null; ui.pdel = { pid: c.pid, plan: c.plan, sid: c.sid, text: c.text }; T.render(); }
    else if (act === 'pe-no') { ui.pdel = null; T.render(); }
    else if (act === 'pe-yes') queuePe(c, { kind: 'delete', text: c.text }, { plan: c.plan, item_text: c.text, delete: true });
    else if (act === 'pe-dismiss') { ui.ppend = ui.ppend.filter(function (r) { return !(same(r, c) && r.err); }); T.render(); }
    return true;
  }
  document.getElementById('view').addEventListener('click', function (e) {
    if (planEditClick(e)) return;
    var rv = e.target.closest('[data-act="review"]');
    if (rv) {
      var lid = rv.getAttribute('data-key'), rk = 'review|' + lid, rs = lastState, RL = rs && rs.launch;
      if (!RL) return;
      if (!RL.available) { ui.launchHint = ui.launchHint === rk ? null : rk; T.render(); return; }
      ui.launchHint = null; ui.launching[rk] = T.now();
      openLink('tirith://review?lane=' + encodeURIComponent(lid) + '&token=' + encodeURIComponent(RL.token));
      T.render();
      setTimeout(T.render, LAUNCH_WAIT + 500);
      return;
    }
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
    var go = e.target.closest('[data-act="goowner"]');
    if (go) {
      var wid = go.getAttribute('data-key');
      ui.needsOpen = ui.needsOpen || {}; ui.needsOpen[wid] = true; T.render();
      var card = document.querySelector('[data-wait="' + wid.replace(/"/g, '') + '"]');
      if (card) { card.scrollIntoView({ block: 'center' }); card.classList.add('flash'); }
      return;
    }
    var ib = e.target.closest('[data-act^="i"]'), act = ib && ib.getAttribute('data-act'), ik = ib && ib.getAttribute('data-key');
    if (act === 'ifinish') { sendInbox('finish', ik, ''); return; }
    if (act === 'iauto') { ui.autoWant = { on: ik === 'on', at: T.now() }; sendInbox('autopilot', 'all|autopilot', ik); return; }
    if (act === 'iearlier') { ui.inboxEarlier[ik] = !ui.inboxEarlier[ik]; T.render(); return; }
    if (act === 'iask') { ui.inboxAsk[ik] = !ui.inboxAsk[ik]; T.render(); if (ui.inboxAsk[ik]) focusAsk(ik); return; }
    if (act === 'isend') { submitAsk(ik); return; }
    if (act === 'iatog') {
      var cur = (lastState.inbox || []).filter(function (r) { return r.id === ik; })[0];
      ui.inboxOpen[ik] = !(ik in ui.inboxOpen ? ui.inboxOpen[ik] : !(cur && cur.status === 'done'));
      T.render(); return;
    }
    var hb = e.target.closest('[data-act="phist"]');
    if (hb) { ui.planHist[hb.getAttribute('data-key')] = !ui.planHist[hb.getAttribute('data-key')]; T.render(); return; }
    var pb = e.target.closest('[data-act^="p"]'), pact = pb && pb.getAttribute('data-act'), pk = pb && pb.getAttribute('data-key');
    if (pact === 'prefresh') { ui.planRefreshAt[pk.split('|')[0]] = T.now(); sendInbox('plan-refresh', pk, ''); return; }
    if (pact === 'pnew') { ui.planForm[pk] = !ui.planForm[pk]; ui.planFormErr[pk] = null; T.render(); if (ui.planForm[pk]) focusForm(pk); return; }
    if (pact === 'pncancel') { ui.planForm[pk] = false; ui.planFormErr[pk] = null; T.render(); return; }
    if (pact === 'pnsend') { submitPlan(pk); return; }
    if (pact === 'pview') { ui.planView[pk] = pb.getAttribute('data-plan'); T.render(); return; }
    if (pact === 'pdonetog') { ui.planDoneOpen[pk] = !ui.planDoneOpen[pk]; T.render(); return; }
    var b = e.target.closest('[data-act="plan"]'); if (!b) return;
    var k = b.getAttribute('data-key');
    ui.planOpen[k] = !ui.planOpen[k];
    T.render();
  });
  document.getElementById('view').addEventListener('input', function (e) {
    var t = e.target, a = t.getAttribute && t.getAttribute('data-act');
    if (a === 'itext') ui.inboxDraft[t.getAttribute('data-key')] = t.value;
    if (a === 'pe-in' && ui.pedit) ui.pedit.draft = t.value;
    if (a === 'pnf') {
      var pid = t.getAttribute('data-key'), d = ui.planDraft[pid] = ui.planDraft[pid] || {};
      d[t.getAttribute('data-field')] = t.value; ui.planFormErr[pid] = null;
    }
  });
  document.getElementById('view').addEventListener('keydown', function (e) {
    var t = e.target; if (!t.getAttribute) return;
    if (t.getAttribute('data-act') === 'pe-in') {
      if (e.key === 'Enter' && !e.isComposing) { e.preventDefault(); ui.pedit.draft = t.value; submitPe(); }
      else if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); ui.pedit = null; T.render(); }
      return;
    }
    if (e.key === 'Escape' && ui.pdel) { ui.pdel = null; T.render(); return; }
    if (t.getAttribute('data-act') !== 'itext') return;
    var key = t.getAttribute('data-key');
    if (e.key === 'Enter' && !e.isComposing) { e.preventDefault(); ui.inboxDraft[key] = t.value; submitAsk(key); }
    else if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); ui.inboxAsk[key] = false; T.render(); }
  });
  setInterval(tickCountdowns, 30000);
})();
